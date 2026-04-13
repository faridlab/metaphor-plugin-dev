//! Metaphor Dev Plugin — development workflow commands.
//!
//! Binary: `metaphor-dev`
//! Commands: dev, lint, test, docs, config, jobs

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;

use metaphor_dev::commands::{
    config::ConfigAction,
    dev::DevAction,
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
    }
}
