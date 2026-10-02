//! `metaphor dev link` — build a service against a local checkout of a module before it is
//! released (ADR-0030, Decision 9).
//!
//! A link is a `[patch.crates-io]` entry in the service's untracked `.cargo/config.toml`,
//! pointing at the module checkout by a path relative to the service. The relative form
//! resolves on the host and, with the metaphora checkout mounted at `/frameworks/metaphora`,
//! inside the dev container too (`/app` + `../../../../frameworks/metaphora/...`).
//!
//! Cargo rewrites `Cargo.lock` while a link is active, so the first link saves the lock and
//! the last unlink puts it back byte for byte; a lock that still names a path source fails
//! the pin probe in CI, which is the second line of defence against committing one.

use anyhow::{anyhow, bail, Context, Result};
use colored::*;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const CONFIG: &str = ".cargo/config.toml";
const LOCK_BACKUP: &str = ".cargo/Cargo.lock.before-link";
const HEADER: &str = "# Local module links written by `metaphor dev link`. Untracked on purpose:\n\
                      # remove them with `metaphor dev unlink <module>` (or `--all`).\n";

/// One `[patch.crates-io]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub krate: String,
    pub path: String,
}

/// The links currently in `<app>/.cargo/config.toml`.
pub fn read_links(app_dir: &Path) -> Result<Vec<Link>> {
    let table = read_config(app_dir)?;
    let Some(patch) = table
        .get("patch")
        .and_then(|p| p.get("crates-io"))
        .and_then(|c| c.as_table())
    else {
        return Ok(Vec::new());
    };
    Ok(patch
        .iter()
        .filter_map(|(k, v)| {
            v.get("path")
                .and_then(|p| p.as_str())
                .map(|p| Link { krate: k.clone(), path: p.to_string() })
        })
        .collect())
}

fn read_config(app_dir: &Path) -> Result<toml::Table> {
    let path = app_dir.join(CONFIG);
    if !path.exists() {
        return Ok(toml::Table::new());
    }
    let text = std::fs::read_to_string(&path)?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn write_config(app_dir: &Path, table: &toml::Table) -> Result<()> {
    let path = app_dir.join(CONFIG);
    if table.is_empty() {
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        return Ok(());
    }
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, format!("{HEADER}\n{}", toml::to_string(table)?))?;
    Ok(())
}

/// Add or replace the link for `krate`. Other keys in the config are kept.
pub fn set_link(app_dir: &Path, link: &Link) -> Result<()> {
    let mut table = read_config(app_dir)?;
    let patch = table
        .entry("patch")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or_else(|| anyhow!("`patch` in {CONFIG} is not a table"))?;
    let crates_io = patch
        .entry("crates-io")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or_else(|| anyhow!("`patch.crates-io` in {CONFIG} is not a table"))?;
    let mut entry = toml::Table::new();
    entry.insert("path".into(), toml::Value::String(link.path.clone()));
    crates_io.insert(link.krate.clone(), toml::Value::Table(entry));
    write_config(app_dir, &table)
}

/// Remove the link for `krate` (every link when `None`). Returns how many links remain.
pub fn remove_links(app_dir: &Path, krate: Option<&str>) -> Result<usize> {
    let mut table = read_config(app_dir)?;
    let mut remaining = 0;
    if let Some(patch) = table.get_mut("patch").and_then(|p| p.as_table_mut()) {
        if let Some(crates_io) = patch.get_mut("crates-io").and_then(|c| c.as_table_mut()) {
            match krate {
                Some(k) => {
                    if crates_io.remove(k).is_none() {
                        bail!("{k} is not linked");
                    }
                }
                None => crates_io.clear(),
            }
            remaining = crates_io.len();
            if crates_io.is_empty() {
                patch.remove("crates-io");
            }
        }
        if patch.is_empty() {
            table.remove("patch");
        }
    } else if let Some(k) = krate {
        bail!("{k} is not linked");
    }
    write_config(app_dir, &table)?;
    Ok(remaining)
}

/// `to` expressed relative to `from` (both absolute), e.g. `../../../../frameworks/metaphora/…`.
pub fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from: Vec<Component> = from.components().collect();
    let to: Vec<Component> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut rel = PathBuf::new();
    for _ in common..from.len() {
        rel.push("..");
    }
    for c in &to[common..] {
        rel.push(c.as_os_str());
    }
    rel
}

/// The package name declared by the module checkout at `dir`.
pub fn package_name(dir: &Path) -> Result<String> {
    let manifest = dir.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest)
        .with_context(|| format!("{} has no Cargo.toml", dir.display()))?;
    let table: toml::Table = toml::from_str(&text)?;
    table
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("{} declares no [package] name", manifest.display()))
}

/// The metaphora checkout next to the workspace: `<root>/../../frameworks/metaphora`, the
/// layout the pin probe assumes. `METAPHORA_DIR` overrides it.
fn metaphora_dir(workspace_root: &Path) -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("METAPHORA_DIR") {
        return Some(PathBuf::from(dir));
    }
    workspace_root
        .parent()
        .and_then(Path::parent)
        .map(|p| p.join("frameworks/metaphora"))
        .filter(|p| p.is_dir())
}

fn ensure_ignored(app_dir: &Path) -> Result<()> {
    let out = Command::new("git")
        .args(["check-ignore", "-q", CONFIG])
        .current_dir(app_dir)
        .status()
        .context("spawning git check-ignore")?;
    if !out.success() {
        bail!(
            "{CONFIG} is not git-ignored in {}; add `.cargo/config.toml` to its .gitignore first, \
             so a local link can never be committed",
            app_dir.display()
        );
    }
    Ok(())
}

/// Whether `cargo metadata` in `app_dir` resolves `krate` from a path under `module_dir`.
fn resolves_locally(app_dir: &Path, krate: &str, module_dir: &Path) -> Result<bool> {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .current_dir(app_dir)
        .output()
        .context("spawning cargo metadata")?;
    if !out.status.success() {
        bail!("cargo metadata failed:\n{}", String::from_utf8_lossy(&out.stderr));
    }
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    let module_dir = module_dir.canonicalize()?;
    Ok(meta["packages"].as_array().into_iter().flatten().any(|p| {
        p["name"] == krate
            && p["source"].is_null()
            && p["manifest_path"]
                .as_str()
                .and_then(|m| Path::new(m).canonicalize().ok())
                .is_some_and(|m| m.starts_with(&module_dir))
    }))
}

/// `metaphor dev link <module> [--path <dir>]`.
pub fn link(module: &str, path: Option<&Path>) -> Result<()> {
    let project = crate::project::resolve()?;
    let app_dir = project.app_dir.canonicalize()?;
    ensure_ignored(&app_dir)?;
    let module_dir = match path {
        Some(p) => p.to_path_buf(),
        None => metaphora_dir(&project.root)
            .ok_or_else(|| anyhow!("no metaphora checkout next to the workspace; pass --path or set METAPHORA_DIR"))?
            .join("modules")
            .join(module),
    };
    let module_dir = module_dir
        .canonicalize()
        .with_context(|| format!("{} does not exist", module_dir.display()))?;
    let krate = package_name(&module_dir)?;

    let lock = app_dir.join("Cargo.lock");
    let backup = app_dir.join(LOCK_BACKUP);
    if read_links(&app_dir)?.is_empty() && lock.exists() {
        std::fs::create_dir_all(backup.parent().unwrap())?;
        std::fs::copy(&lock, &backup)?;
    }
    let rel = relative_path(&app_dir, &module_dir);
    set_link(&app_dir, &Link { krate: krate.clone(), path: rel.to_string_lossy().into_owned() })?;

    // A local version other than the locked one is only picked up once the lock moves to it.
    if !resolves_locally(&app_dir, &krate, &module_dir)? {
        let status = Command::new("cargo")
            .args(["update", "-q", "-p", &krate])
            .current_dir(&app_dir)
            .status()
            .context("spawning cargo update")?;
        if !status.success() {
            let remaining = remove_links(&app_dir, Some(&krate))?;
            restore_lock_if_last(&app_dir, remaining)?;
            bail!("cargo update -p {krate} failed with the link in place; link removed");
        }
    }
    if !resolves_locally(&app_dir, &krate, &module_dir)? {
        let remaining = remove_links(&app_dir, Some(&krate))?;
        restore_lock_if_last(&app_dir, remaining)?;
        bail!(
            "{krate} at {} is not used by the build: the service may not depend on it, or the \
             local version does not satisfy the service's requirement; link removed",
            module_dir.display()
        );
    }
    println!("{} {krate} → {}", "linked".bright_green().bold(), rel.display());
    if !rel.starts_with("../../../../frameworks/metaphora") {
        println!(
            "  {} this path is outside the sibling metaphora checkout, so only host builds see it",
            "note:".yellow()
        );
    }
    println!("  Cargo.lock now names the local copy; `metaphor dev unlink {module}` restores it.");
    Ok(())
}

/// `metaphor dev unlink <module>` / `--all`.
pub fn unlink(module: Option<&str>) -> Result<()> {
    let project = crate::project::resolve()?;
    let app_dir = project.app_dir.canonicalize()?;
    let krate = match module {
        Some(m) => {
            let links = read_links(&app_dir)?;
            // Accept the module (directory) name or the crate name.
            links
                .iter()
                .find(|l| l.krate == m || Path::new(&l.path).file_name().is_some_and(|f| f == m))
                .map(|l| l.krate.clone())
                .ok_or_else(|| anyhow!("{m} is not linked"))?
                .into()
        }
        None => None,
    };
    let remaining = remove_links(&app_dir, krate.as_deref())?;
    restore_lock_if_last(&app_dir, remaining)?;
    println!(
        "{} {}",
        "unlinked".bright_green().bold(),
        krate.as_deref().unwrap_or("every module")
    );
    Ok(())
}

fn restore_lock_if_last(app_dir: &Path, remaining: usize) -> Result<()> {
    let backup = app_dir.join(LOCK_BACKUP);
    if remaining == 0 && backup.exists() {
        std::fs::rename(&backup, app_dir.join("Cargo.lock"))?;
        println!("  Cargo.lock restored to its state before the first link.");
    }
    Ok(())
}

/// `metaphor dev links`.
pub fn list() -> Result<()> {
    let project = crate::project::resolve()?;
    let links = read_links(&project.app_dir)?;
    if links.is_empty() {
        println!("No modules linked.");
    }
    for l in links {
        println!("  {} → {}", l.krate, l.path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(k: &str, p: &str) -> Link {
        Link { krate: k.into(), path: p.into() }
    }

    #[test]
    fn relative_path_climbs_to_the_common_ancestor() {
        let rel = relative_path(
            Path::new("/u/startapp/products/serpa-workspace/apps/serpa-service"),
            Path::new("/u/startapp/frameworks/metaphora/modules/backbone-maintenance"),
        );
        assert_eq!(rel, PathBuf::from("../../../../frameworks/metaphora/modules/backbone-maintenance"));
        // The same relative path from the container's /app lands on the mount point.
        let mut resolved = PathBuf::from("/");
        for c in Path::new("/app").join(&rel).components() {
            match c {
                Component::ParentDir => { resolved.pop(); }
                Component::Normal(n) => resolved.push(n),
                _ => {}
            }
        }
        assert_eq!(resolved, PathBuf::from("/frameworks/metaphora/modules/backbone-maintenance"));
    }

    #[test]
    fn links_are_added_replaced_and_removed_without_touching_other_settings() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".cargo")).unwrap();
        std::fs::write(tmp.path().join(CONFIG), "[build]\njobs = 4\n").unwrap();

        set_link(tmp.path(), &link("backbone-cmms", "../m/backbone-maintenance")).unwrap();
        set_link(tmp.path(), &link("backbone-sapiens", "../m/backbone-sapiens")).unwrap();
        set_link(tmp.path(), &link("backbone-cmms", "../other/backbone-maintenance")).unwrap();
        let mut links = read_links(tmp.path()).unwrap();
        links.sort_by(|a, b| a.krate.cmp(&b.krate));
        assert_eq!(links, vec![link("backbone-cmms", "../other/backbone-maintenance"), link("backbone-sapiens", "../m/backbone-sapiens")]);

        assert_eq!(remove_links(tmp.path(), Some("backbone-cmms")).unwrap(), 1);
        assert!(remove_links(tmp.path(), Some("backbone-cmms")).is_err());
        assert_eq!(remove_links(tmp.path(), None).unwrap(), 0);
        let text = std::fs::read_to_string(tmp.path().join(CONFIG)).unwrap();
        assert!(text.contains("jobs = 4") && !text.contains("patch"), "{text}");
    }

    #[test]
    fn the_config_file_goes_away_with_its_last_link() {
        let tmp = tempfile::tempdir().unwrap();
        set_link(tmp.path(), &link("backbone-cmms", "../x")).unwrap();
        assert!(tmp.path().join(CONFIG).exists());
        remove_links(tmp.path(), None).unwrap();
        assert!(!tmp.path().join(CONFIG).exists());
    }

    #[test]
    fn package_name_reads_the_crate_a_module_directory_publishes() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"backbone-cmms\"\nversion = \"0.7.23\"\n").unwrap();
        assert_eq!(package_name(tmp.path()).unwrap(), "backbone-cmms");
    }
}
