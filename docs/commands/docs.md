# docs — Documentation Generation Commands

Generate, serve, and validate documentation for Metaphor framework projects.

## Overview

The `metaphor-dev docs` command provides tools for generating RustDoc documentation, creating module-specific markdown documentation, extracting API docs from Protocol Buffer definitions, serving docs locally, and checking documentation coverage.

---

## `docs generate`

Generate RustDoc documentation for all crates in the workspace.

### Synopsis

```bash
metaphor-dev docs generate [OPTIONS]
```

### Description

Runs `cargo doc --no-deps --workspace` to generate RustDoc HTML documentation for all crates. After generation, creates a custom HTML index page at `{output}/index.html` that links to each crate's documentation.

The command sets `RUSTDOCFLAGS` to enable cross-crate linking and documentation quality warnings.

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--open` | bool | false | Open documentation in browser after generation |
| `--document-private-items` | bool | false | Include private items in documentation |
| `--output <dir>` | string | `target/doc` | Output directory for documentation |

### Examples

```bash
# Generate documentation for all crates
metaphor-dev docs generate

# Generate and open in browser
metaphor-dev docs generate --open

# Include private items
metaphor-dev docs generate --document-private-items

# Custom output directory
metaphor-dev docs generate --output ./public/docs
```

### Notes

- The generated index page lists all workspace crates with links to their documentation
- Private items documentation is useful for internal developer reference
- The `--open` flag launches the system default browser

---

## `docs module`

Generate comprehensive markdown documentation for a specific module.

### Synopsis

```bash
metaphor-dev docs module <name> [OPTIONS]
```

### Description

Generates a set of markdown documentation files for the specified module in `libs/modules/{name}/docs/`. The command always generates a `README.md` and an `ARCHITECTURE.md`. Optionally generates `API_REFERENCE.md` and `EXAMPLES.md`.

Additionally runs `cargo doc -p metaphor-{name}` to generate RustDoc for the specific module.

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Module name (e.g., "sapiens", "postman", "bucket") |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--with-examples` | bool | false | Include EXAMPLES.md with usage examples |
| `--with-api-reference` | bool | false | Generate API_REFERENCE.md with endpoints |

### Generated Files

| File | Always Generated | Description |
|------|-----------------|-------------|
| `README.md` | Yes | Module overview, features, getting started |
| `ARCHITECTURE.md` | Yes | DDD layer architecture diagram and description |
| `API_REFERENCE.md` | With `--with-api-reference` | REST and gRPC endpoint documentation |
| `EXAMPLES.md` | With `--with-examples` | Code examples and usage patterns |

### Examples

```bash
# Generate basic module documentation
metaphor-dev docs module sapiens

# Generate with all optional docs
metaphor-dev docs module sapiens --with-examples --with-api-reference

# Generate for a custom module
metaphor-dev docs module payments --with-api-reference
```

### Notes

- The module must exist at `libs/modules/{name}/`
- The `ARCHITECTURE.md` file includes a text-based DDD layer diagram showing Domain, Application, Infrastructure, and Presentation layers
- The `API_REFERENCE.md` includes placeholder REST endpoints (`GET /api/v1/{module}`, `POST`, `PUT`, `DELETE`) and gRPC service definitions

---

## `docs api`

Generate API documentation from Protocol Buffer (.proto) files.

### Synopsis

```bash
metaphor-dev docs api <module> [OPTIONS]
```

### Description

Scans the module's proto directory (`libs/modules/{module}/proto/`) for `.proto` files and generates API documentation. Parses proto file contents to extract service definitions, RPC methods, and message types.

The generated documentation is saved to `libs/modules/{module}/docs/PROTO_API.md`.

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `module` | Yes | Target module name |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--format <format>` | string | `markdown` | Output format: `markdown`, `html`, `json` |
| `--with-examples` | bool | false | Include request/response examples |

### Examples

```bash
# Generate API docs in markdown format
metaphor-dev docs api sapiens

# Generate with examples
metaphor-dev docs api sapiens --with-examples
```

### Notes

- Currently only `markdown` format is fully implemented; `html` and `json` formats are planned
- The proto parser extracts `service`, `rpc`, and `message` definitions from `.proto` files
- Proto files are discovered recursively using directory walking

---

## `docs serve`

Serve documentation locally with optional hot-reload.

### Synopsis

```bash
metaphor-dev docs serve [OPTIONS]
```

### Description

First generates the RustDoc documentation (equivalent to `docs generate`), then starts a local HTTP server to serve the documentation files. Attempts to use `miniserve` first; falls back to Python 3's built-in HTTP server if miniserve is not available.

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--port <port>` | u16 | 8080 | Port to serve on |
| `--watch` | bool | false | Watch for changes and regenerate (requires `cargo-watch`) |

### Examples

```bash
# Serve docs on default port (8080)
metaphor-dev docs serve

# Serve on custom port
metaphor-dev docs serve --port 3333

# Serve with hot-reload
metaphor-dev docs serve --watch
```

### Notes

- The server serves files from `target/doc/` by default
- `miniserve` provides a better experience with directory listing and file type icons
- Python 3 fallback uses `python3 -m http.server`
- The `--watch` flag requires `cargo-watch` to be installed

---

## `docs coverage`

Check documentation coverage across the codebase.

### Synopsis

```bash
metaphor-dev docs coverage [OPTIONS]
```

### Description

Runs `cargo doc` with the `-D missing_docs` flag to detect undocumented public items. Counts the number of warnings to calculate a coverage percentage. Reports the result and optionally fails if coverage is below the minimum threshold.

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--module <name>` | string | (all) | Target specific module |
| `--min-coverage <pct>` | u8 | 80 | Minimum coverage percentage required |
| `--strict` | bool | false | Exit with non-zero status if coverage is below minimum |

### Examples

```bash
# Check coverage for all modules
metaphor-dev docs coverage

# Check specific module with strict enforcement
metaphor-dev docs coverage --module sapiens --strict

# Set custom minimum coverage
metaphor-dev docs coverage --min-coverage 90 --strict

# Check coverage for CI pipelines
metaphor-dev docs coverage --strict --min-coverage 80
```

### Notes

- Coverage is calculated based on the ratio of documented vs. undocumented public items
- The `--strict` flag is useful in CI pipelines to enforce documentation standards
- Default minimum coverage is 80%

---

## See Also

- [Getting Started Guide](../guides/getting-started.md) — Initial project setup
- [CI Integration Guide](../guides/ci-integration.md) — Using `docs coverage` in CI pipelines
- [Configuration Reference](../reference/configuration.md) — Module configuration details
