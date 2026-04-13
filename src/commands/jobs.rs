//! Job scheduling commands for Metaphor Jobs

use anyhow::Result;
use anyhow::Context;
use clap::Subcommand;
use colored::*;
use std::path::Path;

#[derive(Subcommand)]
pub enum JobsAction {
    /// Create a new scheduled job
    Create {
        /// Job name (PascalCase, e.g., "DatabaseBackup", "EmailCampaign")
        name: String,

        /// Cron expression (e.g., "0 2 * * *" for daily at 2 AM)
        #[arg(long)]
        cron: String,

        /// Target queue name
        #[arg(long, default_value = "default")]
        queue: String,

        /// Job description
        #[arg(long)]
        description: Option<String>,

        /// Target module name
        #[arg(long)]
        module: Option<String>,

        /// Use predefined job type
        #[arg(long)]
        template: Option<String>,

        /// Add timeout in seconds
        #[arg(long, default_value = "300")]
        timeout: u32,

        /// Set retry attempts
        #[arg(long, default_value = "3")]
        retries: u32,
    },

    /// List available job templates
    Templates {
        /// Show detailed information
        #[arg(long)]
        detailed: bool,
    },

    /// Validate cron expression
    ValidateCron {
        /// Cron expression to validate
        cron: String,

        /// Show next 5 execution times
        #[arg(long)]
        show_next: bool,

        /// Timezone for validation
        #[arg(long, default_value = "UTC")]
        timezone: String,
    },

    /// Generate job scheduler configuration
    Config {
        /// Output format (yaml, json, toml)
        #[arg(long, default_value = "yaml")]
        format: String,

        /// Include PostgreSQL configuration
        #[arg(long)]
        with_postgres: bool,

        /// Include Redis configuration
        #[arg(long)]
        with_redis: bool,

        /// Production-ready configuration
        #[arg(long)]
        production: bool,
    },

    /// Create job example files
    Example {
        /// Example type (basic, advanced, cleanup)
        #[arg(long, default_value = "basic")]
        kind: String,

        /// Target directory for example
        #[arg(long, default_value = "./examples")]
        output: String,
    },

    /// Initialize jobs module in current project
    Init {
        /// Project name
        #[arg(long, default_value = "my_project")]
        project: String,

        /// Add database migration files
        #[arg(long)]
        with_migrations: bool,

        /// Add Docker configuration
        #[arg(long)]
        with_docker: bool,

        /// Add monitoring setup
        #[arg(long)]
        with_monitoring: bool,
    },
}

pub async fn handle_jobs_command(action: &JobsAction) -> Result<()> {
    match action {
        JobsAction::Create {
            name,
            cron,
            queue,
            description,
            module,
            template,
            timeout,
            retries,
        } => {
            println!("⏰ {} scheduled job '{}'",
                "Creating".bright_green(),
                name.bright_cyan()
            );
            println!("   Cron: {}", cron.bright_yellow());
            println!("   Queue: {}", queue.bright_blue());
            println!("   Timeout: {}s", timeout);
            println!("   Retries: {}", retries);

            if let Some(desc) = description {
                println!("   Description: {}", desc);
            }

            if let Some(template_name) = template {
                println!("   Template: {}", template_name.bright_green());
            }

            if let Some(module_name) = module {
                println!("   Module: {}", module_name.bright_magenta());
            }

            // Validate cron expression first
            if !validate_cron_expression(cron) {
                return Err(anyhow::anyhow!("❌ Invalid cron expression: {}", cron));
            }

            // Generate actual job code
            generate_job_code(name, cron, description.as_deref(), module.as_deref()).await?;

            println!("✅ Cron expression is valid");
        }

        JobsAction::Templates { detailed } => {
            println!("📋 Available Job Templates:");
            println!();

            let templates = vec![
                ("daily_backup", "Daily database backup at 2 AM", "0 2 * * *"),
                ("weekly_log_cleanup", "Weekly log cleanup on Sunday at 3 AM", "0 3 * * 0"),
                ("hourly_data_sync", "Hourly data synchronization", "0 * * * *"),
                ("monthly_report", "Monthly analytics report on 1st at 6 AM", "0 6 1 * *"),
                ("session_cleanup", "Session cleanup every 6 hours", "0 */6 * * *"),
                ("email_campaigns", "Email campaigns on weekdays at 9 AM", "0 9 * * 1-5"),
                ("database_maintenance", "Database maintenance weekly on Sunday at 1 AM", "0 1 * * 0"),
                ("cache_warming", "Cache warming every 30 minutes", "*/30 * * * *"),
            ];

            for (name, description, cron) in templates {
                println!("  {}{} - {}",
                    name.bright_green(),
                    ":".bright_black(),
                    description
                );
                println!("    Cron: {}", cron.bright_yellow());
                println!("    Usage: metaphor jobs create --template {} MyJob", name);

                if *detailed {
                    println!("    Features:");
                    match name {
                        "daily_backup" => {
                            println!("      - Full database backup");
                            println!("      - Compression enabled");
                            println!("      - Retention policy");
                        }
                        "weekly_log_cleanup" => {
                            println!("      - Log rotation");
                            println!("      - Old file cleanup");
                            println!("      - Archive creation");
                        }
                        _ => {
                            println!("      - Standard retry policy");
                            println!("      - Error handling");
                            println!("      - Monitoring integration");
                        }
                    }
                }
                println!();
            }
        }

        JobsAction::ValidateCron { cron, show_next, timezone } => {
            println!("🕐 Validating cron expression: {}", cron.bright_yellow());

            if validate_cron_expression(cron) {
                println!("✅ {} is valid", cron.bright_green());

                if *show_next {
                    println!("\n📅 Next 5 execution times ({}):", timezone);
                    match calculate_next_executions(cron, 5) {
                        Ok(times) => {
                            for (i, time) in times.iter().enumerate() {
                                println!("   {}. {}", i + 1, time.format("%Y-%m-%d %H:%M:%S").to_string().bright_cyan());
                            }
                        }
                        Err(e) => {
                            println!("   ⚠️  Could not calculate next executions: {}", e);
                        }
                    }
                }
            } else {
                println!("❌ {} is invalid", cron.bright_red());
                return Err(anyhow::anyhow!("Invalid cron expression"));
            }
        }

        JobsAction::Config { format, with_postgres, with_redis, production } => {
            println!("⚙️ Generating job scheduler configuration...");

            let config = generate_scheduler_config(*with_postgres, *with_redis, *production);

            match format.as_str() {
                "yaml" => {
                    println!("```yaml");
                    println!("{}", config);
                    println!("```");
                }
                "json" => {
                    println!("```json");
                    println!("{}", config);
                    println!("```");
                }
                "toml" => {
                    println!("```toml");
                    println!("{}", config);
                    println!("```");
                }
                _ => {
                    return Err(anyhow::anyhow!("Unsupported format: {}", format));
                }
            }

            println!("\n💡 Save this to your config file (e.g., application.yml)");
        }

        JobsAction::Example { kind, output } => {
            println!("📝 Creating {} job example...", kind.bright_green());

            let example_code = match kind.as_str() {
                "basic" => generate_basic_example(),
                "advanced" => generate_advanced_example(),
                "cleanup" => generate_cleanup_example(),
                _ => {
                    return Err(anyhow::anyhow!("Unknown example type: {}", kind));
                }
            };

            let output_path = Path::new(output);
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file_path = output_path.join(format!("{}_example.rs", kind));
            std::fs::write(&file_path, example_code)?;

            println!("✅ Example created at: {}", file_path.display().to_string().bright_green());
            println!("💡 Run with: cargo run --example {}", kind);
        }

        JobsAction::Init {
            project,
            with_migrations,
            with_docker,
            with_monitoring
        } => {
            println!("🚀 Initializing Metaphor Jobs for project: {}", project.bright_cyan());

            // Create directory structure
            let dirs = vec![
                "src/jobs",
                "src/jobs/templates",
                "config",
                "migrations",
                "examples",
            ];

            for dir in dirs {
                std::fs::create_dir_all(dir)?;
                println!("📁 Created directory: {}", dir);
            }

            // Generate main job scheduler file
            let scheduler_code = generate_scheduler_file(project);
            std::fs::write("src/jobs/scheduler.rs", scheduler_code)?;
            println!("📝 Created: src/jobs/scheduler.rs");

            if *with_migrations {
                println!("📋 Database migrations: enabled");
                // TODO: Generate migration files
            }

            if *with_docker {
                println!("🐳 Docker configuration: enabled");
                // TODO: Generate docker-compose file
            }

            if *with_monitoring {
                println!("📊 Monitoring setup: enabled");
                // TODO: Generate monitoring configuration
            }

            println!("\n✅ Metaphor Jobs initialized successfully!");
            println!("💡 Next steps:");
            println!("   1. Add metaphor-jobs to Cargo.toml");
            println!("   2. Configure your database connection");
            println!("   3. Create your first job: metaphor jobs create MyJob");
        }
    }

    Ok(())
}

fn validate_cron_expression(cron: &str) -> bool {
    // Parse and validate cron expression
    let parts: Vec<&str> = cron.split_whitespace().collect();
    if parts.len() != 5 && parts.len() != 6 {
        // 5 parts for standard cron, 6 for including seconds
        return false;
    }

    // Validate each part
    let (minute, hour, day_month, month, day_week) = (&parts[0], &parts[1], &parts[2], &parts[3], &parts[4]);

    // Validate minute (0-59, *, */n, n-m, list)
    if !validate_cron_field(minute, 0, 59) {
        return false;
    }

    // Validate hour (0-23, *, */n, n-m, list)
    if !validate_cron_field(hour, 0, 23) {
        return false;
    }

    // Validate day of month (1-31, *, */n, n-m, list)
    if !validate_cron_field(day_month, 1, 31) {
        return false;
    }

    // Validate month (1-12, jan-dec, *, */n, n-m, list)
    if !validate_cron_field_month(month) {
        return false;
    }

    // Validate day of week (0-7, sun-sat, *, */n, n-m, list)
    if !validate_cron_field_day_week(day_week) {
        return false;
    }

    true
}

/// Validate a numeric cron field (minute, hour, day of month)
fn validate_cron_field(field: &str, min: u32, max: u32) -> bool {
    if field == "*" {
        return true;
    }

    // Handle */n pattern
    if field.starts_with("*/") {
        if let Ok(n) = field[2..].parse::<u32>() {
            return n <= max && n > 0;
        }
        return false;
    }

    // Handle list (comma-separated values)
    if field.contains(',') {
        for part in field.split(',') {
            if !validate_cron_field(part.trim(), min, max) {
                return false;
            }
        }
        return true;
    }

    // Handle range (n-m)
    if field.contains('-') {
        let range_parts: Vec<&str> = field.split('-').collect();
        if range_parts.len() != 2 {
            return false;
        }
        let start = range_parts[0].parse::<u32>();
        let end = range_parts[1].parse::<u32>();
        if start.is_ok() && end.is_ok() {
            let s = start.unwrap();
            let e = end.unwrap();
            return s >= min && s <= max && e >= min && e <= max && e >= s;
        }
        return false;
    }

    // Simple number
    match field.parse::<u32>() {
        Ok(n) => n >= min && n <= max,
        Err(_) => false,
    }
}

/// Validate month field (1-12 or names)
fn validate_cron_field_month(field: &str) -> bool {
    if field == "*" {
        return true;
    }

    let month_names = vec![
        "jan", "feb", "mar", "apr", "may", "jun",
        "jul", "aug", "sep", "oct", "nov", "dec",
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12",
    ];

    if month_names.contains(&field.to_lowercase().as_str()) {
        return true;
    }

    // Handle */n, lists, and ranges for numeric values
    if field.starts_with("*/") {
        if let Ok(n) = field[2..].parse::<u32>() {
            return n <= 12 && n > 0;
        }
        return false;
    }

    if field.contains(',') {
        for part in field.split(',') {
            if !validate_cron_field_month(part.trim()) {
                return false;
            }
        }
        return true;
    }

    if field.contains('-') {
        let range_parts: Vec<&str> = field.split('-').collect();
        if range_parts.len() != 2 {
            return false;
        }
        let start = range_parts[0].parse::<u32>();
        let end = range_parts[1].parse::<u32>();
        if start.is_ok() && end.is_ok() {
            let s = start.unwrap();
            let e = end.unwrap();
            return s >= 1 && s <= 12 && e >= 1 && e <= 12 && e >= s;
        }
        return false;
    }

    match field.parse::<u32>() {
        Ok(n) => n >= 1 && n <= 12,
        Err(_) => false,
    }
}

/// Validate day of week field (0-7 or names)
fn validate_cron_field_day_week(field: &str) -> bool {
    if field == "*" {
        return true;
    }

    let day_names = vec![
        "sun", "mon", "tue", "wed", "thu", "fri", "sat",
        "0", "1", "2", "3", "4", "5", "6", "7",
    ];

    if day_names.contains(&field.to_lowercase().as_str()) {
        return true;
    }

    // Handle */n, lists, and ranges
    if field.starts_with("*/") {
        if let Ok(n) = field[2..].parse::<u32>() {
            return n <= 7 && n > 0;
        }
        return false;
    }

    if field.contains(',') {
        for part in field.split(',') {
            if !validate_cron_field_day_week(part.trim()) {
                return false;
            }
        }
        return true;
    }

    if field.contains('-') {
        let range_parts: Vec<&str> = field.split('-').collect();
        if range_parts.len() != 2 {
            return false;
        }
        let start = range_parts[0].parse::<u32>();
        let end = range_parts[1].parse::<u32>();
        if start.is_ok() && end.is_ok() {
            let s = start.unwrap();
            let e = end.unwrap();
            return s <= 7 && e <= 7 && e >= s;
        }
        return false;
    }

    match field.parse::<u32>() {
        Ok(n) => n <= 7,
        Err(_) => false,
    }
}

/// Calculate next execution times for a cron expression
fn calculate_next_executions(cron: &str, count: usize) -> Result<Vec<chrono::DateTime<chrono::Utc>>> {
    use chrono::{Timelike, Datelike, Duration};

    let mut executions = Vec::new();
    let mut current = chrono::Utc::now() + Duration::minutes(1); // Start from next minute

    let parts: Vec<&str> = cron.split_whitespace().collect();
    if parts.len() < 5 {
        return Err(anyhow::anyhow!("Invalid cron expression"));
    }

    let minute_expr = parts[0];
    let hour_expr = parts[1];
    let day_month_expr = parts[2];
    let month_expr = parts[3];
    let day_week_expr = parts[4];

    while executions.len() < count {
        // Check if current time matches all cron expressions
        if matches_cron_field(current.minute(), minute_expr, 0, 59)
            && matches_cron_field(current.hour(), hour_expr, 0, 23)
            && matches_cron_field(current.day(), day_month_expr, 1, 31)
            && matches_cron_field(current.month(), month_expr, 1, 12)
            && matches_cron_field(current.weekday().num_days_from_sunday(), day_week_expr, 0, 6)
        {
            executions.push(current);
        }

        // Move to next minute
        current = current + Duration::minutes(1);

        // Safety limit to prevent infinite loops
        if executions.is_empty() && current > chrono::Utc::now() + Duration::days(365 * 4) {
            return Err(anyhow::anyhow!("Could not find next execution time within 4 years"));
        }
    }

    Ok(executions)
}

/// Check if a value matches a cron field expression
fn matches_cron_field(value: u32, expr: &str, min: u32, max: u32) -> bool {
    if expr == "*" {
        return true;
    }

    // Handle */n (every n units)
    if expr.starts_with("*/") {
        if let Ok(n) = expr[2..].parse::<u32>() {
            return value % n == 0;
        }
        return false;
    }

    // Handle list (comma-separated values)
    if expr.contains(',') {
        for part in expr.split(',') {
            if matches_cron_field(value, part.trim(), min, max) {
                return true;
            }
        }
        return false;
    }

    // Handle range (n-m)
    if expr.contains('-') {
        let range_parts: Vec<&str> = expr.split('-').collect();
        if range_parts.len() == 2 {
            if let (Ok(start), Ok(end)) = (
                range_parts[0].parse::<u32>(),
                range_parts[1].parse::<u32>()
            ) {
                return value >= start && value <= end;
            }
        }
        return false;
    }

    // Simple number
    if let Ok(n) = expr.parse::<u32>() {
        return value == n;
    }

    // Handle month names
    if min == 1 && max == 12 {
        let months = vec!["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
        if let Some(pos) = months.iter().position(|&m| m == expr.to_lowercase().as_str()) {
            return value == (pos as u32) + 1;
        }
    }

    // Handle day names
    if min == 0 && max == 7 {
        let days = vec!["sun", "mon", "tue", "wed", "thu", "fri", "sat"];
        if let Some(pos) = days.iter().position(|&d| d == expr.to_lowercase().as_str()) {
            return value == pos as u32;
        }
    }

    false
}

fn generate_scheduler_config(with_postgres: bool, with_redis: bool, production: bool) -> String {
    let mut config = String::new();

    config.push_str("# Metaphor Jobs Scheduler Configuration\n");
    config.push_str("scheduler:\n");
    config.push_str("  poll_interval: 30s\n");
    config.push_str("  max_concurrent_jobs: 20\n");
    config.push_str("  default_timeout: 1800s\n");
    config.push_str("  default_timezone: \"UTC\"\n");
    config.push_str("  cleanup_old_attempts: true\n");
    config.push_str("  cleanup_attempts_older_than_days: 30\n\n");

    if with_postgres {
        config.push_str("database:\n");
        if production {
            config.push_str("  url: \"${DATABASE_URL}\"\n");
        } else {
            config.push_str("  url: \"postgresql://root:password@localhost/metaphor_jobs\"\n");
        }
        config.push_str("  max_connections: 20\n");
        config.push_str("  min_connections: 5\n\n");
    }

    if with_redis {
        config.push_str("queue:\n");
        config.push_str("  type: \"redis\"\n");
        if production {
            config.push_str("  url: \"${REDIS_URL}\"\n");
        } else {
            config.push_str("  url: \"redis://localhost:6379\"\n");
        }
        config.push_str("\n");
    }

    if production {
        config.push_str("monitoring:\n");
        config.push_str("  enabled: true\n");
        config.push_str("  metrics_port: 9090\n");
        config.push_str("  health_check_interval: 30s\n");
        config.push_str("  log_level: \"info\"\n");
        config.push_str("\n");
    }

    config
}

fn generate_basic_example() -> String {
    r#"//! Basic job scheduler example
use metaphor_jobs::{JobScheduler, JobBuilder, JobSchedulerBuilder};
use metaphor_jobs::job_storage::InMemoryJobStorage;
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = Arc::new(InMemoryJobStorage::new());
    let scheduler = JobSchedulerBuilder::new()
        .with_storage(storage)
        .build()?;

    scheduler.start().await?;

    let job = JobBuilder::new()
        .id("my_job".to_string())
        .name("My Scheduled Job")
        .cron("*/5 * * * *") // Every 5 minutes
        .queue("default".to_string())
        .payload(json!({"message": "Hello from scheduled job!"}))
        .build()?;

    scheduler.schedule_job(job).await?;
    println!("✅ Job scheduled successfully!");

    tokio::signal::ctrl_c().await?;
    scheduler.stop().await?;
    Ok(())
}
"#.to_string()
}

fn generate_advanced_example() -> String {
    r#"//! Advanced job scheduler example
use metaphor_jobs::{JobScheduler, JobBuilder, JobSchedulerBuilder};
use metaphor_jobs::job_storage::InMemoryJobStorage;
use metaphor_jobs::job_types;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = Arc::new(InMemoryJobStorage::new());
    let scheduler = JobSchedulerBuilder::new()
        .with_storage(storage)
        .build()?;

    scheduler.start().await?;

    // Use predefined job templates
    let backup_job = job_types::daily_backup()?;
    let cleanup_job = job_types::weekly_log_cleanup()?;

    scheduler.schedule_job(backup_job).await?;
    scheduler.schedule_job(cleanup_job).await?;

    // Custom job with retry policy
    let custom_job = JobBuilder::new()
        .id("data_processing".to_string())
        .name("Data Processing Job")
        .cron("0 2 * * *") // Daily at 2 AM
        .queue("processing".to_string())
        .payload(json!({
            "type": "batch_process",
            "batch_size": 1000
        }))
        .timeout(3600)
        .build()?;

    scheduler.schedule_job(custom_job).await?;
    println!("✅ All jobs scheduled successfully!");

    tokio::signal::ctrl_c().await?;
    scheduler.stop().await?;
    Ok(())
}
"#.to_string()
}

fn generate_cleanup_example() -> String {
    r#"//! Database cleanup automation example
use metaphor_jobs::{JobScheduler, JobBuilder, JobSchedulerBuilder};
use metaphor_jobs::job_storage::InMemoryJobStorage;
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = Arc::new(InMemoryJobStorage::new());
    let scheduler = JobSchedulerBuilder::new()
        .with_storage(storage)
        .build()?;

    scheduler.start().await?;

    // Session cleanup every 6 hours
    let session_cleanup = JobBuilder::new()
        .id("session_cleanup".to_string())
        .name("User Session Cleanup")
        .cron("0 */6 * * *")
        .queue("maintenance".to_string())
        .payload(json!({
            "type": "session_cleanup",
            "older_than_hours": 24
        }))
        .build()?;

    // Log cleanup weekly
    let log_cleanup = JobBuilder::new()
        .id("log_cleanup".to_string())
        .name("Log Cleanup")
        .cron("0 2 * * 0") // Sunday at 2 AM
        .queue("maintenance".to_string())
        .payload(json!({
            "type": "log_cleanup",
            "older_than_days": 30
        }))
        .build()?;

    scheduler.schedule_job(session_cleanup).await?;
    scheduler.schedule_job(log_cleanup).await?;

    println!("✅ Cleanup jobs scheduled successfully!");

    tokio::signal::ctrl_c().await?;
    scheduler.stop().await?;
    Ok(())
}
"#.to_string()
}

fn generate_scheduler_file(project: &str) -> String {
    format!(r#"//! Job scheduler for {0}

use metaphor_jobs::{{JobScheduler, JobSchedulerBuilder}};
use metaphor_jobs::job_storage::{{InMemoryJobStorage, PostgreSQLJobStorage}};
use std::sync::Arc;
use std::time::Duration;

/// Initialize and return the job scheduler
pub async fn init_scheduler() -> Result<Arc<JobScheduler>, Box<dyn std::error::Error>> {{
    #![cfg(debug_assertions)]
    {{
        // Use in-memory storage for development
        let storage = Arc::new(InMemoryJobStorage::new());
        let scheduler = JobSchedulerBuilder::new()
            .with_storage(storage)
            .build()?;
        return Ok(Arc::new(scheduler));
    }}

    #[cfg(not(debug_assertions))]
    {{
        // Use PostgreSQL storage for production
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://root:password@localhost/{0}_jobs".to_string());

        let storage = Arc::new(PostgreSQLJobStorage::new(&database_url).await?);
        let scheduler = JobSchedulerBuilder::new()
            .with_storage(storage)
            .with_config(|config| {{
                config.poll_interval = Duration::from_secs(30);
                config.max_concurrent_jobs = 50;
                config.default_timeout = Duration::from_secs(1800);
                config.auto_start = true;
                config.cleanup_old_attempts = true;
                config.cleanup_attempts_older_than_days = 30;
            }})
            .build()?;

        Ok(Arc::new(scheduler))
    }}
}}

/// Example usage
pub async fn example_usage() -> Result<(), Box<dyn std::error::Error>> {{
    let scheduler = init_scheduler().await?;
    scheduler.start().await?;

    // Schedule your jobs here...

    tokio::signal::ctrl_c().await?;
    scheduler.stop().await?;
    Ok(())
}}
"#, project)
}
/// Generate job code from template
async fn generate_job_code(
    name: &str,
    cron: &str,
    description: Option<&str>,
    module: Option<&str>,
) -> anyhow::Result<()> {
    use std::path::Path;
    
    // Convert job name to different cases
    let pascal_case = to_pascal_case(name);
    let snake_case = to_snake_case(name);
    
    // Determine target directory
    let target_dir = if let Some(module_name) = module {
        // Module-specific jobs directory
        format!("libs/modules/{}/src/infrastructure/jobs", module_name)
    } else {
        // General jobs directory
        "src/jobs".to_string()
    };
    
    // Create directory if it doesn't exist
    std::fs::create_dir_all(&target_dir)?;
    
    // Read embedded template
    let template_content = include_str!("../templates/jobs/job.rs");

    // Prepare replacements
    let job_description = if let Some(desc) = description {
        desc.to_string()
    } else {
        format!("{} job", pascal_case)
    };
    let module_name = module.unwrap_or("jobs");

    // Replace placeholders in template
    let mut code = template_content.to_string();
    code = code.replace("{{PascalCaseJobName}}", &pascal_case);
    code = code.replace("{{snake_case_job_name}}", &snake_case);
    code = code.replace("{{cron_expression}}", cron);
    code = code.replace("{{job_description}}", &job_description);
    code = code.replace("{{module_name}}", module_name);
    
    // Write generated job file
    let output_path = Path::new(&target_dir).join(format!("{}.rs", snake_case));
    std::fs::write(&output_path, code)?;
    
    println!("\n📝 Generated job file: {}", output_path.display());
    
    // Update mod.rs if it exists
    let mod_path = Path::new(&target_dir).join("mod.rs");
    if mod_path.exists() {
        let mut mod_content = std::fs::read_to_string(&mod_path)?;
        let mod_line = format!("pub mod {};", snake_case);
        
        // Check if already included
        if !mod_content.contains(&mod_line) {
            mod_content.push_str("\n");
            mod_content.push_str(&mod_line);
            std::fs::write(&mod_path, mod_content)?;
            println!("📝 Updated mod.rs: {}", mod_path.display());
        }
    } else {
        // Create new mod.rs
        let mod_content = format!("//! Jobs module for {}\n\npub mod {};", module_name, snake_case);
        std::fs::write(&mod_path, mod_content)?;
        println!("📝 Created mod.rs: {}", mod_path.display());
    }
    
    println!("\n💡 Next steps:");
    println!("   1. Implement job logic in the execute() method");
    println!("   2. Register the job with your scheduler");
    println!("   3. Add the job module to your module tree");
    
    Ok(())
}

/// Convert string to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Convert string to snake_case
fn to_snake_case(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_uppercase() {
            format!("_{}", c.to_lowercase())
        } else {
            c.to_string()
        })
        .collect::<String>()
        .trim_start_matches('_')
        .to_string()
}
