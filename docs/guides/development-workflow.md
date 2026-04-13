# Development Workflow

This guide describes day-to-day usage patterns for developing with the Metaphor framework using `metaphor-dev`.

## Typical Development Loop

A standard development cycle follows this pattern:

```
Edit code → Build → Serve → Test → Lint → Commit
```

```bash
# 1. Build to catch compilation errors early
metaphor-dev dev build

# 2. Start the dev server
metaphor-dev dev serve --local

# 3. Run tests
metaphor-dev dev test

# 4. Check code quality before committing
metaphor-dev lint all --strict
```

## Local vs Docker Development

### Local Development (default)

Local mode runs the application directly via `cargo run`. This is the default when you don't specify either `--docker` or `--local`:

```bash
# These are equivalent
metaphor-dev dev serve
metaphor-dev dev serve --local
```

Local mode:
- Runs `cargo run --bin metaphor-app` in `apps/metaphor/`
- Sets `APP_ENV=development` automatically
- Requires external services (PostgreSQL, etc.) to be running separately
- Faster startup for iterative development

### Docker Development

Docker mode uses Docker Compose to orchestrate all services:

```bash
metaphor-dev dev serve --docker
```

Docker mode:
- Starts all services via `docker-compose up -d`
- Includes databases, message queues, and all modules
- Performs health checks after startup
- Better for full-stack integration testing

### Selective Service Startup

Start only the services you need:

```bash
# gRPC services only (no REST/Envoy)
metaphor-dev dev serve --docker --grpc-only

# REST services only (via Envoy proxy)
metaphor-dev dev serve --docker --rest-only
```

## Working with Modules

The Metaphor framework is organized into domain modules. Each module is a self-contained unit with its own proto definitions, business logic, and tests.

### Default Modules

| Module | Port | gRPC Port | Description |
|--------|------|-----------|-------------|
| sapiens | 3003 | 50053 | User management and authentication |
| postman | 3002 | 50052 | Email sending and notifications |
| bucket | 3004 | 50054 | File storage and media management |

### Targeting a Specific Module

Most commands accept a `--module` flag to operate on a single module:

```bash
# Lint only the sapiens module
metaphor-dev lint check --module sapiens

# Run tests for a specific module
metaphor-dev test run sapiens

# Generate docs for a module
metaphor-dev docs module sapiens --with-examples --with-api-reference
```

When using `--module`, the tool targets the Cargo package `metaphor-{module}`.

## Database Migration Workflow

### Creating a New Migration

```bash
# Create a migration file
metaphor-dev dev db create add_users_table
```

### Running Migrations

```bash
# Run all pending migrations
metaphor-dev dev db migrate

# Migrate to a specific version
metaphor-dev dev db migrate --version 20240101120000
```

### Resetting the Database

```bash
# Reset with confirmation prompt
metaphor-dev dev db reset

# Force reset (skip confirmation)
metaphor-dev dev db reset --force
```

> **Note:** Database commands are currently in development (Priority 2.1). Some operations may show placeholder messages.

## Test-Driven Development

### Generating Test Scaffolding

Generate tests for a new entity:

```bash
# Generate unit tests (default)
metaphor-dev test generate User sapiens

# Generate all test types
metaphor-dev test generate User sapiens --all

# Generate for all entities in a module
metaphor-dev test generate-all sapiens
```

Generated test files are placed in `libs/modules/{module}/tests/`:
- `{entity}_unit_tests.rs` — Unit tests for domain logic
- `{entity}_integration_tests.rs` — Integration tests with database
- `{entity}_e2e_tests.rs` — End-to-end tests with HTTP client

### Watch Mode

Run tests automatically when files change:

```bash
# Watch all tests
metaphor-dev test watch

# Watch tests for a specific module
metaphor-dev test watch sapiens

# Watch with a filter pattern
metaphor-dev test watch sapiens --filter "user"
```

### Coverage Reports

Generate and view coverage reports:

```bash
# Generate HTML coverage report
metaphor-dev test coverage

# Generate and open in browser
metaphor-dev test coverage --open

# Generate LCOV report for CI
metaphor-dev test coverage --format lcov
```

## Pre-Commit Quality Checks

Before committing code, run the full quality check suite:

```bash
# Run all checks in strict mode (treats warnings as errors)
metaphor-dev lint all --strict
```

This runs:
1. **Format check** — Ensures code is properly formatted
2. **Compilation check** — Verifies code compiles without errors
3. **Clippy** — Enforces framework linting rules
4. **Security audit** — Scans for known vulnerabilities

### Auto-Fix Issues

Let the tools fix what they can automatically:

```bash
# Auto-fix formatting and clippy issues
metaphor-dev lint all --fix
```

### Individual Checks

Run specific checks when you need them:

```bash
# Just format the code
metaphor-dev lint fmt

# Check formatting without changing files
metaphor-dev lint fmt --check

# Just run clippy
metaphor-dev lint check

# Clippy with pedantic rules
metaphor-dev lint check --pedantic

# Security audit only
metaphor-dev lint audit
```

## Documentation Workflow

### Generating Documentation

```bash
# Generate RustDoc for all crates
metaphor-dev docs generate

# Generate and open in browser
metaphor-dev docs generate --open

# Generate module-specific docs
metaphor-dev docs module sapiens --with-examples --with-api-reference
```

### Serving Documentation Locally

```bash
# Serve docs on port 8080
metaphor-dev docs serve

# Serve with hot-reload
metaphor-dev docs serve --watch
```

### Checking Documentation Coverage

```bash
# Check coverage (target: 80%)
metaphor-dev docs coverage

# Strict enforcement
metaphor-dev docs coverage --strict --min-coverage 90
```

## Job Scheduling Workflow

### Creating a New Job

```bash
# Create a daily backup job
metaphor-dev jobs create DatabaseBackup --cron "0 2 * * *"

# Create in a specific module
metaphor-dev jobs create SessionCleanup --cron "0 */6 * * *" --module sapiens

# Validate your cron expression first
metaphor-dev jobs validate-cron "0 2 * * *" --show-next
```

### Exploring Templates

```bash
# See available templates
metaphor-dev jobs templates --detailed

# Create from a template
metaphor-dev jobs create WeeklyCleanup --cron "0 3 * * 0" --template weekly_log_cleanup
```

### Setting Up the Jobs Module

```bash
# Initialize jobs infrastructure
metaphor-dev jobs init --project my_app --with-migrations --with-docker
```

## Configuration Validation

### Before Deployment

Always validate configuration before deploying:

```bash
# Validate for production
metaphor-dev config validate --env production --strict
```

### Email Configuration

Test SMTP connectivity:

```bash
# Test connection
metaphor-dev config email-verify

# Send a test email
metaphor-dev config email-verify --send-test admin@example.com
```

## See Also

- [Getting Started](getting-started.md) — Initial setup
- [CI Integration](ci-integration.md) — Automated pipelines
- [Configuration Reference](../reference/configuration.md) — Environment variables and config files
