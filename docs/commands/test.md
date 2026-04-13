# metaphor-dev test

Test generation and management commands for the Metaphor Framework.

The `metaphor-dev test` command provides a complete testing workflow: generating unit, integration, and end-to-end tests from entity definitions, running tests with flexible filtering, producing coverage reports, and watching for changes during development.

---

## Subcommands

| Subcommand       | Description                              |
|------------------|------------------------------------------|
| `generate`       | Generate tests for an entity or module   |
| `generate-all`   | Generate tests for all entities in a module |
| `run`            | Run tests                                |
| `coverage`       | Generate test coverage report            |
| `watch`          | Run tests in watch mode                  |
| `summary`        | Show test summary for a module           |

---

## test generate

Generate tests for a specific entity within a module.

### Synopsis

```
metaphor-dev test generate <entity> <module> [OPTIONS]
```

### Description

Generates test files for the given entity inside the target module. The command verifies that the module exists at `libs/modules/{module}`, creates a `tests/` directory inside it if necessary, and writes test files based on the selected test types. It also updates `tests/mod.rs` to include the new test modules.

The entity name is converted to snake_case for file names and PascalCase for struct/type references within the generated code.

### Arguments

| Argument | Required | Description                                             |
|----------|----------|---------------------------------------------------------|
| `entity` | Yes      | Entity name in PascalCase (e.g., `User`, `Payment`)    |
| `module` | Yes      | Target module name                                      |

### Options

| Flag            | Type | Default | Description                                          |
|-----------------|------|---------|------------------------------------------------------|
| `--unit`        | bool | `true`  | Generate unit tests                                  |
| `--integration` | bool | `false` | Generate integration tests                           |
| `--e2e`         | bool | `false` | Generate E2E tests                                   |
| `--all`         | bool | `false` | Generate all test types (overrides individual flags)  |
| `--force`       | bool | `false` | Force overwrite existing test files                   |

### Examples

Generate unit tests (default) for the `User` entity in the `accounts` module:

```bash
metaphor-dev test generate User accounts
```

Generate all test types for the `Payment` entity in the `billing` module:

```bash
metaphor-dev test generate Payment billing --all
```

Generate only integration and E2E tests, overwriting any existing files:

```bash
metaphor-dev test generate Order commerce --integration --e2e --force
```

### Notes

- When `--all` is specified, it sets `--unit`, `--integration`, and `--e2e` to true regardless of their individual values.
- Generated file names follow the pattern `{entity_snake}_{type}_tests.rs` (e.g., `user_unit_tests.rs`).
- The command will error if the module directory does not exist at `libs/modules/{module}`.

---

## test generate-all

Generate tests for every entity found in a module.

### Synopsis

```
metaphor-dev test generate-all <module> [OPTIONS]
```

### Description

Scans the directory `libs/modules/{module}/proto/domain/entity/` for `.proto` files and generates all test types (unit, integration, and E2E) for each entity discovered. This is a convenience command for bootstrapping tests across an entire module.

### Arguments

| Argument | Required | Description        |
|----------|----------|--------------------|
| `module` | Yes      | Target module name |

### Options

| Flag      | Type | Default | Description                          |
|-----------|------|---------|--------------------------------------|
| `--force` | bool | `false` | Force overwrite existing test files   |

### Examples

Generate tests for all entities in the `accounts` module:

```bash
metaphor-dev test generate-all accounts
```

Force-regenerate all tests in the `billing` module:

```bash
metaphor-dev test generate-all billing --force
```

### Notes

- Entity names are derived from the `.proto` file names found in the entity directory.
- All three test types (unit, integration, E2E) are generated for each entity.

---

## test run

Run tests with optional filtering by type, module, and pattern.

### Synopsis

```
metaphor-dev test run [module] [OPTIONS]
```

### Description

Executes tests using `cargo test`. When a module is specified, the command targets the package `metaphor-{module}`. Test type flags control which subset of tests to run, and the `--filter` option allows running only tests whose names match a given pattern.

### Arguments

| Argument | Required | Description                                                       |
|----------|----------|-------------------------------------------------------------------|
| `module` | No       | Target module. When provided, adds `-p metaphor-{module}` to the cargo command. |

### Options

| Flag               | Type   | Default | Description                                              |
|--------------------|--------|---------|----------------------------------------------------------|
| `--unit`           | bool   | `false` | Run only unit tests (adds `--lib`)                       |
| `--integration`    | bool   | `false` | Run only integration tests (adds `--test integration`)   |
| `--e2e`            | bool   | `false` | Run only E2E tests (adds `--test e2e`)                   |
| `--release`        | bool   | `false` | Run tests in release mode                                |
| `--nocapture`      | bool   | `false` | Show test output (adds `-- --nocapture`)                 |
| `--filter <pattern>` | string | —     | Run tests matching the given pattern                     |

### Examples

Run all tests across the workspace:

```bash
metaphor-dev test run
```

Run unit tests for the `accounts` module with output visible:

```bash
metaphor-dev test run accounts --unit --nocapture
```

Run only tests matching "create" in the `billing` module:

```bash
metaphor-dev test run billing --filter create
```

### Notes

- When `--filter` is combined with `--nocapture`, the resulting cargo arguments are `-- --nocapture {pattern}`.
- When `--filter` is used without `--nocapture`, the arguments are `-- {pattern}`.

---

## test coverage

Generate a test coverage report.

### Synopsis

```
metaphor-dev test coverage [module] [OPTIONS]
```

### Description

Generates a code coverage report using `cargo-llvm-cov`. If the tool is not installed, it is automatically installed before proceeding. The report can be produced in HTML, LCOV, or JSON format.

### Arguments

| Argument | Required | Description        |
|----------|----------|--------------------|
| `module` | No       | Target module      |

### Options

| Flag                      | Type   | Default | Description                          |
|---------------------------|--------|---------|--------------------------------------|
| `--format <html\|lcov\|json>` | string | `html`  | Output format for the coverage report |
| `--open`                  | bool   | `false` | Open the coverage report in a browser |

### Examples

Generate an HTML coverage report for the entire workspace:

```bash
metaphor-dev test coverage
```

Generate an LCOV report for the `accounts` module:

```bash
metaphor-dev test coverage accounts --format lcov
```

Generate and immediately open an HTML coverage report:

```bash
metaphor-dev test coverage billing --open
```

### Notes

- `cargo-llvm-cov` is auto-installed via `cargo install cargo-llvm-cov` if it is not found on the system.
- For the `html` format, the command runs `cargo llvm-cov --html`. For other formats, it runs `cargo llvm-cov --{format}`.
- The `--open` flag appends `--open` to the underlying cargo command to launch the report in the default browser.

---

## test watch

Run tests continuously, re-executing on file changes.

### Synopsis

```
metaphor-dev test watch [module] [OPTIONS]
```

### Description

Starts a file watcher that automatically re-runs tests whenever source files change. Uses `cargo-watch` under the hood. If the tool is not installed, it is automatically installed before proceeding.

### Arguments

| Argument | Required | Description        |
|----------|----------|--------------------|
| `module` | No       | Target module      |

### Options

| Flag               | Type   | Default | Description                       |
|--------------------|--------|---------|-----------------------------------|
| `--filter <pattern>` | string | —     | Only run tests matching the pattern |

### Examples

Watch and run all tests:

```bash
metaphor-dev test watch
```

Watch tests for the `accounts` module:

```bash
metaphor-dev test watch accounts
```

Watch and run only tests matching "user" in the `accounts` module:

```bash
metaphor-dev test watch accounts --filter user
```

### Notes

- `cargo-watch` is auto-installed via `cargo install cargo-watch` if it is not found on the system.
- The underlying command is `cargo watch -x "test"`, with optional `-p metaphor-{module}` and filter arguments appended.

---

## test summary

Display a summary of tests in a module.

### Synopsis

```
metaphor-dev test summary <module>
```

### Description

Scans test files located in `libs/modules/{module}/tests/` and produces a summary report. The command counts test annotations and categorizes them by type.

### Arguments

| Argument | Required | Description        |
|----------|----------|--------------------|
| `module` | Yes      | Target module name |

### Examples

Show test summary for the `accounts` module:

```bash
metaphor-dev test summary accounts
```

### Notes

- The command counts occurrences of `#[test]` and `#[tokio::test]` annotations.
- Tests are categorized as unit, integration, or E2E based on filename patterns (e.g., files containing `_unit_tests` are counted as unit tests).

---

## Generated Test Structure

When tests are generated via `test generate` or `test generate-all`, the following structure is created inside the target module:

```
libs/modules/{module}/
  tests/
    mod.rs
    {entity_snake}_unit_tests.rs
    {entity_snake}_integration_tests.rs
    {entity_snake}_e2e_tests.rs
```

### Unit Tests (`{entity_snake}_unit_tests.rs`)

Approximately 10 test functions covering CRUD operations for the entity. These tests validate core business logic without external dependencies.

### Integration Tests (`{entity_snake}_integration_tests.rs`)

Approximately 12 test functions that include database setup and teardown. These tests verify that the entity works correctly with its data layer.

### E2E Tests (`{entity_snake}_e2e_tests.rs`)

Approximately 12 test functions that use an HTTP client (`reqwest`) to test the full request-response cycle through the API endpoints for the entity.

### Module Registration

The `tests/mod.rs` file is updated to include `mod` declarations for each generated test file, ensuring they are compiled and discoverable by the test runner.

---

## See Also

- [Development Workflow Guide](../guides/development-workflow.md)
- [CI Integration Guide](../guides/ci-integration.md)
