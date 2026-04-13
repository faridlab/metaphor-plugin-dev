//! Test generation and management commands for Metaphor Framework
//!
//! This module provides commands for auto-generating and managing tests:
//! - Unit tests from proto definitions
//! - Integration tests for CRUD operations
//! - E2E test scaffolding
//! - Test running with coverage
//!
//! # Commands
//!
//! - `metaphor test generate` - Generate tests for an entity
//! - `metaphor test run` - Run tests with various options
//! - `metaphor test coverage` - Generate test coverage report
//! - `metaphor test watch` - Run tests in watch mode

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Test command actions
#[derive(Subcommand, Clone, Debug)]
pub enum TestAction {
    /// Generate tests for an entity or module
    Generate {
        /// Entity name (PascalCase, e.g., "User", "Payment")
        entity: String,

        /// Target module name
        module: String,

        /// Generate unit tests
        #[arg(long, default_value = "true")]
        unit: bool,

        /// Generate integration tests
        #[arg(long)]
        integration: bool,

        /// Generate E2E tests
        #[arg(long)]
        e2e: bool,

        /// Generate all test types
        #[arg(long)]
        all: bool,

        /// Force overwrite existing tests
        #[arg(long)]
        force: bool,
    },

    /// Generate tests for all entities in a module
    GenerateAll {
        /// Target module name
        module: String,

        /// Force overwrite existing tests
        #[arg(long)]
        force: bool,
    },

    /// Run tests
    Run {
        /// Target module (or all if not specified)
        module: Option<String>,

        /// Run only unit tests
        #[arg(long)]
        unit: bool,

        /// Run only integration tests
        #[arg(long)]
        integration: bool,

        /// Run only E2E tests
        #[arg(long)]
        e2e: bool,

        /// Run tests in release mode
        #[arg(long)]
        release: bool,

        /// Show test output
        #[arg(long)]
        nocapture: bool,

        /// Run tests matching pattern
        #[arg(long)]
        filter: Option<String>,
    },

    /// Generate test coverage report
    Coverage {
        /// Target module (or all if not specified)
        module: Option<String>,

        /// Output format (html, lcov, json)
        #[arg(long, default_value = "html")]
        format: String,

        /// Open coverage report in browser
        #[arg(long)]
        open: bool,
    },

    /// Run tests in watch mode
    Watch {
        /// Target module (or all if not specified)
        module: Option<String>,

        /// Only run tests matching pattern
        #[arg(long)]
        filter: Option<String>,
    },

    /// Show test summary for a module
    Summary {
        /// Target module
        module: String,
    },
}

/// Handle test commands
pub async fn handle_command(action: &TestAction) -> Result<()> {
    match action {
        TestAction::Generate {
            entity,
            module,
            unit,
            integration,
            e2e,
            all,
            force,
        } => {
            let generate_all = *all;
            generate_tests(
                entity,
                module,
                if generate_all { true } else { *unit },
                if generate_all { true } else { *integration },
                if generate_all { true } else { *e2e },
                *force,
            )
            .await
        }

        TestAction::GenerateAll { module, force } => generate_all_tests(module, *force).await,

        TestAction::Run {
            module,
            unit,
            integration,
            e2e,
            release,
            nocapture,
            filter,
        } => {
            run_tests(
                module.as_deref(),
                *unit,
                *integration,
                *e2e,
                *release,
                *nocapture,
                filter.as_deref(),
            )
            .await
        }

        TestAction::Coverage {
            module,
            format,
            open,
        } => generate_coverage(module.as_deref(), format, *open).await,

        TestAction::Watch { module, filter } => {
            watch_tests(module.as_deref(), filter.as_deref()).await
        }

        TestAction::Summary { module } => show_test_summary(module).await,
    }
}

/// Generate tests for an entity
async fn generate_tests(
    entity: &str,
    module: &str,
    unit: bool,
    integration: bool,
    e2e: bool,
    force: bool,
) -> Result<()> {
    println!(
        "{}",
        format!("🧪 Generating tests for {} in module {}", entity, module)
            .bright_cyan()
            .bold()
    );
    println!();

    let module_path = PathBuf::from("libs/modules").join(module);

    if !module_path.exists() {
        anyhow::bail!("Module '{}' not found at {:?}", module, module_path);
    }

    let tests_dir = module_path.join("tests");
    fs::create_dir_all(&tests_dir)?;

    let entity_snake = to_snake_case(entity);
    let entity_pascal = to_pascal_case(entity);

    if unit {
        generate_unit_tests(&tests_dir, &entity_pascal, &entity_snake, module, force).await?;
    }

    if integration {
        generate_integration_tests(&tests_dir, &entity_pascal, &entity_snake, module, force)
            .await?;
    }

    if e2e {
        generate_e2e_tests(&tests_dir, &entity_pascal, &entity_snake, module, force).await?;
    }

    // Update tests/mod.rs
    update_tests_mod(&tests_dir, &entity_snake).await?;

    println!();
    println!(
        "{}",
        "Test generation complete! 🎉".bright_green().bold()
    );
    println!();
    println!("Next steps:");
    println!(
        "  1. Review generated tests in {}",
        format!("libs/modules/{}/tests/", module).bright_white()
    );
    println!(
        "  2. Run tests with: {}",
        format!("metaphor test run --module {}", module).bright_yellow()
    );

    Ok(())
}

/// Generate unit tests for an entity
async fn generate_unit_tests(
    tests_dir: &Path,
    entity_pascal: &str,
    entity_snake: &str,
    module: &str,
    force: bool,
) -> Result<()> {
    let file_path = tests_dir.join(format!("{}_unit_tests.rs", entity_snake));

    if file_path.exists() && !force {
        println!(
            "  {} Unit tests already exist (use --force to overwrite)",
            "⏭️".yellow()
        );
        return Ok(());
    }

    let content = format!(
        r#"//! Unit tests for {pascal} entity
//!
//! These tests verify the domain logic and value objects for {pascal}.
//! Generated by Metaphor CLI.

use anyhow::Result;

// Import domain types from the module
// use {mod_name}::domain::entity::{pascal};
// use {mod_name}::domain::value_object::*;

/// Test {pascal} ID generation and validation
#[test]
fn test_{snake}_id_generation() {{{{
    // Test that IDs are generated correctly
    // let id = {pascal}Id::generate();
    // assert!(!id.value().is_empty());
    // assert!(uuid::Uuid::parse_str(id.value()).is_ok());

    // Placeholder - implement when domain types are available
    assert!(true, "Implement {pascal} ID generation test");
}}}}

/// Test {pascal} ID validation with invalid input
#[test]
fn test_{snake}_id_validation_invalid() {{{{
    // Test that invalid IDs are rejected
    // let result = {pascal}Id::new("invalid-uuid");
    // assert!(result.is_err());

    // Placeholder
    assert!(true, "Implement {pascal} ID validation test");
}}}}

/// Test {pascal} entity creation with valid data
#[test]
fn test_{snake}_creation_valid() {{{{
    // Test entity creation with valid data
    // let entity = {pascal}::create(
    //     {pascal}Name::new("Test Name")?,
    //     "Description".to_string(),
    //     "creator".to_string(),
    // )?;
    // assert!(!entity.id().value().is_empty());

    // Placeholder
    assert!(true, "Implement {pascal} creation test");
}}}}

/// Test {pascal} entity creation with invalid data
#[test]
fn test_{snake}_creation_invalid() {{{{
    // Test that invalid data is rejected
    // let result = {pascal}::create(
    //     {pascal}Name::new("")?,  // Empty name should fail
    //     "Description".to_string(),
    //     "creator".to_string(),
    // );
    // assert!(result.is_err());

    // Placeholder
    assert!(true, "Implement {pascal} validation test");
}}}}

/// Test {pascal} value object immutability
#[test]
fn test_{snake}_value_objects() {{{{
    // Test value objects are immutable
    // let name = {pascal}Name::new("Test")?;
    // assert_eq!(name.value(), "Test");

    // Placeholder
    assert!(true, "Implement {pascal} value object test");
}}}}

/// Test {pascal} entity update
#[test]
fn test_{snake}_update() {{{{
    // Test entity update functionality
    // let mut entity = {pascal}::create(...)?;
    // entity.update_name({pascal}Name::new("New Name")?)?;
    // assert_eq!(entity.name().value(), "New Name");

    // Placeholder
    assert!(true, "Implement {pascal} update test");
}}}}

/// Test {pascal} soft delete
#[test]
fn test_{snake}_soft_delete() {{{{
    // Test soft delete functionality
    // let mut entity = {pascal}::create(...)?;
    // entity.soft_delete("deleter".to_string())?;
    // assert!(entity.is_deleted());

    // Placeholder
    assert!(true, "Implement {pascal} soft delete test");
}}}}

/// Test {pascal} restore after soft delete
#[test]
fn test_{snake}_restore() {{{{
    // Test restore functionality
    // let mut entity = {pascal}::create(...)?;
    // entity.soft_delete("deleter".to_string())?;
    // entity.restore()?;
    // assert!(!entity.is_deleted());

    // Placeholder
    assert!(true, "Implement {pascal} restore test");
}}}}

/// Test {pascal} version increments on update
#[test]
fn test_{snake}_version_increment() {{{{
    // Test optimistic locking version increment
    // let mut entity = {pascal}::create(...)?;
    // let initial_version = entity.version();
    // entity.update_name({pascal}Name::new("New")?)?;
    // assert_eq!(entity.version(), initial_version + 1);

    // Placeholder
    assert!(true, "Implement {pascal} version test");
}}}}

/// Test {pascal} timestamps are set correctly
#[test]
fn test_{snake}_timestamps() {{{{
    // Test created_at and updated_at timestamps
    // let entity = {pascal}::create(...)?;
    // assert!(entity.created_at() <= chrono::Utc::now());
    // assert!(entity.updated_at() >= entity.created_at());

    // Placeholder
    assert!(true, "Implement {pascal} timestamps test");
}}}}

// ============================================================================
// Property-Based Tests (if using proptest/quickcheck)
// ============================================================================

// #[cfg(test)]
// mod property_tests {{{{
//     use super::*;
//     use proptest::prelude::*;
//
//     proptest! {{{{
//         #[test]
//         fn test_{snake}_name_never_empty(name in "[a-zA-Z]{{{{1,100}}}}") {{{{
//             let result = {pascal}Name::new(&name);
//             prop_assert!(result.is_ok());
//             prop_assert!(!result.unwrap().value().is_empty());
//         }}}}
//     }}}}
// }}}}
"#,
        pascal = entity_pascal,
        snake = entity_snake,
        mod_name = module,
    );

    fs::write(&file_path, content)?;

    println!(
        "  {} Generated: tests/{}_unit_tests.rs",
        "✅".green(),
        entity_snake
    );

    Ok(())
}

/// Generate integration tests for an entity
async fn generate_integration_tests(
    tests_dir: &Path,
    entity_pascal: &str,
    entity_snake: &str,
    module: &str,
    force: bool,
) -> Result<()> {
    let file_path = tests_dir.join(format!("{}_integration_tests.rs", entity_snake));

    if file_path.exists() && !force {
        println!(
            "  {} Integration tests already exist (use --force to overwrite)",
            "⏭️".yellow()
        );
        return Ok(());
    }

    let _collection = format!("{}s", entity_snake);

    let content = format!(
        r#"//! Integration tests for {pascal} CRUD operations
//!
//! These tests verify the repository and service layer for {pascal}.
//! Requires a running PostgreSQL database.
//! Generated by Metaphor CLI.

use anyhow::Result;

// Test configuration
const TEST_DATABASE_URL: &str = "postgresql://root:password@localhost:5432/test_{mod_name}";

/// Test {pascal} repository - create operation
#[tokio::test]
#[ignore] // Requires database connection
async fn test_{snake}_repository_create() -> Result<()> {{{{
    // Placeholder - implement when repository is available
    Ok(())
}}}}

/// Test {pascal} repository - find by ID
#[tokio::test]
#[ignore]
async fn test_{snake}_repository_find_by_id() -> Result<()> {{{{
    Ok(())
}}}}

/// Test {pascal} repository - list with pagination
#[tokio::test]
#[ignore]
async fn test_{snake}_repository_list_paginated() -> Result<()> {{{{
    Ok(())
}}}}

/// Test {pascal} repository - update
#[tokio::test]
#[ignore]
async fn test_{snake}_repository_update() -> Result<()> {{{{
    Ok(())
}}}}

/// Test {pascal} repository - soft delete
#[tokio::test]
#[ignore]
async fn test_{snake}_repository_soft_delete() -> Result<()> {{{{
    Ok(())
}}}}

/// Test {pascal} repository - restore
#[tokio::test]
#[ignore]
async fn test_{snake}_repository_restore() -> Result<()> {{{{
    Ok(())
}}}}

/// Test {pascal} repository - hard delete
#[tokio::test]
#[ignore]
async fn test_{snake}_repository_hard_delete() -> Result<()> {{{{
    Ok(())
}}}}

/// Test {pascal} repository - bulk create
#[tokio::test]
#[ignore]
async fn test_{snake}_repository_bulk_create() -> Result<()> {{{{
    Ok(())
}}}}

/// Test {pascal} repository - search
#[tokio::test]
#[ignore]
async fn test_{snake}_repository_search() -> Result<()> {{{{
    Ok(())
}}}}

/// Test {pascal} service - create command
#[tokio::test]
#[ignore]
async fn test_{snake}_service_create_command() -> Result<()> {{{{
    Ok(())
}}}}

/// Test {pascal} service - get query
#[tokio::test]
#[ignore]
async fn test_{snake}_service_get_query() -> Result<()> {{{{
    Ok(())
}}}}

/// Test {pascal} service - list query
#[tokio::test]
#[ignore]
async fn test_{snake}_service_list_query() -> Result<()> {{{{
    Ok(())
}}}}
"#,
        pascal = entity_pascal,
        snake = entity_snake,
        mod_name = module,
    );

    fs::write(&file_path, content)?;

    println!(
        "  {} Generated: tests/{}_integration_tests.rs",
        "✅".green(),
        entity_snake
    );

    Ok(())
}

/// Generate E2E tests for an entity
async fn generate_e2e_tests(
    tests_dir: &Path,
    entity_pascal: &str,
    entity_snake: &str,
    _module: &str,
    force: bool,
) -> Result<()> {
    let file_path = tests_dir.join(format!("{}_e2e_tests.rs", entity_snake));

    if file_path.exists() && !force {
        println!(
            "  {} E2E tests already exist (use --force to overwrite)",
            "⏭️".yellow()
        );
        return Ok(());
    }

    let collection = format!("{}s", entity_snake);

    // Use named parameters to avoid format complexity
    let content = format!(
        r#"//! End-to-end tests for {pascal} API endpoints
//!
//! These tests verify the complete HTTP API flow for {pascal}.
//! Requires a running server instance.
//! Generated by Metaphor CLI.

use anyhow::Result;
use reqwest::Client;
use serde_json::{{json, Value}};

// Test configuration
const BASE_URL: &str = "http://localhost:3000/api/v1";
const COLLECTION: &str = "{col}";

/// HTTP client for tests
fn client() -> Client {{
    Client::new()
}}

/// Get authorization header (implement based on your auth system)
fn auth_header() -> (&'static str, String) {{
    ("Authorization", "Bearer test-token".to_string())
}}

// ============================================================================
// Create Endpoint Tests
// ============================================================================

/// Test POST /api/v1/{col} - successful creation
#[tokio::test]
#[ignore] // Requires running server
async fn test_create_{snake}_success() -> Result<()> {{
    let client = client();
    let url = format!("{{}}/{{}}", BASE_URL, COLLECTION);

    let response = client
        .post(&url)
        .header(auth_header().0, auth_header().1)
        .json(&json!({{
            "name": "E2E Test {pascal}",
            "description": "Created by E2E test"
        }}))
        .send()
        .await?;

    assert!(response.status().is_success(), "Expected 2xx status");

    let body: Value = response.json().await?;
    assert!(body.get("id").is_some(), "Response should contain id");

    Ok(())
}}

/// Test POST /api/v1/{col} - validation error
#[tokio::test]
#[ignore]
async fn test_create_{snake}_validation_error() -> Result<()> {{
    let client = client();
    let url = format!("{{}}/{{}}", BASE_URL, COLLECTION);

    let response = client
        .post(&url)
        .header(auth_header().0, auth_header().1)
        .json(&json!({{
            "name": ""
        }}))
        .send()
        .await?;

    assert!(response.status().is_client_error(), "Expected 4xx status");

    Ok(())
}}

// ============================================================================
// Get Endpoint Tests
// ============================================================================

/// Test GET /api/v1/{col}/{{id}} - successful retrieval
#[tokio::test]
#[ignore]
async fn test_get_{snake}_success() -> Result<()> {{
    let client = client();

    // First create an entity
    let create_url = format!("{{}}/{{}}", BASE_URL, COLLECTION);
    let create_response = client
        .post(&create_url)
        .header(auth_header().0, auth_header().1)
        .json(&json!({{
            "name": "Get Test {pascal}",
            "description": "For get test"
        }}))
        .send()
        .await?;

    let created: Value = create_response.json().await?;
    let id = created["id"].as_str().or(created["data"]["id"].as_str()).unwrap();

    // Now retrieve it
    let get_url = format!("{{}}/{{}}/{{}}", BASE_URL, COLLECTION, id);
    let response = client
        .get(&get_url)
        .header(auth_header().0, auth_header().1)
        .send()
        .await?;

    assert!(response.status().is_success());

    Ok(())
}}

/// Test GET /api/v1/{col}/{{id}} - not found
#[tokio::test]
#[ignore]
async fn test_get_{snake}_not_found() -> Result<()> {{
    let client = client();
    let url = format!("{{}}/{{}}/00000000-0000-0000-0000-000000000000", BASE_URL, COLLECTION);

    let response = client
        .get(&url)
        .header(auth_header().0, auth_header().1)
        .send()
        .await?;

    assert_eq!(response.status().as_u16(), 404);

    Ok(())
}}

// ============================================================================
// List Endpoint Tests
// ============================================================================

/// Test GET /api/v1/{col} - list with pagination
#[tokio::test]
#[ignore]
async fn test_list_{snake}_paginated() -> Result<()> {{
    let client = client();
    let url = format!("{{}}/{{}}?page=1&per_page=10", BASE_URL, COLLECTION);

    let response = client
        .get(&url)
        .header(auth_header().0, auth_header().1)
        .send()
        .await?;

    assert!(response.status().is_success());

    let body: Value = response.json().await?;
    assert!(body.get("data").is_some() || body.get("items").is_some());

    Ok(())
}}

/// Test GET /api/v1/{col} - list with search
#[tokio::test]
#[ignore]
async fn test_list_{snake}_search() -> Result<()> {{
    let client = client();
    let url = format!("{{}}/{{}}?search=test", BASE_URL, COLLECTION);

    let response = client
        .get(&url)
        .header(auth_header().0, auth_header().1)
        .send()
        .await?;

    assert!(response.status().is_success());

    Ok(())
}}

// ============================================================================
// Update Endpoint Tests
// ============================================================================

/// Test PUT /api/v1/{col}/{{id}} - full update
#[tokio::test]
#[ignore]
async fn test_update_{snake}_full() -> Result<()> {{
    let client = client();

    // Create entity first
    let create_url = format!("{{}}/{{}}", BASE_URL, COLLECTION);
    let create_response = client
        .post(&create_url)
        .header(auth_header().0, auth_header().1)
        .json(&json!({{
            "name": "Update Test {pascal}",
            "description": "Original"
        }}))
        .send()
        .await?;

    let created: Value = create_response.json().await?;
    let id = created["id"].as_str().or(created["data"]["id"].as_str()).unwrap();

    // Update it
    let update_url = format!("{{}}/{{}}/{{}}", BASE_URL, COLLECTION, id);
    let response = client
        .put(&update_url)
        .header(auth_header().0, auth_header().1)
        .json(&json!({{
            "name": "Updated {pascal}",
            "description": "Updated description"
        }}))
        .send()
        .await?;

    assert!(response.status().is_success());

    Ok(())
}}

/// Test PATCH /api/v1/{col}/{{id}} - partial update
#[tokio::test]
#[ignore]
async fn test_update_{snake}_partial() -> Result<()> {{
    let client = client();

    // Create entity first
    let create_url = format!("{{}}/{{}}", BASE_URL, COLLECTION);
    let create_response = client
        .post(&create_url)
        .header(auth_header().0, auth_header().1)
        .json(&json!({{
            "name": "Patch Test {pascal}",
            "description": "Original"
        }}))
        .send()
        .await?;

    let created: Value = create_response.json().await?;
    let id = created["id"].as_str().or(created["data"]["id"].as_str()).unwrap();

    // Partial update
    let patch_url = format!("{{}}/{{}}/{{}}", BASE_URL, COLLECTION, id);
    let response = client
        .patch(&patch_url)
        .header(auth_header().0, auth_header().1)
        .json(&json!({{
            "description": "Patched description"
        }}))
        .send()
        .await?;

    assert!(response.status().is_success());

    Ok(())
}}

// ============================================================================
// Delete Endpoint Tests
// ============================================================================

/// Test DELETE /api/v1/{col}/{{id}} - soft delete
#[tokio::test]
#[ignore]
async fn test_delete_{snake}_soft() -> Result<()> {{
    let client = client();

    // Create entity first
    let create_url = format!("{{}}/{{}}", BASE_URL, COLLECTION);
    let create_response = client
        .post(&create_url)
        .header(auth_header().0, auth_header().1)
        .json(&json!({{
            "name": "Delete Test {pascal}",
            "description": "To be deleted"
        }}))
        .send()
        .await?;

    let created: Value = create_response.json().await?;
    let id = created["id"].as_str().or(created["data"]["id"].as_str()).unwrap();

    // Soft delete
    let delete_url = format!("{{}}/{{}}/{{}}", BASE_URL, COLLECTION, id);
    let response = client
        .delete(&delete_url)
        .header(auth_header().0, auth_header().1)
        .send()
        .await?;

    assert!(response.status().is_success() || response.status().as_u16() == 204);

    Ok(())
}}

// ============================================================================
// Bulk Operations Tests
// ============================================================================

/// Test POST /api/v1/{col}/bulk - bulk create
#[tokio::test]
#[ignore]
async fn test_bulk_create_{snake}() -> Result<()> {{
    let client = client();
    let url = format!("{{}}/{{}}/bulk", BASE_URL, COLLECTION);

    let response = client
        .post(&url)
        .header(auth_header().0, auth_header().1)
        .json(&json!({{
            "items": [
                {{ "name": "Bulk 1", "description": "First" }},
                {{ "name": "Bulk 2", "description": "Second" }},
                {{ "name": "Bulk 3", "description": "Third" }}
            ]
        }}))
        .send()
        .await?;

    assert!(response.status().is_success());

    Ok(())
}}

// ============================================================================
// Trash Operations Tests
// ============================================================================

/// Test GET /api/v1/{col}/trash - list deleted items
#[tokio::test]
#[ignore]
async fn test_list_{snake}_trash() -> Result<()> {{
    let client = client();
    let url = format!("{{}}/{{}}/trash", BASE_URL, COLLECTION);

    let response = client
        .get(&url)
        .header(auth_header().0, auth_header().1)
        .send()
        .await?;

    assert!(response.status().is_success());

    Ok(())
}}

/// Test POST /api/v1/{col}/{{id}}/restore - restore deleted item
#[tokio::test]
#[ignore]
async fn test_restore_{snake}() -> Result<()> {{
    let client = client();

    // Create and delete entity first
    let create_url = format!("{{}}/{{}}", BASE_URL, COLLECTION);
    let create_response = client
        .post(&create_url)
        .header(auth_header().0, auth_header().1)
        .json(&json!({{
            "name": "Restore Test {pascal}",
            "description": "To be restored"
        }}))
        .send()
        .await?;

    let created: Value = create_response.json().await?;
    let id = created["id"].as_str().or(created["data"]["id"].as_str()).unwrap();

    // Delete it
    let delete_url = format!("{{}}/{{}}/{{}}", BASE_URL, COLLECTION, id);
    client
        .delete(&delete_url)
        .header(auth_header().0, auth_header().1)
        .send()
        .await?;

    // Restore it
    let restore_url = format!("{{}}/{{}}/{{}}/restore", BASE_URL, COLLECTION, id);
    let response = client
        .post(&restore_url)
        .header(auth_header().0, auth_header().1)
        .send()
        .await?;

    assert!(response.status().is_success());

    Ok(())
}}

// ============================================================================
// Error Handling Tests
// ============================================================================

/// Test unauthorized access
#[tokio::test]
#[ignore]
async fn test_{snake}_unauthorized() -> Result<()> {{
    let client = client();
    let url = format!("{{}}/{{}}", BASE_URL, COLLECTION);

    let response = client
        .get(&url)
        // No auth header
        .send()
        .await?;

    assert!(
        response.status().as_u16() == 401 || response.status().as_u16() == 403,
        "Expected 401 or 403 status"
    );

    Ok(())
}}

/// Test invalid JSON payload
#[tokio::test]
#[ignore]
async fn test_{snake}_invalid_json() -> Result<()> {{
    let client = client();
    let url = format!("{{}}/{{}}", BASE_URL, COLLECTION);

    let response = client
        .post(&url)
        .header(auth_header().0, auth_header().1)
        .header("Content-Type", "application/json")
        .body("{{invalid json}}")
        .send()
        .await?;

    assert!(response.status().is_client_error());

    Ok(())
}}
"#,
        pascal = entity_pascal,
        snake = entity_snake,
        col = collection,
    );

    fs::write(&file_path, content)?;

    println!(
        "  {} Generated: tests/{}_e2e_tests.rs",
        "✅".green(),
        entity_snake
    );

    Ok(())
}

/// Update tests/mod.rs to include new test modules
async fn update_tests_mod(tests_dir: &Path, entity_snake: &str) -> Result<()> {
    let mod_path = tests_dir.join("mod.rs");

    let mut content = if mod_path.exists() {
        fs::read_to_string(&mod_path)?
    } else {
        "//! Test modules for this bounded context\n\n".to_string()
    };

    // Add module declarations if not present
    let modules = [
        format!("mod {}_unit_tests;", entity_snake),
        format!("mod {}_integration_tests;", entity_snake),
        format!("mod {}_e2e_tests;", entity_snake),
    ];

    for module_decl in &modules {
        if !content.contains(module_decl) {
            content.push_str(module_decl);
            content.push('\n');
        }
    }

    fs::write(&mod_path, content)?;

    println!("  {} Updated: tests/mod.rs", "📝".bright_blue());

    Ok(())
}

/// Generate tests for all entities in a module
async fn generate_all_tests(module: &str, force: bool) -> Result<()> {
    println!(
        "{}",
        format!("🧪 Generating tests for all entities in module: {}", module)
            .bright_cyan()
            .bold()
    );
    println!();

    let proto_dir = PathBuf::from("libs/modules")
        .join(module)
        .join("proto/domain/entity");

    if !proto_dir.exists() {
        anyhow::bail!("Proto entity directory not found at {:?}", proto_dir);
    }

    // Find all entity proto files
    let mut entities = Vec::new();

    for entry in fs::read_dir(&proto_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "proto") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                entities.push(to_pascal_case(stem));
            }
        }
    }

    if entities.is_empty() {
        println!("  {} No entity proto files found", "⚠️".yellow());
        return Ok(());
    }

    println!(
        "  {} Found {} entities: {:?}",
        "🔍".bright_blue(),
        entities.len(),
        entities
    );
    println!();

    for entity in entities {
        generate_tests(&entity, module, true, true, true, force).await?;
        println!();
    }

    println!(
        "{}",
        "All tests generated! 🎉".bright_green().bold()
    );

    Ok(())
}

/// Run tests
async fn run_tests(
    module: Option<&str>,
    unit: bool,
    integration: bool,
    e2e: bool,
    release: bool,
    nocapture: bool,
    filter: Option<&str>,
) -> Result<()> {
    println!(
        "{}",
        "🧪 Running tests...".bright_cyan().bold()
    );
    println!();

    let mut args = vec!["test"];

    // Add module filter
    if let Some(m) = module {
        args.push("-p");
        let pkg = format!("metaphor-{}", m);
        args.push(Box::leak(pkg.into_boxed_str()));
    }

    // Add release flag
    if release {
        args.push("--release");
    }

    // Add nocapture flag
    if nocapture {
        args.push("--");
        args.push("--nocapture");
    }

    // Add test type filters
    if unit || integration || e2e {
        if !nocapture {
            args.push("--");
        }

        if unit {
            args.push("unit_test");
        }
        if integration {
            args.push("integration_test");
        }
        if e2e {
            args.push("e2e_test");
        }
    }

    // Add custom filter
    if let Some(f) = filter {
        if !nocapture && !unit && !integration && !e2e {
            args.push("--");
        }
        args.push(Box::leak(f.to_string().into_boxed_str()));
    }

    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to run cargo test")?;

    println!();

    if status.success() {
        println!(
            "{}",
            "All tests passed! ✅".bright_green().bold()
        );
    } else {
        println!(
            "{}",
            "Some tests failed ❌".bright_red().bold()
        );
    }

    Ok(())
}

/// Generate test coverage report
async fn generate_coverage(module: Option<&str>, format: &str, open: bool) -> Result<()> {
    println!(
        "{}",
        "📊 Generating test coverage report...".bright_cyan().bold()
    );
    println!();

    // Check if llvm-cov is installed
    let llvm_cov_check = Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .output();

    if llvm_cov_check.is_err() {
        println!(
            "  {} cargo-llvm-cov not found. Installing...",
            "📦".bright_yellow()
        );

        let install_status = Command::new("cargo")
            .args(["install", "cargo-llvm-cov"])
            .status()?;

        if !install_status.success() {
            anyhow::bail!("Failed to install cargo-llvm-cov");
        }
    }

    let mut args = vec!["llvm-cov"];

    // Add format-specific flags
    match format {
        "html" => args.push("--html"),
        "lcov" => {
            args.push("--lcov");
            args.push("--output-path");
            args.push("coverage.lcov");
        }
        "json" => {
            args.push("--json");
            args.push("--output-path");
            args.push("coverage.json");
        }
        _ => args.push("--html"),
    }

    // Add module filter
    if let Some(m) = module {
        args.push("-p");
        let pkg = format!("metaphor-{}", m);
        args.push(Box::leak(pkg.into_boxed_str()));
    }

    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to run cargo llvm-cov")?;

    if !status.success() {
        anyhow::bail!("Coverage generation failed");
    }

    println!();
    println!(
        "  {} Coverage report generated!",
        "✅".green()
    );

    if open && format == "html" {
        let coverage_path = "target/llvm-cov/html/index.html";
        println!(
            "  {} Opening coverage report...",
            "🌐".bright_blue()
        );

        #[cfg(target_os = "macos")]
        Command::new("open").arg(coverage_path).spawn()?;

        #[cfg(target_os = "linux")]
        Command::new("xdg-open").arg(coverage_path).spawn()?;
    }

    Ok(())
}

/// Run tests in watch mode
async fn watch_tests(module: Option<&str>, filter: Option<&str>) -> Result<()> {
    println!(
        "{}",
        "👁️ Starting test watch mode...".bright_cyan().bold()
    );
    println!();

    // Check if cargo-watch is installed
    let watch_check = Command::new("cargo")
        .args(["watch", "--version"])
        .output();

    if watch_check.is_err() {
        println!(
            "  {} cargo-watch not found. Installing...",
            "📦".bright_yellow()
        );

        let install_status = Command::new("cargo")
            .args(["install", "cargo-watch"])
            .status()?;

        if !install_status.success() {
            anyhow::bail!("Failed to install cargo-watch");
        }
    }

    let mut test_cmd = String::from("cargo test");

    if let Some(m) = module {
        test_cmd.push_str(&format!(" -p metaphor-{}", m));
    }

    if let Some(f) = filter {
        test_cmd.push_str(&format!(" -- {}", f));
    }

    println!(
        "  {} Watching for changes... (Ctrl+C to stop)",
        "🔍".bright_blue()
    );
    println!(
        "  {} Running: {}",
        "▶️".bright_green(),
        test_cmd
    );
    println!();

    let mut child = Command::new("cargo")
        .args(["watch", "-x", &test_cmd.replace("cargo ", "")])
        .spawn()
        .context("Failed to start cargo watch")?;

    child.wait()?;

    Ok(())
}

/// Show test summary for a module
async fn show_test_summary(module: &str) -> Result<()> {
    println!(
        "{}",
        format!("📊 Test summary for module: {}", module)
            .bright_cyan()
            .bold()
    );
    println!();

    let tests_dir = PathBuf::from("libs/modules").join(module).join("tests");

    if !tests_dir.exists() {
        println!("  {} No tests directory found", "⚠️".yellow());
        return Ok(());
    }

    let mut unit_count = 0;
    let mut integration_count = 0;
    let mut e2e_count = 0;

    for entry in fs::read_dir(&tests_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "rs") {
            let content = fs::read_to_string(&path)?;
            let test_count = content.matches("#[test]").count()
                + content.matches("#[tokio::test]").count();

            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if filename.contains("unit") {
                unit_count += test_count;
            } else if filename.contains("integration") {
                integration_count += test_count;
            } else if filename.contains("e2e") {
                e2e_count += test_count;
            }

            println!(
                "  {} {}: {} tests",
                "📄".bright_blue(),
                filename,
                test_count
            );
        }
    }

    println!();
    println!("  Summary:");
    println!("    Unit tests:        {}", unit_count);
    println!("    Integration tests: {}", integration_count);
    println!("    E2E tests:         {}", e2e_count);
    println!("    ─────────────────────");
    println!(
        "    Total:             {}",
        unit_count + integration_count + e2e_count
    );

    Ok(())
}

/// Convert string to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
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
