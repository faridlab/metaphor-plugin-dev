//! Project/workspace discovery for the dev plugin.
//!
//! Resolves the backend-service app the user is targeting so commands like
//! `metaphor dev serve` don't hardcode `apps/metaphor` + `metaphor-app`.
//!
//! Resolution order:
//! 1. Walk up from CWD looking for `metaphor.yaml`.
//!    - If CWD is inside one of the `projects[].path` entries, pick that one.
//!    - Else pick the sole `backend-service` project, or error asking for disambiguation.
//! 2. If no `metaphor.yaml` is found, fall back to CWD if it looks like a
//!    standalone Rust bin crate (has a `Cargo.toml` with a bin target).

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ResolvedProject {
    /// Directory containing `metaphor.yaml`, or the app dir itself if there is none.
    pub root: PathBuf,
    /// Absolute path to the app crate (contains its `Cargo.toml`).
    pub app_dir: PathBuf,
    /// Cargo bin target name to pass to `--bin`.
    pub bin_name: String,
    /// Config directory: `<app_dir>/config`.
    pub config_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
struct MetaphorYaml {
    #[serde(default)]
    projects: Vec<ProjectEntry>,
}

#[derive(Debug, Deserialize)]
struct ProjectEntry {
    name: String,
    #[serde(default, rename = "type")]
    kind: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct CargoToml {
    package: Option<CargoPackage>,
    #[serde(default)]
    bin: Vec<CargoBin>,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CargoBin {
    name: String,
}

pub fn resolve() -> Result<ResolvedProject> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    resolve_from(&cwd)
}

pub fn resolve_from(start: &Path) -> Result<ResolvedProject> {
    if let Some(yaml_path) = find_metaphor_yaml(start) {
        return resolve_with_yaml(&yaml_path, start);
    }
    resolve_standalone(start)
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

fn resolve_with_yaml(yaml_path: &Path, start: &Path) -> Result<ResolvedProject> {
    let root = yaml_path
        .parent()
        .ok_or_else(|| anyhow!("metaphor.yaml has no parent dir"))?
        .to_path_buf();

    let content = std::fs::read_to_string(yaml_path)
        .with_context(|| format!("failed to read {}", yaml_path.display()))?;
    let parsed: MetaphorYaml = serde_yaml::from_str(&content)
        .with_context(|| format!("failed to parse {}", yaml_path.display()))?;

    if parsed.projects.is_empty() {
        return Err(anyhow!(
            "{} has no `projects` entries",
            yaml_path.display()
        ));
    }

    let picked = pick_project(&parsed.projects, &root, start)?;
    let app_dir = root.join(&picked.path);
    if !app_dir.is_dir() {
        return Err(anyhow!(
            "project `{}` path does not exist: {}",
            picked.name,
            app_dir.display()
        ));
    }

    let bin_name = read_bin_name(&app_dir)?;
    let config_dir = app_dir.join("config");

    Ok(ResolvedProject {
        root,
        app_dir,
        bin_name,
        config_dir,
    })
}

fn pick_project<'a>(
    projects: &'a [ProjectEntry],
    root: &Path,
    start: &Path,
) -> Result<&'a ProjectEntry> {
    // Prefer the project whose path contains `start` (user is inside it).
    let start_abs = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    for p in projects {
        let p_abs = root
            .join(&p.path)
            .canonicalize()
            .unwrap_or_else(|_| root.join(&p.path));
        if start_abs.starts_with(&p_abs) {
            return Ok(p);
        }
    }

    // Otherwise pick a sole backend-service.
    let services: Vec<&ProjectEntry> = projects
        .iter()
        .filter(|p| p.kind == "backend-service")
        .collect();
    match services.len() {
        1 => Ok(services[0]),
        0 => Err(anyhow!(
            "no `backend-service` project found in metaphor.yaml; available: {}",
            list_names(projects)
        )),
        _ => Err(anyhow!(
            "multiple backend-service projects found in metaphor.yaml ({}); cd into one or pass --project <name> once supported",
            list_names(&services.iter().map(|p| (*p).clone_shallow()).collect::<Vec<_>>())
        )),
    }
}

impl ProjectEntry {
    fn clone_shallow(&self) -> ProjectEntry {
        ProjectEntry {
            name: self.name.clone(),
            kind: self.kind.clone(),
            path: self.path.clone(),
        }
    }
}

fn list_names(projects: &[ProjectEntry]) -> String {
    projects
        .iter()
        .map(|p| p.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn resolve_standalone(start: &Path) -> Result<ResolvedProject> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let cargo = dir.join("Cargo.toml");
        if cargo.is_file() {
            if let Some(bin_name) = read_bin_name_opt(dir)? {
                let app_dir = dir.to_path_buf();
                return Ok(ResolvedProject {
                    root: app_dir.clone(),
                    config_dir: app_dir.join("config"),
                    app_dir,
                    bin_name,
                });
            }
        }
        cur = dir.parent();
    }
    Err(anyhow!(
        "could not find metaphor.yaml or a Cargo.toml with a bin target above {}",
        start.display()
    ))
}

fn read_bin_name(app_dir: &Path) -> Result<String> {
    read_bin_name_opt(app_dir)?.ok_or_else(|| {
        anyhow!(
            "no bin target found in {}/Cargo.toml",
            app_dir.display()
        )
    })
}

fn read_bin_name_opt(app_dir: &Path) -> Result<Option<String>> {
    let cargo_toml = app_dir.join("Cargo.toml");
    if !cargo_toml.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&cargo_toml)
        .with_context(|| format!("failed to read {}", cargo_toml.display()))?;
    let parsed: CargoToml = toml::from_str(&content)
        .with_context(|| format!("failed to parse {}", cargo_toml.display()))?;
    if let Some(first) = parsed.bin.first() {
        return Ok(Some(first.name.clone()));
    }
    if let Some(pkg) = parsed.package {
        // Cargo auto-discovers src/main.rs → bin named after the package.
        if app_dir.join("src/main.rs").is_file() {
            return Ok(Some(pkg.name));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
    }

    fn sample_cargo(name: &str) -> String {
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
        )
    }

    #[test]
    fn resolves_sole_backend_service_from_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("metaphor.yaml"),
            "version: 1\nprojects:\n- name: svc\n  type: backend-service\n  path: apps/svc\n",
        );
        write(&root.join("apps/svc/Cargo.toml"), &sample_cargo("svc"));
        write(&root.join("apps/svc/src/main.rs"), "fn main() {}\n");

        let project = resolve_from(root).unwrap();
        assert_eq!(project.bin_name, "svc");
        assert_eq!(project.app_dir, root.join("apps/svc"));
        assert_eq!(project.config_dir, root.join("apps/svc/config"));
    }

    #[test]
    fn resolves_when_cwd_is_inside_app_dir() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("metaphor.yaml"),
            "version: 1\nprojects:\n- name: alpha\n  type: backend-service\n  path: apps/alpha\n- name: beta\n  type: backend-service\n  path: apps/beta\n",
        );
        write(&root.join("apps/alpha/Cargo.toml"), &sample_cargo("alpha"));
        write(&root.join("apps/alpha/src/main.rs"), "fn main() {}\n");
        write(&root.join("apps/beta/Cargo.toml"), &sample_cargo("beta"));
        write(&root.join("apps/beta/src/main.rs"), "fn main() {}\n");

        let project = resolve_from(&root.join("apps/beta")).unwrap();
        assert_eq!(project.bin_name, "beta");
    }

    #[test]
    fn errors_on_ambiguous_multiple_services() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("metaphor.yaml"),
            "version: 1\nprojects:\n- name: a\n  type: backend-service\n  path: apps/a\n- name: b\n  type: backend-service\n  path: apps/b\n",
        );
        write(&root.join("apps/a/Cargo.toml"), &sample_cargo("a"));
        write(&root.join("apps/b/Cargo.toml"), &sample_cargo("b"));

        let err = resolve_from(root).unwrap_err().to_string();
        assert!(err.contains("multiple backend-service"));
    }

    #[test]
    fn prefers_explicit_bin_over_package_name() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("metaphor.yaml"),
            "version: 1\nprojects:\n- name: svc\n  type: backend-service\n  path: apps/svc\n",
        );
        write(
            &root.join("apps/svc/Cargo.toml"),
            "[package]\nname = \"svc\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"svc-app\"\npath = \"src/main.rs\"\n",
        );
        write(&root.join("apps/svc/src/main.rs"), "fn main() {}\n");

        let project = resolve_from(root).unwrap();
        assert_eq!(project.bin_name, "svc-app");
    }

    #[test]
    fn falls_back_to_standalone_cargo_project() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write(&dir.join("Cargo.toml"), &sample_cargo("lonely"));
        write(&dir.join("src/main.rs"), "fn main() {}\n");

        let project = resolve_from(dir).unwrap();
        assert_eq!(project.bin_name, "lonely");
        assert_eq!(project.app_dir, dir);
    }
}
