# CI Integration

This guide shows how to use `metaphor-dev` in CI/CD pipelines for automated quality checks, testing, and validation.

## GitHub Actions Workflow

### Complete Pipeline

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  quality:
    name: Code Quality
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - uses: Swatinem/rust-cache@v2

      - name: Install metaphor-dev
        run: cargo install --path .

      - name: Format check
        run: metaphor-dev lint fmt --check

      - name: Compilation check
        run: metaphor-dev lint compile

      - name: Clippy (strict)
        run: metaphor-dev lint check --strict

      - name: Security audit
        run: metaphor-dev lint audit

  test:
    name: Tests
    runs-on: ubuntu-latest
    needs: quality
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_USER: postgres
          POSTGRES_PASSWORD: password
          POSTGRES_DB: testdb
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    env:
      DATABASE_URL: postgresql://postgres:password@localhost:5432/testdb
      JWT_SECRET: ci-test-secret-at-least-32-characters-long
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2

      - name: Install metaphor-dev
        run: cargo install --path .

      - name: Run tests
        run: metaphor-dev test run

      - name: Generate coverage (LCOV)
        run: metaphor-dev test coverage --format lcov

      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          files: lcov.info
          fail_ci_if_error: false

  config:
    name: Configuration Validation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2

      - name: Install metaphor-dev
        run: cargo install --path .

      - name: Validate config (production)
        run: metaphor-dev config validate --strict --env production

  docs:
    name: Documentation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2

      - name: Install metaphor-dev
        run: cargo install --path .

      - name: Check doc coverage
        run: metaphor-dev docs coverage --strict --min-coverage 80

      - name: Generate docs
        run: metaphor-dev docs generate

      - name: Upload docs artifact
        uses: actions/upload-artifact@v4
        with:
          name: documentation
          path: target/doc/
```

### Minimal Pipeline

For smaller projects or faster feedback:

```yaml
name: CI

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2

      - name: Install metaphor-dev
        run: cargo install --path .

      - name: All quality checks (strict)
        run: metaphor-dev lint all --strict

      - name: Run tests
        run: metaphor-dev dev test
```

## Key Commands for CI

### Quality Gates

| Command | Purpose | Fails on |
|---------|---------|----------|
| `lint fmt --check` | Code formatting | Unformatted code |
| `lint compile` | Compilation | Compiler errors |
| `lint check --strict` | Linting | Any clippy warning |
| `lint audit` | Security | Known vulnerabilities |
| `lint all --strict` | All of the above | Any of the above |

### Testing

| Command | Purpose | Produces |
|---------|---------|----------|
| `dev test` | Run all tests | Test results |
| `test run` | Run with filtering | Test results |
| `test coverage --format lcov` | Coverage report | `lcov.info` file |
| `test coverage --format html` | HTML coverage | `target/llvm-cov/html/` |

### Validation

| Command | Purpose | Fails on |
|---------|---------|----------|
| `config validate --strict` | Config validation | Any warning or error |
| `config validate --strict --env production` | Production config | Production-specific issues |
| `docs coverage --strict` | Doc coverage | Below 80% (default) |

## Exit Codes

All `metaphor-dev` commands follow standard exit code conventions:

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Failure (test failures, lint errors, validation errors) |
| 2 | Usage error (invalid arguments) |

The `--strict` flag on `lint all`, `config validate`, and `docs coverage` ensures non-zero exit codes on warnings, making them suitable for CI gate enforcement.

## Artifacts

### Coverage Reports

```yaml
- name: Generate HTML coverage
  run: metaphor-dev test coverage --format html

- name: Upload coverage report
  uses: actions/upload-artifact@v4
  with:
    name: coverage-report
    path: target/llvm-cov/html/
```

### Documentation

```yaml
- name: Generate documentation
  run: metaphor-dev docs generate

- name: Upload documentation
  uses: actions/upload-artifact@v4
  with:
    name: documentation
    path: target/doc/
```

## Tips

- **Cache Cargo dependencies** using `Swatinem/rust-cache@v2` to speed up builds significantly
- **Split jobs** for parallelism — quality checks, tests, and docs can run concurrently
- **Use `needs`** to create dependencies between jobs (e.g., only run tests if quality checks pass)
- **Set `CARGO_TERM_COLOR=always`** for colored output in CI logs
- **Run `lint all --strict`** as a single command for simpler pipelines, or split into individual commands for granular failure reporting

## See Also

- [Getting Started](getting-started.md) — Local setup
- [Development Workflow](development-workflow.md) — Day-to-day usage
- [Configuration Reference](../reference/configuration.md) — Environment variables for CI
