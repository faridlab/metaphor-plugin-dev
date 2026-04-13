//! Code quality and linting commands for Metaphor Framework
//!
//! This module provides commands for maintaining code quality:
//! - Clippy linting with custom rules
//! - Code formatting with rustfmt
//! - Compilation checks
//! - Security audits
//! - Dependency checks
//!
//! # Commands
//!
//! - `metaphor lint` - Run clippy with framework rules
//! - `metaphor lint fix` - Auto-fix linting issues
//! - `metaphor lint fmt` - Format code with rustfmt
//! - `metaphor lint check` - Quick compilation check
//! - `metaphor lint audit` - Security audit

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::*;
use std::process::Command;

/// Lint command actions
#[derive(Subcommand, Clone, Debug)]
pub enum LintAction {
    /// Run clippy linter with framework rules
    Check {
        /// Target module (or all if not specified)
        #[arg(long)]
        module: Option<String>,

        /// Treat warnings as errors
        #[arg(long)]
        strict: bool,

        /// Fix issues automatically where possible
        #[arg(long)]
        fix: bool,

        /// Show all warnings (including allowed ones)
        #[arg(long)]
        pedantic: bool,
    },

    /// Format code with rustfmt
    Fmt {
        /// Target module (or all if not specified)
        #[arg(long)]
        module: Option<String>,

        /// Check formatting without making changes
        #[arg(long)]
        check: bool,

        /// Show diff of changes
        #[arg(long)]
        diff: bool,
    },

    /// Quick compilation check without building
    Compile {
        /// Target module (or all if not specified)
        #[arg(long)]
        module: Option<String>,

        /// Check in release mode
        #[arg(long)]
        release: bool,
    },

    /// Run security audit on dependencies
    Audit {
        /// Fix vulnerable dependencies where possible
        #[arg(long)]
        fix: bool,

        /// Output format (text, json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Check for outdated dependencies
    Outdated {
        /// Show only direct dependencies
        #[arg(long)]
        direct: bool,

        /// Show compatible updates only
        #[arg(long)]
        compatible: bool,
    },

    /// Run all quality checks (lint, fmt, compile)
    All {
        /// Target module (or all if not specified)
        #[arg(long)]
        module: Option<String>,

        /// Treat warnings as errors
        #[arg(long)]
        strict: bool,

        /// Auto-fix issues where possible
        #[arg(long)]
        fix: bool,
    },

    /// Show clippy configuration for the project
    Config,
}

/// Handle lint commands
pub async fn handle_command(action: &LintAction) -> Result<()> {
    match action {
        LintAction::Check {
            module,
            strict,
            fix,
            pedantic,
        } => run_clippy(module.as_deref(), *strict, *fix, *pedantic).await,

        LintAction::Fmt {
            module,
            check,
            diff,
        } => run_fmt(module.as_deref(), *check, *diff).await,

        LintAction::Compile { module, release } => run_compile(module.as_deref(), *release).await,

        LintAction::Audit { fix, format } => run_audit(*fix, format).await,

        LintAction::Outdated { direct, compatible } => run_outdated(*direct, *compatible).await,

        LintAction::All { module, strict, fix } => {
            run_all_checks(module.as_deref(), *strict, *fix).await
        }

        LintAction::Config => show_config().await,
    }
}

/// Run clippy linter
async fn run_clippy(
    module: Option<&str>,
    strict: bool,
    fix: bool,
    pedantic: bool,
) -> Result<()> {
    println!(
        "{}",
        "🔍 Running Clippy linter...".bright_cyan().bold()
    );
    println!();

    let mut args = vec!["clippy"];

    // Add module filter
    if let Some(m) = module {
        args.push("-p");
        args.push(Box::leak(format!("metaphor-{}", m).into_boxed_str()));
    } else {
        args.push("--workspace");
    }

    // Add fix flag
    if fix {
        args.push("--fix");
        args.push("--allow-dirty");
        args.push("--allow-staged");
    }

    args.push("--");

    // Framework-specific clippy rules
    if strict {
        args.push("-D");
        args.push("warnings");
    }

    if pedantic {
        args.push("-W");
        args.push("clippy::pedantic");
    }

    // Always deny these
    args.push("-D");
    args.push("clippy::unwrap_used");
    args.push("-D");
    args.push("clippy::expect_used");

    // Warn on these
    args.push("-W");
    args.push("clippy::todo");
    args.push("-W");
    args.push("clippy::dbg_macro");
    args.push("-W");
    args.push("clippy::print_stdout");
    args.push("-W");
    args.push("clippy::print_stderr");

    // Async-specific rules
    args.push("-W");
    args.push("clippy::large_futures");
    args.push("-W");
    args.push("clippy::redundant_async_block");
    args.push("-W");
    args.push("clippy::unused_async");

    // Allow these (framework-specific exceptions)
    args.push("-A");
    args.push("clippy::module_inception");
    args.push("-A");
    args.push("clippy::too_many_arguments");

    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to run cargo clippy")?;

    println!();

    if status.success() {
        println!(
            "  {} Clippy passed!",
            "✅".green()
        );
    } else if fix {
        println!(
            "  {} Some issues were fixed, please review changes",
            "🔧".yellow()
        );
    } else {
        println!(
            "  {} Clippy found issues",
            "❌".red()
        );
        if !fix {
            println!(
                "  {} Run with --fix to auto-fix where possible",
                "💡".bright_blue()
            );
        }
    }

    Ok(())
}

/// Run rustfmt
async fn run_fmt(module: Option<&str>, check: bool, diff: bool) -> Result<()> {
    println!(
        "{}",
        "🎨 Running rustfmt...".bright_cyan().bold()
    );
    println!();

    let mut args = vec!["fmt"];

    // Add module filter
    if let Some(m) = module {
        args.push("-p");
        args.push(Box::leak(format!("metaphor-{}", m).into_boxed_str()));
    } else {
        args.push("--all");
    }

    if check {
        args.push("--check");
    }

    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to run cargo fmt")?;

    // If diff requested and not in check mode, show what changed
    if diff && !check {
        println!();
        println!("  {} Changes made:", "📝".bright_blue());

        Command::new("git")
            .args(["diff", "--stat"])
            .status()?;
    }

    println!();

    if status.success() {
        if check {
            println!(
                "  {} Code is properly formatted!",
                "✅".green()
            );
        } else {
            println!(
                "  {} Code formatted!",
                "✅".green()
            );
        }
    } else if check {
        println!(
            "  {} Code needs formatting",
            "❌".red()
        );
        println!(
            "  {} Run without --check to fix",
            "💡".bright_blue()
        );
    }

    Ok(())
}

/// Run compilation check
async fn run_compile(module: Option<&str>, release: bool) -> Result<()> {
    println!(
        "{}",
        "🔨 Running compilation check...".bright_cyan().bold()
    );
    println!();

    let mut args = vec!["check"];

    if let Some(m) = module {
        args.push("-p");
        args.push(Box::leak(format!("metaphor-{}", m).into_boxed_str()));
    } else {
        args.push("--workspace");
    }

    if release {
        args.push("--release");
    }

    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to run cargo check")?;

    println!();

    if status.success() {
        println!(
            "  {} Compilation successful!",
            "✅".green()
        );
    } else {
        println!(
            "  {} Compilation failed",
            "❌".red()
        );
    }

    Ok(())
}

/// Run security audit
async fn run_audit(fix: bool, format: &str) -> Result<()> {
    println!(
        "{}",
        "🔒 Running security audit...".bright_cyan().bold()
    );
    println!();

    // Check if cargo-audit is installed
    let audit_check = Command::new("cargo")
        .args(["audit", "--version"])
        .output();

    if audit_check.is_err() {
        println!(
            "  {} cargo-audit not found. Installing...",
            "📦".bright_yellow()
        );

        let install_status = Command::new("cargo")
            .args(["install", "cargo-audit"])
            .status()?;

        if !install_status.success() {
            anyhow::bail!("Failed to install cargo-audit");
        }
    }

    let mut args = vec!["audit"];

    if fix {
        args.push("fix");
    }

    match format {
        "json" => {
            args.push("--json");
        }
        _ => {}
    }

    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to run cargo audit")?;

    println!();

    if status.success() {
        println!(
            "  {} No known vulnerabilities found!",
            "✅".green()
        );
    } else {
        println!(
            "  {} Security vulnerabilities detected",
            "⚠️".yellow()
        );
        if !fix {
            println!(
                "  {} Run with --fix to attempt auto-fix",
                "💡".bright_blue()
            );
        }
    }

    Ok(())
}

/// Check for outdated dependencies
async fn run_outdated(direct: bool, compatible: bool) -> Result<()> {
    println!(
        "{}",
        "📦 Checking for outdated dependencies...".bright_cyan().bold()
    );
    println!();

    // Check if cargo-outdated is installed
    let outdated_check = Command::new("cargo")
        .args(["outdated", "--version"])
        .output();

    if outdated_check.is_err() {
        println!(
            "  {} cargo-outdated not found. Installing...",
            "📦".bright_yellow()
        );

        let install_status = Command::new("cargo")
            .args(["install", "cargo-outdated"])
            .status()?;

        if !install_status.success() {
            anyhow::bail!("Failed to install cargo-outdated");
        }
    }

    let mut args = vec!["outdated"];

    if direct {
        args.push("--root-deps-only");
    }

    if compatible {
        args.push("--compatible");
    }

    Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to run cargo outdated")?;

    Ok(())
}

/// Run all quality checks
async fn run_all_checks(module: Option<&str>, strict: bool, fix: bool) -> Result<()> {
    println!(
        "{}",
        "🔍 Running all quality checks...".bright_cyan().bold()
    );
    println!();

    let mut all_passed = true;

    // 1. Format check/fix
    println!(
        "{}",
        "Step 1/4: Code formatting".bright_white().bold()
    );
    if let Err(e) = run_fmt(module, !fix, false).await {
        println!("  {} Formatting check failed: {}", "❌".red(), e);
        all_passed = false;
    }
    println!();

    // 2. Compilation check
    println!(
        "{}",
        "Step 2/4: Compilation check".bright_white().bold()
    );
    if let Err(e) = run_compile(module, false).await {
        println!("  {} Compilation check failed: {}", "❌".red(), e);
        all_passed = false;
    }
    println!();

    // 3. Clippy
    println!(
        "{}",
        "Step 3/4: Clippy linting".bright_white().bold()
    );
    if let Err(e) = run_clippy(module, strict, fix, false).await {
        println!("  {} Clippy failed: {}", "❌".red(), e);
        all_passed = false;
    }
    println!();

    // 4. Security audit (optional, don't fail on this)
    println!(
        "{}",
        "Step 4/4: Security audit".bright_white().bold()
    );
    if let Err(e) = run_audit(false, "text").await {
        println!(
            "  {} Security audit had issues: {}",
            "⚠️".yellow(),
            e
        );
    }

    println!();
    println!("{}", "═".repeat(50).bright_white());
    println!();

    if all_passed {
        println!(
            "{}",
            "All quality checks passed! 🎉".bright_green().bold()
        );
    } else {
        println!(
            "{}",
            "Some quality checks failed ❌".bright_red().bold()
        );
        if !fix {
            println!(
                "  {} Run with --fix to attempt auto-fixes",
                "💡".bright_blue()
            );
        }
    }

    Ok(())
}

/// Show clippy configuration
async fn show_config() -> Result<()> {
    println!(
        "{}",
        "📋 Metaphor Framework Clippy Configuration".bright_cyan().bold()
    );
    println!();

    println!(
        "{}",
        "Denied lints (errors):".bright_white().bold()
    );
    println!("  - clippy::unwrap_used    (use ? or handle errors)");
    println!("  - clippy::expect_used    (use ? or handle errors)");
    println!();

    println!(
        "{}",
        "Warned lints:".bright_white().bold()
    );
    println!("  - clippy::todo           (mark incomplete code)");
    println!("  - clippy::dbg_macro      (remove debug macros)");
    println!("  - clippy::print_stdout   (use tracing instead)");
    println!("  - clippy::print_stderr   (use tracing instead)");
    println!();

    println!(
        "{}",
        "Async-specific lints (warnings):".bright_white().bold()
    );
    println!("  - clippy::large_futures          (avoid large futures on stack)");
    println!("  - clippy::redundant_async_block  (simplify unnecessary async blocks)");
    println!("  - clippy::unused_async           (remove async from non-awaiting fns)");
    println!();

    println!(
        "{}",
        "Allowed lints (framework exceptions):".bright_white().bold()
    );
    println!("  - clippy::module_inception     (we use domain/domain.rs pattern)");
    println!("  - clippy::too_many_arguments   (builder pattern uses many args)");
    println!();

    println!(
        "{}",
        "To add project-specific configuration:".bright_white().bold()
    );
    println!("  Create a clippy.toml in the project root with:");
    println!();
    println!("  ```toml");
    println!("  # clippy.toml");
    println!("  cognitive-complexity-threshold = 25");
    println!("  type-complexity-threshold = 300");
    println!("  ```");
    println!();

    println!(
        "{}",
        "Or add to Cargo.toml:".bright_white().bold()
    );
    println!();
    println!("  ```toml");
    println!("  [lints.clippy]");
    println!("  unwrap_used = \"deny\"");
    println!("  expect_used = \"deny\"");
    println!("  todo = \"warn\"");
    println!("  ```");

    Ok(())
}
