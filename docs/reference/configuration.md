# Configuration Reference

Complete reference for all configuration files and environment variables used by the Metaphor framework.

## Configuration Files

### `apps/metaphor/config/application.yml`

The main application configuration file. Loaded by `DevConfig::load()` when running `metaphor-dev dev serve`.

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
apps/metaphor/config/application-{APP_ENV}.yml
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

## See Also

- [config validate](../commands/config.md#config-validate) — Configuration validation command
- [config email-verify](../commands/config.md#config-email-verify) — SMTP verification command
- [Getting Started](../guides/getting-started.md) — Initial setup
