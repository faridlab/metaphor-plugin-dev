# Clippy Rules Reference

Complete reference for the clippy linting rules enforced by `metaphor-dev lint check`.

## Overview

The Metaphor framework enforces a set of clippy rules to maintain code quality, prevent common bugs, and ensure consistent patterns across the codebase. These rules are applied automatically when running `lint check`, `lint all`, or any command that invokes clippy.

## Denied Lints (Errors)

These lints are treated as errors and will cause the linter to fail. They enforce safe error handling practices.

| Lint | Rule | Description |
|------|------|-------------|
| `clippy::unwrap_used` | `-D clippy::unwrap_used` | Disallows `.unwrap()` calls |
| `clippy::expect_used` | `-D clippy::expect_used` | Disallows `.expect()` calls |

### Rationale

Using `.unwrap()` or `.expect()` on `Option` or `Result` types causes a panic if the value is `None` or `Err`. In a production server, panics terminate the current task (or the entire process), leading to service disruption.

### What to Use Instead

```rust
// Instead of .unwrap()
let value = some_option.unwrap();           // BAD: panics on None

let value = some_option
    .context("description of why this should exist")?;  // GOOD: returns error

let value = some_option.unwrap_or_default(); // GOOD: provides fallback

// Instead of .expect()
let value = some_result.expect("message");   // BAD: panics on Err

let value = some_result
    .context("what we were trying to do")?;  // GOOD: propagates error

if let Some(value) = some_option {           // GOOD: handles both cases
    // use value
}
```

## Warned Lints

These lints produce warnings. In `--strict` mode, warnings become errors.

### General Warnings

| Lint | Rule | Description |
|------|------|-------------|
| `clippy::todo` | `-W clippy::todo` | Flags `todo!()` macro usage |
| `clippy::dbg_macro` | `-W clippy::dbg_macro` | Flags `dbg!()` macro usage |
| `clippy::print_stdout` | `-W clippy::print_stdout` | Flags `println!()` usage |
| `clippy::print_stderr` | `-W clippy::print_stderr` | Flags `eprintln!()` usage |

#### `clippy::todo`

The `todo!()` macro marks incomplete code. It panics at runtime with a "not yet implemented" message. Warnings help track incomplete implementations before they reach production.

#### `clippy::dbg_macro`

The `dbg!()` macro is useful during development but should not be left in production code. It writes to stderr and can expose internal state.

#### `clippy::print_stdout` / `clippy::print_stderr`

Direct printing (`println!`, `eprintln!`) bypasses the structured logging system. Use the `tracing` crate instead:

```rust
// Instead of println!
println!("User {} logged in", user_id);        // BAD: unstructured

tracing::info!(user_id = %user_id, "User logged in"); // GOOD: structured
```

> **Note:** The `metaphor-dev` CLI itself uses `println!` for user-facing output, which is appropriate for CLI tools. This rule targets application/library code.

### Async-Specific Warnings

| Lint | Rule | Description |
|------|------|-------------|
| `clippy::large_futures` | `-W clippy::large_futures` | Flags futures that are too large for stack allocation |
| `clippy::redundant_async_block` | `-W clippy::redundant_async_block` | Flags unnecessary `async {}` blocks |
| `clippy::unused_async` | `-W clippy::unused_async` | Flags `async fn` that never uses `.await` |

#### `clippy::large_futures`

Large futures consume excessive stack space and can cause stack overflows in deeply nested async call chains. Box large futures instead:

```rust
// If the future is too large
let result = Box::pin(large_async_operation()).await;
```

#### `clippy::redundant_async_block`

Flags patterns like `async { some_future.await }` where the wrapping async block adds no value.

#### `clippy::unused_async`

Flags functions marked `async fn` that never use `.await`. Removing `async` from these functions avoids the overhead of creating a future:

```rust
// BAD: async but doesn't await anything
async fn compute(x: i32) -> i32 { x * 2 }

// GOOD: just a regular function
fn compute(x: i32) -> i32 { x * 2 }
```

## Allowed Lints (Framework Exceptions)

These lints are explicitly allowed because they conflict with Metaphor framework patterns.

| Lint | Rule | Description |
|------|------|-------------|
| `clippy::module_inception` | `-A clippy::module_inception` | Allows `domain/domain.rs` pattern |
| `clippy::too_many_arguments` | `-A clippy::too_many_arguments` | Allows builder-pattern constructors |

### `clippy::module_inception`

Clippy normally warns when a module contains a file with the same name (e.g., `domain/domain.rs`). The Metaphor framework uses this pattern intentionally for DDD-style module organization:

```
libs/modules/sapiens/src/
├── domain/
│   ├── mod.rs
│   └── domain.rs      # This is intentional
├── application/
│   ├── mod.rs
│   └── application.rs # This is intentional
```

### `clippy::too_many_arguments`

Builder-pattern constructors and complex configuration functions may require many parameters. The framework allows this rather than forcing artificial struct wrapping for every case.

## Pedantic Mode

Enable pedantic mode with `--pedantic` to get additional warnings from clippy's pedantic lint group:

```bash
metaphor-dev lint check --pedantic
```

Pedantic mode adds `-W clippy::pedantic`, which enables ~100 additional lints covering code style, performance, and correctness. These are informational and may produce many warnings in existing codebases.

Useful pedantic lints include:
- `clippy::needless_pass_by_value` — Suggests borrowing instead of taking ownership
- `clippy::redundant_closure_for_method_calls` — Suggests method references
- `clippy::cast_possible_truncation` — Warns about potentially lossy casts
- `clippy::inefficient_to_string` — Suggests more efficient string conversions

## Strict Mode

Enable strict mode with `--strict` to treat all warnings as errors:

```bash
metaphor-dev lint check --strict
```

This adds `-D warnings` to the clippy invocation, causing any warning to fail the lint check. Recommended for CI pipelines.

## Customization

### Via `clippy.toml`

Create a `clippy.toml` file in the project root to customize thresholds:

```toml
# clippy.toml
cognitive-complexity-threshold = 25
type-complexity-threshold = 300
```

### Via `Cargo.toml`

Add lint configuration to your workspace `Cargo.toml`:

```toml
[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
todo = "warn"
```

### Inline Suppression

For specific cases where a lint should be suppressed:

```rust
#[allow(clippy::unwrap_used)]  // Safe: we just checked is_some() above
let value = option.unwrap();
```

Use inline suppression sparingly and always include a comment explaining why the suppression is safe.

## See Also

- [lint check](../commands/lint.md#lint-check) — Running clippy
- [lint config](../commands/lint.md#lint-config) — Viewing configuration
- [lint all](../commands/lint.md#lint-all) — Running all quality checks
- [CI Integration](../guides/ci-integration.md) — Using strict mode in CI
