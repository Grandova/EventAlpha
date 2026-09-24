use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Row, Sqlite};
use std::path::Path;
use std::str::FromStr;
use tracing::info;

use crate::config::AppConfig;
use crate::types::{BankrollState, SystemEvent};

pub type DbPool = Pool<Sqlite>;

pub struct Database {
    pool: DbPool,
}

impl Database {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let db_url = &config.database.url;

        // If file-based SQLite, ensure parent directory exists
        if db_url.starts_with("sqlite://") && !db_url.contains(":memory:") {
            let path_part = db_url.trim_start_matches("sqlite://");
            let clean_path = path_part.split('?').next().unwrap_or(path_part);
            if let Some(parent) = Path::new(clean_path).parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).with_context(|| {
                        format!("Failed to create database directory at {:?}", parent)
                    })?;
                }
            }
        }

        let connect_options = SqliteConnectOptions::from_str(db_url)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_millis(5000))
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(config.database.max_connections)
            .min_connections(config.database.min_connections)
            .acquire_timeout(std::time::Duration::from_secs(
                config.database.acquire_timeout_secs,
            ))
            .connect_with(connect_options)
            .await
            .with_context(|| format!("Failed to connect to database at {}", db_url))?;

        let db = Database { pool };
        db.run_migrations().await?;

        info!("Database initialized and migrations applied successfully.");
        Ok(db)
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// Automatically run embedded initial schema migration
    pub async fn run_migrations(&self) -> Result<()> {
        let migration_sql = include_str!("../../migrations/0001_initial_schema.sql");
        sqlx::raw_sql(migration_sql)
            .execute(&self.pool)
            .await
            .context("Failed to execute initial schema migrations")?;
        Ok(())
    }

    /// Record a system event / audit entry
    pub async fn record_system_event(
        &self,
        event_type: &str,
        severity: &str,
        component: &str,
        message: &str,
        payload_json: Option<&str>,
    ) -> Result<()> {
        let now_ms = Utc::now().timestamp_millis();
        sqlx::query(
            r#"
            INSERT INTO system_events (event_type, severity, component, message, payload_json, timestamp)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(event_type)
        .bind(severity)
        .bind(component)
        .bind(message)
        .bind(payload_json)
        .bind(now_ms)
        .execute(&self.pool)
        .await
        .context("Failed to insert system event")?;
        Ok(())
    }

    /// Get current bankroll state or initialize it if fresh database
    pub async fn get_or_init_bankroll(&self, config: &AppConfig) -> Result<BankrollState> {
        let row_opt = sqlx::query(
            r#"
            SELECT active_bankroll, locked_profit, total_equity
            FROM bankroll_history
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query latest bankroll history")?;

        match row_opt {
            Some(row) => {
                let active: f64 = row.get("active_bankroll");
                let locked: f64 = row.get("locked_profit");
                let total: f64 = row.get("total_equity");

                let is_halted = active <= config.bankroll.minimum;
                let halt_reason = if is_halted {
                    Some(format!(
                        "Active bankroll ({:.2}) <= minimum threshold ({:.2})",
                        active, config.bankroll.minimum
                    ))
                } else {
                    None
                };

                Ok(BankrollState {
                    initial_bankroll: config.bankroll.initial,
                    active_bankroll: active,
                    bankroll_cap: config.bankroll.cap,
                    locked_profit: locked,
                    total_equity: total,
                    minimum_bankroll: config.bankroll.minimum,
                    mode: config.bankroll.mode,
                    daily_loss_current: 0.0,
                    consecutive_losses: 0,
                    peak_equity: total,
                    current_drawdown: 0.0,
                    is_trading_halted: is_halted,
                    halt_reason,
                })
            }
            None => {
                // Initialize default bankroll entry
                let initial = config.bankroll.initial;
                let now_ms = Utc::now().timestamp_millis();

                sqlx::query(
                    r#"
                    INSERT INTO bankroll_history (timestamp, active_bankroll, locked_profit, total_equity, change_amount, reason, trade_id, created_at)
                    VALUES (?, ?, 0.0, ?, 0.0, 'SYSTEM_INIT', NULL, ?)
                    "#,
                )
                .bind(now_ms)
                .bind(initial)
                .bind(initial)
                .bind(now_ms)
                .execute(&self.pool)
                .await
                .context("Failed to insert initial bankroll record")?;

                info!(
                    "Initialized new Bankroll: active={:.2} USDC, cap={:.2} USDC, mode={:?}",
                    initial, config.bankroll.cap, config.bankroll.mode
                );

                Ok(BankrollState {
                    initial_bankroll: initial,
                    active_bankroll: initial,
                    bankroll_cap: config.bankroll.cap,
                    locked_profit: 0.0,
                    total_equity: initial,
                    minimum_bankroll: config.bankroll.minimum,
                    mode: config.bankroll.mode,
                    daily_loss_current: 0.0,
                    consecutive_losses: 0,
                    peak_equity: initial,
                    current_drawdown: 0.0,
                    is_trading_halted: false,
                    halt_reason: None,
                })
            }
        }
    }

    /// Fetch recent system events
    pub async fn get_recent_events(&self, limit: i64) -> Result<Vec<SystemEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT event_type, severity, component, message, payload_json, timestamp
            FROM system_events
            ORDER BY id DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch system events")?;

        let events = rows
            .into_iter()
            .map(|r| SystemEvent {
                event_type: r.get("event_type"),
                severity: r.get("severity"),
                component: r.get("component"),
                message: r.get("message"),
                payload_json: r.get("payload_json"),
                timestamp_ms: r.get("timestamp"),
            })
            .collect();

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn test_config() -> AppConfig {
        let yaml = r#"
mode: "paper"
safety:
  real_trading_enabled: false
  safety_signature: "PAPER_TRADING_ONLY_SAFETY_LOCK_ENGAGED"
server:
  host: "127.0.0.1"
  port: 8080
  cors_allowed_origins: []
database:
  url: "sqlite://:memory:"
  max_connections: 2
  min_connections: 1
  acquire_timeout_secs: 5
bankroll:
  initial: 10.0
  cap: 10.0
  minimum: 2.0
  mode: "capital_recovery"
position:
  mode: "fixed"
  stake: 1.0
  stake_percent: 0.10
  max_stake: 1.0
strategy:
  strategy_version: "v1.0.0"
  min_probability: 0.70
  min_net_edge: 0.05
  max_entry_price: 0.85
  max_spread: 0.04
  min_liquidity: 300.0
  min_time_remaining_sec: 15
  max_time_remaining_sec: 285
  score_thresholds:
    skip_below: 60.0
    low: 60.0
    medium: 70.0
    high: 80.0
    very_high: 90.0
risk:
  daily_loss_limit: 2.0
  max_drawdown: 0.20
  max_consecutive_losses: 5
  cooldown_minutes: 30
execution:
  latency_ms: 250
  orderbook_depth_fill: true
  fee_rate: 0.012
  slippage_rate: 0.005
freshness:
  stale_timeout_ms: 2000
  max_clock_skew_ms: 1000
assets: ["BTC", "ETH", "SOL"]
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();
        AppConfig::load_from_path(file.path()).unwrap()
    }

    #[tokio::test]
    async fn test_database_initialization_and_migration() {
        let config = test_config();
        let db = Database::new(&config).await.expect("Database should initialize");

        // Verify tables exist by inserting and retrieving an event
        db.record_system_event("TEST_EVENT", "INFO", "TEST", "Testing database", None)
            .await
            .expect("Should record event");

        let events = db.get_recent_events(10).await.expect("Should fetch events");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "TEST_EVENT");
    }

    #[tokio::test]
    async fn test_bankroll_initialization() {
        let config = test_config();
        let db = Database::new(&config).await.expect("Database should initialize");

        let bankroll = db.get_or_init_bankroll(&config).await.expect("Bankroll should init");
        assert_eq!(bankroll.active_bankroll, 10.0);
        assert_eq!(bankroll.locked_profit, 0.0);
        assert_eq!(bankroll.total_equity, 10.0);
        assert!(!bankroll.is_trading_halted);
    }
}
