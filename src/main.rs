//! Metaphor Dev Plugin — development workflow commands.
//!
//! Binary: `metaphor-dev`
//! Commands: dev, lint, test, docs, config, jobs, docker, deploy, chaos

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;

use metaphor_dev::commands::{
    config::ConfigAction,
    deploy::DeployAction,
    dev::DevAction,
    docker::DockerAction,
    docs::DocsAction,
    jobs::JobsAction,
    lint::LintAction,
    test::TestAction,
};

#[derive(Parser)]
#[command(
    name = "metaphor-dev",
    version,
    about = "Development workflow plugin for Metaphor CLI",
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Development workflow commands
    Dev {
        #[command(subcommand)]
        action: DevAction,
    },

    /// Code quality and linting commands
    Lint {
        #[command(subcommand)]
        action: LintAction,
    },

    /// Test generation and management commands
    Test {
        #[command(subcommand)]
        action: TestAction,
    },

    /// Documentation generation commands
    Docs {
        #[command(subcommand)]
        action: DocsAction,
    },

    /// Configuration validation and management commands
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Job scheduling commands
    Jobs {
        #[command(subcommand)]
        action: JobsAction,
    },

    /// Local docker compose lifecycle (reads metaphor.deploy.yaml).
    Docker {
        #[command(subcommand)]
        action: DockerAction,
    },

    /// Remote deployment: build, push, roll out, roll back.
    Deploy {
        #[command(subcommand)]
        action: DeployAction,
    },

    /// Chaos gate — forward to the workspace's fault-injection kit
    /// (`deployment/chaos/run.sh`). Dev stack only.
    #[command(trailing_var_arg = true, allow_external_subcommands = true)]
    Chaos {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::init();
    }

    println!("{}", "⚡ Metaphor Dev".bright_green().bold());
    println!();

    match &cli.command {
        Command::Dev { action } => metaphor_dev::commands::dev::handle_command(action).await,
        Command::Lint { action } => metaphor_dev::commands::lint::handle_command(action).await,
        Command::Test { action } => metaphor_dev::commands::test::handle_command(action).await,
        Command::Docs { action } => metaphor_dev::commands::docs::handle_command(action).await,
        Command::Config { action } => metaphor_dev::commands::config::handle_config_command(action).await,
        Command::Jobs { action } => metaphor_dev::commands::jobs::handle_jobs_command(action).await,
        Command::Docker { action } => metaphor_dev::commands::docker::handle_command(action).await,
        Command::Deploy { action } => metaphor_dev::commands::deploy::handle_command(action).await,
        Command::Chaos { args } => metaphor_dev::commands::chaos::handle_chaos_command(&args).await,
    }
}
