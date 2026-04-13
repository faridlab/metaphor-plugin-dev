# config — Configuration Validation Commands

Validate application configuration files, environment variables, and SMTP connectivity.

## Overview

The `metaphor-dev config` command provides tools for catching configuration issues early — before they cause runtime failures. It validates `.env` files for common mistakes, checks YAML configuration for security concerns, verifies environment variables meet requirements, and tests SMTP email connectivity.

---

## `config validate`

Validate application configuration across multiple sources.

### Synopsis

```bash
metaphor-dev config validate [OPTIONS]
```

### Description

Performs a three-phase validation of the application configuration:

1. **Phase 1: `.env` file validation** — Checks the `.env` file for common issues
2. **Phase 2: YAML config file validation** — Checks `apps/metaphor/config/application.yml` and environment-specific overlays
3. **Phase 3: Environment variable validation** — Checks loaded environment variables for correctness

Each issue is reported with a severity level and optional suggestion for resolution.

### Severity Levels

| Level | Meaning |
|-------|---------|
| **ERROR** | Critical issue that will cause failures |
| **WARN** | Potential problem that should be addressed |
| **INFO** | Informational note or best practice suggestion |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--strict` | bool | false | Treat warnings as errors (exit with non-zero status) |
| `--env <environment>` | string | from `RUST_ENV` or `"development"` | Target environment to validate for |

### Phase 1: `.env` File Checks

The validator reads `.env` line by line and checks for:

| Check | Severity | Description |
|-------|----------|-------------|
| Shell expansion risk | Warning | Values containing unescaped `$` that may cause shell expansion |
| Quoted values | Info | Values wrapped in quotes (dotenvy includes quotes literally) |
| Empty critical values | Warning | `DATABASE_URL`, `JWT_SECRET`, or `SMTP_HOST` set to empty string |
| Placeholder values | Warning | Values containing `changeme`, `TODO`, `FIXME`, `xxx`, or `your_*_here` |
| SMTP port mismatch | Warning | `SMTP_PORT=587` with `MAIL_ENCRYPTION=ssl` (587 uses STARTTLS, not SSL) |

### Phase 2: YAML Config Checks

Validates YAML configuration files at:
- `apps/metaphor/config/application.yml`
- `apps/metaphor/config/application-{env}.yml` (if exists)

| Check | Severity | Description |
|-------|----------|-------------|
| Default credentials | Warning | Database URL containing `password` or `root:root` |
| Hardcoded DB URL | Info | Non-environment-variable database URLs in production |
| JWT secret placeholder | Error | JWT secret set to `secret`, `changeme`, etc. |
| Environment mismatch | Warning | Configuration not matching the target environment |

### Phase 3: Environment Variable Checks

After loading `.env` via dotenvy, validates:

| Variable | Check | Severity |
|----------|-------|----------|
| `DATABASE_URL` | Contains default credentials (`postgres:password`) | Warning |
| `DATABASE_URL` | Uses `localhost` in production | Warning |
| `JWT_SECRET` | Not set | Error |
| `JWT_SECRET` | Length < 32 characters | Warning |
| `SMTP_HOST` | Not set (when SMTP is expected) | Warning |
| `SMTP_PORT` | Invalid or unusual port | Info |
| `SMTP_USER` | Not set (when SMTP_HOST is configured) | Warning |
| `SMTP_PASSWORD` | Not set (when SMTP_HOST is configured) | Warning |
| `EMAIL_FROM` / `SMTP_FROM` | Not set (when SMTP_HOST is configured) | Warning |

### Examples

```bash
# Basic validation (development environment)
metaphor-dev config validate

# Validate for production
metaphor-dev config validate --env production

# Strict mode for CI pipelines
metaphor-dev config validate --strict

# Strict validation for staging
metaphor-dev config validate --strict --env staging
```

### Output Example

```
⚙️  Configuration Validator
  Environment: development

  📄 Checking .env file...
  [WARN] .env: Value contains '$' which may cause shell expansion (line 12)
         Tip: Escape with '\$' or use single quotes in shell
  [INFO] .env: Value is quoted — dotenvy includes quotes literally (line 5)
         Tip: Remove surrounding quotes from the value

  📄 Checking YAML configuration...
  [WARN] yaml: Database URL contains default credentials
         Tip: Use environment-specific credentials

  📄 Checking environment variables...
  [WARN] env: JWT_SECRET is shorter than 32 characters
         Tip: Use a longer secret for better security

  Summary: 0 errors, 3 warnings, 1 info
```

### Notes

- The `--strict` flag is designed for CI pipelines where any potential issue should block deployment
- The `.env` file is loaded via the `dotenvy` crate, which includes surrounding quotes literally (unlike `dotenv` in some other languages)
- Environment detection uses `RUST_ENV` first, then falls back to `"development"`

---

## `config email-verify`

Verify email/SMTP configuration and optionally send a test email.

### Synopsis

```bash
metaphor-dev config email-verify [OPTIONS]
```

### Description

Tests SMTP connectivity by reading email configuration from environment variables and attempting to establish a connection to the SMTP server. Validates port semantics, SSL/TLS configuration, and credentials.

Uses the `lettre` crate for SMTP transport.

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--send-test <email>` | string | (none) | Send a test email to this address |

### Environment Variables Read

| Variable | Required | Description |
|----------|----------|-------------|
| `SMTP_HOST` | Yes | SMTP server hostname |
| `SMTP_PORT` | Yes | SMTP server port |
| `SMTP_USER` | Yes | SMTP authentication username |
| `SMTP_PASSWORD` | Yes | SMTP authentication password |
| `EMAIL_FROM` or `SMTP_FROM` | Yes | Sender email address |

### Port Semantics

| Port | Protocol | Description |
|------|----------|-------------|
| 25 | Plain SMTP | Unencrypted (not recommended) |
| 465 | SMTPS (Implicit SSL) | SSL/TLS connection from the start |
| 587 | SMTP + STARTTLS | Starts plain, upgrades to TLS |
| 1025 | MailHog | Local development email testing |
| 2525 | MailHog (alt) | Alternative MailHog port |

### Examples

```bash
# Verify SMTP configuration (connection test only)
metaphor-dev config email-verify

# Verify and send a test email
metaphor-dev config email-verify --send-test admin@example.com
```

### Troubleshooting

| Issue | Possible Cause | Solution |
|-------|---------------|----------|
| Connection refused | Wrong host or port | Verify `SMTP_HOST` and `SMTP_PORT` |
| Authentication failed | Wrong credentials | Check `SMTP_USER` and `SMTP_PASSWORD` |
| TLS handshake error | Port/protocol mismatch | Port 465 requires SSL, port 587 requires STARTTLS |
| Timeout | Firewall blocking | Check firewall rules for outbound SMTP |
| Connection works but email not received | Sender not authorized | Verify `EMAIL_FROM` is authorized on the SMTP server |

### Notes

- For local development, consider using [MailHog](https://github.com/mailhog/MailHog) on port 1025
- The `--send-test` flag sends an actual email — use a real address you can check
- SSL/TLS validation uses the system's native TLS implementation

---

## See Also

- [Configuration Reference](../reference/configuration.md) — Complete environment variable reference
- [Getting Started Guide](../guides/getting-started.md) — Initial project setup
- [CI Integration Guide](../guides/ci-integration.md) — Using `config validate --strict` in pipelines
