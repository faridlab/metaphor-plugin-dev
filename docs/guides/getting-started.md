# Getting Started

This guide walks you through installing and using `metaphor-dev` for the first time.

## Prerequisites

### Required

- **Rust toolchain** (edition 2021 or later)

  Install via [rustup](https://rustup.rs/):

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **Cargo** — included with the Rust toolchain

### Optional

These tools are auto-installed when needed, but you can install them ahead of time:

```bash
# Security auditing
cargo install cargo-audit

# Dependency freshness checking
cargo install cargo-outdated

# Code coverage
cargo install cargo-llvm-cov

# File-watching test runner
cargo install cargo-watch
```

For Docker-based development:

- [Docker](https://docs.docker.com/get-docker/)
- [Docker Compose](https://docs.docker.com/compose/install/)

## Installation

### From source

```bash
git clone https://github.com/faridlab/metaphor-plugin-dev.git
cd metaphor-plugin-dev
cargo install --path .
```

### Build only

```bash
cargo build --release
# Binary at: target/release/metaphor-dev
```

### Verify installation

```bash
metaphor-dev --version
# metaphor-plugin-dev 0.1.0

metaphor-dev --help
```

## First Run

### 1. Build the project

Start by building the entire workspace to make sure everything compiles:

```bash
metaphor-dev dev build
```

For an optimized release build:

```bash
metaphor-dev dev build --release
```

### 2. Check code quality

Run all quality checks — formatting, compilation, linting, and security audit:

```bash
metaphor-dev lint all
```

This runs 4 steps in sequence:
1. Code formatting check (rustfmt)
2. Compilation check (cargo check)
3. Clippy linting with framework rules
4. Security audit (cargo-audit)

### 3. Start the development server

Start the application locally:

```bash
metaphor-dev dev serve --local
```

This runs `cargo run --bin metaphor-app` in the `apps/metaphor/` directory and makes the following endpoints available:

| Endpoint | URL |
|----------|-----|
| REST API | http://localhost:3000/api/v1 |
| gRPC | localhost:50051 |
| Health check | http://localhost:3000/health |

Press `Ctrl+C` to stop the server.

### 4. Run tests

Run all tests:

```bash
metaphor-dev dev test
```

Run only unit tests:

```bash
metaphor-dev dev test --unit-only
```

### 5. Validate configuration

Check your configuration files for common issues:

```bash
metaphor-dev config validate
```

## Project Structure

The Metaphor framework follows a modular monolith architecture:

```
your-project/
├── apps/
│   └── metaphor/
│       ├── config/
│       │   ├── application.yml              # Main configuration
│       │   └── application-development.yml  # Environment overlay
│       └── src/
├── libs/
│   └── modules/
│       ├── sapiens/    # User management module
│       ├── postman/    # Email service module
│       └── bucket/     # File storage module
├── .env                # Environment variables
├── docker-compose.yml  # Docker services
└── Cargo.toml          # Workspace manifest
```

### Key directories

| Directory | Purpose |
|-----------|---------|
| `apps/metaphor/` | Main application binary |
| `apps/metaphor/config/` | YAML configuration files |
| `libs/modules/{name}/` | Individual domain modules |
| `libs/modules/{name}/proto/` | Protocol Buffer definitions |
| `libs/modules/{name}/tests/` | Module test files |
| `libs/modules/{name}/docs/` | Module documentation |

## What's Next

- [Development Workflow](development-workflow.md) — Day-to-day usage patterns
- [CI Integration](ci-integration.md) — Setting up CI/CD pipelines

### Command References

- [dev](../commands/dev.md) — Development server, building, testing, database
- [lint](../commands/lint.md) — Code quality enforcement
- [test](../commands/test.md) — Test generation and management
- [docs](../commands/docs.md) — Documentation generation
- [config](../commands/config.md) — Configuration validation
- [jobs](../commands/jobs.md) — Job scheduling
