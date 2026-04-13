# Job Templates Reference

Complete reference for all 8 built-in job templates available via `metaphor-dev jobs templates`.

## Overview

Job templates provide pre-configured starting points for common background tasks. Each template includes a cron expression, description, and (in detailed mode) a feature list. Use templates with `metaphor-dev jobs create --template <name>`.

## Templates

### `daily_backup`

| Field | Value |
|-------|-------|
| **Cron** | `0 2 * * *` |
| **Schedule** | Every day at 2:00 AM |
| **Description** | Automated database and file backup |

**Features:**
- Full database dump (PostgreSQL `pg_dump`)
- File system backup for uploaded media
- Compressed archive creation
- Retention policy enforcement (remove old backups)
- Backup verification checksums

**Usage:**

```bash
metaphor-dev jobs create DatabaseBackup --cron "0 2 * * *" --template daily_backup
```

---

### `weekly_log_cleanup`

| Field | Value |
|-------|-------|
| **Cron** | `0 3 * * 0` |
| **Schedule** | Every Sunday at 3:00 AM |
| **Description** | Clean up old log files and entries |

**Features:**
- Remove application logs older than configured retention period
- Clean up audit trail entries
- Compress and archive important logs
- Free disk space reporting

**Usage:**

```bash
metaphor-dev jobs create LogCleanup --cron "0 3 * * 0" --template weekly_log_cleanup
```

---

### `hourly_data_sync`

| Field | Value |
|-------|-------|
| **Cron** | `0 * * * *` |
| **Schedule** | Every hour at minute 0 |
| **Description** | Synchronize data between services |

**Features:**
- Cross-service data reconciliation
- Incremental sync (only changed records)
- Conflict detection and resolution
- Sync status reporting

**Usage:**

```bash
metaphor-dev jobs create DataSync --cron "0 * * * *" --template hourly_data_sync
```

---

### `monthly_report`

| Field | Value |
|-------|-------|
| **Cron** | `0 8 1 * *` |
| **Schedule** | 1st day of every month at 8:00 AM |
| **Description** | Generate monthly analytics report |

**Features:**
- User activity summary
- API usage statistics
- Error rate analysis
- Performance metrics aggregation
- Report delivery via email

**Usage:**

```bash
metaphor-dev jobs create MonthlyReport --cron "0 8 1 * *" --template monthly_report
```

---

### `session_cleanup`

| Field | Value |
|-------|-------|
| **Cron** | `*/30 * * * *` |
| **Schedule** | Every 30 minutes |
| **Description** | Remove expired user sessions |

**Features:**
- Scan for expired session tokens
- Bulk deletion of expired sessions
- Active session count reporting
- Memory/storage reclamation

**Usage:**

```bash
metaphor-dev jobs create SessionCleanup --cron "*/30 * * * *" --template session_cleanup
```

---

### `email_campaigns`

| Field | Value |
|-------|-------|
| **Cron** | `0 9 * * 1` |
| **Schedule** | Every Monday at 9:00 AM |
| **Description** | Process scheduled email campaigns |

**Features:**
- Batch email processing with rate limiting
- Template rendering with personalization
- Delivery tracking and bounce handling
- Campaign analytics collection

**Usage:**

```bash
metaphor-dev jobs create EmailCampaign --cron "0 9 * * 1" --template email_campaigns
```

---

### `database_maintenance`

| Field | Value |
|-------|-------|
| **Cron** | `0 4 * * 0` |
| **Schedule** | Every Sunday at 4:00 AM |
| **Description** | Run VACUUM, ANALYZE, and index maintenance |

**Features:**
- PostgreSQL `VACUUM ANALYZE` for table optimization
- Index rebuilding for fragmented indexes
- Dead tuple cleanup
- Table bloat detection and reporting
- Statistics refresh for query planner

**Usage:**

```bash
metaphor-dev jobs create DbMaintenance --cron "0 4 * * 0" --template database_maintenance
```

---

### `cache_warming`

| Field | Value |
|-------|-------|
| **Cron** | `*/15 * * * *` |
| **Schedule** | Every 15 minutes |
| **Description** | Pre-warm frequently accessed caches |

**Features:**
- Identify high-traffic cache keys
- Pre-load frequently accessed data
- Cache hit rate monitoring
- Configurable warming strategies

**Usage:**

```bash
metaphor-dev jobs create CacheWarmer --cron "*/15 * * * *" --template cache_warming
```

## Summary Table

| Template | Cron | Schedule | Best For |
|----------|------|----------|----------|
| `daily_backup` | `0 2 * * *` | Daily 2 AM | Data protection |
| `weekly_log_cleanup` | `0 3 * * 0` | Sunday 3 AM | Disk space management |
| `hourly_data_sync` | `0 * * * *` | Hourly | Multi-service consistency |
| `monthly_report` | `0 8 1 * *` | 1st of month 8 AM | Business analytics |
| `session_cleanup` | `*/30 * * * *` | Every 30 min | Security and performance |
| `email_campaigns` | `0 9 * * 1` | Monday 9 AM | Marketing automation |
| `database_maintenance` | `0 4 * * 0` | Sunday 4 AM | Database performance |
| `cache_warming` | `*/15 * * * *` | Every 15 min | Application performance |

## See Also

- [jobs create](../commands/jobs.md#jobs-create) — Creating jobs from templates
- [jobs templates](../commands/jobs.md#jobs-templates) — Listing templates
- [jobs validate-cron](../commands/jobs.md#jobs-validate-cron) — Validating cron expressions
