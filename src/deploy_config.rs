//! `metaphor.deploy.yaml` — deployment targets for `metaphor docker` and
//! `metaphor deploy`.
//!
//! Lives at workspace root next to `metaphor.yaml`. Defines named environments
//! and the images each one builds/runs. `docker *` commands operate on a local
//! environment (no `host`); `deploy *` commands operate on a remote environment
//! (has `host`, reached over SSH).
//!
//! Example:
//! ```yaml
//! version: 1
//! defaults:
//!   registry: ghcr.io/faridlab
//!   compose_file: deployment/compose.yaml
//!   ssh_user: deploy
//!   deploy_dir: /srv/app
//!   migrate_command: "metaphor migration run-all"
//!
//! environments:
//!   dev:
//!     env_file: .env.dev
//!     images:
//!       api:
//!         context: apps/api
//!         tag_env: SERVICE_TAG
//!
//!   prod:
//!     host: prod.example.com
//!     env_file: .env.prod
//!     require_confirm: true
//!     images:
//!       api:
//!         context: apps/api
//!         tag_env: SERVICE_TAG
//!         push: true
//! ```

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const DEPLOY_FILE: &str = "metaphor.deploy.yaml";

#[derive(Debug, Deserialize)]
pub struct DeployManifest {
    #[serde(default = "default_version")]
    pub version: u32,

    #[serde(default)]
    pub defaults: Defaults,

    pub environments: BTreeMap<String, EnvironmentSpec>,
}

fn default_version() -> u32 { 1 }

#[derive(Debug, Default, Deserialize)]
pub struct Defaults {
    pub registry: Option<String>,
    pub compose_file: Option<String>,
    pub ssh_user: Option<String>,
    pub deploy_dir: Option<String>,
    pub migrate_command: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EnvironmentSpec {
    /// When present, treat this env as a remote target and shell commands over SSH.
    /// When absent, commands run locally (suitable for `metaphor docker *`).
    pub host: Option<String>,

    pub ssh_user: Option<String>,

    /// Directory on the remote host holding the compose file + env file.
    pub deploy_dir: Option<String>,

    /// Compose file path relative to workspace root (local) or `deploy_dir` (remote).
    pub compose_file: Option<String>,

    /// `.env` file path relative to workspace root (local) or `deploy_dir` (remote).
    pub env_file: Option<String>,

    pub registry: Option<String>,

    /// If true, `metaphor deploy push` prompts for confirmation before pushing.
    #[serde(default)]
    pub require_confirm: bool,

    #[serde(default)]
    pub images: BTreeMap<String, ImageSpec>,
}

#[derive(Debug, Deserialize)]
pub struct ImageSpec {
    /// Build context directory, relative to workspace root.
    pub context: String,

    /// Optional Dockerfile path relative to `context`.
    pub dockerfile: Option<String>,

    /// Registry override — falls back to env registry, then defaults.registry.
    pub registry: Option<String>,

    /// Image name (without registry/tag). Defaults to the map key.
    pub name: Option<String>,

    /// The env var in the env_file that tracks this image's tag.
    /// E.g. `SERVICE_TAG`. Used by `deploy push` to update the file.
    pub tag_env: Option<String>,

    /// Extra docker `--build-arg` pairs baked in at build time.
    #[serde(default)]
    pub build_args: BTreeMap<String, String>,

    /// Whether to push this image during `deploy push`. Defaults true for remote envs.
    pub push: Option<bool>,
}

#[derive(Debug)]
pub struct Resolved {
    pub workspace_root: PathBuf,
    pub manifest: DeployManifest,
}

impl Resolved {
    /// Load `metaphor.deploy.yaml` starting at CWD.
    pub fn load() -> Result<Self> {
        let cwd = std::env::current_dir().context("failed to read current directory")?;
        Self::load_from(&cwd)
    }

    pub fn load_from(start: &Path) -> Result<Self> {
        let yaml_path = find_deploy_yaml(start).ok_or_else(|| {
            anyhow!(
                "{} not found — create one at the workspace root (next to metaphor.yaml)",
                DEPLOY_FILE
            )
        })?;
        let workspace_root = yaml_path
            .parent()
            .ok_or_else(|| anyhow!("{} has no parent dir", DEPLOY_FILE))?
            .to_path_buf();

        let raw = std::fs::read_to_string(&yaml_path)
            .with_context(|| format!("failed to read {}", yaml_path.display()))?;
        let manifest: DeployManifest = serde_yaml::from_str(&raw)
            .with_context(|| format!("failed to parse {}", yaml_path.display()))?;

        if manifest.version != 1 {
            return Err(anyhow!(
                "{}: unsupported version {}, expected 1",
                DEPLOY_FILE,
                manifest.version
            ));
        }
        if manifest.environments.is_empty() {
            return Err(anyhow!("{}: no environments defined", DEPLOY_FILE));
        }

        Ok(Self {
            workspace_root,
            manifest,
        })
    }

    pub fn environment(&self, name: &str) -> Result<&EnvironmentSpec> {
        self.manifest.environments.get(name).ok_or_else(|| {
            let available: Vec<&str> =
                self.manifest.environments.keys().map(|s| s.as_str()).collect();
            anyhow!(
                "environment '{}' not found in {}; available: {}",
                name,
                DEPLOY_FILE,
                available.join(", ")
            )
        })
    }

    /// Compose file resolved to an absolute path on the local workspace.
    /// For remote envs this is the path the operator uses when cp-ing files
    /// to the server; not the path docker uses on the remote host.
    pub fn local_compose_file(&self, env: &EnvironmentSpec) -> PathBuf {
        let rel = env
            .compose_file
            .clone()
            .or_else(|| self.manifest.defaults.compose_file.clone())
            .unwrap_or_else(|| "deployment/compose.yaml".to_string());
        self.workspace_root.join(rel)
    }

    /// Env file resolved to an absolute path on the local workspace.
    pub fn local_env_file(&self, env: &EnvironmentSpec, env_name: &str) -> PathBuf {
        let rel = env
            .env_file
            .clone()
            .unwrap_or_else(|| format!(".env.{env_name}"));
        self.workspace_root.join(rel)
    }

    /// Compose file path on the remote host (relative paths resolved against deploy_dir).
    pub fn remote_compose_file(&self, env: &EnvironmentSpec) -> String {
        env.compose_file
            .clone()
            .or_else(|| self.manifest.defaults.compose_file.clone())
            .unwrap_or_else(|| "compose.yaml".to_string())
    }

    pub fn remote_env_file(&self, env: &EnvironmentSpec, env_name: &str) -> String {
        env.env_file
            .clone()
            .unwrap_or_else(|| format!(".env.{env_name}"))
    }

    pub fn deploy_dir(&self, env: &EnvironmentSpec) -> Result<String> {
        env.deploy_dir
            .clone()
            .or_else(|| self.manifest.defaults.deploy_dir.clone())
            .ok_or_else(|| {
                anyhow!("deploy_dir not set for remote environment (add to defaults or environments.<name>)")
            })
    }

    pub fn ssh_user(&self, env: &EnvironmentSpec) -> Option<String> {
        env.ssh_user
            .clone()
            .or_else(|| self.manifest.defaults.ssh_user.clone())
    }

    pub fn registry(&self, env: &EnvironmentSpec) -> Option<String> {
        env.registry
            .clone()
            .or_else(|| self.manifest.defaults.registry.clone())
    }
}

fn find_deploy_yaml(start: &Path) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let candidate = dir.join(DEPLOY_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
        cur = dir.parent();
    }
    None
}

/// "local" environments have no `host` — docker commands run on the caller's machine.
pub fn is_local(env: &EnvironmentSpec) -> bool {
    env.host.is_none()
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

    const SAMPLE: &str = r#"
version: 1
defaults:
  registry: ghcr.io/me
  compose_file: deployment/compose.yaml
  ssh_user: deploy
  deploy_dir: /srv/app
environments:
  dev:
    env_file: .env.dev
    images:
      api:
        context: apps/api
        tag_env: SERVICE_TAG
  prod:
    host: prod.example.com
    require_confirm: true
    env_file: .env.prod
    images:
      api:
        context: apps/api
        tag_env: SERVICE_TAG
        push: true
"#;

    #[test]
    fn loads_and_resolves_paths() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(&root.join("metaphor.deploy.yaml"), SAMPLE);

        let r = Resolved::load_from(root).unwrap();
        assert_eq!(r.workspace_root, root);
        assert!(r.manifest.environments.contains_key("dev"));
        assert!(r.manifest.environments.contains_key("prod"));

        let dev = r.environment("dev").unwrap();
        assert!(is_local(dev));
        assert_eq!(
            r.local_compose_file(dev),
            root.join("deployment/compose.yaml")
        );
        assert_eq!(r.local_env_file(dev, "dev"), root.join(".env.dev"));

        let prod = r.environment("prod").unwrap();
        assert!(!is_local(prod));
        assert!(prod.require_confirm);
        assert_eq!(r.ssh_user(prod).as_deref(), Some("deploy"));
        assert_eq!(r.registry(prod).as_deref(), Some("ghcr.io/me"));
        assert_eq!(r.deploy_dir(prod).unwrap(), "/srv/app");
    }

    #[test]
    fn walks_up_to_find_deploy_file() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(&root.join("metaphor.deploy.yaml"), SAMPLE);
        let nested = root.join("apps/api/src");
        fs::create_dir_all(&nested).unwrap();

        let r = Resolved::load_from(&nested).unwrap();
        assert_eq!(r.workspace_root, root);
    }

    #[test]
    fn errors_when_env_missing() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(&root.join("metaphor.deploy.yaml"), SAMPLE);

        let r = Resolved::load_from(root).unwrap();
        let e = r.environment("staging").unwrap_err().to_string();
        assert!(e.contains("environment 'staging' not found"));
        assert!(e.contains("dev, prod"));
    }

    #[test]
    fn rejects_unsupported_version() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("metaphor.deploy.yaml"),
            "version: 2\nenvironments:\n  dev:\n    images: {}\n",
        );
        let e = Resolved::load_from(root).unwrap_err().to_string();
        assert!(e.contains("unsupported version 2"));
    }
}
