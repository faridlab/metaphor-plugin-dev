# metaphor-plugin-dev

> Development workflow plugin for Metaphor CLI (dev, lint, test, docs, config, jobs)

[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](Cargo.toml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)

A comprehensive development toolkit for the [Metaphor Framework](https://github.com/faridlab/metaphor-plugin-dev) — a modular monolith architecture built with Rust. This plugin provides commands for local development, code quality enforcement, test generation, documentation, configuration validation, and job scheduling.

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
```

The binary will be available at `target/release/metaphor-dev`.

## Quick Start

```bash
# Build the project
metaphor-dev dev build

# Start local development server
metaphor-dev dev serve --local

# Run all quality checks
metaphor-dev lint all

# Run tests with coverage
metaphor-dev dev test --coverage

# Validate configuration
metaphor-dev config validate
```

## Command Summary

| Command | Description |
|---------|-------------|
| **dev** | |
| `dev serve` | Start development servers (gRPC + REST) |
| `dev test` | Run all tests (unit + integration + E2E) |
| `dev build` | Build the entire project |
| `dev db migrate` | Run database migrations |
| `dev db create <name>` | Create a new migration |
| `dev db reset` | Reset database (drop and recreate) |
| **lint** | |
| `lint check` | Run clippy linter with framework rules |
| `lint fmt` | Format code with rustfmt |
| `lint compile` | Quick compilation check without building |
| `lint audit` | Run security audit on dependencies |
| `lint outdated` | Check for outdated dependencies |
| `lint all` | Run all quality checks (fmt + compile + clippy + audit) |
| `lint config` | Show clippy configuration |
| **test** | |
| `test generate <entity> <module>` | Generate tests for an entity |
| `test generate-all <module>` | Generate tests for all entities in a module |
| `test run` | Run tests with filtering options |
| `test coverage` | Generate test coverage report |
| `test watch` | Run tests in watch mode |
| `test summary <module>` | Show test summary for a module |
| **docs** | |
| `docs generate` | Generate RustDoc documentation for all crates |
| `docs module <name>` | Generate documentation for a specific module |
| `docs api <module>` | Generate API documentation from proto files |
| `docs serve` | Serve documentation locally with hot-reload |
| `docs coverage` | Check documentation coverage |
| **config** | |
| `config validate` | Validate application configuration |
| `config email-verify` | Verify email/SMTP configuration |
| **jobs** | |
| `jobs create <name>` | Create a new scheduled job |
| `jobs templates` | List available job templates |
| `jobs validate-cron <expr>` | Validate a cron expression |
| `jobs config` | Generate job scheduler configuration |
| `jobs example` | Create job example files |
| `jobs init` | Initialize jobs module in current project |

## Global Options

| Flag | Description |
|------|-------------|
| `-v`, `--verbose` | Enable verbose output (sets `RUST_LOG=debug`) |
| `--version` | Print version information |
| `-h`, `--help` | Print help information |

## Requirements

### Required

- **Rust toolchain** (edition 2021) — [Install Rust](https://rustup.rs/)
- **Cargo** — included with the Rust toolchain

### Optional (auto-installed when needed)

| Tool | Used by | Purpose |
|------|---------|---------|
| `cargo-audit` | `lint audit` | Security vulnerability scanning |
| `cargo-outdated` | `lint outdated` | Dependency freshness checking |
| `cargo-llvm-cov` | `test coverage` | Code coverage reports |
| `cargo-watch` | `test watch` | File-watching test runner |
| Docker / Docker Compose | `dev serve --docker` | Container-based development |
| `miniserve` or Python 3 | `docs serve` | Local documentation server |
| `protoc` | `docs api` | Protocol Buffer compiler |

## Documentation

### Command Reference

Detailed documentation for each command category:

- [dev](docs/commands/dev.md) — Development workflow commands (serve, test, build, database)
- [lint](docs/commands/lint.md) — Code quality and linting commands
- [test](docs/commands/test.md) — Test generation and management
- [docs](docs/commands/docs.md) — Documentation generation
- [config](docs/commands/config.md) — Configuration validation
- [jobs](docs/commands/jobs.md) — Job scheduling commands

### Guides

- [Getting Started](docs/guides/getting-started.md) — Installation, prerequisites, and first run
- [Development Workflow](docs/guides/development-workflow.md) — Day-to-day usage patterns
- [CI Integration](docs/guides/ci-integration.md) — Using metaphor-dev in CI/CD pipelines

### Reference

- [Configuration](docs/reference/configuration.md) — Config file formats, environment variables
- [Job Templates](docs/reference/job-templates.md) — All 8 built-in job templates
- [Clippy Rules](docs/reference/clippy-rules.md) — Linting rules enforced by the framework

## Project Structure

```
metaphor-plugin-dev/
├── Cargo.toml              # Project manifest
├── Cargo.lock              # Dependency lock file
├── README.md               # This file
├── docs/                   # Documentation
│   ├── commands/           # Command reference
│   ├── guides/             # Usage guides
│   └── reference/          # Reference material
└── src/
    ├── main.rs             # CLI entry point and command dispatch
    ├── lib.rs              # Library entry point
    ├── commands/
    │   ├── mod.rs           # Command module exports
    │   ├── dev.rs           # Development workflow commands
    │   ├── lint.rs          # Code quality and linting
    │   ├── test.rs          # Test generation and management
    │   ├── docs.rs          # Documentation generation
    │   ├── config.rs        # Configuration validation
    │   └── jobs.rs          # Job scheduling
    └── templates/
        └── jobs/
            └── job.rs       # Job file template for code generation
```

## License

MIT License. See [LICENSE](LICENSE) for details.

## Author

Farid Hidayat
