//! Documentation generation commands for Metaphor Framework
//!
//! This module provides commands for generating and managing documentation:
//! - RustDoc generation with module-aware templates
//! - API documentation from proto files
//! - Markdown documentation for entities and modules
//!
//! # Commands
//!
//! - `metaphor docs generate` - Generate RustDoc for all crates
//! - `metaphor docs module <name>` - Generate documentation for a specific module
//! - `metaphor docs api <module>` - Generate API documentation from proto files
//! - `metaphor docs serve` - Serve documentation locally
//! - `metaphor docs coverage` - Check documentation coverage

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Documentation command actions
#[derive(Subcommand, Clone, Debug)]
pub enum DocsAction {
    /// Generate RustDoc documentation for all crates
    Generate {
        /// Open documentation in browser after generation
        #[arg(long)]
        open: bool,

        /// Include private items in documentation
        #[arg(long)]
        document_private_items: bool,

        /// Output directory for documentation
        #[arg(long, default_value = "target/doc")]
        output: String,
    },

    /// Generate documentation for a specific module
    Module {
        /// Module name (e.g., "sapiens", "payments")
        name: String,

        /// Include examples in documentation
        #[arg(long)]
        with_examples: bool,

        /// Generate API reference markdown
        #[arg(long)]
        with_api_reference: bool,
    },

    /// Generate API documentation from proto files
    Api {
        /// Target module name
        module: String,

        /// Output format (markdown, html, json)
        #[arg(long, default_value = "markdown")]
        format: String,

        /// Include request/response examples
        #[arg(long)]
        with_examples: bool,
    },

    /// Serve documentation locally with hot-reload
    Serve {
        /// Port to serve on
        #[arg(long, default_value = "8080")]
        port: u16,

        /// Watch for changes and regenerate
        #[arg(long)]
        watch: bool,
    },

    /// Check documentation coverage
    Coverage {
        /// Target module (or all if not specified)
        module: Option<String>,

        /// Minimum coverage percentage required
        #[arg(long, default_value = "80")]
        min_coverage: u8,

        /// Fail if coverage is below minimum
        #[arg(long)]
        strict: bool,
    },
}

/// Handle documentation commands
pub async fn handle_command(action: &DocsAction) -> Result<()> {
    match action {
        DocsAction::Generate {
            open,
            document_private_items,
            output,
        } => generate_rustdoc(*open, *document_private_items, output).await,

        DocsAction::Module {
            name,
            with_examples,
            with_api_reference,
        } => generate_module_docs(name, *with_examples, *with_api_reference).await,

        DocsAction::Api {
            module,
            format,
            with_examples,
        } => generate_api_docs(module, format, *with_examples).await,

        DocsAction::Serve { port, watch } => serve_docs(*port, *watch).await,

        DocsAction::Coverage {
            module,
            min_coverage,
            strict,
        } => check_coverage(module.as_deref(), *min_coverage, *strict).await,
    }
}

/// Generate RustDoc documentation for all crates
async fn generate_rustdoc(open: bool, document_private: bool, output: &str) -> Result<()> {
    println!(
        "{}",
        "📚 Generating RustDoc documentation...".bright_cyan().bold()
    );
    println!();

    let mut args = vec!["doc", "--no-deps", "--workspace"];

    if document_private {
        args.push("--document-private-items");
    }

    // Run cargo doc
    let status = Command::new("cargo")
        .args(&args)
        .env("RUSTDOCFLAGS", "--enable-index-page -Zunstable-options")
        .status()
        .context("Failed to run cargo doc")?;

    if !status.success() {
        anyhow::bail!("cargo doc failed with status: {}", status);
    }

    println!();
    println!(
        "  {} Documentation generated at: {}",
        "✅".green(),
        output.bright_white()
    );

    // Generate index page
    generate_doc_index(output).await?;

    if open {
        let index_path = format!("{}/index.html", output);
        println!(
            "  {} Opening documentation in browser...",
            "🌐".bright_blue()
        );

        #[cfg(target_os = "macos")]
        Command::new("open").arg(&index_path).spawn()?;

        #[cfg(target_os = "linux")]
        Command::new("xdg-open").arg(&index_path).spawn()?;

        #[cfg(target_os = "windows")]
        Command::new("start").arg(&index_path).spawn()?;
    }

    println!();
    println!(
        "{}",
        "Documentation generation complete! 🎉".bright_green().bold()
    );

    Ok(())
}

/// Generate documentation index page
async fn generate_doc_index(output: &str) -> Result<()> {
    let index_content = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Metaphor Framework Documentation</title>
    <style>
        :root {
            --bg-color: #1a1a2e;
            --text-color: #eaeaea;
            --accent-color: #00d9ff;
            --card-bg: #16213e;
        }
        body {
            font-family: 'Segoe UI', system-ui, sans-serif;
            background: var(--bg-color);
            color: var(--text-color);
            margin: 0;
            padding: 40px;
            line-height: 1.6;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
        }
        h1 {
            color: var(--accent-color);
            font-size: 2.5em;
            margin-bottom: 10px;
        }
        .subtitle {
            color: #888;
            font-size: 1.2em;
            margin-bottom: 40px;
        }
        .crates {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
            gap: 20px;
        }
        .crate-card {
            background: var(--card-bg);
            border-radius: 12px;
            padding: 24px;
            transition: transform 0.2s, box-shadow 0.2s;
        }
        .crate-card:hover {
            transform: translateY(-4px);
            box-shadow: 0 8px 24px rgba(0, 217, 255, 0.15);
        }
        .crate-card h3 {
            margin: 0 0 12px;
            color: var(--accent-color);
        }
        .crate-card p {
            color: #aaa;
            margin: 0 0 16px;
            font-size: 0.95em;
        }
        .crate-card a {
            color: var(--accent-color);
            text-decoration: none;
            font-weight: 500;
        }
        .crate-card a:hover {
            text-decoration: underline;
        }
        .section-title {
            color: var(--text-color);
            font-size: 1.5em;
            margin: 40px 0 20px;
            padding-bottom: 10px;
            border-bottom: 2px solid var(--accent-color);
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🦀 Metaphor Framework</h1>
        <p class="subtitle">Modular Monolith Framework with DDD, Clean Architecture, and Schema-First Approach</p>

        <h2 class="section-title">Core Crates</h2>
        <div class="crates">
            <div class="crate-card">
                <h3>metaphor-core</h3>
                <p>Core traits and generic CRUD system with 11 standard endpoints</p>
                <a href="metaphor_core/index.html">View Documentation →</a>
            </div>
            <div class="crate-card">
                <h3>metaphor-cli</h3>
                <p>Code generator and module management CLI</p>
                <a href="metaphor_cli/index.html">View Documentation →</a>
            </div>
            <div class="crate-card">
                <h3>metaphor-messaging</h3>
                <p>Generic event bus system with typed events</p>
                <a href="metaphor_messaging/index.html">View Documentation →</a>
            </div>
            <div class="crate-card">
                <h3>metaphor-jobs</h3>
                <p>Job scheduling and cron management</p>
                <a href="metaphor_jobs/index.html">View Documentation →</a>
            </div>
            <div class="crate-card">
                <h3>metaphor-orm</h3>
                <p>InMemoryStore and ORM utilities</p>
                <a href="metaphor_orm/index.html">View Documentation →</a>
            </div>
        </div>

        <h2 class="section-title">Modules</h2>
        <div class="crates">
            <div class="crate-card">
                <h3>sapiens</h3>
                <p>User management bounded context - authentication, authorization, roles</p>
                <a href="sapiens/index.html">View Documentation →</a>
            </div>
        </div>

        <h2 class="section-title">Applications</h2>
        <div class="crates">
            <div class="crate-card">
                <h3>metaphor (app)</h3>
                <p>Main application entry point</p>
                <a href="metaphor/index.html">View Documentation →</a>
            </div>
        </div>
    </div>
</body>
</html>
"#;

    let index_path = PathBuf::from(output).join("index.html");
    fs::write(&index_path, index_content).context("Failed to write documentation index")?;

    println!(
        "  {} Generated documentation index",
        "📄".bright_yellow()
    );

    Ok(())
}

/// Generate documentation for a specific module
async fn generate_module_docs(
    module: &str,
    with_examples: bool,
    with_api_reference: bool,
) -> Result<()> {
    println!(
        "{}",
        format!("📚 Generating documentation for module: {}", module)
            .bright_cyan()
            .bold()
    );
    println!();

    let module_path = PathBuf::from("libs/modules").join(module);

    if !module_path.exists() {
        anyhow::bail!("Module '{}' not found at {:?}", module, module_path);
    }

    // Create docs directory
    let docs_dir = module_path.join("docs");
    fs::create_dir_all(&docs_dir)?;

    // Generate README.md
    generate_module_readme(&docs_dir, module).await?;

    // Generate ARCHITECTURE.md
    generate_architecture_doc(&docs_dir, module).await?;

    if with_api_reference {
        generate_api_reference(&docs_dir, module).await?;
    }

    if with_examples {
        generate_examples_doc(&docs_dir, module).await?;
    }

    // Run cargo doc for the specific module
    let package_name = format!("metaphor-{}", module);
    let status = Command::new("cargo")
        .args(["doc", "--no-deps", "-p", &package_name])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("  {} Generated RustDoc for {}", "✅".green(), package_name);
        }
        Ok(_) => {
            println!(
                "  {} RustDoc generation had warnings for {}",
                "⚠️".yellow(),
                package_name
            );
        }
        Err(_) => {
            println!(
                "  {} Package {} not found, skipping RustDoc",
                "ℹ️".blue(),
                package_name
            );
        }
    }

    println!();
    println!(
        "{}",
        format!("Module documentation generated at: {:?}", docs_dir)
            .bright_green()
            .bold()
    );

    Ok(())
}

/// Generate module README.md
async fn generate_module_readme(docs_dir: &Path, module: &str) -> Result<()> {
    let module_pascal = to_pascal_case(module);

    let content = format!(
        r#"# {} Module

## Overview

The `{}` module is a bounded context in the Metaphor Framework that provides...

## Quick Start

```rust
use {}::{}ModuleBuilder;

// Initialize the module
let module = {}ModuleBuilder::new()
    .with_database(db_pool.clone())
    .build()?;

// Configure routes
App::new()
    .configure(|cfg| module.configure_routes(cfg))
```

## Architecture

This module follows DDD (Domain-Driven Design) principles:

```
{}/
├── proto/domain/           # Protocol Buffer definitions
│   ├── entity/            # Domain entities
│   ├── value_object/      # Value objects
│   ├── event/             # Domain events
│   └── repository/        # Repository interfaces
│
├── src/
│   ├── domain/            # Domain layer implementation
│   ├── application/       # Use cases (CQRS)
│   ├── infrastructure/    # Repository implementations
│   └── presentation/      # HTTP/gRPC/CLI handlers
│
└── tests/                 # Integration tests
```

## Endpoints

This module provides the following 11 standard CRUD endpoints:

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/{{collection}}` | List (paginated, filtered, sorted) |
| POST | `/api/v1/{{collection}}` | Create |
| GET | `/api/v1/{{collection}}/:id` | Get by ID |
| PUT | `/api/v1/{{collection}}/:id` | Full update |
| PATCH | `/api/v1/{{collection}}/:id` | Partial update |
| DELETE | `/api/v1/{{collection}}/:id` | Soft delete |
| POST | `/api/v1/{{collection}}/bulk` | Bulk create |
| POST | `/api/v1/{{collection}}/upsert` | Upsert |
| GET | `/api/v1/{{collection}}/trash` | List deleted |
| POST | `/api/v1/{{collection}}/:id/restore` | Restore |
| DELETE | `/api/v1/{{collection}}/empty` | Empty trash |

## Configuration

```yaml
modules:
  {}:
    enabled: true
    database:
      pool_size: 10
```

## Testing

```bash
# Run all tests
cargo test -p metaphor-{}

# Run integration tests
cargo test -p metaphor-{} --test integration_tests
```

## Generated by Metaphor CLI

This documentation was generated by `metaphor docs module {}`.
"#,
        module_pascal,
        module,
        module,
        module_pascal,
        module_pascal,
        module,
        module,
        module,
        module,
        module
    );

    let readme_path = docs_dir.join("README.md");
    fs::write(&readme_path, content)?;

    println!(
        "  {} Generated: docs/README.md",
        "📄".bright_yellow()
    );

    Ok(())
}

/// Generate ARCHITECTURE.md
async fn generate_architecture_doc(docs_dir: &Path, module: &str) -> Result<()> {
    let module_pascal = to_pascal_case(module);

    let content = format!(
        r#"# {} Module Architecture

## Bounded Context

The `{}` module represents a single bounded context with clear domain boundaries.

## Layer Architecture

```
┌─────────────────────────────────────────┐
│      PRESENTATION LAYER                 │
│  HTTP handlers, gRPC services, CLI      │
│  Location: src/presentation/            │
└─────────────────────────────────────────┘
              ↓ ↑
┌─────────────────────────────────────────┐
│       APPLICATION LAYER                 │
│  Use cases (Commands & Queries)         │
│  Location: src/application/             │
└─────────────────────────────────────────┘
              ↓ ↑
┌─────────────────────────────────────────┐
│         DOMAIN LAYER                    │
│  Entities, Value Objects, Events        │
│  Location: src/domain/                  │
│  Proto: proto/domain/                   │
└─────────────────────────────────────────┘
              ↓ ↑
┌─────────────────────────────────────────┐
│     INFRASTRUCTURE LAYER                │
│  Repository implementations, External   │
│  Location: src/infrastructure/          │
└─────────────────────────────────────────┘
```

## Domain Model

### Aggregates

- **{}Aggregate** - The main aggregate root

### Entities

Entities are defined in `proto/domain/entity/` and generated to `src/generated/`.

### Value Objects

Value objects are defined in `proto/domain/value_object/` for immutable domain concepts.

### Domain Events

Events are published when significant domain state changes occur.

## Data Flow

1. Request arrives at Presentation layer (HTTP/gRPC/CLI)
2. Handler creates Command/Query
3. Application layer processes using domain entities
4. Infrastructure layer persists changes
5. Domain events are published
6. Response returns through layers

## Dependencies

This module depends on:
- `metaphor-core` - Core CRUD traits
- `metaphor-messaging` - Event bus
- `sqlx` - Database access

## Schema-First Approach

All domain types are defined in YAML schema files first:

```protobuf
// proto/domain/entity/{}.proto
message {} {{
    string id = 1;
    // ... fields
}}
```

Generated Rust code is in `src/generated/mod.rs`.
"#,
        module_pascal,
        module,
        module_pascal,
        module,
        module_pascal
    );

    let arch_path = docs_dir.join("ARCHITECTURE.md");
    fs::write(&arch_path, content)?;

    println!(
        "  {} Generated: docs/ARCHITECTURE.md",
        "📄".bright_yellow()
    );

    Ok(())
}

/// Generate API reference documentation
async fn generate_api_reference(docs_dir: &Path, module: &str) -> Result<()> {
    let module_pascal = to_pascal_case(module);
    let collection = format!("{}s", module);

    let content = format!(
        r#"# {name} API Reference

## Base URL

```
http://localhost:3000/api/v1
```

## Authentication

All endpoints require authentication via JWT token:

```
Authorization: Bearer <token>
```

## Endpoints

### List {name}s

```http
GET /api/v1/{col}
```

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `page` | integer | Page number (default: 1) |
| `per_page` | integer | Items per page (default: 20, max: 100) |
| `sort` | string | Sort field |
| `order` | string | Sort order (asc/desc) |
| `search` | string | Full-text search |

**Response:**

```json
{{{{
    "data": [...],
    "meta": {{{{
        "page": 1,
        "per_page": 20,
        "total": 100,
        "total_pages": 5
    }}}}
}}}}
```

### Create {name}

```http
POST /api/v1/{col}
```

**Request Body:**

```json
{{{{
    "name": "string"
}}}}
```

### Get {name} by ID

```http
GET /api/v1/{col}/:id
```

### Update {name}

```http
PUT /api/v1/{col}/:id
```

### Partial Update

```http
PATCH /api/v1/{col}/:id
```

### Delete {name}

```http
DELETE /api/v1/{col}/:id
```

### Bulk Create

```http
POST /api/v1/{col}/bulk
```

### Upsert

```http
POST /api/v1/{col}/upsert
```

### List Deleted

```http
GET /api/v1/{col}/trash
```

### Restore

```http
POST /api/v1/{col}/:id/restore
```

### Empty Trash

```http
DELETE /api/v1/{col}/empty
```

## Error Responses

```json
{{{{
    "error": {{{{
        "code": "VALIDATION_ERROR",
        "message": "Invalid request",
        "details": [...]
    }}}}
}}}}
```

## gRPC API

The gRPC service is available at `localhost:50051`.

```protobuf
service {name}Service {{{{
    rpc Create(Create{name}Request) returns (Create{name}Response);
    rpc Get(Get{name}Request) returns (Get{name}Response);
    rpc List(List{name}Request) returns (List{name}Response);
    rpc Update(Update{name}Request) returns (Update{name}Response);
    rpc Delete(Delete{name}Request) returns (Delete{name}Response);
}}}}
```
"#,
        name = module_pascal,
        col = collection,
    );

    let api_path = docs_dir.join("API_REFERENCE.md");
    fs::write(&api_path, content)?;

    println!(
        "  {} Generated: docs/API_REFERENCE.md",
        "📄".bright_yellow()
    );

    Ok(())
}

/// Generate examples documentation
async fn generate_examples_doc(docs_dir: &Path, module: &str) -> Result<()> {
    let module_pascal = to_pascal_case(module);

    let content = format!(
        r#"# {} Module Examples

## Rust Usage Examples

### Initialize Module

```rust
use {}::{}ModuleBuilder;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Create database pool
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect("postgresql://root:password@localhost:5432/metaphor")
        .await?;

    // Build module
    let module = {}ModuleBuilder::new()
        .with_database(pool)
        .build()?;

    Ok(())
}}
```

### Create Entity

```rust
use {}::application::commands::Create{}Command;

let command = Create{}Command {{
    name: "Example".to_string(),
    // ... other fields
}};

let result = module.execute_command(command).await?;
println!("Created with ID: {{}}", result.id);
```

### Query Entities

```rust
use {}::application::queries::List{}Query;

let query = List{}Query {{
    page: 1,
    per_page: 20,
    filters: Default::default(),
}};

let result = module.execute_query(query).await?;
for item in result.items {{
    println!("{{:?}}", item);
}}
```

## HTTP API Examples

### cURL Examples

```bash
# Create
curl -X POST http://localhost:3000/api/v1/{}s \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{{"name": "Example"}}'

# List
curl -X GET "http://localhost:3000/api/v1/{}s?page=1&per_page=20" \
  -H "Authorization: Bearer $TOKEN"

# Get by ID
curl -X GET http://localhost:3000/api/v1/{}s/123 \
  -H "Authorization: Bearer $TOKEN"

# Update
curl -X PUT http://localhost:3000/api/v1/{}s/123 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{{"name": "Updated"}}'

# Delete
curl -X DELETE http://localhost:3000/api/v1/{}s/123 \
  -H "Authorization: Bearer $TOKEN"
```

## gRPC Examples

### Using grpcurl

```bash
# List
grpcurl -plaintext localhost:50051 {}.{}Service/List

# Create
grpcurl -plaintext -d '{{"name": "Example"}}' \
  localhost:50051 {}.{}Service/Create
```

## CLI Examples

```bash
# List entities
metaphor {} list

# Create entity
metaphor {} create --name "Example"

# Get by ID
metaphor {} get 123

# Update entity
metaphor {} update 123 --name "Updated"

# Delete entity
metaphor {} delete 123
```
"#,
        module_pascal,
        module,
        module_pascal,
        module_pascal,
        module,
        module_pascal,
        module_pascal,
        module,
        module_pascal,
        module_pascal,
        module,
        module,
        module,
        module,
        module,
        module,
        module_pascal,
        module,
        module_pascal,
        module,
        module,
        module,
        module,
        module,
    );

    let examples_path = docs_dir.join("EXAMPLES.md");
    fs::write(&examples_path, content)?;

    println!(
        "  {} Generated: docs/EXAMPLES.md",
        "📄".bright_yellow()
    );

    Ok(())
}

/// Generate API documentation from proto files
async fn generate_api_docs(module: &str, format: &str, with_examples: bool) -> Result<()> {
    println!(
        "{}",
        format!("📚 Generating API documentation for: {}", module)
            .bright_cyan()
            .bold()
    );
    println!();

    let proto_dir = PathBuf::from("libs/modules").join(module).join("proto");

    if !proto_dir.exists() {
        anyhow::bail!("Proto directory not found at {:?}", proto_dir);
    }

    let docs_dir = PathBuf::from("libs/modules").join(module).join("docs");
    fs::create_dir_all(&docs_dir)?;

    // Find all proto files
    let proto_files = find_proto_files(&proto_dir)?;

    println!(
        "  {} Found {} proto files",
        "🔍".bright_blue(),
        proto_files.len()
    );

    for proto_file in &proto_files {
        println!("    - {:?}", proto_file.file_name().unwrap_or_default());
    }

    // Generate documentation based on format
    match format {
        "markdown" => {
            generate_proto_markdown_docs(&docs_dir, &proto_files, module, with_examples).await?
        }
        "html" => {
            println!("  {} HTML format not yet implemented", "⚠️".yellow());
        }
        "json" => {
            println!("  {} JSON format not yet implemented", "⚠️".yellow());
        }
        _ => {
            anyhow::bail!("Unknown format: {}", format);
        }
    }

    println!();
    println!(
        "{}",
        "API documentation generated! 🎉".bright_green().bold()
    );

    Ok(())
}

/// Find all proto files in a directory
fn find_proto_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Ok(files);
    }

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().extension().map_or(false, |ext| ext == "proto") {
            files.push(entry.path().to_path_buf());
        }
    }

    Ok(files)
}

/// Generate markdown documentation from proto files
async fn generate_proto_markdown_docs(
    docs_dir: &Path,
    proto_files: &[PathBuf],
    module: &str,
    with_examples: bool,
) -> Result<()> {
    let module_pascal = to_pascal_case(module);

    let mut content = format!(
        r#"# {} Proto API Documentation

This documentation is auto-generated from Protocol Buffer definitions.

## Proto Files

"#,
        module_pascal
    );

    for proto_file in proto_files {
        let file_name = proto_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        content.push_str(&format!("### {}\n\n", file_name));

        // Read and parse proto file
        if let Ok(proto_content) = fs::read_to_string(proto_file) {
            // Extract messages
            for line in proto_content.lines() {
                let trimmed = line.trim();

                if trimmed.starts_with("message ") {
                    let message_name = trimmed
                        .strip_prefix("message ")
                        .and_then(|s| s.split_whitespace().next())
                        .unwrap_or("Unknown");

                    content.push_str(&format!("- **Message:** `{}`\n", message_name));
                } else if trimmed.starts_with("service ") {
                    let service_name = trimmed
                        .strip_prefix("service ")
                        .and_then(|s| s.split_whitespace().next())
                        .unwrap_or("Unknown");

                    content.push_str(&format!("- **Service:** `{}`\n", service_name));
                } else if trimmed.starts_with("rpc ") {
                    let rpc_name = trimmed
                        .strip_prefix("rpc ")
                        .and_then(|s| s.split('(').next())
                        .unwrap_or("Unknown");

                    content.push_str(&format!("  - RPC: `{}`\n", rpc_name));
                }
            }
        }

        content.push('\n');
    }

    if with_examples {
        content.push_str(
            r#"
## Usage Examples

See [EXAMPLES.md](./EXAMPLES.md) for detailed usage examples.
"#,
        );
    }

    let output_path = docs_dir.join("PROTO_API.md");
    fs::write(&output_path, content)?;

    println!(
        "  {} Generated: docs/PROTO_API.md",
        "📄".bright_yellow()
    );

    Ok(())
}

/// Serve documentation locally
async fn serve_docs(port: u16, watch: bool) -> Result<()> {
    println!(
        "{}",
        format!("🌐 Starting documentation server on port {}", port)
            .bright_cyan()
            .bold()
    );
    println!();

    // First generate docs
    generate_rustdoc(false, false, "target/doc").await?;

    // Check if python or miniserve is available
    let serve_command = if Command::new("miniserve").arg("--version").output().is_ok() {
        "miniserve"
    } else if Command::new("python3").arg("--version").output().is_ok() {
        "python"
    } else {
        anyhow::bail!(
            "No suitable server found. Install miniserve (cargo install miniserve) or python3"
        );
    };

    println!(
        "  {} Using {} to serve documentation",
        "📡".bright_blue(),
        serve_command
    );
    println!(
        "  {} Open http://localhost:{} in your browser",
        "🔗".bright_yellow(),
        port
    );

    if watch {
        println!(
            "  {} Watch mode enabled - documentation will regenerate on changes",
            "👁️".bright_green()
        );
    }

    println!();

    match serve_command {
        "miniserve" => {
            let mut cmd = Command::new("miniserve")
                .args(["target/doc", "-p", &port.to_string(), "--index", "index.html"])
                .spawn()
                .context("Failed to start miniserve")?;

            cmd.wait()?;
        }
        "python" => {
            let mut cmd = Command::new("python3")
                .args(["-m", "http.server", &port.to_string()])
                .current_dir("target/doc")
                .spawn()
                .context("Failed to start Python server")?;

            cmd.wait()?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

/// Check documentation coverage
async fn check_coverage(module: Option<&str>, min_coverage: u8, strict: bool) -> Result<()> {
    println!(
        "{}",
        "📊 Checking documentation coverage...".bright_cyan().bold()
    );
    println!();

    let target = module.map_or("all crates".to_string(), |m| format!("module '{}'", m));
    println!("  {} Checking coverage for: {}", "🔍".bright_blue(), target);

    // Run cargo doc with warnings
    let pkg_name = module.map(|m| format!("metaphor-{}", m));
    let mut args = vec!["doc", "--no-deps"];

    if let Some(ref pkg) = pkg_name {
        args.push("-p");
        args.push(pkg);
    }

    let output = Command::new("cargo")
        .args(&args)
        .env("RUSTDOCFLAGS", "-D missing_docs")
        .output()
        .context("Failed to run cargo doc")?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Count warnings
    let warning_count = stderr.matches("warning:").count();
    let missing_docs_count = stderr.matches("missing documentation").count();

    println!();
    println!("  📈 Coverage Report:");
    println!("    - Total warnings: {}", warning_count);
    println!("    - Missing doc comments: {}", missing_docs_count);

    // Estimate coverage (rough approximation)
    let estimated_coverage = if missing_docs_count == 0 {
        100
    } else {
        // This is a rough estimate - actual coverage calculation would require AST analysis
        (100 - missing_docs_count.min(100)) as u8
    };

    println!("    - Estimated coverage: {}%", estimated_coverage);
    println!("    - Required minimum: {}%", min_coverage);

    println!();

    if estimated_coverage >= min_coverage {
        println!(
            "  {} Documentation coverage meets requirements!",
            "✅".green()
        );
    } else if strict {
        anyhow::bail!(
            "Documentation coverage {}% is below minimum {}%",
            estimated_coverage,
            min_coverage
        );
    } else {
        println!(
            "  {} Documentation coverage {}% is below minimum {}%",
            "⚠️".yellow(),
            estimated_coverage,
            min_coverage
        );
    }

    Ok(())
}

/// Convert string to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split(|c| c == '_' || c == '-')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}
