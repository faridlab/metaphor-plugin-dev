# Configuration Reference

Complete reference for all configuration files and environment variables used by the Metaphor framework.

## Configuration Files

### `<app_dir>/config/application.yml`

The main application configuration file. Loaded by `DevConfig::load()` when running `metaphor-dev dev serve`. `<app_dir>` is resolved from `metaphor.yaml` (the backend-service project containing the CWD, or the sole `backend-service` entry) — for example `apps/bersihir-service/config/application.yml`.

```yaml
# Server configuration
server:
  host: 0.0.0.0
  port: 3000

# Module configuration
modules:
  sapiens:
    enabled: true
    port: 3003
    grpc_port: 50053
    description: "User management and authentication"

  postman:
    enabled: true
    port: 3002
    grpc_port: 50052
    description: "Email sending and notification service"

  bucket:
    enabled: true
    port: 3004
    grpc_port: 50054
    description: "File storage and media management"
```

#### Fields

| Section | Field | Type | Default | Description |
|---------|-------|------|---------|-------------|
| `server` | `host` | string | `0.0.0.0` | Server bind address |
| `server` | `port` | u16 | `3000` | Main server port |
| `modules.{name}` | `enabled` | bool | `true` | Whether the module is active |
| `modules.{name}` | `port` | u16 | varies | REST port for the module |
| `modules.{name}` | `grpc_port` | u16 | varies | gRPC port for the module |
| `modules.{name}` | `description` | string | varies | Human-readable module description |

### Environment Overlays

The configuration system supports environment-specific overlays. After loading the base `application.yml`, it loads an overlay file based on the `APP_ENV` environment variable:

```
<app_dir>/config/application-{APP_ENV}.yml
```

For example:
- `application-development.yml` — Development overrides
- `application-staging.yml` — Staging overrides
- `application-production.yml` — Production overrides

Overlay values merge with and override the base configuration.

### `.env` File

Environment variables file loaded by the `dotenvy` crate at runtime. Located at the project root.

```env
# Database
DATABASE_URL=postgresql://postgres:password@localhost:5432/metaphordb

# Authentication
JWT_SECRET=your-secret-key-at-least-32-characters-long

# SMTP / Email
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_USER=
SMTP_PASSWORD=
EMAIL_FROM=noreply@example.com

# Environment
RUST_ENV=development
APP_ENV=development
RUST_LOG=info
```

#### Important: dotenvy Behavior

The `dotenvy` crate (used by Metaphor) handles `.env` files differently from some other implementations:

- **Quotes are included literally** — `KEY="value"` sets the value to `"value"` (with quotes), not `value`
- **No shell expansion** — `$VAR` is treated as the literal string `$VAR`
- **No command substitution** — `$(command)` is treated literally

The `config validate` command warns about these common mistakes.

## Environment Variables

### Complete Reference

| Variable | Required | Default | Used By | Description |
|----------|----------|---------|---------|-------------|
| `DATABASE_URL` | Yes | — | Application | PostgreSQL connection string |
| `JWT_SECRET` | Yes | — | Application | JSON Web Token signing secret |
| `SMTP_HOST` | For email | — | `config email-verify` | SMTP server hostname |
| `SMTP_PORT` | For email | — | `config email-verify` | SMTP server port |
| `SMTP_USER` | For email | — | `config email-verify` | SMTP authentication username |
| `SMTP_PASSWORD` | For email | — | `config email-verify` | SMTP authentication password |
| `EMAIL_FROM` | For email | — | `config email-verify` | Default sender email address |
| `SMTP_FROM` | For email | — | `config email-verify` | Alternative to `EMAIL_FROM` |
| `RUST_ENV` | No | `development` | `config validate` | Runtime environment name |
| `APP_ENV` | No | `development` | `dev serve` | Config overlay selector |
| `RUST_LOG` | No | (none) | Logging | Log level filter (e.g., `debug`, `info`) |

### DATABASE_URL

PostgreSQL connection string format:

```
postgresql://user:password@host:port/database
```

**Validation checks:**
- Warns if using default credentials (`postgres:password`)
- Warns if using `localhost` in production environment
- Error if not set

### JWT_SECRET

Secret key for signing JSON Web Tokens.

**Validation checks:**
- Error if not set
- Warning if shorter than 32 characters
- Error if set to placeholder values (`secret`, `changeme`, etc.)

### SMTP Configuration

| Variable | Port Semantics |
|----------|---------------|
| Port 25 | Plain SMTP (unencrypted) |
| Port 465 | SMTPS (implicit SSL/TLS) |
| Port 587 | SMTP with STARTTLS |
| Port 1025 | MailHog (local development) |
| Port 2525 | MailHog (alternative) |

**Validation checks:**
- Warning if `SMTP_PORT=587` with `MAIL_ENCRYPTION=ssl` (port 587 uses STARTTLS, not SSL)
- Warning if credentials not set when `SMTP_HOST` is configured
- Warning if sender address not set

## Default Service Ports

| Service | REST Port | gRPC Port | Health Endpoint |
|---------|-----------|-----------|-----------------|
| Metaphor (API Gateway) | 3000 | 50051 | `/health` |
| Sapiens (User Management) | 3003 | 50053 | `/health` |
| Postman (Email Service) | 3002 | 50052 | `/health` |
| Bucket (File Storage) | 3004 | 50054 | `/health` |

These defaults are defined in `DevConfig::default()` and can be overridden via `application.yml`.

## `metaphor.deploy.yaml`

Workspace-root config that drives [`docker`](../commands/docker.md) and [`deploy`](../commands/deploy.md). Lives next to `metaphor.yaml`. Loaded by walking up from the current directory until the file is found; the directory containing it is the **workspace root** for path resolution.

Environments without `host:` are *local* (operated by `docker`). Environments with `host:` are *remote* (operated by `deploy`).

```yaml
version: 1

defaults:
  registry: ghcr.io/your-org             # default registry for pushed images
  compose_file: deployment/compose.yaml  # default compose file (rel. to workspace or deploy_dir)
  ssh_user: deploy                       # default SSH user for remote envs
  deploy_dir: /srv/app                   # default working dir on remote hosts
  migrate_command: "metaphor migration run-all"

environments:
  dev:                                   # local — no host:
    env_file: deployment/.env.dev
    images:
      api:
        context: apps/api
        tag_env: SERVICE_TAG

  uat:                                   # remote
    host: uat.example.com
    env_file: deployment/.env.uat
    images:
      api:
        context: apps/api
        tag_env: SERVICE_TAG

  prod:
    host: example.com
    env_file: deployment/.env.prod
    require_confirm: true                # prompts before push/rollback unless --yes
    images:
      api:
        context: apps/api
        tag_env: SERVICE_TAG
        build_args:
          BUILD_PROFILE: release
```

### Top-level fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | u32 | yes | Schema version. Currently `1`. |
| `defaults` | object | no | Per-key fallbacks consumed by every environment. See below. |
| `environments` | map<string, [Environment](#environment-fields)> | yes | At least one entry. Map key is the env name passed to commands. |

### `defaults` fields

| Field | Type | Used for | Description |
|-------|------|----------|-------------|
| `registry` | string | `deploy push` | Image registry prefix (e.g. `ghcr.io/myorg`). Falls back from per-image → per-env → defaults. |
| `compose_file` | string | `docker`, `deploy` | Compose file path. Falls back from per-env → defaults → `deployment/compose.yaml`. |
| `ssh_user` | string | `deploy` | SSH user concatenated as `<user>@<host>`. Falls back from per-env → defaults. |
| `deploy_dir` | string | `deploy` | Remote working directory. Falls back from per-env → defaults. Required (no implicit default) when invoking `deploy`. |
| `migrate_command` | string | `deploy push`, `deploy migrate` | Command run inside `docker compose run --rm migrations`. Defaults to `metaphor migration run-all`. |

### Environment fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `host` | string | only for remote | Hostname/IP. **Presence marks the env as remote.** |
| `ssh_user` | string | no | Per-env override for `defaults.ssh_user`. |
| `deploy_dir` | string | no | Per-env override for `defaults.deploy_dir`. |
| `compose_file` | string | no | Per-env override for `defaults.compose_file`. Resolved against the workspace root (locally) or `deploy_dir` (remotely). |
| `env_file` | string | no | Per-env path. Defaults to `.env.<env>`. Resolved against the workspace root locally and `deploy_dir` remotely. |
| `registry` | string | no | Per-env override for `defaults.registry`. |
| `require_confirm` | bool | no | If `true`, `deploy push` and `deploy rollback` prompt for confirmation unless `--yes` is given. Defaults to `false`. |
| `images` | map<string, [Image](#image-fields)> | yes for `deploy push`/`docker build` paths that consult it | Image build specs, keyed by image name. |

### Image fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `context` | string | yes | Docker build context, relative to workspace root. |
| `dockerfile` | string | no | Dockerfile path relative to `context`. |
| `name` | string | no | Image name (without registry/tag). Defaults to the map key. |
| `registry` | string | no | Per-image override for env / defaults registry. |
| `tag_env` | string | no | Env-file variable that tracks this image's tag (e.g. `SERVICE_TAG`). `deploy push` rewrites this on every release. |
| `build_args` | map<string,string> | no | `--build-arg` pairs forwarded to `docker buildx build`. |
| `push` | bool | no | Push after build. Defaults to `true` for images under remote envs. |

### Path resolution

| Path | Resolved relative to |
|------|---------------------|
| `compose_file` (local — `docker *`) | Workspace root (directory containing `metaphor.deploy.yaml`) |
| `compose_file` (remote — `deploy *`) | `deploy_dir` on the remote host |
| `env_file` (local) | Workspace root |
| `env_file` (remote) | `deploy_dir` on the remote host |
| `images.<key>.context` | Workspace root |
| `images.<key>.dockerfile` | The image's `context` |

### Example: minimal docker-only setup

```yaml
version: 1
defaults:
  compose_file: docker-compose.yml
environments:
  dev:
    env_file: .env
    images: {}
```

### Example: full beta deployment (single VPS)

```yaml
version: 1
defaults:
  registry: ghcr.io/myorg
  compose_file: deployment/compose.yaml
  ssh_user: deploy
  deploy_dir: /srv/myapp
environments:
  dev:
    env_file: deployment/.env.dev
    images:
      api:    { context: apps/api,    tag_env: API_TAG }
      web:    { context: apps/web,    tag_env: WEB_TAG, build_args: { VITE_API_BASE_URL: "http://localhost:3000" } }
  prod:
    host: myapp.example.com
    env_file: deployment/.env.prod
    require_confirm: true
    images:
      api:    { context: apps/api,    tag_env: API_TAG }
      web:    { context: apps/web,    tag_env: WEB_TAG, build_args: { VITE_API_BASE_URL: "https://api.myapp.example.com" } }
```

## See Also

- [config validate](../commands/config.md#config-validate) — Configuration validation command
- [config email-verify](../commands/config.md#config-email-verify) — SMTP verification command
- [docker](../commands/docker.md) — Local compose lifecycle driven by `metaphor.deploy.yaml`
- [deploy](../commands/deploy.md) — Remote deployment driven by `metaphor.deploy.yaml`
- [Getting Started](../guides/getting-started.md) — Initial setup
