//! Configuration validation and management commands
//!
//! Provides CLI tools to validate application configuration,
//! check .env files for common issues, and verify SMTP connectivity.

use anyhow::Result;
use colored::*;
use std::path::Path;

/// Severity level for configuration issues
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "{}", "ERROR".red().bold()),
            Severity::Warning => write!(f, "{}", "WARN".yellow().bold()),
            Severity::Info => write!(f, "{}", "INFO".blue()),
        }
    }
}

/// A single configuration issue found during validation
#[derive(Debug, Clone)]
pub struct ConfigIssue {
    pub severity: Severity,
    pub category: String,
    pub message: String,
    pub suggestion: Option<String>,
    pub line: Option<usize>,
}

impl std::fmt::Display for ConfigIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "  [{}] {}: {}", self.severity, self.category.cyan(), self.message)?;
        if let Some(line) = self.line {
            write!(f, " (line {})", line)?;
        }
        if let Some(ref suggestion) = self.suggestion {
            write!(f, "\n         {} {}", "Tip:".bright_cyan(), suggestion)?;
        }
        Ok(())
    }
}

/// Handle config subcommands
pub async fn handle_config_command(action: &ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Validate { strict, env } => handle_validate(*strict, env.as_deref()).await,
        ConfigAction::EmailVerify { send_test } => handle_email_verify(send_test.as_deref()).await,
    }
}

/// Config subcommand actions
#[derive(clap::Subcommand, Debug)]
pub enum ConfigAction {
    /// Validate application configuration
    ///
    /// Checks configuration files, environment variables, and .env files
    /// for common issues, missing values, and security concerns.
    ///
    /// Examples:
    ///   metaphor config validate
    ///   metaphor config validate --strict
    ///   metaphor config validate --env production
    Validate {
        /// Treat warnings as errors (exit with non-zero status)
        #[arg(long)]
        strict: bool,

        /// Target environment to validate for (default: from RUST_ENV or "development")
        #[arg(long)]
        env: Option<String>,
    },

    /// Verify email/SMTP configuration
    ///
    /// Tests the SMTP connection and optionally sends a test email.
    ///
    /// Examples:
    ///   metaphor config email-verify
    ///   metaphor config email-verify --send-test user@example.com
    EmailVerify {
        /// Send a test email to this address
        #[arg(long)]
        send_test: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// config validate
// ---------------------------------------------------------------------------

async fn handle_validate(strict: bool, env_override: Option<&str>) -> Result<()> {
    println!("{}", "⚙️  Configuration Validator".bright_green().bold());
    println!();

    let mut issues: Vec<ConfigIssue> = Vec::new();

    // 1. Validate .env file
    println!("{}", "Checking .env file...".dimmed());
    let env_path = Path::new(".env");
    if env_path.exists() {
        let env_issues = validate_env_file(env_path);
        issues.extend(env_issues);
    } else {
        issues.push(ConfigIssue {
            severity: Severity::Info,
            category: ".env".to_string(),
            message: "No .env file found in current directory".to_string(),
            suggestion: Some("Create a .env file or ensure environment variables are set externally".to_string()),
            line: None,
        });
    }

    // 2. Validate configuration files
    println!("{}", "Checking configuration files...".dimmed());
    let environment = env_override
        .map(|s| s.to_string())
        .unwrap_or_else(|| std::env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string()));
    let config_issues = validate_config_files(&environment);
    issues.extend(config_issues);

    // 3. Validate environment variables
    println!("{}", "Checking environment variables...".dimmed());
    let env_var_issues = validate_environment_variables(&environment);
    issues.extend(env_var_issues);

    // Print results
    println!();
    if issues.is_empty() {
        println!("{}", "✅ All configuration checks passed!".green().bold());
        return Ok(());
    }

    let errors = issues.iter().filter(|i| i.severity == Severity::Error).count();
    let warnings = issues.iter().filter(|i| i.severity == Severity::Warning).count();
    let infos = issues.iter().filter(|i| i.severity == Severity::Info).count();

    println!("{}", "Configuration Issues:".bold());
    println!();
    for issue in &issues {
        println!("{}", issue);
    }

    println!();
    println!(
        "Summary: {} error(s), {} warning(s), {} info(s)",
        if errors > 0 { errors.to_string().red().bold() } else { "0".normal() },
        if warnings > 0 { warnings.to_string().yellow().bold() } else { "0".normal() },
        infos.to_string().blue(),
    );

    if errors > 0 || (strict && warnings > 0) {
        println!();
        if strict && warnings > 0 && errors == 0 {
            println!("{}", "Failed: --strict mode treats warnings as errors".red());
        }
        std::process::exit(1);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// .env file validation
// ---------------------------------------------------------------------------

/// Validate a .env file for common issues
fn validate_env_file(path: &Path) -> Vec<ConfigIssue> {
    let mut issues = Vec::new();

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            issues.push(ConfigIssue {
                severity: Severity::Error,
                category: ".env".to_string(),
                message: format!("Cannot read .env file: {}", e),
                suggestion: None,
                line: None,
            });
            return issues;
        }
    };

    let critical_keys = ["DATABASE_URL", "JWT_SECRET"];
    let placeholder_patterns = ["changeme", "change-me", "your-secret", "your-super-secret", "TODO", "FIXME", "xxx"];

    for (line_num, line) in content.lines().enumerate() {
        let line_number = line_num + 1;
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Parse KEY=VALUE
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim();
            let value = trimmed[eq_pos + 1..].trim();

            // Check 1: Unescaped $ in value (shell expansion risk)
            if value.contains('$') {
                let has_unescaped_dollar = value.chars()
                    .enumerate()
                    .any(|(i, c)| c == '$' && (i == 0 || value.as_bytes()[i - 1] != b'\\'));

                if has_unescaped_dollar {
                    issues.push(ConfigIssue {
                        severity: Severity::Warning,
                        category: ".env".to_string(),
                        message: format!(
                            "`{}` contains unescaped `$` which may cause shell expansion",
                            key
                        ),
                        suggestion: Some(format!(
                            "Escape `$` with `\\$`. Example: {}={}",
                            key,
                            value.replace('$', "\\$")
                        )),
                        line: Some(line_number),
                    });
                }
            }

            // Check 2: Quoted values (dotenvy reads quotes literally)
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                // Only warn if the value has matching outer quotes
                if value.len() >= 2 {
                    issues.push(ConfigIssue {
                        severity: Severity::Warning,
                        category: ".env".to_string(),
                        message: format!(
                            "`{}` has quoted value — dotenvy includes quotes as literal characters",
                            key
                        ),
                        suggestion: Some(format!(
                            "Remove surrounding quotes: {}={}",
                            key,
                            &value[1..value.len() - 1]
                        )),
                        line: Some(line_number),
                    });
                }
            }

            // Check 3: Empty critical values
            if value.is_empty() && critical_keys.contains(&key) {
                issues.push(ConfigIssue {
                    severity: Severity::Error,
                    category: ".env".to_string(),
                    message: format!("`{}` is empty", key),
                    suggestion: Some(format!("Set a valid value for {}", key)),
                    line: Some(line_number),
                });
            }

            // Check 4: Placeholder values
            let value_lower = value.to_lowercase();
            for pattern in &placeholder_patterns {
                if value_lower.contains(pattern) {
                    issues.push(ConfigIssue {
                        severity: Severity::Warning,
                        category: ".env".to_string(),
                        message: format!(
                            "`{}` appears to contain a placeholder value: \"{}\"",
                            key, value
                        ),
                        suggestion: Some("Replace with the actual value before deploying".to_string()),
                        line: Some(line_number),
                    });
                    break;
                }
            }

            // Check 5: SMTP port = 1025 (MailHog default)
            if key == "SMTP_PORT" && value == "1025" {
                issues.push(ConfigIssue {
                    severity: Severity::Info,
                    category: ".env".to_string(),
                    message: "SMTP_PORT is 1025 (MailHog default)".to_string(),
                    suggestion: Some("Use port 465 (SSL) or 587 (STARTTLS) for production".to_string()),
                    line: Some(line_number),
                });
            }
        }
    }

    issues
}

// ---------------------------------------------------------------------------
// Configuration file validation
// ---------------------------------------------------------------------------

/// Validate YAML configuration files
fn validate_config_files(environment: &str) -> Vec<ConfigIssue> {
    let mut issues = Vec::new();
    let is_dev = environment == "development" || environment == "dev" || environment == "local";

    // Check base config exists
    let base_config = Path::new("config/application.yml");
    if !base_config.exists() {
        // Also check apps/metaphor/config/
        let alt_config = Path::new("apps/metaphor/config/application.yml");
        if !alt_config.exists() {
            issues.push(ConfigIssue {
                severity: Severity::Warning,
                category: "config".to_string(),
                message: "No application.yml found in config/ or apps/metaphor/config/".to_string(),
                suggestion: Some("Configuration will use code defaults only".to_string()),
                line: None,
            });
            return issues;
        }
    }

    // Try to parse the config file
    let config_path = if base_config.exists() {
        base_config
    } else {
        Path::new("apps/metaphor/config/application.yml")
    };

    let content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(e) => {
            issues.push(ConfigIssue {
                severity: Severity::Error,
                category: "config".to_string(),
                message: format!("Cannot read {}: {}", config_path.display(), e),
                suggestion: None,
                line: None,
            });
            return issues;
        }
    };

    // Check for common config issues in the raw YAML
    // (We parse the raw text rather than substituted values to detect template patterns)

    // Check database URL patterns
    if let Some(db_line) = content.lines().find(|l| l.contains("url:") && l.contains("postgresql")) {
        if !is_dev {
            if db_line.contains("root:password") || db_line.contains("postgres:postgres") {
                issues.push(ConfigIssue {
                    severity: Severity::Warning,
                    category: "config".to_string(),
                    message: "Database URL contains default credentials".to_string(),
                    suggestion: Some("Use strong credentials in production".to_string()),
                    line: None,
                });
            }
        }

        // Check if it uses env var substitution
        if !db_line.contains("${") {
            issues.push(ConfigIssue {
                severity: Severity::Info,
                category: "config".to_string(),
                message: "Database URL is hardcoded (not using ${DATABASE_URL} substitution)".to_string(),
                suggestion: Some("Use ${DATABASE_URL:default} for environment-specific overrides".to_string()),
                line: None,
            });
        }
    }

    // Check JWT secret
    if let Some(jwt_line) = content.lines().find(|l| l.contains("jwt_secret")) {
        if !is_dev {
            let line_lower = jwt_line.to_lowercase();
            if line_lower.contains("change") || line_lower.contains("your-") || line_lower.contains("secret-key") {
                issues.push(ConfigIssue {
                    severity: Severity::Warning,
                    category: "config".to_string(),
                    message: "JWT secret appears to be a placeholder".to_string(),
                    suggestion: Some("Set a strong, unique secret for production".to_string()),
                    line: None,
                });
            }
        }
    }

    // Check environment-specific config
    let env_config = format!("config/application-{}.yml", environment);
    if !Path::new(&env_config).exists() && !is_dev {
        let alt_env_config = format!("apps/metaphor/config/application-{}.yml", environment);
        if !Path::new(&alt_env_config).exists() {
            issues.push(ConfigIssue {
                severity: Severity::Info,
                category: "config".to_string(),
                message: format!("No environment-specific config found for '{}'", environment),
                suggestion: Some(format!("Create {} for environment-specific overrides", env_config)),
                line: None,
            });
        }
    }

    issues
}

// ---------------------------------------------------------------------------
// Environment variable validation
// ---------------------------------------------------------------------------

/// Validate critical environment variables
fn validate_environment_variables(environment: &str) -> Vec<ConfigIssue> {
    let mut issues = Vec::new();
    let is_dev = environment == "development" || environment == "dev" || environment == "local";

    // Check DATABASE_URL
    match std::env::var("DATABASE_URL") {
        Ok(url) => {
            if !is_dev {
                if url.contains("root:password") || url.contains("postgres:postgres") {
                    issues.push(ConfigIssue {
                        severity: Severity::Warning,
                        category: "env".to_string(),
                        message: "DATABASE_URL contains default credentials".to_string(),
                        suggestion: Some("Use strong credentials for non-development environments".to_string()),
                        line: None,
                    });
                }
                if url.contains("localhost") || url.contains("127.0.0.1") {
                    issues.push(ConfigIssue {
                        severity: Severity::Info,
                        category: "env".to_string(),
                        message: format!("DATABASE_URL points to localhost in '{}' environment", environment),
                        suggestion: None,
                        line: None,
                    });
                }
            }
        }
        Err(_) => {
            issues.push(ConfigIssue {
                severity: Severity::Info,
                category: "env".to_string(),
                message: "DATABASE_URL not set (will use config file value)".to_string(),
                suggestion: None,
                line: None,
            });
        }
    }

    // Check JWT_SECRET
    match std::env::var("JWT_SECRET") {
        Ok(secret) => {
            if !is_dev && secret.len() < 32 {
                issues.push(ConfigIssue {
                    severity: Severity::Warning,
                    category: "env".to_string(),
                    message: format!("JWT_SECRET is only {} characters (recommend >= 32)", secret.len()),
                    suggestion: Some("Use a longer, random secret for production".to_string()),
                    line: None,
                });
            }
        }
        Err(_) => {
            if !is_dev {
                issues.push(ConfigIssue {
                    severity: Severity::Warning,
                    category: "env".to_string(),
                    message: "JWT_SECRET not set in environment".to_string(),
                    suggestion: Some("Set JWT_SECRET for authentication to work correctly".to_string()),
                    line: None,
                });
            }
        }
    }

    // Check SMTP configuration
    let smtp_host = std::env::var("SMTP_HOST").unwrap_or_default();
    let smtp_port = std::env::var("SMTP_PORT").unwrap_or_default();

    if !smtp_host.is_empty() {
        if smtp_port == "1025" && !is_dev {
            issues.push(ConfigIssue {
                severity: Severity::Warning,
                category: "env".to_string(),
                message: "SMTP_PORT is 1025 (MailHog default) in non-development environment".to_string(),
                suggestion: Some("Use port 465 (SSL) or 587 (STARTTLS) for production SMTP".to_string()),
                line: None,
            });
        }

        if smtp_port == "465" || smtp_port == "587" {
            // Check credentials are set
            if std::env::var("SMTP_USER").unwrap_or_default().is_empty() {
                issues.push(ConfigIssue {
                    severity: Severity::Warning,
                    category: "env".to_string(),
                    message: "SMTP_USER not set but SMTP_HOST and production port are configured".to_string(),
                    suggestion: Some("Set SMTP_USER for authentication with the mail server".to_string()),
                    line: None,
                });
            }
            if std::env::var("SMTP_PASSWORD").unwrap_or_default().is_empty() {
                issues.push(ConfigIssue {
                    severity: Severity::Warning,
                    category: "env".to_string(),
                    message: "SMTP_PASSWORD not set but SMTP_HOST and production port are configured".to_string(),
                    suggestion: Some("Set SMTP_PASSWORD for authentication with the mail server".to_string()),
                    line: None,
                });
            }
        }
    }

    issues
}

// ---------------------------------------------------------------------------
// email-verify
// ---------------------------------------------------------------------------

async fn handle_email_verify(send_test: Option<&str>) -> Result<()> {
    println!("{}", "📧 Email Configuration Verifier".bright_green().bold());
    println!();

    // Load .env if present
    let _ = dotenvy::dotenv();

    // Read SMTP configuration from environment
    let smtp_host = std::env::var("SMTP_HOST").unwrap_or_default();
    let smtp_port: u16 = std::env::var("SMTP_PORT")
        .unwrap_or_else(|_| "587".to_string())
        .parse()
        .unwrap_or(587);
    let smtp_user = std::env::var("SMTP_USER").unwrap_or_default();
    let smtp_password = std::env::var("SMTP_PASSWORD").unwrap_or_default();
    let smtp_from = std::env::var("EMAIL_FROM")
        .or_else(|_| std::env::var("SMTP_FROM"))
        .unwrap_or_default();

    println!("SMTP Configuration:");
    println!("  Host:     {}", if smtp_host.is_empty() { "(not set)".red().to_string() } else { smtp_host.clone().green().to_string() });
    println!("  Port:     {}", smtp_port);
    println!("  Username: {}", if smtp_user.is_empty() { "(not set)".dimmed().to_string() } else { smtp_user.clone() });
    println!("  Password: {}", if smtp_password.is_empty() { "(not set)".red().to_string() } else { format!("({} chars)", smtp_password.len()).dimmed().to_string() });
    println!("  From:     {}", if smtp_from.is_empty() { "(not set)".dimmed().to_string() } else { smtp_from.clone() });
    println!();

    if smtp_host.is_empty() {
        println!("{}", "SMTP_HOST is not set. Cannot verify email configuration.".red());
        println!("{}", "Set SMTP_HOST, SMTP_PORT, SMTP_USER, SMTP_PASSWORD in .env or environment".dimmed());
        std::process::exit(1);
    }

    // Basic validation
    println!("{}", "Running basic validation...".dimmed());

    let use_ssl = smtp_port == 465;
    let use_tls = smtp_port == 587;

    if smtp_port == 1025 {
        println!("  {} Port 1025 is MailHog default (development only)", "⚠".yellow());
    } else if use_ssl {
        println!("  {} Port 465 → SSL/TLS (implicit TLS wrapper)", "✓".green());
    } else if use_tls {
        println!("  {} Port 587 → STARTTLS (opportunistic TLS)", "✓".green());
    } else if smtp_port == 25 {
        println!("  {} Port 25 → Plain (no encryption, development only)", "⚠".yellow());
    } else {
        println!("  {} Port {} is non-standard for SMTP", "⚠".yellow(), smtp_port);
    }

    if smtp_user.is_empty() && (use_ssl || use_tls) {
        println!("  {} SMTP_USER not set — authentication may fail", "⚠".yellow());
    }
    if smtp_password.is_empty() && (use_ssl || use_tls) {
        println!("  {} SMTP_PASSWORD not set — authentication may fail", "⚠".yellow());
    }
    if smtp_from.is_empty() {
        println!("  {} EMAIL_FROM not set — using SMTP_USER as sender", "⚠".yellow());
    }

    // Test SMTP connection using lettre
    println!();
    println!("{}", "Testing SMTP connection...".dimmed());

    match test_smtp_connection(&smtp_host, smtp_port, &smtp_user, &smtp_password, use_ssl, use_tls).await {
        Ok(()) => {
            println!("  {} SMTP connection successful!", "✓".green());
        }
        Err(e) => {
            println!("  {} SMTP connection failed: {}", "✗".red(), e);
            println!();
            println!("{}", "Common fixes:".yellow().bold());
            if e.to_string().contains("TLS") || e.to_string().contains("tls") || e.to_string().contains("SSL") {
                println!("  - Check if port {} matches the expected protocol (465=SSL, 587=STARTTLS)", smtp_port);
            }
            if e.to_string().contains("authentication") || e.to_string().contains("5.7.8") {
                println!("  - Verify SMTP_USER and SMTP_PASSWORD are correct");
                println!("  - Check for unescaped $ in password (use \\$ in .env)");
            }
            if e.to_string().contains("connect") || e.to_string().contains("timeout") {
                println!("  - Verify SMTP_HOST is reachable: openssl s_client -connect {}:{}", smtp_host, smtp_port);
            }
            std::process::exit(1);
        }
    }

    // Optionally send test email
    if let Some(recipient) = send_test {
        println!();
        println!("Sending test email to {}...", recipient.cyan());

        let from = if smtp_from.is_empty() { &smtp_user } else { &smtp_from };

        match send_test_email(&smtp_host, smtp_port, &smtp_user, &smtp_password, from, recipient, use_ssl, use_tls).await {
            Ok(()) => {
                println!("  {} Test email sent to {}", "✓".green(), recipient);
                println!("  Check the recipient's inbox (and spam folder)");
            }
            Err(e) => {
                println!("  {} Failed to send test email: {}", "✗".red(), e);
                std::process::exit(1);
            }
        }
    }

    println!();
    println!("{}", "✅ Email verification complete".green().bold());
    Ok(())
}

/// Test SMTP connection
async fn test_smtp_connection(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    use_ssl: bool,
    use_tls: bool,
) -> Result<()> {
    use lettre::transport::smtp::client::{Tls, TlsParameters};
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{SmtpTransport, Transport};

    let tls_params = TlsParameters::builder(host.to_string())
        .build()
        .map_err(|e| anyhow::anyhow!("TLS parameters build failed: {}", e))?;

    let transport = if use_ssl || port == 465 {
        let mut builder = SmtpTransport::builder_dangerous(host)
            .port(port)
            .tls(Tls::Wrapper(tls_params));

        if !username.is_empty() && !password.is_empty() {
            builder = builder.credentials(Credentials::new(username.to_string(), password.to_string()));
        }
        builder.build()
    } else if use_tls {
        let mut builder = SmtpTransport::relay(host)
            .map_err(|e| anyhow::anyhow!("SMTP relay build failed: {}", e))?
            .port(port)
            .tls(Tls::Opportunistic(tls_params));

        if !username.is_empty() && !password.is_empty() {
            builder = builder.credentials(Credentials::new(username.to_string(), password.to_string()));
        }
        builder.build()
    } else {
        let mut builder = SmtpTransport::builder_dangerous(host)
            .port(port);

        if !username.is_empty() && !password.is_empty() {
            builder = builder.credentials(Credentials::new(username.to_string(), password.to_string()));
        }
        builder.build()
    };

    transport.test_connection()
        .map_err(|e| anyhow::anyhow!("SMTP connection test failed: {}", e))?;

    Ok(())
}

/// Send a test email
async fn send_test_email(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    from: &str,
    to: &str,
    use_ssl: bool,
    use_tls: bool,
) -> Result<()> {
    use lettre::transport::smtp::client::{Tls, TlsParameters};
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{SmtpTransport, Transport, Message};
    use lettre::message::header::ContentType;

    let tls_params = TlsParameters::builder(host.to_string())
        .build()
        .map_err(|e| anyhow::anyhow!("TLS parameters build failed: {}", e))?;

    let transport = if use_ssl || port == 465 {
        let mut builder = SmtpTransport::builder_dangerous(host)
            .port(port)
            .tls(Tls::Wrapper(tls_params));
        if !username.is_empty() && !password.is_empty() {
            builder = builder.credentials(Credentials::new(username.to_string(), password.to_string()));
        }
        builder.build()
    } else if use_tls {
        let mut builder = SmtpTransport::relay(host)
            .map_err(|e| anyhow::anyhow!("SMTP relay failed: {}", e))?
            .port(port)
            .tls(Tls::Opportunistic(tls_params));
        if !username.is_empty() && !password.is_empty() {
            builder = builder.credentials(Credentials::new(username.to_string(), password.to_string()));
        }
        builder.build()
    } else {
        let mut builder = SmtpTransport::builder_dangerous(host)
            .port(port);
        if !username.is_empty() && !password.is_empty() {
            builder = builder.credentials(Credentials::new(username.to_string(), password.to_string()));
        }
        builder.build()
    };

    let email = Message::builder()
        .from(from.parse().map_err(|e| anyhow::anyhow!("Invalid FROM address '{}': {}", from, e))?)
        .to(to.parse().map_err(|e| anyhow::anyhow!("Invalid TO address '{}': {}", to, e))?)
        .subject("Metaphor Framework - Test Email")
        .header(ContentType::TEXT_PLAIN)
        .body("This is a test email from the Metaphor Framework CLI.\n\nIf you received this, your SMTP configuration is working correctly.".to_string())
        .map_err(|e| anyhow::anyhow!("Failed to build email: {}", e))?;

    transport.send(&email)
        .map_err(|e| anyhow::anyhow!("Failed to send email: {}", e))?;

    Ok(())
}
