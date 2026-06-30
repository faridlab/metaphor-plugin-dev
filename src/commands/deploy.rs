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
use std::collections::BTreeMap;

use super::deploy_history::{self, HistoryRecord};

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
    ///
    /// With no flags, rolls back to the previous successful push recorded in
    /// `deployment/history/<env>.jsonl`. Failed pushes and rollbacks are skipped
    /// so `--steps 1` always means "the deploy that was deployed before this one".
    Rollback {
        env: String,

        /// Roll back to this exact tag. Bypasses history.
        #[arg(long = "to", conflicts_with = "steps")]
        to: Option<String>,

        /// Number of successful pushes to step back from current. Default: 1.
        #[arg(long, conflicts_with = "to", default_value = "1")]
        steps: usize,

        #[arg(long, short)]
        yes: bool,
    },

    /// Show deployment history for an environment.
    History {
        env: String,

        /// Maximum number of records to show.
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Read history from the remote host instead of the local workspace.
        #[arg(long)]
        remote: bool,

        #[arg(long)]
        json: bool,
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

    /// Deploy ONE pre-built service from the registry.
    ///
    /// Bumps the service's `*_TAG` in the env file, pulls + `up -d` only that
    /// service on the remote, then shows its status. No build, no migrate —
    /// the image must already be in the registry (e.g. just pushed by CI).
    /// Records the deploy in history, unlike the old `deploy-service.sh`.
    Service {
        /// Environment name from metaphor.deploy.yaml.
        env: String,

        /// Service to deploy (a compose/image key under the env's `images`).
        service: String,

        /// Image tag already present in the registry (e.g. v0.1.2).
        tag: String,

        /// Print the commands that would run without executing them.
        #[arg(long)]
        dry_run: bool,

        /// Proceed without interactive confirmation (required for `require_confirm` envs).
        #[arg(long, short)]
        yes: bool,
    },

    /// Bump a service's `*_TAG` in the LOCAL env file only — no build, no SSH,
    /// no deploy. Pairs with a later `deploy service`/`push`; stages the change
    /// so you can review the diff and commit before deploying.
    Bump {
        /// Environment name from metaphor.deploy.yaml.
        env: String,

        /// Service whose tag var to bump (a key under the env's `images`).
        #[arg(long = "service", value_name = "SERVICE")]
        service: String,

        /// New tag value (e.g. v0.1.2).
        #[arg(long)]
        tag: String,
    },

    /// Validate local prod env files before a push (no SSH).
    ///
    /// 1. Per-service contract: every `^[A-Z_]+=` var in each image's
    ///    `<context>/.env.prod.example` must be present, non-empty and
    ///    non-placeholder in `<context>/.env.prod`.
    /// 2. Compose interpolation: `docker compose -f <compose> --env-file <env>
    ///    config` must resolve every `${VAR:?}` reference.
    Preflight {
        /// Environment name from metaphor.deploy.yaml.
        env: String,
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
        DeployAction::Rollback { env, to, steps, yes } => {
            let env_spec = require_remote(&resolved, env)?;
            let target = match to {
                Some(t) => RollbackTarget::Tag(t.clone()),
                None => RollbackTarget::Steps(*steps),
            };
            rollback(&resolved, env, env_spec, target, *yes)
        }
        DeployAction::History {
            env,
            limit,
            remote,
            json,
        } => history(&resolved, env, *limit, *remote, *json),
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
        DeployAction::Service {
            env,
            service,
            tag,
            dry_run,
            yes,
        } => {
            let env_spec = require_remote(&resolved, env)?;
            deploy_service(&resolved, env, env_spec, service, tag, *dry_run, *yes)
        }
        DeployAction::Bump { env, service, tag } => {
            let env_spec = resolved.environment(env)?;
            bump(&resolved, env, env_spec, service, tag)
        }
        DeployAction::Preflight { env } => {
            let env_spec = resolved.environment(env)?;
            preflight(&resolved, env, env_spec)
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

    let inner = push_inner(resolved, env_name, env, opts, &tag);

    // History epilogue (skipped on dry-run; that's a preview, not a real deploy).
    if !opts.dry_run {
        let image_tags = uniform_image_tags(env, &tag);
        let record = match &inner {
            Ok(snapshot) => HistoryRecord::new_push(tag.clone(), image_tags, snapshot.clone()),
            Err(e) => HistoryRecord::new_push(tag.clone(), image_tags, None)
                .with_failure(&e.to_string()),
        };
        if let Err(e) = deploy_history::append_record(&resolved.workspace_root, env_name, &record) {
            eprintln!("{} failed to record history: {e}", "warning:".yellow().bold());
        }
        if matches!(record.status, deploy_history::Status::Success) {
            mirror_history_to_remote(resolved, env_name, env, &record);
        }
    }

    inner.map(|_| {
        println!(
            "\n{} deployed tag {} to {}",
            "✓".green().bold(),
            tag.bright_cyan(),
            env_name.bright_cyan()
        );
    })
}

/// The actual push body. Returns `Ok(Some(snapshot_basename))` on success when
/// a snapshot was written, `Ok(None)` for a successful push that didn't write
/// one (dry-run, `--skip-env-update`).
fn push_inner(
    resolved: &Resolved,
    env_name: &str,
    env: &EnvironmentSpec,
    opts: &PushOptions,
    tag: &str,
) -> Result<Option<String>> {
    // 1. Build & push images
    if opts.skip_build {
        println!("{} skipping build (per --skip-build)", "●".yellow());
    } else {
        for (name, image) in &env.images {
            build_and_push(resolved, env, image, name, tag, opts.dry_run)?;
        }
    }

    // 2. Update env file tags locally
    if !opts.skip_env_update {
        let env_file = resolved.local_env_file(env, env_name);
        update_env_file_tags(&env_file, &env.images, tag, opts.dry_run)?;
    }

    // 3. Snapshot the env file (after tag update) for audit/rollback.
    let snapshot = if opts.dry_run || opts.skip_env_update {
        None
    } else {
        snapshot_env_file(resolved, env, env_name, tag).ok()
    };

    // 4. scp env file to remote
    scp_env_file(resolved, env, env_name, opts.dry_run)?;

    // 5. Remote compose pull + up
    remote_compose_action(resolved, env_name, env, "pull", &[], opts.dry_run)?;
    remote_compose_action(resolved, env_name, env, "up", &["-d".into()], opts.dry_run)?;

    // 6. Optional migrations
    if !opts.skip_migrate {
        migrate(resolved, env_name, env, opts.dry_run)?;
    } else {
        println!("{} skipping migrations (per --skip-migrate)", "●".yellow());
    }

    Ok(snapshot)
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

enum RollbackTarget {
    Steps(usize),
    Tag(String),
}

fn rollback(
    resolved: &Resolved,
    env_name: &str,
    env: &EnvironmentSpec,
    target: RollbackTarget,
    yes: bool,
) -> Result<()> {
    let records = deploy_history::read_records(&resolved.workspace_root, env_name)
        .unwrap_or_default();
    let from_tag =
        deploy_history::current_deployed_tag(&records).unwrap_or_else(|| "(unknown)".to_string());

    let (to_tag, image_tags, source_label) = match &target {
        RollbackTarget::Steps(n) => {
            if records.is_empty() {
                bail!(
                    "no deployment/history/{}.jsonl yet — nothing to roll back to. Pass --to <sha> for an explicit rollback.",
                    env_name
                );
            }
            // Step 0 is current deploy; step 1 is the one before. The user passes the step
            // count; we look up `n` (so --steps 1 → records[1] in success-only view).
            let rec = deploy_history::find_previous_successful_push(&records, *n)
                .ok_or_else(|| {
                    anyhow!(
                        "no successful push {} step(s) back in deployment/history/{}.jsonl",
                        n,
                        env_name
                    )
                })?;
            if rec.tag == from_tag {
                bail!(
                    "step {} resolves to the currently-deployed tag '{}'; nothing to do",
                    n,
                    rec.tag
                );
            }
            (rec.tag.clone(), rec.image_tags.clone(), format!("{n} step(s) back"))
        }
        RollbackTarget::Tag(tag) => (
            tag.clone(),
            uniform_image_tags(env, tag),
            format!("tag {tag}"),
        ),
    };

    if env.require_confirm && !yes {
        confirm(&format!(
            "Roll back '{}' ({}) from '{}' to '{}' ({})?",
            env_name,
            env.host.as_deref().unwrap_or("?"),
            from_tag,
            to_tag,
            source_label
        ))?;
    }

    let env_file = resolved.local_env_file(env, env_name);
    update_env_file_tags_from_map(&env_file, &env.images, &image_tags, &to_tag, false)?;
    scp_env_file(resolved, env, env_name, false)?;
    remote_compose_action(resolved, env_name, env, "pull", &[], false)?;
    remote_compose_action(resolved, env_name, env, "up", &["-d".into()], false)?;

    let snapshot = snapshot_env_file(resolved, env, env_name, &to_tag).ok();
    let record = HistoryRecord::new_rollback(from_tag, to_tag.clone(), image_tags, snapshot);
    if let Err(e) = deploy_history::append_record(&resolved.workspace_root, env_name, &record) {
        eprintln!("{} failed to record history: {e}", "warning:".yellow().bold());
    }
    mirror_history_to_remote(resolved, env_name, env, &record);

    println!(
        "\n{} rolled '{}' back to {} ({})",
        "✓".green().bold(),
        env_name.bright_cyan(),
        to_tag.bright_cyan(),
        source_label
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

// ───────────────────── single-service deploy / bump / preflight ─────────────────────

/// Return the subset of `env.images` selected by `services`.
/// Empty selection = all images. Errors if a requested name is not an image key.
fn select_images<'a>(
    env: &'a EnvironmentSpec,
    services: &[String],
) -> Result<BTreeMap<&'a String, &'a ImageSpec>> {
    if services.is_empty() {
        return Ok(env.images.iter().collect());
    }
    let mut out = BTreeMap::new();
    for s in services {
        let (key, spec) = env.images.get_key_value(s).ok_or_else(|| {
            let avail: Vec<&str> = env.images.keys().map(|k| k.as_str()).collect();
            anyhow!(
                "service '{}' not found in env images; available: {}",
                s,
                avail.join(", ")
            )
        })?;
        out.insert(key, spec);
    }
    Ok(out)
}

/// Like `update_env_file_tags`, but only for the selected images, and STRICT:
/// errors if a named image has no `tag_env`. (A full-sweep push silently skips
/// those, but an explicit selection means the operator asked for this service.)
fn update_env_file_tags_selected(
    env_file: &Path,
    selected: &BTreeMap<&String, &ImageSpec>,
    tag: &str,
    dry_run: bool,
) -> Result<()> {
    if !env_file.is_file() {
        bail!("local env file not found at {}", env_file.display());
    }
    let mut content = std::fs::read_to_string(env_file)
        .with_context(|| format!("reading {}", env_file.display()))?;

    for (name, image) in selected {
        let var = image.tag_env.as_deref().ok_or_else(|| {
            anyhow!(
                "service '{}' has no tag_env in metaphor.deploy.yaml; cannot set its tag",
                name
            )
        })?;
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

/// Deploy a single pre-built service: bump its tag, scp the env file, then
/// pull + `up -d` + `ps` only that service on the remote. Records history.
#[allow(clippy::too_many_arguments)]
fn deploy_service(
    resolved: &Resolved,
    env_name: &str,
    env: &EnvironmentSpec,
    service: &str,
    tag: &str,
    dry_run: bool,
    yes: bool,
) -> Result<()> {
    let svc = vec![service.to_string()];
    let selected = select_images(env, &svc)?; // validates the name exists

    print_header(env_name, env, tag);
    println!(
        "{} single service: {}",
        "●".bright_blue(),
        service.bright_cyan()
    );

    if env.require_confirm && !yes && !dry_run {
        confirm(&format!(
            "About to deploy service '{}' at tag '{}' to '{}' ({}). Proceed?",
            service,
            tag,
            env_name,
            env.host.as_deref().unwrap_or("?")
        ))?;
    }

    let inner = deploy_service_inner(resolved, env_name, env, &selected, service, tag, dry_run);

    // History epilogue (skipped on dry-run; that's a preview, not a real deploy).
    if !dry_run {
        let image_tags: BTreeMap<String, String> =
            std::iter::once((service.to_string(), tag.to_string())).collect();
        let record = match &inner {
            Ok(snapshot) => HistoryRecord::new_push(tag.to_string(), image_tags, snapshot.clone()),
            Err(e) => {
                HistoryRecord::new_push(tag.to_string(), image_tags, None).with_failure(&e.to_string())
            }
        };
        if let Err(e) = deploy_history::append_record(&resolved.workspace_root, env_name, &record) {
            eprintln!("{} failed to record history: {e}", "warning:".yellow().bold());
        }
        if matches!(record.status, deploy_history::Status::Success) {
            mirror_history_to_remote(resolved, env_name, env, &record);
        }
    }

    inner.map(|_| {
        println!(
            "\n{} deployed service {} at tag {} to {}",
            "✓".green().bold(),
            service.bright_cyan(),
            tag.bright_cyan(),
            env_name.bright_cyan()
        );
    })
}

fn deploy_service_inner(
    resolved: &Resolved,
    env_name: &str,
    env: &EnvironmentSpec,
    selected: &BTreeMap<&String, &ImageSpec>,
    service: &str,
    tag: &str,
    dry_run: bool,
) -> Result<Option<String>> {
    // 1. Bump just this service's *_TAG in the local env file.
    let env_file = resolved.local_env_file(env, env_name);
    update_env_file_tags_selected(&env_file, selected, tag, dry_run)?;

    // 2. Snapshot for audit/rollback (skip on dry-run).
    let snapshot = if dry_run {
        None
    } else {
        snapshot_env_file(resolved, env, env_name, tag).ok()
    };

    // 3. scp env file to remote.
    scp_env_file(resolved, env, env_name, dry_run)?;

    // 4. Pull + up + ps — only this service.
    let svc = vec![service.to_string()];
    remote_compose_action(resolved, env_name, env, "pull", &svc, dry_run)?;
    let up_args = vec!["-d".to_string(), service.to_string()];
    remote_compose_action(resolved, env_name, env, "up", &up_args, dry_run)?;
    remote_compose_action(resolved, env_name, env, "ps", &svc, dry_run)?;

    Ok(snapshot)
}

/// Bump a service's `*_TAG` in the LOCAL env file only. No SSH, no deploy.
fn bump(
    resolved: &Resolved,
    env_name: &str,
    env: &EnvironmentSpec,
    service: &str,
    tag: &str,
) -> Result<()> {
    let svc = vec![service.to_string()];
    let selected = select_images(env, &svc)?;
    let image = selected.values().next().expect("exactly one image selected");
    let var = image.tag_env.as_deref().ok_or_else(|| {
        anyhow!(
            "service '{}' has no tag_env in metaphor.deploy.yaml; cannot bump its tag",
            service
        )
    })?;

    let env_file = resolved.local_env_file(env, env_name);

    // No-op detection (matches bump-prod-tag.sh): if already at this tag, stop.
    if let Some(current) = read_env_var(&env_file, var) {
        if current == tag {
            println!("{} {} already at {}. No change.", "●".yellow(), var, tag.bright_cyan());
            return Ok(());
        }
    }

    update_env_file_tags_selected(&env_file, &selected, tag, false)?;
    println!(
        "\n{} {} → {} in {}",
        "✓".green().bold(),
        var,
        tag.bright_cyan(),
        env_file.display()
    );
    println!(
        "  Next: review the diff, commit, then `metaphor deploy service {env_name} {service} {tag}`"
    );
    Ok(())
}

/// Read the first `KEY=VALUE` occurrence's value from an env file, if present.
fn read_env_var(env_file: &Path, key: &str) -> Option<String> {
    let content = std::fs::read_to_string(env_file).ok()?;
    parse_env_values(&content).get(key).cloned()
}

// ────────────────────────────── preflight ──────────────────────────────

const PLACEHOLDER_RE: &str = r"^(CHANGE_ME|TODO|REPLACE_ME|FILL_|XXX+)|<[^>]+>";

/// Validate local prod env files before a push (local-only, no SSH).
fn preflight(resolved: &Resolved, env_name: &str, env: &EnvironmentSpec) -> Result<()> {
    println!(
        "{} preflight {} ({})",
        "▶".bright_blue().bold(),
        env_name.bright_cyan(),
        "validating local env files".bright_black()
    );

    let mut failed = false;

    // Layer 1: per-service contract check. Auto-discovers any image whose
    // `<context>/.env.prod.example` exists; skips those without one (webapps).
    for (name, image) in &env.images {
        let ctx = resolved.workspace_root.join(&image.context);
        let contract = ctx.join(".env.prod.example");
        let actual = ctx.join(".env.prod");
        if !contract.is_file() {
            continue;
        }
        if !actual.is_file() {
            eprintln!(
                "  {} {name}: runtime env file missing: {}",
                "✗".red().bold(),
                actual.display()
            );
            failed = true;
            continue;
        }
        let req = required_vars(&std::fs::read_to_string(&contract)?);
        let have = parse_env_values(&std::fs::read_to_string(&actual)?);
        let (missing, placeholders) = check_contract(&req, &have);
        if missing.is_empty() && placeholders.is_empty() {
            println!(
                "  {} {name}: all {} contract vars present",
                "✓".green().bold(),
                req.len()
            );
        } else {
            for v in &missing {
                eprintln!("      {} [missing]     {v}", "✗".red());
            }
            for v in &placeholders {
                eprintln!("      {} [placeholder] {v}", "✗".red());
            }
            failed = true;
        }
    }
    if failed {
        bail!("preflight contract check failed");
    }

    // Layer 2: compose interpolation check (`docker compose config`).
    let compose = resolved.local_compose_file(env);
    let env_file = resolved.local_env_file(env, env_name);
    let out = Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg(&compose)
        .arg("--env-file")
        .arg(&env_file)
        .arg("config")
        .current_dir(&resolved.workspace_root)
        .output()
        .context("running `docker compose config`")?;
    if !out.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
        bail!(
            "compose validation failed: unresolved ${{VAR:?}} refs in {}",
            env_file.display()
        );
    }
    println!(
        "  {} all compose-interpolated vars resolve in {}",
        "✓".green().bold(),
        env_file.display()
    );
    println!("\n{} preflight passed", "✓".green().bold());
    Ok(())
}

/// Extract `^[A-Z_]+=` keys from a contract (`.env.prod.example`) file.
fn required_vars(contract: &str) -> Vec<String> {
    contract
        .lines()
        .filter_map(|l| {
            let l = l.trim_start();
            let key = l.split('=').next().unwrap_or("");
            if !key.is_empty()
                && key.bytes().all(|b| b.is_ascii_uppercase() || b == b'_')
                && l[key.len()..].starts_with('=')
            {
                Some(key.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Parse `KEY=VALUE` lines into a map (first occurrence wins, like `head -1`).
fn parse_env_values(actual: &str) -> std::collections::HashMap<String, String> {
    let mut m = std::collections::HashMap::new();
    for line in actual.lines() {
        let t = line.trim_start();
        if t.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            m.entry(k.trim().to_string()).or_insert_with(|| v.to_string());
        }
    }
    m
}

/// Returns (missing, placeholders) for required vars against parsed values.
fn check_contract(
    req: &[String],
    have: &std::collections::HashMap<String, String>,
) -> (Vec<String>, Vec<String>) {
    let re = regex::Regex::new(PLACEHOLDER_RE).expect("static placeholder regex");
    let (mut missing, mut placeholders) = (vec![], vec![]);
    for var in req {
        match have.get(var) {
            None => missing.push(var.clone()),
            Some(v) if v.is_empty() => missing.push(var.clone()),
            Some(v) if re.is_match(v) => placeholders.push(format!("{var}={v}")),
            _ => {}
        }
    }
    (missing, placeholders)
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

// ────────────────────────────── history support ──────────────────────────────

/// Build the per-image tag map for a uniform-tag deploy (every image gets the same tag).
fn uniform_image_tags(env: &EnvironmentSpec, tag: &str) -> BTreeMap<String, String> {
    env.images
        .keys()
        .map(|k| (k.clone(), tag.to_string()))
        .collect()
}

/// Read the env file currently on disk and write a snapshot copy under
/// `deployment/history/snapshots/`. Returns the snapshot basename.
fn snapshot_env_file(
    resolved: &Resolved,
    env: &EnvironmentSpec,
    env_name: &str,
    tag: &str,
) -> Result<String> {
    let env_file = resolved.local_env_file(env, env_name);
    let content = std::fs::read_to_string(&env_file)
        .with_context(|| format!("reading {} for snapshot", env_file.display()))?;
    deploy_history::write_snapshot(&resolved.workspace_root, env_name, &content, tag)
}

/// Like `update_env_file_tags`, but pulls each image's tag from a per-image map.
/// Falls back to `default_tag` for images not present in the map.
fn update_env_file_tags_from_map(
    env_file: &Path,
    images: &BTreeMap<String, ImageSpec>,
    image_tags: &BTreeMap<String, String>,
    default_tag: &str,
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
            std::fs::write(env_file, "")
                .with_context(|| format!("writing {}", env_file.display()))?;
        }
    }

    let mut content = std::fs::read_to_string(env_file)
        .with_context(|| format!("reading {}", env_file.display()))?;

    for (image_key, image) in images.iter() {
        let Some(var) = &image.tag_env else { continue };
        let value = image_tags
            .get(image_key)
            .map(|s| s.as_str())
            .unwrap_or(default_tag);
        content = replace_or_append_kv(&content, var, value);
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
    std::fs::write(env_file, content)
        .with_context(|| format!("writing {}", env_file.display()))?;
    Ok(())
}

/// Best-effort copy the local history JSONL and (if any) the matching snapshot
/// to the remote host. Failures are logged as warnings; the local file remains
/// the source of truth.
fn mirror_history_to_remote(
    resolved: &Resolved,
    env_name: &str,
    env: &EnvironmentSpec,
    record: &HistoryRecord,
) {
    if env.host.is_none() {
        return;
    }
    if let Err(e) = mirror_history_inner(resolved, env_name, env, record) {
        eprintln!(
            "{} failed to mirror history to remote: {e}",
            "warning:".yellow().bold()
        );
    }
}

fn mirror_history_inner(
    resolved: &Resolved,
    env_name: &str,
    env: &EnvironmentSpec,
    record: &HistoryRecord,
) -> Result<()> {
    let ssh = ssh_target(resolved, env)?;
    let deploy_dir = resolved.deploy_dir(env)?;
    let remote_history_dir = format!("{deploy_dir}/history");
    let remote_snapshots_dir = format!("{remote_history_dir}/snapshots");

    // Make sure remote dirs exist (combined into one mkdir to keep it cheap).
    let mkdir_cmd = format!(
        "mkdir -p {} {}",
        shell_quote(&remote_history_dir),
        shell_quote(&remote_snapshots_dir)
    );
    let status = Command::new("ssh").arg(&ssh).arg(&mkdir_cmd).status()?;
    if !status.success() {
        bail!("ssh mkdir exited {status}");
    }

    // Whole-file scp of JSONL — small (one record per push, KB-scale).
    let local_jsonl = deploy_history::history_file(&resolved.workspace_root, env_name);
    if local_jsonl.is_file() {
        let dest = format!("{ssh}:{remote_history_dir}/{env_name}.jsonl");
        let status = Command::new("scp").arg(&local_jsonl).arg(&dest).status()?;
        if !status.success() {
            bail!("scp jsonl exited {status}");
        }
    }

    // scp the snapshot for this record, if present.
    if let Some(snap) = &record.snapshot {
        let local_snap = deploy_history::snapshots_dir(&resolved.workspace_root).join(snap);
        if local_snap.is_file() {
            let dest = format!("{ssh}:{remote_snapshots_dir}/{snap}");
            let status = Command::new("scp").arg(&local_snap).arg(&dest).status()?;
            if !status.success() {
                bail!("scp snapshot exited {status}");
            }
        }
    }
    Ok(())
}

// ────────────────────────────── history subcommand ──────────────────────────────

fn history(
    resolved: &Resolved,
    env_name: &str,
    limit: usize,
    remote: bool,
    json: bool,
) -> Result<()> {
    let raw = if remote {
        let env_spec = require_remote(resolved, env_name)?;
        read_remote_history(resolved, env_spec, env_name)?
    } else {
        let path = deploy_history::history_file(&resolved.workspace_root, env_name);
        if path.is_file() {
            std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?
        } else {
            String::new()
        }
    };

    let path_for_errors =
        deploy_history::history_file(&resolved.workspace_root, env_name);
    let records = deploy_history::parse_jsonl(&raw, &path_for_errors)?;

    if json {
        // newest first
        let view: Vec<&HistoryRecord> = records.iter().rev().take(limit).collect();
        println!("{}", serde_json::to_string_pretty(&view)?);
    } else {
        print!("{}", deploy_history::render_table(&records, Some(limit)));
    }
    Ok(())
}

fn read_remote_history(
    resolved: &Resolved,
    env: &EnvironmentSpec,
    env_name: &str,
) -> Result<String> {
    let ssh = ssh_target(resolved, env)?;
    let deploy_dir = resolved.deploy_dir(env)?;
    let path = format!("{deploy_dir}/history/{env_name}.jsonl");
    let cmd = format!("test -f {p} && cat {p} || true", p = shell_quote(&path));
    let output = Command::new("ssh")
        .arg(&ssh)
        .arg(&cmd)
        .output()
        .context("running ssh + cat for remote history")?;
    if !output.status.success() {
        bail!(
            "ssh exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8(output.stdout).context("remote history not utf-8")?)
}

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
    use tempfile::TempDir;

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

    // ───────────── single-service deploy / bump / preflight ─────────────

    fn img(context: &str, tag_env: Option<&str>) -> ImageSpec {
        ImageSpec {
            context: context.into(),
            dockerfile: None,
            registry: None,
            name: None,
            tag_env: tag_env.map(|s| s.into()),
            build_args: BTreeMap::new(),
            push: None,
        }
    }

    fn env_with(images: Vec<(&str, ImageSpec)>) -> EnvironmentSpec {
        EnvironmentSpec {
            host: Some("h".into()),
            ssh_user: None,
            deploy_dir: None,
            compose_file: None,
            env_file: None,
            registry: None,
            require_confirm: false,
            images: images.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        }
    }

    #[test]
    fn select_images_empty_returns_all() {
        let env = env_with(vec![
            ("api", img("apps/api", Some("SERVICE_TAG"))),
            ("web", img("apps/web", Some("WEB_TAG"))),
        ]);
        let sel = select_images(&env, &[]).unwrap();
        assert_eq!(sel.len(), 2);
    }

    #[test]
    fn select_images_subset() {
        let env = env_with(vec![
            ("api", img("apps/api", Some("SERVICE_TAG"))),
            ("web", img("apps/web", Some("WEB_TAG"))),
        ]);
        let sel = select_images(&env, &["web".to_string()]).unwrap();
        assert_eq!(sel.len(), 1);
        assert!(sel.contains_key(&"web".to_string()));
    }

    #[test]
    fn select_images_unknown_errors_with_available() {
        let env = env_with(vec![("api", img("apps/api", Some("SERVICE_TAG")))]);
        let e = select_images(&env, &["nope".to_string()]).unwrap_err().to_string();
        assert!(e.contains("not found"));
        assert!(e.contains("available: api"));
    }

    #[test]
    fn update_selected_errors_on_missing_tag_env() {
        let tmp = TempDir::new().unwrap();
        let envf = tmp.path().join(".env.prod");
        std::fs::write(&envf, "FOO=bar\n").unwrap();
        let env = env_with(vec![("svc", img("apps/svc", None))]);
        let sel = select_images(&env, &["svc".to_string()]).unwrap();
        let e = update_env_file_tags_selected(&envf, &sel, "v1", false)
            .unwrap_err()
            .to_string();
        assert!(e.contains("no tag_env"));
    }

    #[test]
    fn update_selected_writes_only_selected_tag() {
        let tmp = TempDir::new().unwrap();
        let envf = tmp.path().join(".env.prod");
        std::fs::write(&envf, "SERVICE_TAG=v0\nWEB_TAG=v0\n").unwrap();
        let env = env_with(vec![
            ("api", img("apps/api", Some("SERVICE_TAG"))),
            ("web", img("apps/web", Some("WEB_TAG"))),
        ]);
        let sel = select_images(&env, &["web".to_string()]).unwrap();
        update_env_file_tags_selected(&envf, &sel, "v9", false).unwrap();
        let out = std::fs::read_to_string(&envf).unwrap();
        assert!(out.contains("SERVICE_TAG=v0"));
        assert!(out.contains("WEB_TAG=v9"));
    }

    #[test]
    fn required_vars_picks_uppercase_assignments() {
        let c = "# comment\nFOO=1\nBAR_BAZ=2\nlower=3\n  INDENTED=4\nNOEQ\n";
        let req = required_vars(c);
        assert_eq!(req, vec!["FOO", "BAR_BAZ", "INDENTED"]);
    }

    #[test]
    fn parse_env_values_first_occurrence_wins() {
        let m = parse_env_values("A=1\nA=2\nB=\nC=a=b\n# D=x\n");
        assert_eq!(m.get("A").unwrap(), "1");
        assert_eq!(m.get("B").unwrap(), "");
        assert_eq!(m.get("C").unwrap(), "a=b");
        assert!(!m.contains_key("D"));
    }

    #[test]
    fn check_contract_flags_missing_empty_and_placeholder() {
        let req = vec![
            "OK".to_string(),
            "MISSING".to_string(),
            "EMPTY".to_string(),
            "PH".to_string(),
            "ANGLE".to_string(),
        ];
        let mut have = std::collections::HashMap::new();
        have.insert("OK".to_string(), "v0.5.2".to_string());
        have.insert("EMPTY".to_string(), "".to_string());
        have.insert("PH".to_string(), "CHANGE_ME_please".to_string());
        have.insert("ANGLE".to_string(), "<your-key>".to_string());
        let (missing, placeholders) = check_contract(&req, &have);
        assert!(missing.contains(&"MISSING".to_string()));
        assert!(missing.contains(&"EMPTY".to_string()));
        assert!(placeholders.iter().any(|p| p.starts_with("PH=")));
        assert!(placeholders.iter().any(|p| p.starts_with("ANGLE=")));
        assert_eq!(missing.len(), 2);
        assert_eq!(placeholders.len(), 2);
    }
}
