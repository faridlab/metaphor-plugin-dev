//! `metaphor chaos` — run the workspace's chaos gate.
//!
//! Thin forwarder to the workspace's fault-injection kit at
//! `deployment/chaos/run.sh` (practice doc: the workspace's
//! `docs/chaos/README.md`):
//!   - resolves the workspace root by walking up to `metaphor.yaml`
//!   - refuses loudly when the workspace ships no kit
//!   - forwards every argument verbatim — the bash driver stays the single
//!     source of truth for the catalog, the flags (`--dry-run`, `--full`)
//!     and the verdict semantics (`chaos_exit=0|1`)
//!
//! No fault logic lives in Rust on purpose: experiments are bash + docker
//! compose so they stay auditable side-by-side with the runlogs they write.

use anyhow::{bail, Context, Result};
use colored::*;
use std::process::Command;

use crate::project;

/// Forward `metaphor chaos <args…>` to `deployment/chaos/run.sh <args…>`.
///
/// Exits with the driver's exit code so CI and agents can gate on it.
pub async fn handle_chaos_command(args: &[String]) -> Result<()> {
    // The chaos kit is workspace-level: it must resolve in multi-app
    // workspaces too, where `project::resolve()` would demand a single
    // backend-service project.
    let root = project::workspace_root()?;
    let run_sh = root.join("deployment").join("chaos").join("run.sh");

    if !run_sh.is_file() {
        bail!(
            "this workspace ships no chaos kit (expected {})\n  workspaces created before the kit shipped can copy deployment/chaos/ from the metaphor-workspace template; see docs/chaos/README.md for the practice",
            run_sh.display()
        );
    }

    eprintln!("{} chaos kit: {}", "→".bright_black(), run_sh.display());

    let status = Command::new("bash")
        .arg(&run_sh)
        .args(args)
        .current_dir(&root)
        .status()
        .with_context(|| "failed to spawn the chaos driver — is `bash` available?")?;

    match status.code() {
        Some(0) => Ok(()),
        Some(code) => std::process::exit(code),
        None => bail!("chaos driver terminated by a signal"),
    }
}
