//! `metaphor docker` — local docker compose lifecycle.
//!
//! Thin, opinionated wrapper around `docker compose` that:
//!   - resolves compose file + env file from `metaphor.deploy.yaml`
//!   - picks the environment via `--env` (default: `dev`)
//!   - only operates on *local* environments (ones without `host:`)
//!
//! Remote environments are handled by `metaphor deploy`.

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use colored::*;
use std::path::Path;
use std::process::Command;

use crate::deploy_config::{self, EnvironmentSpec, Resolved};

#[derive(Subcommand)]
pub enum DockerAction {
    /// Start the stack (`docker compose up`).
    Up {
        /// Target environment in metaphor.deploy.yaml.
        #[arg(long, default_value = "dev")]
        env: String,

        /// Run in foreground (default is detached).
        #[arg(long)]
        attach: bool,

        /// Build images before starting.
        #[arg(long)]
        build: bool,

        /// Only start these services (repeatable).
        #[arg(long = "service", value_name = "SERVICE")]
        services: Vec<String>,
    },

    /// Stop and remove the stack (`docker compose down`).
    Down {
        #[arg(long, default_value = "dev")]
        env: String,

        /// Also remove named volumes (destructive).
        #[arg(long)]
        volumes: bool,
    },

    /// Tail logs.
    Logs {
        #[arg(long, default_value = "dev")]
        env: String,

        /// Follow output.
        #[arg(long, short)]
        follow: bool,

        /// Number of lines from the end.
        #[arg(long, default_value = "200")]
        tail: String,

        /// Limit to a service.
        #[arg(long = "service", value_name = "SERVICE")]
        service: Option<String>,
    },

    /// Show running containers.
    Ps {
        #[arg(long, default_value = "dev")]
        env: String,
    },

    /// Restart a service.
    Restart {
        #[arg(long, default_value = "dev")]
        env: String,

        #[arg(long = "service", value_name = "SERVICE")]
        service: String,
    },

    /// Pull images defined in compose.
    Pull {
        #[arg(long, default_value = "dev")]
        env: String,

        #[arg(long = "service", value_name = "SERVICE")]
        services: Vec<String>,
    },

    /// Build images defined in compose.
    Build {
        #[arg(long, default_value = "dev")]
        env: String,

        /// Push after build.
        #[arg(long)]
        push: bool,

        #[arg(long = "service", value_name = "SERVICE")]
        services: Vec<String>,
    },
}

pub async fn handle_command(action: &DockerAction) -> Result<()> {
    let resolved = Resolved::load()?;
    let env_name = env_name_of(action);
    let env = resolved.environment(env_name)?;

    if !deploy_config::is_local(env) {
        bail!(
            "environment '{}' is remote (host: {:?}) — use `metaphor deploy` instead",
            env_name,
            env.host.as_deref().unwrap_or("")
        );
    }

    let compose_file = resolved.local_compose_file(env);
    let env_file = resolved.local_env_file(env, env_name);

    if !compose_file.is_file() {
        bail!(
            "compose file not found at {}",
            compose_file.display()
        );
    }
    if !env_file.is_file() {
        eprintln!(
            "{} env file not found at {} — continuing without it",
            "warning:".yellow().bold(),
            env_file.display()
        );
    }

    let cwd = &resolved.workspace_root;
    print_header(env_name, &compose_file, &env_file);

    match action {
        DockerAction::Up { attach, build, services, .. } => {
            let mut args = base_args(&compose_file, &env_file);
            args.push("up".into());
            if !attach {
                args.push("-d".into());
            }
            if *build {
                args.push("--build".into());
            }
            for s in services {
                args.push(s.clone());
            }
            run_docker(&args, cwd)
        }
        DockerAction::Down { volumes, .. } => {
            let mut args = base_args(&compose_file, &env_file);
            args.push("down".into());
            if *volumes {
                args.push("-v".into());
            }
            run_docker(&args, cwd)
        }
        DockerAction::Logs { follow, tail, service, .. } => {
            let mut args = base_args(&compose_file, &env_file);
            args.push("logs".into());
            if *follow {
                args.push("-f".into());
            }
            args.push("--tail".into());
            args.push(tail.clone());
            if let Some(s) = service {
                args.push(s.clone());
            }
            run_docker(&args, cwd)
        }
        DockerAction::Ps { .. } => {
            let mut args = base_args(&compose_file, &env_file);
            args.push("ps".into());
            run_docker(&args, cwd)
        }
        DockerAction::Restart { service, .. } => {
            let mut args = base_args(&compose_file, &env_file);
            args.push("restart".into());
            args.push(service.clone());
            run_docker(&args, cwd)
        }
        DockerAction::Pull { services, .. } => {
            let mut args = base_args(&compose_file, &env_file);
            args.push("pull".into());
            for s in services {
                args.push(s.clone());
            }
            run_docker(&args, cwd)
        }
        DockerAction::Build { push, services, .. } => {
            let mut args = base_args(&compose_file, &env_file);
            args.push("build".into());
            for s in services {
                args.push(s.clone());
            }
            run_docker(&args, cwd)?;
            if *push {
                let mut args = base_args(&compose_file, &env_file);
                args.push("push".into());
                for s in services {
                    args.push(s.clone());
                }
                run_docker(&args, cwd)?;
            }
            Ok(())
        }
    }
}

fn env_name_of(action: &DockerAction) -> &str {
    match action {
        DockerAction::Up { env, .. }
        | DockerAction::Down { env, .. }
        | DockerAction::Logs { env, .. }
        | DockerAction::Ps { env }
        | DockerAction::Restart { env, .. }
        | DockerAction::Pull { env, .. }
        | DockerAction::Build { env, .. } => env,
    }
}

/// Base args common to every `docker compose` invocation: file + env-file.
fn base_args(compose_file: &Path, env_file: &Path) -> Vec<String> {
    let mut args = vec!["compose".to_string()];
    args.push("-f".into());
    args.push(compose_file.display().to_string());
    if env_file.is_file() {
        args.push("--env-file".into());
        args.push(env_file.display().to_string());
    }
    args
}

fn run_docker(args: &[String], cwd: &Path) -> Result<()> {
    let mut cmd = Command::new("docker");
    cmd.args(args).current_dir(cwd);

    eprintln!(
        "{} docker {}",
        "→".bright_black(),
        args.join(" ").bright_black()
    );

    let status = cmd
        .status()
        .with_context(|| "failed to spawn `docker` — is it installed and on PATH?")?;
    if !status.success() {
        bail!("docker compose exited with {}", status);
    }
    Ok(())
}

fn print_header(env: &str, compose_file: &Path, env_file: &Path) {
    println!(
        "{} {}",
        "Environment:".bright_black(),
        env.bright_cyan().bold()
    );
    println!(
        "{} {}",
        "Compose:    ".bright_black(),
        compose_file.display()
    );
    println!(
        "{} {}",
        "Env file:   ".bright_black(),
        env_file.display()
    );
    println!();
}

#[allow(dead_code)]
pub fn resolve_env<'a>(resolved: &'a Resolved, name: &str) -> Result<&'a EnvironmentSpec> {
    resolved.environment(name)
}
