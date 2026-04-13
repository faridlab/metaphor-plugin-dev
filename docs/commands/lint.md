# metaphor-dev lint

Code quality and linting commands for the Metaphor Framework.

The `metaphor-dev lint` command provides a unified interface for running linters, formatters, security audits, and compilation checks across your Metaphor project. It enforces framework-specific Clippy rules and supports targeting individual modules or the entire workspace.

## Subcommands

| Subcommand | Description |
|------------|-------------|
| `lint check` | Run clippy linter with framework rules |
| `lint fmt` | Format code with rustfmt |
| `lint compile` | Quick compilation check without building |
| `lint audit` | Run security audit on dependencies |
| `lint outdated` | Check for outdated dependencies |
| `lint all` | Run all quality checks in sequence |
| `lint config` | Show clippy configuration for the project |

---

## lint check

Run the Clippy linter with Metaphor Framework rules.

### Synopsis

```
metaphor-dev lint check [OPTIONS]
```

### Description

Runs `cargo clippy` with a curated set of lint rules tailored to the Metaphor Framework. The command enforces strict error handling practices (no `unwrap` or `expect`), flags common development leftovers (`todo!`, `dbg!`, `println!`), and includes async-specific checks.

### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--module <name>` | string | _(none)_ | Target module (targets package `metaphor-{name}`). If not specified, runs on `--workspace`. |
| `--strict` | bool | `false` | Treat warnings as errors (adds `-D warnings`). |
| `--fix` | bool | `false` | Fix issues automatically where possible (adds `--fix --allow-dirty --allow-staged`). |
| `--pedantic` | bool | `false` | Show all warnings including pedantic (adds `-W clippy::pedantic`). |

### Clippy Rules Enforced

**Denied (errors):**

| Lint | Reason |
|------|--------|
| `clippy::unwrap_used` | Prevents panics from `.unwrap()` calls; use proper error handling instead. |
| `clippy::expect_used` | Prevents panics from `.expect()` calls; use `?` or explicit error handling. |

**Warned:**

| Lint | Reason |
|------|--------|
| `clippy::todo` | Flags `todo!()` macros that should not reach production. |
| `clippy::dbg_macro` | Flags `dbg!()` macros left over from debugging. |
| `clippy::print_stdout` | Flags `println!()` calls; use structured logging instead. |
| `clippy::print_stderr` | Flags `eprintln!()` calls; use structured logging instead. |

**Async warnings:**

| Lint | Reason |
|------|--------|
| `clippy::large_futures` | Warns about futures that are too large and may cause stack issues. |
| `clippy::redundant_async_block` | Flags unnecessary async blocks wrapping already-async code. |
| `clippy::unused_async` | Flags async functions that never actually await. |

**Allowed (framework exceptions):**

| Lint | Rationale |
|------|-----------|
| `clippy::module_inception` | Metaphor's module structure intentionally uses same-named inner modules. |
| `clippy::too_many_arguments` | Framework builder patterns and configuration functions often require many parameters. |

### Examples

```bash
# Run clippy on the entire workspace
metaphor-dev lint check

# Run clippy on a specific module
metaphor-dev lint check --module core

# Run in strict mode (warnings become errors)
metaphor-dev lint check --strict

# Auto-fix issues
metaphor-dev lint check --fix

# Run with pedantic rules enabled
metaphor-dev lint check --pedantic

# Combine strict mode with a specific module
metaphor-dev lint check --module auth --strict
```

### Notes

- When `--module` is provided, Clippy targets the package `metaphor-{name}` (e.g., `--module core` targets `metaphor-core`).
- The `--fix` flag passes `--allow-dirty --allow-staged` so it works in repositories with uncommitted changes.
- Using `--pedantic` can produce a large number of warnings; it is best suited for code review rather than CI.

---

## lint fmt

Format code with rustfmt.

### Synopsis

```
metaphor-dev lint fmt [OPTIONS]
```

### Description

Runs `cargo fmt` to format Rust source code. By default, it formats all code in place. Use `--check` for a dry-run that exits with a non-zero status if any files are not properly formatted (useful for CI). Use `--diff` to see a summary of changes after formatting.

### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--module <name>` | string | _(none)_ | Target module (targets package `metaphor-{name}`). If not specified, runs on `--all`. |
| `--check` | bool | `false` | Check formatting without making changes (dry run). |
| `--diff` | bool | `false` | Show git diff after formatting (only when not in `--check` mode). |

### Examples

```bash
# Format all code in the workspace
metaphor-dev lint fmt

# Check formatting without modifying files
metaphor-dev lint fmt --check

# Format a specific module and show what changed
metaphor-dev lint fmt --module core --diff

# Check formatting for a specific module (useful in CI)
metaphor-dev lint fmt --module auth --check
```

### Notes

- When `--diff` is used, the command runs `git diff --stat` after formatting to show a summary of changed files.
- The `--diff` flag is ignored when `--check` is active, since no files are modified during a check run.

---

## lint compile

Quick compilation check without building.

### Synopsis

```
metaphor-dev lint compile [OPTIONS]
```

### Description

Runs `cargo check` to verify that the code compiles without producing build artifacts. This is significantly faster than a full build and is useful for rapid feedback during development or as an early CI step.

### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--module <name>` | string | _(none)_ | Target module (targets package `metaphor-{name}`). If not specified, runs on `--workspace`. |
| `--release` | bool | `false` | Check in release mode (may surface additional warnings due to different optimization settings). |

### Examples

```bash
# Check the entire workspace compiles
metaphor-dev lint compile

# Check a specific module
metaphor-dev lint compile --module api

# Check in release mode
metaphor-dev lint compile --release
```

### Notes

- This does not produce any binaries or libraries; it only verifies that the code is syntactically and semantically valid.
- Release mode checks can catch issues that only appear with optimizations enabled.

---

## lint audit

Run a security audit on dependencies.

### Synopsis

```
metaphor-dev lint audit [OPTIONS]
```

### Description

Runs `cargo audit` to check project dependencies against the RustSec Advisory Database. Identifies known vulnerabilities in your dependency tree. If `cargo-audit` is not installed, the command will install it automatically before running.

### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--fix` | bool | `false` | Fix vulnerable dependencies where possible (runs `cargo audit fix`). |
| `--format <format>` | string | `"text"` | Output format. Accepts `text` or `json`. |

### Examples

```bash
# Run a security audit
metaphor-dev lint audit

# Run audit with JSON output (useful for CI tooling)
metaphor-dev lint audit --format json

# Attempt to fix vulnerable dependencies
metaphor-dev lint audit --fix
```

### Notes

- `cargo-audit` is auto-installed if it is not found on your system.
- The `--fix` flag attempts to update vulnerable dependencies to patched versions. Review the changes before committing.
- JSON output is useful for integrating with security dashboards and CI reporting tools.

---

## lint outdated

Check for outdated dependencies.

### Synopsis

```
metaphor-dev lint outdated [OPTIONS]
```

### Description

Runs `cargo outdated` to list dependencies that have newer versions available. If `cargo-outdated` is not installed, the command will install it automatically before running.

### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--direct` | bool | `false` | Show only direct/root dependencies (adds `--root-deps-only`). |
| `--compatible` | bool | `false` | Show compatible (semver-compatible) updates only. |

### Examples

```bash
# Check all dependencies for updates
metaphor-dev lint outdated

# Show only direct dependencies
metaphor-dev lint outdated --direct

# Show only semver-compatible updates
metaphor-dev lint outdated --compatible
```

### Notes

- `cargo-outdated` is auto-installed if it is not found on your system.
- Use `--direct` to reduce noise and focus on dependencies you directly control.
- Compatible updates are generally safe to apply without breaking changes.

---

## lint all

Run all quality checks in sequence.

### Synopsis

```
metaphor-dev lint all [OPTIONS]
```

### Description

Runs a 4-step quality pipeline that covers formatting, compilation, linting, and security. This is the recommended command for pre-commit checks and CI pipelines.

### Pipeline Steps

| Step | Check | Failure behavior |
|------|-------|-----------------|
| 1/4 | Code formatting (`fmt`) | Fails the run |
| 2/4 | Compilation check (`compile`) | Fails the run |
| 3/4 | Clippy linting (`check`) | Fails the run |
| 4/4 | Security audit (`audit`) | Does **not** fail the overall run |

### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--module <name>` | string | _(none)_ | Target module (targets package `metaphor-{name}`). |
| `--strict` | bool | `false` | Treat warnings as errors. |
| `--fix` | bool | `false` | Auto-fix issues where possible. When set, `fmt` runs in fix mode; when not set, `fmt` runs in `--check` mode. |

### Examples

```bash
# Run the full quality pipeline
metaphor-dev lint all

# Run all checks on a specific module
metaphor-dev lint all --module core

# Run all checks in strict mode
metaphor-dev lint all --strict

# Run all checks and auto-fix what can be fixed
metaphor-dev lint all --fix
```

### Notes

- The security audit step (step 4) is intentionally non-blocking. A vulnerable dependency should not prevent you from shipping a formatting or compilation fix. Address audit findings separately.
- When `--fix` is set, the formatting step applies changes in place. When `--fix` is not set, the formatting step runs as a check-only dry run.
- This command is the recommended single entry point for CI quality gates.

---

## lint config

Show the clippy configuration for the project.

### Synopsis

```
metaphor-dev lint config
```

### Description

Displays the full set of Clippy lint rules configured for the Metaphor Framework. This includes denied lints, warned lints, async-specific lints, and allowed exceptions, each with an explanation of why the rule is enforced or suppressed.

The output also includes instructions for adding project-specific configuration via `clippy.toml` or `Cargo.toml`.

### Examples

```bash
# Display the current clippy configuration
metaphor-dev lint config
```

### Notes

- This command takes no options.
- Use this to understand which rules are active before running `lint check`.

---

## CI Integration Tips

The `lint all` command is designed to be the single quality gate in your CI pipeline. Below is an example GitHub Actions snippet:

```yaml
name: Code Quality

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - name: Cache cargo registry and build
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Run all quality checks
        run: metaphor-dev lint all --strict

      - name: Check for outdated dependencies
        run: metaphor-dev lint outdated --direct
        continue-on-error: true
```

For stricter CI pipelines, you can run individual steps for better granularity and reporting:

```yaml
      - name: Check formatting
        run: metaphor-dev lint fmt --check

      - name: Compile check
        run: metaphor-dev lint compile

      - name: Clippy (strict)
        run: metaphor-dev lint check --strict

      - name: Security audit
        run: metaphor-dev lint audit --format json
        continue-on-error: true
```

---

## See Also

- [Clippy Rules Reference](../reference/clippy-rules.md) -- Detailed explanation of all enforced lint rules.
- [CI Integration Guide](../guides/ci-integration.md) -- Full guide for setting up continuous integration with Metaphor.
