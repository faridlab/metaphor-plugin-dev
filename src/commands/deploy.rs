//! `metaphor deploy` — ship to a remote environment.
//!
//! Workflow (push):
//!   1. resolve env from metaphor.deploy.yaml (must have `host:`)
//!   2. compute tag (git short SHA by default, override with --tag)
//!   3. `docker buildx build --push` each image under env.images, tagging
//!      both `:{tag}` and `:{env}` (moving pointer)
//!   4. update `*_TAG=<sha>` entries in the local env file (unless --skip-env-update)
//!   5. scp env file to `host:deploy_dir/<env_file_name>`
//!   6. ssh host → `docker compose pull && docker compose up -d`
//!   7. tail logs briefly to confirm rollout
//!
//! Intentionally simple. Deliberately does not invent a bespoke orchestration
//! layer — defers to docker + ssh + compose, which the operator already knows.

use anyhow::{anyhow, bail, Context, Result};
use clap::Subcommand;
use colored::*;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::deploy_config::{self, EnvironmentSpec, ImageSpec, Resolved};

#[derive(Subcommand)]
pub enum DeployAction {
    /// Build, push, and roll out a new release.
    Push {
        /// Environment name from metaphor.deploy.yaml.
        env: String,

        /// Image tag to use (defaults to short git SHA).
        #[arg(long)]
        tag: Option<String>,

        /// Skip the `docker buildx build --push` step (images already in registry).
        #[arg(long)]
        skip_build: bool,

        /// Skip migrations after rollout.
        #[arg(long)]
        skip_migrate: bool,

        /// Don't update *_TAG values in the local env file.
        #[arg(long)]
        skip_env_update: bool,

        /// Print the commands that would run without executing them.
        #[arg(long)]
        dry_run: bool,

        /// Proceed without interactive confirmation (required for `require_confirm` envs).
        #[arg(long, short)]
        yes: bool,
    },

    /// Roll back to a previous tag already in the registry.
    Rollback {
        env: String,

        /// Tag to roll back to. Required — there is no implicit previous-tag memory.
        #[arg(long = "to")]
        to: String,

        #[arg(long, short)]
        yes: bool,
    },

    /// `docker compose ps` over SSH.
    Status { env: String },

    /// `docker compose logs` over SSH.
    Logs {
        env: String,

        #[arg(long = "service", value_name = "SERVICE")]
        service: Option<String>,

        #[arg(long, default_value = "200")]
        tail: String,

        #[arg(long, short)]
        follow: bool,
    },

    /// Run database migrations against the remote env over an SSH tunnel.
    Migrate {
        env: String,

        /// Print the tunnel + migrate commands without executing.
        #[arg(long)]
        dry_run: bool,
    },

    /// Delegate to the workspace's infra project (./deploy.sh or `make deploy`).
    ///
    /// Migrated from the legacy `metaphor deploy` core command. Use this when
    /// your repo is structured around an `infra` project that owns deployment
    /// scripts, rather than `metaphor.deploy.yaml`.
    Exec {
        /// Select a specific infra project when multiple are registered.
        #[arg(long)]
        infra: Option<String>,

        /// Extra arguments forwarded to deploy.sh / make deploy.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

pub async fn handle_command(action: &DeployAction) -> Result<()> {
    // `exec` is the legacy infra-project workflow and intentionally does not
    // require metaphor.deploy.yaml. Handle it before loading the deploy config.
    if let DeployAction::Exec { infra, args } = action {
        let cwd = std::env::current_dir().context("failed to read current directory")?;
        return exec_infra(&cwd, infra.as_deref(), args);
    }

    let resolved = Resolved::load()?;

    match action {
        DeployAction::Push {
            env,
            tag,
            skip_build,
            skip_migrate,
            skip_env_update,
            dry_run,
            yes,
        } => {
            let env_spec = require_remote(&resolved, env)?;
            let opts = PushOptions {
                tag: tag.clone(),
                skip_build: *skip_build,
                skip_migrate: *skip_migrate,
                skip_env_update: *skip_env_update,
                dry_run: *dry_run,
                yes: *yes,
            };
            push(&resolved, env, env_spec, &opts)
        }
        DeployAction::Rollback { env, to, yes } => {
            let env_spec = require_remote(&resolved, env)?;
            rollback(&resolved, env, env_spec, to, *yes)
        }
        DeployAction::Status { env } => {
            let env_spec = require_remote(&resolved, env)?;
            remote_compose(&resolved, env, env_spec, &["ps".into()])
        }
        DeployAction::Logs {
            env,
            service,
            tail,
            follow,
        } => {
            let env_spec = require_remote(&resolved, env)?;
            let mut args: Vec<String> = vec!["logs".into()];
            if *follow {
                args.push("-f".into());
            }
            args.push("--tail".into());
            args.push(tail.clone());
            if let Some(s) = service {
                args.push(s.clone());
            }
            remote_compose(&resolved, env, env_spec, &args)
        }
        DeployAction::Migrate { env, dry_run } => {
            let env_spec = require_remote(&resolved, env)?;
            migrate(&resolved, env, env_spec, *dry_run)
        }
        DeployAction::Exec { .. } => unreachable!("Exec is handled before this match"),
    }
}

fn require_remote<'a>(resolved: &'a Resolved, env: &str) -> Result<&'a EnvironmentSpec> {
    let spec = resolved.environment(env)?;
    if deploy_config::is_local(spec) {
        bail!(
            "environment '{}' is local (no host: set) — use `metaphor docker` instead",
            env
        );
    }
    Ok(spec)
}

// ────────────────────────────── push ──────────────────────────────

struct PushOptions {
    tag: Option<String>,
    skip_build: bool,
    skip_migrate: bool,
    skip_env_update: bool,
    dry_run: bool,
    yes: bool,
}

fn push(
    resolved: &Resolved,
    env_name: &str,
    env: &EnvironmentSpec,
    opts: &PushOptions,
) -> Result<()> {
    let tag = match &opts.tag {
        Some(t) => t.clone(),
        None => git_short_sha(&resolved.workspace_root)?,
    };

    print_header(env_name, env, &tag);

    if env.require_confirm && !opts.yes && !opts.dry_run {
        confirm(&format!(
            "About to deploy tag '{}' to '{}' ({}). Proceed?",
            tag,
            env_name,
            env.host.as_deref().unwrap_or("?")
        ))?;
    }

    // 1. Build & push images
    if opts.skip_build {
        println!("{} skipping build (per --skip-build)", "●".yellow());
    } else {
        for (name, image) in &env.images {
            build_and_push(resolved, env, image, name, &tag, opts.dry_run)?;
        }
    }

    // 2. Update env file tags locally
    if !opts.skip_env_update {
        let env_file = resolved.local_env_file(env, env_name);
        update_env_file_tags(&env_file, &env.images, &tag, opts.dry_run)?;
    }

    // 3. scp env file to remote
    scp_env_file(resolved, env, env_name, opts.dry_run)?;

    // 4. Remote compose pull + up
    remote_compose_action(resolved, env_name, env, "pull", &[], opts.dry_run)?;
    remote_compose_action(resolved, env_name, env, "up", &["-d".into()], opts.dry_run)?;

    // 5. Optional migrations
    if !opts.skip_migrate {
        migrate(resolved, env_name, env, opts.dry_run)?;
    } else {
        println!("{} skipping migrations (per --skip-migrate)", "●".yellow());
    }

    println!("\n{} deployed tag {} to {}", "✓".green().bold(), tag.bright_cyan(), env_name.bright_cyan());
    Ok(())
}

fn build_and_push(
    resolved: &Resolved,
    env: &EnvironmentSpec,
    image: &ImageSpec,
    image_key: &str,
    tag: &str,
    dry_run: bool,
) -> Result<()> {
    let context = resolved.workspace_root.join(&image.context);
    if !context.is_dir() {
        bail!(
            "image '{}': context {} is not a directory",
            image_key,
            context.display()
        );
    }

    let registry = image
        .registry
        .clone()
        .or_else(|| resolved.registry(env))
        .ok_or_else(|| {
            anyhow!(
                "image '{}': no registry configured (set registry on image, env, or defaults)",
                image_key
            )
        })?;

    let name = image.name.clone().unwrap_or_else(|| image_key.to_string());
    let sha_tag = format!("{registry}/{name}:{tag}");

    let push = image.push.unwrap_or(true);
    let mut args: Vec<String> = vec![
        "buildx".into(),
        "build".into(),
        "--platform".into(),
        "linux/amd64".into(),
        "-t".into(),
        sha_tag.clone(),
    ];
    if let Some(dockerfile) = &image.dockerfile {
        args.push("-f".into());
        args.push(dockerfile.clone());
    }
    for (k, v) in &image.build_args {
        args.push("--build-arg".into());
        args.push(format!("{k}={v}"));
    }
    if push {
        args.push("--push".into());
    } else {
        args.push("--load".into());
    }
    args.push(".".into());

    println!(
        "{} {} → {}",
        "build".bright_blue().bold(),
        image_key,
        sha_tag
    );
    run_in(&args, &context, dry_run, "docker")
}

fn update_env_file_tags(
    env_file: &Path,
    images: &std::collections::BTreeMap<String, ImageSpec>,
    tag: &str,
    dry_run: bool,
) -> Result<()> {
    if !env_file.is_file() {
        eprintln!(
            "{} env file {} does not exist; creating a minimal one",
            "warning:".yellow().bold(),
            env_file.display()
        );
        if !dry_run {
            if let Some(parent) = env_file.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(env_file, "").with_context(|| format!("writing {}", env_file.display()))?;
        }
    }

    let mut content = std::fs::read_to_string(env_file)
        .with_context(|| format!("reading {}", env_file.display()))?;

    for image in images.values() {
        let Some(var) = &image.tag_env else { continue };
        content = replace_or_append_kv(&content, var, tag);
    }

    println!(
        "{} update tags in {}",
        "env".bright_blue().bold(),
        env_file.display()
    );

    if dry_run {
        println!("{} (dry-run: not writing)", "  ●".bright_black());
        return Ok(());
    }
    std::fs::write(env_file, content).with_context(|| format!("writing {}", env_file.display()))?;
    Ok(())
}

fn replace_or_append_kv(content: &str, key: &str, value: &str) -> String {
    let new_line = format!("{key}={value}");
    let mut found = false;
    let mut out = String::with_capacity(content.len() + new_line.len());
    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(&format!("{key}=")) {
            let _ = rest; // ignore old value
            out.push_str(&new_line);
            out.push('\n');
            found = true;
        } else if trimmed.starts_with(&format!("{key} =")) {
            out.push_str(&new_line);
            out.push('\n');
            found = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !found {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&new_line);
        out.push('\n');
    }
    out
}

fn scp_env_file(
    resolved: &Resolved,
    env: &EnvironmentSpec,
    env_name: &str,
    dry_run: bool,
) -> Result<()> {
    let local = resolved.local_env_file(env, env_name);
    if !local.is_file() {
        bail!("local env file not found at {}", local.display());
    }
    let deploy_dir = resolved.deploy_dir(env)?;
    let remote_rel = resolved.remote_env_file(env, env_name);
    let ssh_host = ssh_target(resolved, env)?;

    let dest = format!("{ssh_host}:{deploy_dir}/{remote_rel}");
    println!("{} scp {} → {}", "env".bright_blue().bold(), local.display(), dest);

    let args: Vec<String> = vec![local.display().to_string(), dest];
    run_in(&args, Path::new("."), dry_run, "scp")
}

fn remote_compose_action(
    resolved: &Resolved,
    env_name: &str,
    env: &EnvironmentSpec,
    subcmd: &str,
    extra: &[String],
    dry_run: bool,
) -> Result<()> {
    let mut args: Vec<String> = vec![subcmd.to_string()];
    args.extend(extra.iter().cloned());

    println!(
        "{} docker compose {} on {}",
        "remote".bright_blue().bold(),
        args.join(" "),
        env_name
    );

    if dry_run {
        let cmd = build_remote_compose_cmd(resolved, env, &args)?;
        println!("  {} {}", "→".bright_black(), cmd);
        return Ok(());
    }
    remote_compose(resolved, env_name, env, &args)
}

fn remote_compose(
    resolved: &Resolved,
    _env_name: &str,
    env: &EnvironmentSpec,
    args: &[String],
) -> Result<()> {
    let cmd = build_remote_compose_cmd(resolved, env, args)?;
    let ssh_host = ssh_target(resolved, env)?;
    let ssh_args: Vec<String> = vec![ssh_host, cmd];
    run_in(&ssh_args, Path::new("."), false, "ssh")
}

fn build_remote_compose_cmd(
    resolved: &Resolved,
    env: &EnvironmentSpec,
    args: &[String],
) -> Result<String> {
    let deploy_dir = resolved.deploy_dir(env)?;
    let compose_file = resolved.remote_compose_file(env);
    let env_file = resolved.remote_env_file(env, default_env_name(resolved, env));

    let tail = args
        .iter()
        .map(|a| shell_quote(a))
        .collect::<Vec<_>>()
        .join(" ");

    Ok(format!(
        "cd {deploy_dir} && docker compose -f {compose_file} --env-file {env_file} {tail}"
    ))
}

fn default_env_name<'a>(resolved: &'a Resolved, env: &'a EnvironmentSpec) -> &'a str {
    // Reverse lookup: we have a reference to the env, find its name.
    for (name, spec) in &resolved.manifest.environments {
        if std::ptr::eq(spec, env) {
            return name;
        }
    }
    "env"
}

// ────────────────────────────── rollback ──────────────────────────────

fn rollback(
    resolved: &Resolved,
    env_name: &str,
    env: &EnvironmentSpec,
    to: &str,
    yes: bool,
) -> Result<()> {
    if env.require_confirm && !yes {
        confirm(&format!(
            "Roll back '{}' ({}) to tag '{}'?",
            env_name,
            env.host.as_deref().unwrap_or("?"),
            to
        ))?;
    }

    let env_file = resolved.local_env_file(env, env_name);
    update_env_file_tags(&env_file, &env.images, to, false)?;
    scp_env_file(resolved, env, env_name, false)?;
    remote_compose_action(resolved, env_name, env, "pull", &[], false)?;
    remote_compose_action(resolved, env_name, env, "up", &["-d".into()], false)?;

    println!(
        "\n{} rolled '{}' back to {}",
        "✓".green().bold(),
        env_name.bright_cyan(),
        to.bright_cyan()
    );
    Ok(())
}

// ────────────────────────────── migrate ──────────────────────────────

fn migrate(
    resolved: &Resolved,
    env_name: &str,
    env: &EnvironmentSpec,
    dry_run: bool,
) -> Result<()> {
    let cmd = resolved
        .manifest
        .defaults
        .migrate_command
        .clone()
        .unwrap_or_else(|| "metaphor migration run-all".to_string());

    // For now we run the migration command directly against the remote compose
    // stack by execing it through the service container. Rationale: SSH tunnels
    // to the DB are operator-specific (port choice, user, DSN shape) and adding
    // that complexity here is premature. Users who need tunnel-based migrations
    // can override migrate_command with e.g. a local wrapper script.
    let service = "migrations";
    let args = vec![
        "run".into(),
        "--rm".into(),
        service.into(),
        "sh".into(),
        "-lc".into(),
        cmd.clone(),
    ];
    println!(
        "{} docker compose run --rm {} '{}' on {}",
        "migrate".bright_blue().bold(),
        service,
        cmd,
        env_name
    );
    remote_compose_action(resolved, env_name, env, "run", &args, dry_run)
}

// ────────────────────────────── helpers ──────────────────────────────

fn ssh_target(resolved: &Resolved, env: &EnvironmentSpec) -> Result<String> {
    let host = env
        .host
        .as_ref()
        .ok_or_else(|| anyhow!("environment has no host"))?;
    let user = resolved.ssh_user(env);
    Ok(match user {
        Some(u) => format!("{u}@{host}"),
        None => host.clone(),
    })
}

fn run_in(args: &[String], cwd: &Path, dry_run: bool, bin: &str) -> Result<()> {
    eprintln!(
        "{} {} {}",
        "→".bright_black(),
        bin,
        args.join(" ").bright_black()
    );
    if dry_run {
        return Ok(());
    }
    let status = Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("spawning `{bin}`"))?;
    if !status.success() {
        bail!("{bin} exited with {status}");
    }
    Ok(())
}

fn git_short_sha(workspace_root: &Path) -> Result<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(workspace_root)
        .output()
        .context("running `git rev-parse` for tag (pass --tag to skip git lookup)")?;
    if !out.status.success() {
        bail!(
            "git rev-parse failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn confirm(prompt: &str) -> Result<()> {
    eprint!("{} [y/N] ", prompt.bright_yellow().bold());
    std::io::stderr().flush().ok();
    let mut buf = String::new();
    std::io::stdin()
        .read_line(&mut buf)
        .context("reading confirmation")?;
    let answer = buf.trim().to_lowercase();
    if answer != "y" && answer != "yes" {
        bail!("aborted by user");
    }
    Ok(())
}

fn shell_quote(s: &str) -> String {
    // Quote for POSIX sh. Wraps in single quotes and escapes internal quotes.
    if s.is_empty() {
        return "''".into();
    }
    if s.chars().all(|c| c.is_ascii_alphanumeric() || "-_=./:".contains(c)) {
        return s.into();
    }
    let escaped = s.replace('\'', r"'\''");
    format!("'{escaped}'")
}

fn print_header(env_name: &str, env: &EnvironmentSpec, tag: &str) {
    println!(
        "{} {}",
        "Environment:".bright_black(),
        env_name.bright_cyan().bold()
    );
    println!(
        "{} {}",
        "Host:       ".bright_black(),
        env.host.as_deref().unwrap_or("?")
    );
    println!("{} {}", "Tag:        ".bright_black(), tag.bright_cyan());
    println!();
}

#[allow(dead_code)]
fn _unused(_: PathBuf) {} // keep PathBuf import if future edits need it

// ────────────────────────────── exec (legacy infra-project) ──────────────────────────────

/// Walk up from `start` until a `metaphor.yaml` is found, parse the project
/// table, locate the `infra` project, and run its `./deploy.sh` (or
/// `make deploy` as a fallback). Forwarding `args` verbatim. Migrated from the
/// core CLI's old `cmd_deploy.rs` so all deploy-shaped verbs live here.
fn exec_infra(start: &Path, infra: Option<&str>, args: &[String]) -> Result<()> {
    let (workspace_root, projects) = load_metaphor_yaml(start)?;
    let project = pick_infra(&projects, infra)?;

    let dir = if std::path::Path::new(&project.path).is_absolute() {
        PathBuf::from(&project.path)
    } else {
        workspace_root.join(&project.path)
    };
    if !dir.is_dir() {
        bail!(
            "infra project '{}' not found on disk at {}",
            project.name,
            dir.display()
        );
    }

    let script = dir.join("deploy.sh");
    let makefile = dir.join("Makefile");

    let (label, status) = if is_executable(&script) {
        let mut cmd = Command::new(&script);
        cmd.current_dir(&dir).args(args);
        ("./deploy.sh", cmd.status())
    } else if makefile.exists() {
        let mut cmd = Command::new("make");
        cmd.current_dir(&dir).arg("deploy").args(args);
        ("make deploy", cmd.status())
    } else {
        bail!(
            "infra project '{}' has no deploy.sh or Makefile; add one and try again",
            project.name
        );
    };

    let status = status.with_context(|| format!("spawning {label}"))?;
    if !status.success() {
        bail!("{label} exited with status: {status}");
    }
    Ok(())
}

/// Minimal metaphor.yaml parser — only the fields `exec` needs. Avoids pulling
/// in metaphor-workspace as a dep (plugin-dev ships independently of the core
/// CLI's internal crates).
#[derive(serde::Deserialize)]
struct MetaphorYamlMin {
    #[serde(default)]
    projects: Vec<ProjectEntryMin>,
}

#[derive(Debug, serde::Deserialize, Clone)]
struct ProjectEntryMin {
    name: String,
    #[serde(default, rename = "type")]
    project_type: String,
    path: String,
}

fn load_metaphor_yaml(start: &Path) -> Result<(PathBuf, Vec<ProjectEntryMin>)> {
    let yaml_path = find_metaphor_yaml(start).ok_or_else(|| {
        anyhow!(
            "no metaphor.yaml found above {}; deploy exec needs a workspace",
            start.display()
        )
    })?;
    let workspace_root = yaml_path
        .parent()
        .ok_or_else(|| anyhow!("metaphor.yaml has no parent dir"))?
        .to_path_buf();
    let raw = std::fs::read_to_string(&yaml_path)
        .with_context(|| format!("failed to read {}", yaml_path.display()))?;
    let parsed: MetaphorYamlMin = serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse {}", yaml_path.display()))?;
    Ok((workspace_root, parsed.projects))
}

fn find_metaphor_yaml(start: &Path) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let candidate = dir.join("metaphor.yaml");
        if candidate.is_file() {
            return Some(candidate);
        }
        cur = dir.parent();
    }
    None
}

fn pick_infra<'a>(
    projects: &'a [ProjectEntryMin],
    name: Option<&str>,
) -> Result<&'a ProjectEntryMin> {
    if let Some(name) = name {
        let p = projects
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow!("project '{}' not found in metaphor.yaml", name))?;
        if p.project_type != "infra" {
            bail!("project '{}' is type '{}', not 'infra'", p.name, p.project_type);
        }
        return Ok(p);
    }
    let infras: Vec<&ProjectEntryMin> = projects.iter().filter(|p| p.project_type == "infra").collect();
    match infras.len() {
        0 => bail!("no project with type: infra in this workspace"),
        1 => Ok(infras[0]),
        n => bail!(
            "{n} infra projects registered ({}); disambiguate with --infra=<name>",
            infras.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")
        ),
    }
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(p) {
        Ok(md) => md.is_file() && md.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn is_executable(p: &Path) -> bool {
    std::fs::metadata(p).map(|md| md.is_file()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_existing_key() {
        let s = "FOO=old\nBAR=keep\n";
        let out = replace_or_append_kv(s, "FOO", "new");
        assert_eq!(out, "FOO=new\nBAR=keep\n");
    }

    #[test]
    fn appends_missing_key() {
        let s = "BAR=keep\n";
        let out = replace_or_append_kv(s, "FOO", "new");
        assert_eq!(out, "BAR=keep\nFOO=new\n");
    }

    #[test]
    fn handles_missing_trailing_newline() {
        let s = "BAR=keep";
        let out = replace_or_append_kv(s, "FOO", "new");
        assert!(out.ends_with("FOO=new\n"));
    }

    #[test]
    fn shell_quote_safe_chars() {
        assert_eq!(shell_quote("abc-123"), "abc-123");
        assert_eq!(shell_quote("./path/to:file"), "./path/to:file");
    }

    #[test]
    fn shell_quote_unsafe_chars() {
        assert_eq!(shell_quote("hello world"), "'hello world'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    fn proj(name: &str, t: &str) -> ProjectEntryMin {
        ProjectEntryMin {
            name: name.into(),
            project_type: t.into(),
            path: format!("./{name}"),
        }
    }

    #[test]
    fn pick_infra_sole() {
        let ps = vec![proj("api", "backend-service"), proj("infra", "infra")];
        let p = pick_infra(&ps, None).unwrap();
        assert_eq!(p.name, "infra");
    }

    #[test]
    fn pick_infra_none_errors() {
        let ps = vec![proj("api", "backend-service")];
        let e = pick_infra(&ps, None).unwrap_err().to_string();
        assert!(e.contains("no project with type: infra"));
    }

    #[test]
    fn pick_infra_multiple_requires_disambiguation() {
        let ps = vec![proj("infra-staging", "infra"), proj("infra-prod", "infra")];
        let e = pick_infra(&ps, None).unwrap_err().to_string();
        assert!(e.contains("--infra="));
        let p = pick_infra(&ps, Some("infra-prod")).unwrap();
        assert_eq!(p.name, "infra-prod");
    }

    #[test]
    fn pick_infra_rejects_non_infra_name() {
        let ps = vec![proj("api", "backend-service"), proj("infra", "infra")];
        let e = pick_infra(&ps, Some("api")).unwrap_err().to_string();
        assert!(e.contains("not 'infra'"));
    }
}
