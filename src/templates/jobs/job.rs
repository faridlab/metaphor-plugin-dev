//! {{PascalCaseJobName}} Job
//!
//! {{job_description}}
//!
//! ## Schedule
//! Cron Expression: `{{cron_expression}}`
//!
//! ## Usage
//! This job is automatically registered with the job scheduler.
//!
//! ## Implementation
//! TODO: Add your job logic in the execute function

use anyhow::Result;
use chrono::Utc;
use serde_json::json;
use tracing::{info, error, warn};

use metaphor_jobs::JobBuilder;

/// {{PascalCaseJobName}} Job Configuration
pub struct {{PascalCaseJobName}}Job {
    /// Job identifier
    pub id: String,

    /// Job name
    pub name: String,

    /// Cron expression for scheduling
    pub cron_expression: String,

    /// Queue name for this job
    pub queue_name: String,

    /// Job timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for {{PascalCaseJobName}}Job {
    fn default() -> Self {
        Self {
            id: "{{snake_case_job_name}}".to_string(),
            name: "{{PascalCaseJobName}}".to_string(),
            cron_expression: "{{cron_expression}}".to_string(),
            queue_name: "default_queue".to_string(),
            timeout_seconds: 300, // 5 minutes
        }
    }
}

impl {{PascalCaseJobName}}Job {
    /// Create a new {{PascalCaseJobName}} job configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set custom cron expression
    pub fn with_cron(mut self, cron: &str) -> Self {
        self.cron_expression = cron.to_string();
        self
    }

    /// Set custom queue
    pub fn with_queue(mut self, queue: &str) -> Self {
        self.queue_name = queue.to_string();
        self
    }

    /// Set custom timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Build the job for registration with the scheduler
    pub fn build_job(&self) -> Result<metaphor_jobs::Job> {
        let payload = self.build_payload()?;

        JobBuilder::new()
            .id(self.id.clone())
            .name(self.name.clone())
            .description("{{job_description}}")
            .cron(&self.cron_expression)
            .queue(self.queue_name.clone())
            .payload(payload)
            .timeout(self.timeout_seconds)
            .max_retries(3)
            .retry_backoff(60) // 1 minute
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build job: {}", e))
    }

    /// Build the job payload
    fn build_payload(&self) -> Result<serde_json::Value> {
        // TODO: Add job-specific payload configuration
        // Example:
        // Ok(json!({
        //     "type": "{{snake_case_job_name}}",
        //     "mode": "standard",
        //     "batch_size": 100,
        //     "dry_run": false,
        // }))

        Ok(json!({
            "type": "{{snake_case_job_name}}",
            "created_at": Utc::now().to_rfc3339(),
        }))
    }

    /// Execute the job logic
    ///
    /// This is where you implement the actual job logic.
    /// TODO: Replace this placeholder implementation with your job logic.
    pub async fn execute(&self, payload: &serde_json::Value) -> Result<()> {
        info!("Starting {{PascalCaseJobName}} job");
        info!("Payload: {}", payload);

        // TODO: Implement your job logic here
        // Example scenarios:

        // 1. Database cleanup:
        // let db = get_db_connection().await?;
        // let deleted_rows = sqlx::query("DELETE FROM old_records WHERE created_at < $1")
        //     .bind(Utc::now() - Duration::days(30))
        //     .execute(&db)
        //     .await?;
        // info!("Deleted {} old records", deleted_rows);

        // 2. Data sync:
        // let api_client = ApiClient::new()?;
        // let data = api_client.fetch_data().await?;
        // let synced = sync_data_to_db(&data).await?;
        // info!("Synced {} records", synced);

        // 3. Report generation:
        // let report = generate_report().await?;
        // let path = save_report(&report).await?;
        // info!("Report saved to: {}", path);
        // send_notification(&path).await?;

        // 4. Cache warming:
        // let cache = get_cache_client().await?;
        // let keys = get_frequently_accessed_keys().await?;
        // for key in keys {
        //     cache.warm(&key).await?;
        // }
        // info!("Warmed {} cache entries", keys.len());

        // Placeholder implementation
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        info!("{{PascalCaseJobName}} job completed successfully");

        Ok(())
    }

    /// Validate job payload before execution
    pub fn validate_payload(&self, payload: &serde_json::Value) -> Result<()> {
        // TODO: Add payload validation
        // Example:
        // if let Some(mode) = payload.get("mode") {
        //     if mode != "standard" && mode != "aggressive" {
        //         return Err(anyhow::anyhow!("Invalid mode: {}", mode));
        //     }
        // }

        Ok(())
    }

    /// Handle job execution errors
    pub fn handle_error(&self, error: &anyhow::Error) {
        error!("{{PascalCaseJobName}} job failed: {}", error);

        // TODO: Add error handling logic
        // Example:
        // - Send alert to monitoring system
        // - Log to error tracking service
        // - Notify administrators
        // - Create incident ticket
    }
}

/// Helper function to register {{PascalCaseJobName}} job with scheduler
///
/// # Usage
/// ```ignore
/// use metaphor_jobs::JobScheduler;
/// use {{module_name}}::jobs::{{snake_case_job_name}}::{{PascalCaseJobName}}Job;
///
/// async fn register_jobs(scheduler: &JobScheduler) -> Result<()> {
///     let job = {{PascalCaseJobName}}Job::new()
///         .with_cron("{{cron_expression}}")
///         .with_queue("default_queue");
///
///     let job_config = job.build_job()?;
///     scheduler.schedule_job(job_config).await?;
///
///     Ok(())
/// }
/// ```
pub async fn register_job(scheduler: &metaphor_jobs::JobScheduler) -> Result<()> {
    let job = {{PascalCaseJobName}}Job::new();
    let job_config = job.build_job()?;

    scheduler.schedule_job(job_config).await?;
    tracing::info!("Registered {} job with scheduler", job.name);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_creation() {
        let job = {{PascalCaseJobName}}Job::new();
        assert_eq!(job.id, "{{snake_case_job_name}}");
        assert_eq!(job.name, "{{PascalCaseJobName}}");
    }

    #[test]
    fn test_job_build() {
        let job = {{PascalCaseJobName}}Job::new();
        let result = job.build_job();
        assert!(result.is_ok(), "Job build should succeed");
    }

    #[test]
    fn test_custom_cron() {
        let job = {{PascalCaseJobName}}Job::new()
            .with_cron("0 */6 * * *");
        assert_eq!(job.cron_expression, "0 */6 * * *");
    }

    #[test]
    fn test_custom_queue() {
        let job = {{PascalCaseJobName}}Job::new()
            .with_queue("custom_queue");
        assert_eq!(job.queue_name, "custom_queue");
    }

    #[test]
    fn test_custom_timeout() {
        let job = {{PascalCaseJobName}}Job::new()
            .with_timeout(600);
        assert_eq!(job.timeout_seconds, 600);
    }
}
