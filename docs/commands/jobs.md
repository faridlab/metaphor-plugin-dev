# jobs — Job Scheduling Commands

Create, configure, and manage cron-scheduled background jobs for the Metaphor framework.

## Overview

The `metaphor-dev jobs` command provides tools for creating scheduled background jobs, validating cron expressions, generating scheduler configuration, and scaffolding job modules. Jobs are Rust structs generated from templates with built-in scheduling, retry logic, and timeout handling.

### Subcommand Summary

| Subcommand | Description |
|------------|-------------|
| `jobs create <name>` | Create a new scheduled job from a template |
| `jobs templates` | List available job templates |
| `jobs validate-cron <expr>` | Validate a cron expression and show next executions |
| `jobs config` | Generate job scheduler configuration |
| `jobs example` | Create job example files |
| `jobs init` | Initialize jobs module in current project |

---

## `jobs create`

Create a new scheduled job.

### Synopsis

```bash
metaphor-dev jobs create <name> --cron <expr> [OPTIONS]
```

### Description

Generates a Rust source file for a new scheduled job using the built-in template at `src/templates/jobs/job.rs`. The generated file includes a job struct with builder methods, execution logic, error handling, retry configuration, and unit tests.

The job name is converted to PascalCase for the struct name and snake_case for the file name. For example, `database_backup` becomes `DatabaseBackupJob` struct in `database_backup.rs`.

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Job name in PascalCase (e.g., "DatabaseBackup", "EmailCampaign") |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--cron <expr>` | string | (required) | Cron expression (e.g., `"0 2 * * *"` for daily at 2 AM) |
| `--queue <name>` | string | `"default"` | Target queue name for job dispatching |
| `--description <text>` | string | (none) | Human-readable job description |
| `--module <name>` | string | (none) | Target module (outputs to `libs/modules/{name}/src/infrastructure/jobs/`) |
| `--template <name>` | string | (none) | Use a predefined job template |
| `--timeout <seconds>` | u32 | 300 | Job timeout in seconds |
| `--retries <count>` | u32 | 3 | Number of retry attempts on failure |

### Generated File Structure

The generated job file contains:

```rust
pub struct {{PascalCase}}Job {
    id: String,
    name: String,
    cron_expression: String,
    queue_name: String,
    timeout_seconds: u32,
}

impl {{PascalCase}}Job {
    pub fn new() -> Self { ... }
    pub fn with_cron(mut self, cron: &str) -> Self { ... }
    pub fn with_queue(mut self, queue: &str) -> Self { ... }
    pub fn with_timeout(mut self, timeout: u32) -> Self { ... }
    pub async fn execute(&self) -> Result<()> { ... }
    fn validate_payload(&self) -> Result<()> { ... }
    fn handle_error(&self, error: &str) { ... }
}

pub fn register_job() -> {{PascalCase}}Job { ... }

#[cfg(test)]
mod tests { ... }
```

### Examples

```bash
# Create a daily backup job
metaphor-dev jobs create DatabaseBackup --cron "0 2 * * *" --description "Daily database backup"

# Create a job in a specific module with custom timeout
metaphor-dev jobs create SessionCleanup --cron "0 */6 * * *" --module sapiens --timeout 600

# Create a job with custom queue and retry count
metaphor-dev jobs create EmailCampaign --cron "0 9 * * 1" --queue "email" --retries 5

# Create from a predefined template
metaphor-dev jobs create WeeklyReport --cron "0 8 * * 1" --template monthly_report
```

### Notes

- The `--cron` flag is required and validated before file generation
- Without `--module`, the job file is created in the current directory
- The generated file includes placeholder comments (`// TODO:`) for implementation-specific logic
- Built-in tests verify job creation and configuration

---

## `jobs templates`

List available job templates.

### Synopsis

```bash
metaphor-dev jobs templates [OPTIONS]
```

### Description

Displays all 8 built-in job templates with their cron expressions, descriptions, and (optionally) features.

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--detailed` | bool | false | Show features and additional details for each template |

### Available Templates

| Template | Cron Expression | Schedule | Description |
|----------|----------------|----------|-------------|
| `daily_backup` | `0 2 * * *` | Daily at 2:00 AM | Automated database and file backup |
| `weekly_log_cleanup` | `0 3 * * 0` | Sunday at 3:00 AM | Clean up old log files and entries |
| `hourly_data_sync` | `0 * * * *` | Every hour | Synchronize data between services |
| `monthly_report` | `0 8 1 * *` | 1st of month at 8:00 AM | Generate monthly analytics report |
| `session_cleanup` | `*/30 * * * *` | Every 30 minutes | Remove expired user sessions |
| `email_campaigns` | `0 9 * * 1` | Monday at 9:00 AM | Process scheduled email campaigns |
| `database_maintenance` | `0 4 * * 0` | Sunday at 4:00 AM | Run VACUUM, ANALYZE, and index maintenance |
| `cache_warming` | `*/15 * * * *` | Every 15 minutes | Pre-warm frequently accessed caches |

### Examples

```bash
# List all templates
metaphor-dev jobs templates

# Show detailed information
metaphor-dev jobs templates --detailed
```

---

## `jobs validate-cron`

Validate a cron expression and optionally show upcoming execution times.

### Synopsis

```bash
metaphor-dev jobs validate-cron <cron> [OPTIONS]
```

### Description

Parses and validates a cron expression field by field. Supports standard 5-field cron syntax (minute, hour, day-of-month, month, day-of-week). Validates numeric ranges, wildcards, step values, ranges, and lists. Month and day-of-week fields accept named values (jan-dec, sun-sat).

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `cron` | Yes | Cron expression to validate (e.g., `"0 2 * * *"`) |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--show-next` | bool | false | Show next 5 execution times |
| `--timezone <tz>` | string | `"UTC"` | Timezone for execution time display |

### Cron Expression Format

```
┌───────────── minute (0-59)
│ ┌───────────── hour (0-23)
│ │ ┌───────────── day of month (1-31)
│ │ │ ┌───────────── month (1-12 or jan-dec)
│ │ │ │ ┌───────────── day of week (0-7 or sun-sat, 0 and 7 = Sunday)
│ │ │ │ │
* * * * *
```

### Supported Syntax

| Syntax | Meaning | Example |
|--------|---------|---------|
| `*` | Any value | `* * * * *` (every minute) |
| `*/n` | Every n intervals | `*/15 * * * *` (every 15 minutes) |
| `n` | Specific value | `0 2 * * *` (at minute 0, hour 2) |
| `n-m` | Range | `0 9-17 * * *` (hours 9 through 17) |
| `n,m` | List | `0 2,14 * * *` (at 2 AM and 2 PM) |
| `jan`-`dec` | Month names | `0 0 1 jan *` (January 1st) |
| `sun`-`sat` | Day names | `0 0 * * mon` (every Monday) |

### Examples

```bash
# Validate a cron expression
metaphor-dev jobs validate-cron "0 2 * * *"

# Validate and show next executions
metaphor-dev jobs validate-cron "*/15 * * * *" --show-next

# Validate with timezone
metaphor-dev jobs validate-cron "0 9 * * 1" --show-next --timezone "Asia/Jakarta"
```

### Notes

- Uses the `chrono` crate to calculate next execution times
- Day-of-week accepts both 0 and 7 for Sunday
- Invalid expressions report which specific field failed validation

---

## `jobs config`

Generate job scheduler configuration.

### Synopsis

```bash
metaphor-dev jobs config [OPTIONS]
```

### Description

Generates a scheduler configuration file in the specified format. Optionally includes PostgreSQL persistence, Redis queue, and production-optimized settings.

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--format <fmt>` | string | `"yaml"` | Output format: `yaml`, `json`, `toml` |
| `--with-postgres` | bool | false | Include PostgreSQL configuration for job persistence |
| `--with-redis` | bool | false | Include Redis configuration for job queuing |
| `--production` | bool | false | Generate production-ready configuration |

### Examples

```bash
# Generate basic YAML config
metaphor-dev jobs config

# Generate production config with all backends
metaphor-dev jobs config --production --with-postgres --with-redis

# Generate JSON config
metaphor-dev jobs config --format json

# Generate TOML config with PostgreSQL
metaphor-dev jobs config --format toml --with-postgres
```

### Notes

- The generated configuration includes scheduler settings (thread pool, polling interval)
- `--production` adjusts settings for reliability (longer timeouts, more retries, structured logging)
- PostgreSQL configuration includes connection pool settings
- Redis configuration includes connection URL and queue prefix

---

## `jobs example`

Create job example files.

### Synopsis

```bash
metaphor-dev jobs example [OPTIONS]
```

### Description

Generates example Rust files demonstrating how to create and configure scheduled jobs. Three example types are available, each showcasing different patterns.

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--kind <type>` | string | `"basic"` | Example type: `basic`, `advanced`, `cleanup` |
| `--output <dir>` | string | `"./examples"` | Target directory for example files |

### Example Types

| Kind | Description |
|------|-------------|
| `basic` | Simple job with `JobScheduler` and `JobBuilder`, demonstrating basic scheduling |
| `advanced` | Advanced job with error handling, retries, and metrics (referenced but uses basic pattern) |
| `cleanup` | Database cleanup job with connection handling and batch processing |

### Examples

```bash
# Generate basic example
metaphor-dev jobs example

# Generate cleanup example in custom directory
metaphor-dev jobs example --kind cleanup --output ./src/examples

# Generate advanced example
metaphor-dev jobs example --kind advanced
```

---

## `jobs init`

Initialize jobs module in current project.

### Synopsis

```bash
metaphor-dev jobs init [OPTIONS]
```

### Description

Scaffolds the directory structure and starter files for a jobs module. Creates the source directory, configuration files, and optionally adds database migrations, Docker configuration, and monitoring setup.

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--project <name>` | string | `"my_project"` | Project name (used in generated code) |
| `--with-migrations` | bool | false | Add database migration files for job tables |
| `--with-docker` | bool | false | Add Docker Compose configuration for workers |
| `--with-monitoring` | bool | false | Add monitoring and metrics setup |

### Generated Structure

```
src/jobs/
├── mod.rs           # Module declaration
├── scheduler.rs     # Scheduler initialization and configuration
config/
├── jobs.yml         # Job scheduler configuration
migrations/          # (with --with-migrations)
├── create_jobs_table.sql
examples/
├── basic_job.rs     # Example job implementation
```

### Examples

```bash
# Basic initialization
metaphor-dev jobs init

# Full initialization with all options
metaphor-dev jobs init --project my_app --with-migrations --with-docker --with-monitoring

# Initialize with database support
metaphor-dev jobs init --project payments --with-migrations
```

### Notes

- The `scheduler.rs` file includes a `setup_scheduler()` function that configures the job runner
- Migration files create the `scheduled_jobs` table with columns for job state, scheduling, and retry tracking
- Docker configuration adds a `worker` service to `docker-compose.yml`

---

## See Also

- [Job Templates Reference](../reference/job-templates.md) — Detailed template documentation
- [Configuration Reference](../reference/configuration.md) — Environment variable reference
- [Development Workflow Guide](../guides/development-workflow.md) — Using jobs in development
