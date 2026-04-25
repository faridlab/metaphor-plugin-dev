//! Append-only deployment history + env-file snapshots.
//!
//! On every `deploy push` and `deploy rollback`, we append a record to
//! `<workspace>/deployment/history/<env>.jsonl` and copy the env file used
//! to `<workspace>/deployment/history/snapshots/.env.<env>.<ts>-<sha>`.
//! After a successful remote step, we mirror both to the server under
//! `<deploy_dir>/history/`.
//!
//! The local file is the source of truth (committed to git, survives laptop
//! loss). The server mirror is for ops convenience.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Push,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Success,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub ts: DateTime<Utc>,
    pub action: Action,
    pub status: Status,
    pub tag: String,
    pub image_tags: BTreeMap<String, String>,
    pub deployer: String,
    /// Filename (basename) of the env-file snapshot, if one was written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    /// For rollback: the tag we rolled away from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_from_tag: Option<String>,
    /// Short error message if status == Failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl HistoryRecord {
    pub fn new_push(
        tag: String,
        image_tags: BTreeMap<String, String>,
        snapshot: Option<String>,
    ) -> Self {
        Self {
            ts: Utc::now(),
            action: Action::Push,
            status: Status::Success,
            tag,
            image_tags,
            deployer: deployer_id(),
            snapshot,
            rollback_from_tag: None,
            error: None,
        }
    }

    pub fn new_rollback(
        from_tag: String,
        to_tag: String,
        image_tags: BTreeMap<String, String>,
        snapshot: Option<String>,
    ) -> Self {
        Self {
            ts: Utc::now(),
            action: Action::Rollback,
            status: Status::Success,
            tag: to_tag,
            image_tags,
            deployer: deployer_id(),
            snapshot,
            rollback_from_tag: Some(from_tag),
            error: None,
        }
    }

    pub fn with_failure(mut self, err: &str) -> Self {
        self.status = Status::Failed;
        // Keep error short to avoid bloating the JSONL.
        self.error = Some(truncate(err, 240));
        self
    }
}

pub fn history_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("deployment/history")
}

pub fn snapshots_dir(workspace_root: &Path) -> PathBuf {
    history_dir(workspace_root).join("snapshots")
}

pub fn history_file(workspace_root: &Path, env_name: &str) -> PathBuf {
    history_dir(workspace_root).join(format!("{env_name}.jsonl"))
}

/// Write a snapshot of the env file into `deployment/history/snapshots/`.
/// Returns the basename (e.g. `.env.prod.20260425140200-abc1234`).
pub fn write_snapshot(
    workspace_root: &Path,
    env_name: &str,
    env_file_content: &str,
    sha: &str,
) -> Result<String> {
    let dir = snapshots_dir(workspace_root);
    fs::create_dir_all(&dir)
        .with_context(|| format!("creating {}", dir.display()))?;
    let stamp = Utc::now().format("%Y%m%d%H%M%S");
    let basename = format!(".env.{env_name}.{stamp}-{sha}");
    let path = dir.join(&basename);
    fs::write(&path, env_file_content)
        .with_context(|| format!("writing snapshot {}", path.display()))?;
    Ok(basename)
}

/// Append one record to `deployment/history/<env>.jsonl`.
pub fn append_record(workspace_root: &Path, env_name: &str, record: &HistoryRecord) -> Result<()> {
    let dir = history_dir(workspace_root);
    fs::create_dir_all(&dir)
        .with_context(|| format!("creating {}", dir.display()))?;
    let path = history_file(workspace_root, env_name);
    let json = serde_json::to_string(record).context("serializing history record")?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening {} for append", path.display()))?;
    writeln!(file, "{json}").with_context(|| format!("writing to {}", path.display()))?;
    Ok(())
}

/// Read all records, oldest → newest (file order).
pub fn read_records(workspace_root: &Path, env_name: &str) -> Result<Vec<HistoryRecord>> {
    let path = history_file(workspace_root, env_name);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    parse_jsonl(&raw, &path)
}

pub fn parse_jsonl(raw: &str, source_for_errors: &Path) -> Result<Vec<HistoryRecord>> {
    let mut out = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let rec: HistoryRecord = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "parsing history record line {} in {}",
                i + 1,
                source_for_errors.display()
            )
        })?;
        out.push(rec);
    }
    Ok(out)
}

/// Find the tag of the n-th most recent **successful push**, where step=1 means
/// "the push immediately before the current one". The current state is the
/// most recent successful push (step=0).
///
/// Failed pushes and rollbacks are skipped: rollbacks would create a loop, and
/// failed pushes never actually changed the deployed state.
pub fn find_previous_successful_push(
    records: &[HistoryRecord],
    steps_back: usize,
) -> Option<&HistoryRecord> {
    let successes: Vec<&HistoryRecord> = records
        .iter()
        .rev()
        .filter(|r| r.action == Action::Push && r.status == Status::Success)
        .collect();
    successes.get(steps_back).copied()
}

/// Return the most recent record overall (push or rollback), used to determine
/// "what tag is currently deployed" for `rollback_from_tag`. Successful only.
pub fn current_deployed_tag(records: &[HistoryRecord]) -> Option<String> {
    records
        .iter()
        .rev()
        .find(|r| r.status == Status::Success)
        .map(|r| r.tag.clone())
}

fn deployer_id() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let host = hostname_or_unknown();
    format!("{user}@{host}")
}

fn hostname_or_unknown() -> String {
    // No hostname crate; shell out. Failure is non-fatal.
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Pretty-print the table form used by `metaphor deploy history <env>`.
pub fn render_table(records: &[HistoryRecord], limit: Option<usize>) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let view: Vec<&HistoryRecord> = records.iter().rev().collect();
    let view: &[&HistoryRecord] = match limit {
        Some(n) => &view[..view.len().min(n)],
        None => &view[..],
    };

    if view.is_empty() {
        return "(no deployment history yet)\n".to_string();
    }
    writeln!(
        out,
        "{:<20}  {:<8}  {:<10}  {:<3}  {}",
        "TIMESTAMP (UTC)", "ACTION", "TAG", "OK", "DEPLOYER"
    )
    .unwrap();
    for r in view {
        let ts = r.ts.format("%Y-%m-%d %H:%M:%S").to_string();
        let action = match r.action {
            Action::Push => "push",
            Action::Rollback => "rollback",
        };
        let ok = match r.status {
            Status::Success => "✓",
            Status::Failed => "✗",
        };
        writeln!(
            out,
            "{:<20}  {:<8}  {:<10}  {:<3}  {}",
            ts, action, r.tag, ok, r.deployer
        )
        .unwrap();
        if let Some(err) = &r.error {
            writeln!(out, "    error: {err}").unwrap();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn rec(action: Action, status: Status, tag: &str) -> HistoryRecord {
        HistoryRecord {
            ts: Utc::now(),
            action,
            status,
            tag: tag.into(),
            image_tags: BTreeMap::new(),
            deployer: "alice@laptop".into(),
            snapshot: None,
            rollback_from_tag: None,
            error: None,
        }
    }

    #[test]
    fn append_and_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let r = rec(Action::Push, Status::Success, "abc1234");
        append_record(tmp.path(), "prod", &r).unwrap();
        append_record(tmp.path(), "prod", &r).unwrap();
        let back = read_records(tmp.path(), "prod").unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].tag, "abc1234");
    }

    #[test]
    fn missing_file_returns_empty() {
        let tmp = TempDir::new().unwrap();
        assert!(read_records(tmp.path(), "prod").unwrap().is_empty());
    }

    #[test]
    fn previous_successful_push_skips_failures_and_rollbacks() {
        let records = vec![
            rec(Action::Push, Status::Success, "v1"),
            rec(Action::Push, Status::Failed, "broken"), // skipped
            rec(Action::Push, Status::Success, "v2"),
            rec(Action::Rollback, Status::Success, "v1"), // skipped
            rec(Action::Push, Status::Success, "v3"),     // current (step 0)
        ];
        // step=0 is current, step=1 is the one before
        assert_eq!(
            find_previous_successful_push(&records, 0).unwrap().tag,
            "v3"
        );
        assert_eq!(
            find_previous_successful_push(&records, 1).unwrap().tag,
            "v2"
        );
        assert_eq!(
            find_previous_successful_push(&records, 2).unwrap().tag,
            "v1"
        );
        assert!(find_previous_successful_push(&records, 3).is_none());
    }

    #[test]
    fn current_deployed_tag_picks_latest_success() {
        let records = vec![
            rec(Action::Push, Status::Success, "v1"),
            rec(Action::Push, Status::Failed, "broken"),
            rec(Action::Rollback, Status::Success, "v0"),
        ];
        assert_eq!(current_deployed_tag(&records), Some("v0".to_string()));
    }

    #[test]
    fn snapshot_writes_to_disk() {
        let tmp = TempDir::new().unwrap();
        let name =
            write_snapshot(tmp.path(), "prod", "FOO=bar\nBAZ=qux\n", "abc1234").unwrap();
        assert!(name.starts_with(".env.prod."));
        assert!(name.ends_with("-abc1234"));
        let content = fs::read_to_string(snapshots_dir(tmp.path()).join(&name)).unwrap();
        assert!(content.contains("FOO=bar"));
    }

    #[test]
    fn render_table_handles_empty() {
        assert!(render_table(&[], None).contains("(no deployment history yet)"));
    }

    #[test]
    fn render_table_shows_newest_first_with_limit() {
        let records = vec![
            rec(Action::Push, Status::Success, "v1"),
            rec(Action::Push, Status::Success, "v2"),
            rec(Action::Push, Status::Success, "v3"),
        ];
        let out = render_table(&records, Some(2));
        // newest first → v3 should appear before v2; v1 not present
        let i3 = out.find("v3").unwrap();
        let i2 = out.find("v2").unwrap();
        assert!(i3 < i2);
        assert!(!out.contains("v1"));
    }

    #[test]
    fn parse_jsonl_rejects_garbage_with_line_number() {
        let raw = r#"{"ts":"2026-04-25T00:00:00Z","action":"push","status":"success","tag":"v1","image_tags":{},"deployer":"alice@laptop"}
this is not json"#;
        let p = std::path::Path::new("test.jsonl");
        let err = parse_jsonl(raw, p).unwrap_err().to_string();
        assert!(err.contains("line 2"));
    }
}
