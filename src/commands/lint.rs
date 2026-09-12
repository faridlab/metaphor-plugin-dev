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

/// What one quality gate did.
///
/// A gate that finds a problem MUST reach the process exit code. Printing a
/// red cross and returning `Ok(())` makes every scripted `metaphor lint` — and
/// every CI job built on it — report success while the code is broken, which
/// is worse than having no gate at all. Every runner below therefore reports
/// an outcome instead of swallowing the child process's exit status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    /// The gate ran and found nothing.
    Passed,
    /// The gate ran and found something. The string says what.
    Failed(String),
    /// The gate could not run at all. The string says why.
    Skipped(String),
}

impl GateOutcome {
    /// Collapse a single gate into a command result.
    ///
    /// `require_tools` promotes a skip to a failure, so a pipeline cannot go
    /// green merely because a linter was missing from the image.
    pub fn into_result(self, gate: &str, require_tools: bool) -> Result<()> {
        match self {
            GateOutcome::Passed => Ok(()),
            GateOutcome::Failed(why) => anyhow::bail!("{gate} failed: {why}"),
            GateOutcome::Skipped(why) if require_tools => {
                anyhow::bail!("{gate} did not run: {why} (--require-tools)")
            }
            GateOutcome::Skipped(_) => Ok(()),
        }
    }
}

/// Is a cargo subcommand installed?
///
/// `cargo <sub> --version` exits non-zero with "no such command" when the
/// subcommand is missing; spawning only fails when cargo itself is absent. An
/// earlier version tested `output().is_err()`, which is false in both cases,
/// so a missing tool fell through to the real invocation and its "no such
/// command" exit was reported as if the check had found problems.
fn cargo_subcommand_available(sub: &str) -> bool {
    Command::new("cargo")
        .args([sub, "--version"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// Print the aggregate result of a multi-gate run and decide its exit status.
///
/// Advisory gates are reported but never decide the outcome; `require_tools`
/// makes a skipped gate count as a failure.
fn summarize(gates: &[(String, GateOutcome, bool)], require_tools: bool) -> Result<()> {
    let mut blocking: Vec<String> = Vec::new();

    for (name, outcome, advisory) in gates {
        match outcome {
            GateOutcome::Passed => println!("  {} {}", "✅".green(), name),
            GateOutcome::Failed(why) => {
                if *advisory {
                    println!("  {} {} — {} (advisory)", "⚠️".yellow(), name, why);
                } else {
                    println!("  {} {} — {}", "❌".red(), name, why);
                    blocking.push(name.clone());
                }
            }
            GateOutcome::Skipped(why) => {
                println!("  {} {} — skipped: {}", "⏭️".bright_yellow(), name, why);
                if require_tools && !*advisory {
                    blocking.push(name.clone());
                }
            }
        }
    }

    println!();

    if blocking.is_empty() {
        println!("{}", "All quality checks passed! 🎉".bright_green().bold());
        Ok(())
    } else {
        anyhow::bail!("quality checks failed: {}", blocking.join(", "))
    }
}

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

        /// Fail instead of skipping when a gate's tool is not installed
        #[arg(long)]
        require_tools: bool,
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

        /// Treat warnings as errors, and let a security finding fail the run
        #[arg(long)]
        strict: bool,

        /// Auto-fix issues where possible
        #[arg(long)]
        fix: bool,

        /// Fail instead of skipping when a gate's tool is not installed
        #[arg(long)]
        require_tools: bool,
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
            require_tools,
        } => run_clippy(module.as_deref(), *strict, *fix, *pedantic, *require_tools)
            .await?
            .into_result("clippy", *require_tools),

        LintAction::Fmt {
            module,
            check,
            diff,
        } => run_fmt(module.as_deref(), *check, *diff)
            .await?
            .into_result("rustfmt", false),

        LintAction::Compile { module, release } => run_compile(module.as_deref(), *release)
            .await?
            .into_result("compilation check", false),

        LintAction::Audit { fix, format } => run_audit(*fix, format)
            .await?
            .into_result("security audit", false),

        LintAction::Outdated { direct, compatible } => run_outdated(*direct, *compatible).await,

        LintAction::All {
            module,
            strict,
            fix,
            require_tools,
        } => run_all_checks(module.as_deref(), *strict, *fix, *require_tools).await,

        LintAction::Config => show_config().await,
    }
}

/// Run the ADR-0014 company-fence declarations gate before clippy.
///
/// Scope follows CWD: inside a schema module (a `schema/models/index.model.yaml`
/// at CWD) it runs the per-module `schema validate`, which hard-fails on THAT
/// module's missing `company_fence:` posture — a module's lint must not go red
/// over still-unswept siblings in the surrounding workspace, or no module could
/// land its declaration before the sweep completes. Everywhere else under a
/// workspace it runs `validate-workspace` (cross-module FKs + one explicit
/// posture per schema module). Skips with a note when there is no workspace to
/// sweep or the schema plugin binary is not installed — those skips are
/// reported to the caller so `--require-tools` can turn them into failures.
async fn run_schema_declarations_gate() -> Result<GateOutcome> {
    let cwd = std::env::current_dir()?;

    // No metaphor.yaml above CWD → nothing to sweep (e.g. a bare crate repo).
    let mut dir = cwd.clone();
    loop {
        if dir.join("metaphor.yaml").is_file() || dir.join("metaphor.yml").is_file() {
            break;
        }
        if !dir.pop() {
            println!(
                "  {} no metaphor.yaml found above CWD — skipping schema declarations gate",
                "⏭️".bright_yellow()
            );
            return Ok(GateOutcome::Skipped(
                "no metaphor.yaml above the working directory".to_string(),
            ));
        }
    }

    // Per-module validate resolves the module from CWD via the workspace, so it
    // needs the workspace check above to have passed before it can run.
    let per_module = cwd.join("schema/models/index.model.yaml").is_file();
    let mut command = Command::new("metaphor-schema");
    if per_module {
        command.arg("schema").arg("validate");
    } else {
        command.arg("validate-workspace");
    }

    println!(
        "{}",
        if per_module {
            "🛡️  Checking schema declarations (company_fence posture for this module)..."
                .bright_cyan()
                .bold()
        } else {
            "🛡️  Checking schema declarations (company_fence + cross-module FKs)..."
                .bright_cyan()
                .bold()
        }
    );

    let status = match command.status() {
        Ok(status) => status,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!(
                "  {} metaphor-schema not on PATH — skipping schema declarations gate \
                 (install the schema plugin to enable it)",
                "⏭️".bright_yellow()
            );
            return Ok(GateOutcome::Skipped(
                "metaphor-schema is not on PATH".to_string(),
            ));
        }
        Err(e) => {
            return Err(e).context(if per_module {
                "Failed to run metaphor-schema schema validate"
            } else {
                "Failed to run metaphor-schema validate-workspace"
            })
        }
    };

    println!();

    if !status.success() {
        return Ok(GateOutcome::Failed(format!(
            "every schema module needs an explicit 'company_fence:' posture \
             (strict | shared_blank | shared_tree | none) in its index.model.yaml{}",
            if per_module {
                ""
            } else {
                ", and every cross-module FK must resolve"
            }
        )));
    }

    Ok(GateOutcome::Passed)
}

/// Run clippy linter
async fn run_clippy(
    module: Option<&str>,
    strict: bool,
    fix: bool,
    pedantic: bool,
    require_tools: bool,
) -> Result<GateOutcome> {
    println!("{}", "🔍 Running Clippy linter...".bright_cyan().bold());
    println!();

    // Declarations gate first — a fence posture drift is a schema bug, and there
    // is no point paying for a clippy run on a workspace that already fails it.
    match run_schema_declarations_gate().await? {
        GateOutcome::Passed => {}
        GateOutcome::Failed(why) => {
            return Ok(GateOutcome::Failed(format!(
                "schema declarations gate failed — {why}"
            )))
        }
        GateOutcome::Skipped(why) if require_tools => {
            return Ok(GateOutcome::Failed(format!(
                "schema declarations gate did not run: {why} (--require-tools)"
            )))
        }
        GateOutcome::Skipped(_) => {}
    }

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
        println!("  {} Clippy passed!", "✅".green());
        return Ok(GateOutcome::Passed);
    }

    // A non-zero clippy exit means findings remain — including under --fix,
    // which exits zero once it has rewritten everything it can repair.
    println!("  {} Clippy found issues", "❌".red());
    if fix {
        println!(
            "  {} Some issues were fixed; the rest need a human",
            "🔧".yellow()
        );
    } else {
        println!(
            "  {} Run with --fix to auto-fix where possible",
            "💡".bright_blue()
        );
    }

    Ok(GateOutcome::Failed(
        "clippy reported findings that remain unfixed".to_string(),
    ))
}

/// Run rustfmt
async fn run_fmt(module: Option<&str>, check: bool, diff: bool) -> Result<GateOutcome> {
    println!("{}", "🎨 Running rustfmt...".bright_cyan().bold());
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

        Command::new("git").args(["diff", "--stat"]).status()?;
    }

    println!();

    if status.success() {
        if check {
            println!("  {} Code is properly formatted!", "✅".green());
        } else {
            println!("  {} Code formatted!", "✅".green());
        }
        return Ok(GateOutcome::Passed);
    }

    if check {
        println!("  {} Code needs formatting", "❌".red());
        println!("  {} Run without --check to fix", "💡".bright_blue());
        return Ok(GateOutcome::Failed("code is not rustfmt-clean".to_string()));
    }

    // Outside --check a non-zero exit means rustfmt itself could not finish,
    // which is a harder failure than unformatted code.
    println!("  {} rustfmt exited with an error", "❌".red());
    Ok(GateOutcome::Failed(
        "rustfmt could not format the tree".to_string(),
    ))
}

/// Run compilation check
async fn run_compile(module: Option<&str>, release: bool) -> Result<GateOutcome> {
    println!("{}", "🔨 Running compilation check...".bright_cyan().bold());
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
        println!("  {} Compilation successful!", "✅".green());
        return Ok(GateOutcome::Passed);
    }

    println!("  {} Compilation failed", "❌".red());
    Ok(GateOutcome::Failed("the tree does not compile".to_string()))
}

/// Run security audit
async fn run_audit(fix: bool, format: &str) -> Result<GateOutcome> {
    println!("{}", "🔒 Running security audit...".bright_cyan().bold());
    println!();

    // Report a missing tool as a skip instead of installing software behind the
    // caller's back — and never let "no such subcommand" masquerade as a finding.
    if !cargo_subcommand_available("audit") {
        println!(
            "  {} cargo-audit is not installed — skipping the security audit \
             (cargo install cargo-audit)",
            "⏭️".bright_yellow()
        );
        return Ok(GateOutcome::Skipped(
            "cargo-audit is not installed".to_string(),
        ));
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

    // Captured rather than streamed so a tool error can be told apart from a real
    // finding: cargo-audit exits non-zero for both, and reporting vulnerabilities
    // when the advisory database merely failed to parse sends an operator chasing
    // CVEs that were never read.
    let output = Command::new("cargo")
        .args(&args)
        .output()
        .context("Failed to run cargo audit")?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    println!();

    if output.status.success() {
        println!("  {} No known vulnerabilities found!", "✅".green());
        return Ok(GateOutcome::Passed);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if let Some(problem) = tool_error_line(&stderr) {
        println!(
            "  {} cargo-audit could not complete the scan — nothing was audited",
            "⏭️".bright_yellow()
        );
        return Ok(GateOutcome::Skipped(format!(
            "cargo-audit could not run: {problem}"
        )));
    }

    println!("  {} Security vulnerabilities detected", "⚠️".yellow());
    if !fix {
        println!(
            "  {} Run with --fix to attempt auto-fix",
            "💡".bright_blue()
        );
    }

    Ok(GateOutcome::Failed(
        "cargo-audit reported known vulnerabilities".to_string(),
    ))
}

/// First line on which a tool reported that it could not do its job.
///
/// cargo-audit prefixes its own failures with `error:` and reports findings
/// without one, which is the only signal separating "the scan found something"
/// from "the scan never happened".
fn tool_error_line(stderr: &str) -> Option<String> {
    stderr
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("error:") || line.starts_with("error "))
        .map(|line| {
            let line = line.trim_start_matches("error:").trim();
            line.chars().take(160).collect()
        })
}

/// Check for outdated dependencies
async fn run_outdated(direct: bool, compatible: bool) -> Result<()> {
    println!(
        "{}",
        "📦 Checking for outdated dependencies..."
            .bright_cyan()
            .bold()
    );
    println!();

    // Informational command: a missing tool is reported, never installed silently.
    if !cargo_subcommand_available("outdated") {
        println!(
            "  {} cargo-outdated is not installed — nothing to report \
             (cargo install cargo-outdated)",
            "⏭️".bright_yellow()
        );
        return Ok(());
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
async fn run_all_checks(
    module: Option<&str>,
    strict: bool,
    fix: bool,
    require_tools: bool,
) -> Result<()> {
    println!(
        "{}",
        "🔍 Running all quality checks...".bright_cyan().bold()
    );
    println!();

    // (name, outcome, advisory) — an advisory gate is reported but never
    // decides the exit status, unless --strict promotes it.
    let mut gates: Vec<(String, GateOutcome, bool)> = Vec::new();

    // 1. Format check/fix
    println!("{}", "Step 1/4: Code formatting".bright_white().bold());
    gates.push((
        "code formatting".to_string(),
        run_fmt(module, !fix, false).await?,
        false,
    ));
    println!();

    // 2. Compilation check
    println!("{}", "Step 2/4: Compilation check".bright_white().bold());
    gates.push((
        "compilation".to_string(),
        run_compile(module, false).await?,
        false,
    ));
    println!();

    // 3. Clippy
    println!("{}", "Step 3/4: Clippy linting".bright_white().bold());
    gates.push((
        "clippy".to_string(),
        run_clippy(module, strict, fix, false, require_tools).await?,
        false,
    ));
    println!();

    // 4. Security audit — advisory by default because an advisory published
    // upstream today would otherwise break a build that changed nothing.
    // --strict makes it blocking.
    println!("{}", "Step 4/4: Security audit".bright_white().bold());
    gates.push((
        "security audit".to_string(),
        run_audit(false, "text").await?,
        !strict,
    ));

    println!();
    println!("{}", "═".repeat(50).bright_white());
    println!();

    let verdict = summarize(&gates, require_tools);

    if verdict.is_err() && !fix {
        println!(
            "  {} Run with --fix to attempt auto-fixes",
            "💡".bright_blue()
        );
    }

    verdict
}

/// Show clippy configuration
async fn show_config() -> Result<()> {
    println!(
        "{}",
        "📋 Metaphor Framework Clippy Configuration"
            .bright_cyan()
            .bold()
    );
    println!();

    println!("{}", "Denied lints (errors):".bright_white().bold());
    println!("  - clippy::unwrap_used    (use ? or handle errors)");
    println!("  - clippy::expect_used    (use ? or handle errors)");
    println!();

    println!("{}", "Warned lints:".bright_white().bold());
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
        "Allowed lints (framework exceptions):"
            .bright_white()
            .bold()
    );
    println!("  - clippy::module_inception     (we use domain/domain.rs pattern)");
    println!("  - clippy::too_many_arguments   (builder pattern uses many args)");
    println!();

    println!(
        "{}",
        "To add project-specific configuration:"
            .bright_white()
            .bold()
    );
    println!("  Create a clippy.toml in the project root with:");
    println!();
    println!("  ```toml");
    println!("  # clippy.toml");
    println!("  cognitive-complexity-threshold = 25");
    println!("  type-complexity-threshold = 300");
    println!("  ```");
    println!();

    println!("{}", "Or add to Cargo.toml:".bright_white().bold());
    println!();
    println!("  ```toml");
    println!("  [lints.clippy]");
    println!("  unwrap_used = \"deny\"");
    println!("  expect_used = \"deny\"");
    println!("  todo = \"warn\"");
    println!("  ```");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate(name: &str, outcome: GateOutcome, advisory: bool) -> (String, GateOutcome, bool) {
        (name.to_string(), outcome, advisory)
    }

    #[test]
    fn a_failing_gate_is_an_error_not_a_printed_cross() {
        let err = GateOutcome::Failed("clippy reported findings".to_string())
            .into_result("clippy", false)
            .unwrap_err();
        assert!(err.to_string().contains("clippy failed"));
    }

    #[test]
    fn a_passing_gate_is_ok() {
        assert!(GateOutcome::Passed.into_result("clippy", false).is_ok());
    }

    #[test]
    fn a_skipped_gate_passes_by_default_and_fails_under_require_tools() {
        let skipped = || GateOutcome::Skipped("metaphor-schema is not on PATH".to_string());
        assert!(skipped().into_result("clippy", false).is_ok());

        let err = skipped().into_result("clippy", true).unwrap_err();
        assert!(err.to_string().contains("did not run"));
        assert!(err.to_string().contains("metaphor-schema is not on PATH"));
    }

    #[test]
    fn one_failed_gate_fails_the_whole_run() {
        let gates = vec![
            gate("code formatting", GateOutcome::Passed, false),
            gate("compilation", GateOutcome::Passed, false),
            gate(
                "clippy",
                GateOutcome::Failed("findings remain".to_string()),
                false,
            ),
        ];
        let err = summarize(&gates, false).unwrap_err();
        assert!(err.to_string().contains("clippy"));
    }

    #[test]
    fn every_failed_gate_is_named_in_the_verdict() {
        let gates = vec![
            gate(
                "code formatting",
                GateOutcome::Failed("not rustfmt-clean".to_string()),
                false,
            ),
            gate(
                "compilation",
                GateOutcome::Failed("does not compile".to_string()),
                false,
            ),
        ];
        let message = summarize(&gates, false).unwrap_err().to_string();
        assert!(message.contains("code formatting"));
        assert!(message.contains("compilation"));
    }

    #[test]
    fn an_advisory_finding_does_not_fail_the_run() {
        let gates = vec![
            gate("clippy", GateOutcome::Passed, false),
            gate(
                "security audit",
                GateOutcome::Failed("known vulnerabilities".to_string()),
                true,
            ),
        ];
        assert!(summarize(&gates, false).is_ok());
    }

    #[test]
    fn a_blocking_audit_fails_the_run() {
        let gates = vec![gate(
            "security audit",
            GateOutcome::Failed("known vulnerabilities".to_string()),
            false,
        )];
        assert!(summarize(&gates, false).is_err());
    }

    #[test]
    fn a_skipped_gate_only_fails_the_run_when_tools_are_required() {
        let gates = vec![
            gate("clippy", GateOutcome::Passed, false),
            gate(
                "security audit",
                GateOutcome::Skipped("cargo-audit is not installed".to_string()),
                false,
            ),
        ];
        assert!(summarize(&gates, false).is_ok());
        assert!(summarize(&gates, true).is_err());
    }

    #[test]
    fn a_skipped_advisory_gate_never_fails_the_run() {
        let gates = vec![gate(
            "security audit",
            GateOutcome::Skipped("cargo-audit is not installed".to_string()),
            true,
        )];
        assert!(summarize(&gates, true).is_ok());
    }

    #[test]
    fn a_broken_advisory_database_is_not_reported_as_a_finding() {
        let stderr = "    Fetching advisory database\n\
                      error: error loading advisory database: parse error\n";
        let problem = tool_error_line(stderr).expect("the tool error should be recognised");
        assert!(problem.contains("advisory database"));
    }

    #[test]
    fn a_clean_run_has_no_tool_error() {
        assert!(tool_error_line("    Scanning Cargo.lock for vulnerabilities\n").is_none());
    }

    #[test]
    fn an_all_green_run_is_ok() {
        let gates = vec![
            gate("code formatting", GateOutcome::Passed, false),
            gate("compilation", GateOutcome::Passed, false),
            gate("clippy", GateOutcome::Passed, false),
        ];
        assert!(summarize(&gates, true).is_ok());
    }
}
