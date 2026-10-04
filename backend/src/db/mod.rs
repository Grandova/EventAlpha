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
            .foreign_keys(true)
            .pragma("cache_size", "-16000")
            .pragma("temp_store", "memory")
            .pragma("mmap_size", "268435456");

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

    /// Automatically run embedded initial and evolution schema migrations
    pub async fn run_migrations(&self) -> Result<()> {
        let migration_sql_1 = include_str!("../../migrations/0001_initial_schema.sql");
        sqlx::raw_sql(migration_sql_1)
            .execute(&self.pool)
            .await
            .context("Failed to execute initial schema migrations (0001)")?;

        let migration_sql_2 = include_str!("../../migrations/0002_real_trading_and_self_learning.sql");
        sqlx::raw_sql(migration_sql_2)
            .execute(&self.pool)
            .await
            .context("Failed to execute real trading & self learning migrations (0002)")?;

        let migration_sql_3 = include_str!("../../migrations/0003_live_bankroll.sql");
        sqlx::raw_sql(migration_sql_3)
            .execute(&self.pool)
            .await
            .context("Failed to execute live bankroll migrations (0003)")?;

        // 4. Ensure up_token_id and down_token_id columns exist in markets table
        let has_tokens = sqlx::query("SELECT up_token_id FROM markets LIMIT 1")
            .fetch_one(&self.pool)
            .await
            .is_ok();
        if !has_tokens {
            let _ = sqlx::query("ALTER TABLE markets ADD COLUMN up_token_id TEXT")
                .execute(&self.pool)
                .await;
            let _ = sqlx::query("ALTER TABLE markets ADD COLUMN down_token_id TEXT")
                .execute(&self.pool)
                .await;
        }

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

    /// Get current LIVE bankroll state or initialize it from active Polymarket account
    pub async fn get_or_init_live_bankroll(&self) -> Result<BankrollState> {
        let row_opt = sqlx::query(
            r#"
            SELECT active_bankroll, locked_profit, total_equity
            FROM live_bankroll_history
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query latest live bankroll history")?;

        let active_account = self.get_active_account().await.ok().flatten();

        match row_opt {
            Some(row) => {
                let mut active: f64 = row.get("active_bankroll");
                let locked: f64 = row.get("locked_profit");
                if let Some(ref acc) = active_account {
                    if acc.balance_usdc > 0.0 || active <= 0.0 {
                        active = acc.balance_usdc;
                    }
                }
                let total = active + locked;
                let min_floor = 0.00;
                let is_halted = active <= min_floor && active_account.is_some() && active_account.as_ref().unwrap().balance_usdc <= 0.0;
                let halt_reason = if is_halted {
                    Some("实盘可用资金已耗尽 ($0.00 USDC)".to_string())
                } else {
                    None
                };

                Ok(BankrollState {
                    initial_bankroll: active,
                    active_bankroll: active,
                    bankroll_cap: active.max(10.0),
                    locked_profit: locked,
                    total_equity: total,
                    minimum_bankroll: min_floor,
                    mode: crate::types::BankrollMode::CapitalRecovery,
                    daily_loss_current: 0.0,
                    consecutive_losses: 0,
                    peak_equity: total,
                    current_drawdown: 0.0,
                    is_trading_halted: is_halted,
                    halt_reason,
                })
            }
            None => {
                let initial = active_account.as_ref().map(|a| a.balance_usdc).unwrap_or(0.0);
                let now_ms = Utc::now().timestamp_millis();
                sqlx::query(
                    r#"
                    INSERT INTO live_bankroll_history (timestamp, active_bankroll, locked_profit, total_equity, change_amount, reason, trade_id, created_at)
                    VALUES (?, ?, 0.0, ?, 0.0, 'LIVE_INIT', NULL, ?)
                    "#,
                )
                .bind(now_ms)
                .bind(initial)
                .bind(initial)
                .bind(now_ms)
                .execute(&self.pool)
                .await
                .context("Failed to insert initial live bankroll record")?;

                info!(
                    "Initialized new Live Bankroll: active={:.2} USDC (Polymarket Account: {})",
                    initial,
                    active_account.as_ref().map(|a| a.label.as_str()).unwrap_or("None")
                );

                Ok(BankrollState {
                    initial_bankroll: initial,
                    active_bankroll: initial,
                    bankroll_cap: initial.max(10.0),
                    locked_profit: 0.0,
                    total_equity: initial,
                    minimum_bankroll: 0.50,
                    mode: crate::types::BankrollMode::CapitalRecovery,
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

    /// Insert a prediction signal record
    pub async fn insert_prediction(&self, signal: &crate::types::PredictionSignal) -> Result<()> {
        let now_ms = Utc::now().timestamp_millis();
        let asset_str = signal.asset.to_string();
        let action_str = match signal.action {
            crate::types::SignalAction::BuyUp => "BUY_UP",
            crate::types::SignalAction::BuyDown => "BUY_DOWN",
            crate::types::SignalAction::Skip => "SKIP",
        };
        let conf_str = match signal.confidence {
            crate::types::ConfidenceLevel::Skip => "SKIP",
            crate::types::ConfidenceLevel::Low => "LOW",
            crate::types::ConfidenceLevel::Medium => "MEDIUM",
            crate::types::ConfidenceLevel::High => "HIGH",
            crate::types::ConfidenceLevel::VeryHigh => "VERY_HIGH",
        };

        // Ensure parent market exists in case of synthetic/test runs
        let condition_id = format!("cond_{}", signal.market_id);
        let _ = sqlx::query(
            r#"
            INSERT OR IGNORE INTO markets (id, condition_id, asset, start_time, end_time, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, 'active', ?, ?)
            "#,
        )
        .bind(&signal.market_id)
        .bind(&condition_id)
        .bind(&asset_str)
        .bind(signal.timestamp_ms)
        .bind(signal.timestamp_ms + 300_000)
        .bind(now_ms)
        .bind(now_ms)
        .execute(&self.pool)
        .await;

        sqlx::query(
            r#"
            INSERT INTO predictions (
                prediction_id, market_id, asset, timestamp, model_version,
                p_up, p_down, fair_value_up, fair_value_down, market_implied_up,
                gross_edge, estimated_fee, estimated_slippage, net_edge,
                confidence, signal_score, recommended_action, decision_reason, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&signal.prediction_id)
        .bind(&signal.market_id)
        .bind(&asset_str)
        .bind(signal.timestamp_ms)
        .bind(&signal.model_version)
        .bind(signal.p_up)
        .bind(signal.p_down)
        .bind(signal.fair_value_up)
        .bind(signal.fair_value_down)
        .bind(signal.market_implied_up)
        .bind(signal.gross_edge)
        .bind(signal.estimated_fee)
        .bind(signal.estimated_slippage)
        .bind(signal.net_edge)
        .bind(conf_str)
        .bind(signal.signal_score)
        .bind(action_str)
        .bind(&signal.decision_reason)
        .bind(now_ms)
        .execute(&self.pool)
        .await
        .context("Failed to insert prediction signal")?;

        Ok(())
    }

    /// Retrieve recent prediction signals
    pub async fn get_recent_predictions(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::types::PredictionSignal>> {
        let rows = sqlx::query(
            r#"
            SELECT prediction_id, market_id, asset, timestamp, model_version,
                   p_up, p_down, fair_value_up, fair_value_down, market_implied_up,
                   gross_edge, estimated_fee, estimated_slippage, net_edge,
                   confidence, signal_score, recommended_action, decision_reason
            FROM predictions
            ORDER BY id DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query recent predictions")?;

        let mut signals = Vec::new();
        for r in rows {
            let asset_str: String = r.get("asset");
            let asset: crate::types::Asset = asset_str.parse().unwrap_or(crate::types::Asset::BTC);

            let action_str: String = r.get("recommended_action");
            let action = match action_str.as_str() {
                "BUY_UP" => crate::types::SignalAction::BuyUp,
                "BUY_DOWN" => crate::types::SignalAction::BuyDown,
                _ => crate::types::SignalAction::Skip,
            };

            let conf_str: String = r.get("confidence");
            let confidence = match conf_str.as_str() {
                "LOW" => crate::types::ConfidenceLevel::Low,
                "MEDIUM" => crate::types::ConfidenceLevel::Medium,
                "HIGH" => crate::types::ConfidenceLevel::High,
                "VERY_HIGH" => crate::types::ConfidenceLevel::VeryHigh,
                _ => crate::types::ConfidenceLevel::Skip,
            };

            signals.push(crate::types::PredictionSignal {
                prediction_id: r.get("prediction_id"),
                market_id: r.get("market_id"),
                asset,
                timestamp_ms: r.get("timestamp"),
                model_version: r.get("model_version"),
                p_up: r.get("p_up"),
                p_down: r.get("p_down"),
                fair_value_up: r.get("fair_value_up"),
                fair_value_down: r.get("fair_value_down"),
                market_implied_up: r.get("market_implied_up"),
                gross_edge: r.get("gross_edge"),
                estimated_fee: r.get("estimated_fee"),
                estimated_slippage: r.get("estimated_slippage"),
                net_edge: r.get("net_edge"),
                signal_score: r.get("signal_score"),
                confidence,
                action,
                decision_reason: r.get("decision_reason"),
                gate_results: Vec::new(),
                score_breakdown: None,
            });
        }

        Ok(signals)
    }

    /// Insert a paper order record
    pub async fn insert_paper_order(&self, order: &crate::types::PaperOrder) -> Result<()> {
        let now_ms = Utc::now().timestamp_millis();
        let asset_str = order.asset.to_string();
        let side_str = order.side.to_string();

        let condition_id = format!("cond_{}", order.market_id);
        let _ = sqlx::query(
            r#"
            INSERT OR IGNORE INTO markets (id, condition_id, asset, start_time, end_time, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, 'active', ?, ?)
            "#,
        )
        .bind(&order.market_id)
        .bind(&condition_id)
        .bind(&asset_str)
        .bind(order.timestamp_ms)
        .bind(order.timestamp_ms + 300_000)
        .bind(now_ms)
        .bind(now_ms)
        .execute(&self.pool)
        .await;

        sqlx::query(
            r#"
            INSERT INTO paper_orders (
                order_id, market_id, asset, side, stake, shares,
                quote_price, fill_price, slippage, fee, status, signal_id, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&order.order_id)
        .bind(&order.market_id)
        .bind(&asset_str)
        .bind(&side_str)
        .bind(order.stake)
        .bind(order.shares)
        .bind(order.quote_price)
        .bind(order.fill_price)
        .bind(order.slippage)
        .bind(order.fee)
        .bind(&order.status)
        .bind(&order.signal_id)
        .bind(order.timestamp_ms)
        .execute(&self.pool)
        .await
        .context("Failed to insert paper order")?;

        Ok(())
    }

    /// Retrieve recent paper orders
    pub async fn get_recent_paper_orders(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::types::PaperOrder>> {
        let rows = sqlx::query(
            r#"
            SELECT order_id, market_id, asset, side, stake, shares,
                   quote_price, fill_price, slippage, fee, status, signal_id, created_at
            FROM paper_orders
            ORDER BY id DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query paper orders")?;

        let mut orders = Vec::new();
        for r in rows {
            let asset_str: String = r.get("asset");
            let asset: crate::types::Asset = asset_str.parse().unwrap_or(crate::types::Asset::BTC);
            let side_str: String = r.get("side");
            let side = if side_str.to_uppercase() == "UP" {
                crate::types::MarketSide::Up
            } else {
                crate::types::MarketSide::Down
            };

            orders.push(crate::types::PaperOrder {
                order_id: r.get("order_id"),
                market_id: r.get("market_id"),
                asset,
                side,
                stake: r.get("stake"),
                shares: r.get("shares"),
                quote_price: r.get("quote_price"),
                fill_price: r.get("fill_price"),
                slippage: r.get("slippage"),
                fee: r.get("fee"),
                status: r.get("status"),
                signal_id: r.get("signal_id"),
                timestamp_ms: r.get("created_at"),
            });
        }

        Ok(orders)
    }

    /// Insert a paper position
    pub async fn insert_paper_position(&self, pos: &crate::types::PaperPosition) -> Result<()> {
        let now_ms = Utc::now().timestamp_millis();
        let asset_str = pos.asset.to_string();
        let side_str = pos.side.to_string();

        let condition_id = format!("cond_{}", pos.market_id);
        let _ = sqlx::query(
            r#"
            INSERT OR IGNORE INTO markets (id, condition_id, asset, start_time, end_time, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, 'active', ?, ?)
            "#,
        )
        .bind(&pos.market_id)
        .bind(&condition_id)
        .bind(&asset_str)
        .bind(pos.entry_time_ms)
        .bind(pos.entry_time_ms + 300_000)
        .bind(now_ms)
        .bind(now_ms)
        .execute(&self.pool)
        .await;

        sqlx::query(
            r#"
            INSERT INTO paper_positions (
                position_id, market_id, asset, side, entry_time,
                entry_price, stake, shares, status, settled_at, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&pos.position_id)
        .bind(&pos.market_id)
        .bind(&asset_str)
        .bind(&side_str)
        .bind(pos.entry_time_ms)
        .bind(pos.entry_price)
        .bind(pos.stake)
        .bind(pos.shares)
        .bind(&pos.status)
        .bind(pos.settled_at_ms)
        .bind(pos.created_at_ms)
        .execute(&self.pool)
        .await
        .context("Failed to insert paper position")?;

        Ok(())
    }

    /// Retrieve active (OPEN) paper positions
    pub async fn get_active_positions(&self) -> Result<Vec<crate::types::PaperPosition>> {
        let rows = sqlx::query(
            r#"
            SELECT position_id, market_id, asset, side, entry_time,
                   entry_price, stake, shares, status, settled_at, created_at
            FROM paper_positions
            WHERE status = 'OPEN'
            ORDER BY id DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to query active paper positions")?;

        let mut positions = Vec::new();
        for r in rows {
            let asset_str: String = r.get("asset");
            let asset: crate::types::Asset = asset_str.parse().unwrap_or(crate::types::Asset::BTC);
            let side_str: String = r.get("side");
            let side = if side_str.to_uppercase() == "UP" {
                crate::types::MarketSide::Up
            } else {
                crate::types::MarketSide::Down
            };

            positions.push(crate::types::PaperPosition {
                position_id: r.get("position_id"),
                order_id: "".to_string(),
                market_id: r.get("market_id"),
                asset,
                side,
                entry_time_ms: r.get("entry_time"),
                entry_price: r.get("entry_price"),
                stake: r.get("stake"),
                shares: r.get("shares"),
                status: r.get("status"),
                settled_at_ms: r.get("settled_at"),
                created_at_ms: r.get("created_at"),
            });
        }

        Ok(positions)
    }

    /// Retrieve a single position by position_id
    pub async fn get_position(&self, position_id: &str) -> Result<Option<crate::types::PaperPosition>> {
        let row_opt = sqlx::query(
            r#"
            SELECT position_id, market_id, asset, side, entry_time,
                   entry_price, stake, shares, status, settled_at, created_at
            FROM paper_positions
            WHERE position_id = ?
            "#,
        )
        .bind(position_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query position by id")?;

        match row_opt {
            Some(r) => {
                let asset_str: String = r.get("asset");
                let asset: crate::types::Asset = asset_str.parse().unwrap_or(crate::types::Asset::BTC);
                let side_str: String = r.get("side");
                let side = if side_str.to_uppercase() == "UP" {
                    crate::types::MarketSide::Up
                } else {
                    crate::types::MarketSide::Down
                };

                Ok(Some(crate::types::PaperPosition {
                    position_id: r.get("position_id"),
                    order_id: "".to_string(),
                    market_id: r.get("market_id"),
                    asset,
                    side,
                    entry_time_ms: r.get("entry_time"),
                    entry_price: r.get("entry_price"),
                    stake: r.get("stake"),
                    shares: r.get("shares"),
                    status: r.get("status"),
                    settled_at_ms: r.get("settled_at"),
                    created_at_ms: r.get("created_at"),
                }))
            }
            None => Ok(None),
        }
    }

    /// Retrieve all positions history
    pub async fn get_positions_history(&self, limit: i64) -> Result<Vec<crate::types::PaperPosition>> {
        let rows = sqlx::query(
            r#"
            SELECT position_id, market_id, asset, side, entry_time,
                   entry_price, stake, shares, status, settled_at, created_at
            FROM paper_positions
            ORDER BY id DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query position history")?;

        let mut positions = Vec::new();
        for r in rows {
            let asset_str: String = r.get("asset");
            let asset: crate::types::Asset = asset_str.parse().unwrap_or(crate::types::Asset::BTC);
            let side_str: String = r.get("side");
            let side = if side_str.to_uppercase() == "UP" {
                crate::types::MarketSide::Up
            } else {
                crate::types::MarketSide::Down
            };

            positions.push(crate::types::PaperPosition {
                position_id: r.get("position_id"),
                order_id: "".to_string(),
                market_id: r.get("market_id"),
                asset,
                side,
                entry_time_ms: r.get("entry_time"),
                entry_price: r.get("entry_price"),
                stake: r.get("stake"),
                shares: r.get("shares"),
                status: r.get("status"),
                settled_at_ms: r.get("settled_at"),
                created_at_ms: r.get("created_at"),
            });
        }

        Ok(positions)
    }

    /// Update position status upon settlement or closure
    pub async fn update_position_status(
        &self,
        position_id: &str,
        status: &str,
        settled_at_ms: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE paper_positions
            SET status = ?, settled_at = ?
            WHERE position_id = ?
            "#,
        )
        .bind(status)
        .bind(settled_at_ms)
        .bind(position_id)
        .execute(&self.pool)
        .await
        .context("Failed to update paper position status")?;

        Ok(())
    }

    /// Record a bankroll modification transaction
    pub async fn record_bankroll_entry(
        &self,
        active: f64,
        locked: f64,
        total: f64,
        change: f64,
        reason: &str,
        trade_id: Option<&str>,
    ) -> Result<()> {
        let now_ms = Utc::now().timestamp_millis();
        sqlx::query(
            r#"
            INSERT INTO bankroll_history (timestamp, active_bankroll, locked_profit, total_equity, change_amount, reason, trade_id, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(now_ms)
        .bind(active)
        .bind(locked)
        .bind(total)
        .bind(change)
        .bind(reason)
        .bind(trade_id)
        .bind(now_ms)
        .execute(&self.pool)
        .await
        .context("Failed to record bankroll history entry")?;

        Ok(())
    }

    /// Retrieve bankroll transaction history
    pub async fn get_bankroll_history(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::types::BankrollHistoryEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT id, timestamp, active_bankroll, locked_profit, total_equity, change_amount, reason, trade_id
            FROM bankroll_history
            ORDER BY id DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query bankroll history")?;

        let list = rows
            .into_iter()
            .map(|r| crate::types::BankrollHistoryEntry {
                id: r.get("id"),
                timestamp_ms: r.get("timestamp"),
                active_bankroll: r.get("active_bankroll"),
                locked_profit: r.get("locked_profit"),
                total_equity: r.get("total_equity"),
                change_amount: r.get("change_amount"),
                reason: r.get("reason"),
                trade_id: r.get("trade_id"),
            })
            .collect();

        Ok(list)
    }

    /// Record a LIVE bankroll modification transaction
    pub async fn record_live_bankroll_entry(
        &self,
        active: f64,
        locked: f64,
        total: f64,
        change: f64,
        reason: &str,
        trade_id: Option<&str>,
    ) -> Result<()> {
        let now_ms = Utc::now().timestamp_millis();
        sqlx::query(
            r#"
            INSERT INTO live_bankroll_history (timestamp, active_bankroll, locked_profit, total_equity, change_amount, reason, trade_id, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(now_ms)
        .bind(active)
        .bind(locked)
        .bind(total)
        .bind(change)
        .bind(reason)
        .bind(trade_id)
        .bind(now_ms)
        .execute(&self.pool)
        .await
        .context("Failed to record live bankroll history entry")?;

        Ok(())
    }

    /// Retrieve LIVE bankroll transaction history
    pub async fn get_live_bankroll_history(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::types::BankrollHistoryEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT id, timestamp, active_bankroll, locked_profit, total_equity, change_amount, reason, trade_id
            FROM live_bankroll_history
            ORDER BY id DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query live bankroll history")?;

        let list = rows
            .into_iter()
            .map(|r| crate::types::BankrollHistoryEntry {
                id: r.get("id"),
                timestamp_ms: r.get("timestamp"),
                active_bankroll: r.get("active_bankroll"),
                locked_profit: r.get("locked_profit"),
                total_equity: r.get("total_equity"),
                change_amount: r.get("change_amount"),
                reason: r.get("reason"),
                trade_id: r.get("trade_id"),
            })
            .collect();

        Ok(list)
    }

    /// Retrieve live risk configuration
    pub async fn get_live_risk_config(&self) -> Result<crate::config::RiskConfig> {
        let row_opt = sqlx::query(
            r#"
            SELECT daily_loss_limit, max_consecutive_losses, max_drawdown, cooldown_minutes
            FROM live_risk_config
            WHERE id = 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query live risk config")?;

        match row_opt {
            Some(r) => Ok(crate::config::RiskConfig {
                daily_loss_limit: r.get("daily_loss_limit"),
                max_consecutive_losses: r.get::<i64, _>("max_consecutive_losses") as u32,
                max_drawdown: r.get("max_drawdown"),
                cooldown_minutes: r.get::<i64, _>("cooldown_minutes") as u32,
            }),
            None => Ok(crate::config::RiskConfig {
                daily_loss_limit: 5.0,
                max_drawdown: 0.20,
                max_consecutive_losses: 3,
                cooldown_minutes: 15,
            }),
        }
    }

    /// Save or update live risk configuration
    pub async fn save_live_risk_config(&self, cfg: &crate::config::RiskConfig) -> Result<()> {
        let now_ms = Utc::now().timestamp_millis();
        sqlx::query(
            r#"
            INSERT INTO live_risk_config (id, daily_loss_limit, max_consecutive_losses, max_drawdown, cooldown_minutes, minimum_bankroll, updated_at)
            VALUES (1, ?, ?, ?, ?, 0.50, ?)
            ON CONFLICT(id) DO UPDATE SET
                daily_loss_limit = excluded.daily_loss_limit,
                max_consecutive_losses = excluded.max_consecutive_losses,
                max_drawdown = excluded.max_drawdown,
                cooldown_minutes = excluded.cooldown_minutes,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(cfg.daily_loss_limit)
        .bind(cfg.max_consecutive_losses as i64)
        .bind(cfg.max_drawdown)
        .bind(cfg.cooldown_minutes as i64)
        .bind(now_ms)
        .execute(&self.pool)
        .await
        .context("Failed to save live risk config")?;

        Ok(())
    }

    /// Insert a settled paper trade result with full balance attribution
    pub async fn insert_paper_result(&self, res: &crate::types::PaperResult) -> Result<()> {
        let now_ms = Utc::now().timestamp_millis();
        let asset_str = res.asset.to_string();
        let side_str = res.side.to_string();

        // Ensure parent market exists
        let condition_id = format!("cond_{}", res.market_id);
        let _ = sqlx::query(
            r#"
            INSERT OR IGNORE INTO markets (id, condition_id, asset, start_time, end_time, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, 'resolved', ?, ?)
            "#,
        )
        .bind(&res.market_id)
        .bind(&condition_id)
        .bind(&asset_str)
        .bind(res.entry_time_ms)
        .bind(res.entry_time_ms + 300_000)
        .bind(now_ms)
        .bind(now_ms)
        .execute(&self.pool)
        .await;

        // Ensure parent paper order exists
        let _ = sqlx::query(
            r#"
            INSERT OR IGNORE INTO paper_orders (
                order_id, market_id, asset, side, stake, shares,
                quote_price, fill_price, slippage, fee, status, signal_id, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'FILLED', NULL, ?)
            "#,
        )
        .bind(&res.order_id)
        .bind(&res.market_id)
        .bind(&asset_str)
        .bind(&side_str)
        .bind(res.stake)
        .bind(res.shares)
        .bind(res.entry_price)
        .bind(res.fill_price)
        .bind(res.slippage)
        .bind(res.fee)
        .bind(res.entry_time_ms)
        .execute(&self.pool)
        .await;

        sqlx::query(
            r#"
            INSERT INTO paper_results (
                result_id, order_id, market_id, asset, side, entry_time,
                entry_price, fill_price, shares, stake, predicted_probability,
                model_confidence, gross_edge, net_edge, fee, slippage,
                outcome, payout, pnl, bankroll_before, bankroll_after,
                locked_profit_before, locked_profit_after, strategy_version,
                model_version, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&res.result_id)
        .bind(&res.order_id)
        .bind(&res.market_id)
        .bind(&asset_str)
        .bind(&side_str)
        .bind(res.entry_time_ms)
        .bind(res.entry_price)
        .bind(res.fill_price)
        .bind(res.shares)
        .bind(res.stake)
        .bind(res.predicted_probability)
        .bind(&res.model_confidence)
        .bind(res.gross_edge)
        .bind(res.net_edge)
        .bind(res.fee)
        .bind(res.slippage)
        .bind(&res.outcome)
        .bind(res.payout)
        .bind(res.pnl)
        .bind(res.bankroll_before)
        .bind(res.bankroll_after)
        .bind(res.locked_profit_before)
        .bind(res.locked_profit_after)
        .bind(&res.strategy_version)
        .bind(&res.model_version)
        .bind(res.created_at_ms)
        .execute(&self.pool)
        .await
        .context("Failed to insert paper result")?;

        Ok(())
    }

    /// Retrieve recent paper results
    pub async fn get_paper_results(&self, limit: i64) -> Result<Vec<crate::types::PaperResult>> {
        let rows = sqlx::query(
            r#"
            SELECT result_id, order_id, market_id, asset, side, entry_time,
                   entry_price, fill_price, shares, stake, predicted_probability,
                   model_confidence, gross_edge, net_edge, fee, slippage,
                   outcome, payout, pnl, bankroll_before, bankroll_after,
                   locked_profit_before, locked_profit_after, strategy_version,
                   model_version, created_at
            FROM paper_results
            ORDER BY id DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query paper results")?;

        let mut results = Vec::new();
        for r in rows {
            let asset_str: String = r.get("asset");
            let asset: crate::types::Asset = asset_str.parse().unwrap_or(crate::types::Asset::BTC);
            let side_str: String = r.get("side");
            let side = if side_str.to_uppercase() == "UP" {
                crate::types::MarketSide::Up
            } else {
                crate::types::MarketSide::Down
            };

            results.push(crate::types::PaperResult {
                result_id: r.get("result_id"),
                order_id: r.get("order_id"),
                market_id: r.get("market_id"),
                asset,
                side,
                entry_time_ms: r.get("entry_time"),
                entry_price: r.get("entry_price"),
                fill_price: r.get("fill_price"),
                shares: r.get("shares"),
                stake: r.get("stake"),
                predicted_probability: r.get("predicted_probability"),
                model_confidence: r.get("model_confidence"),
                gross_edge: r.get("gross_edge"),
                net_edge: r.get("net_edge"),
                fee: r.get("fee"),
                slippage: r.get("slippage"),
                outcome: r.get("outcome"),
                payout: r.get("payout"),
                pnl: r.get("pnl"),
                bankroll_before: r.get("bankroll_before"),
                bankroll_after: r.get("bankroll_after"),
                locked_profit_before: r.get("locked_profit_before"),
                locked_profit_after: r.get("locked_profit_after"),
                strategy_version: r.get("strategy_version"),
                model_version: r.get("model_version"),
                created_at_ms: r.get("created_at"),
            });
        }

        Ok(results)
    }

    /// Calculate summary trade statistics across all paper results
    pub async fn get_trade_statistics(&self) -> Result<crate::types::TradeStatistics> {
        let results = self.get_paper_results(10_000).await?;

        let total_trades = results.len() as i64;
        let mut winning_trades = 0i64;
        let mut losing_trades = 0i64;
        let mut void_trades = 0i64;
        let mut total_pnl = 0.0;
        let mut gross_profit = 0.0;
        let mut gross_loss = 0.0;
        let mut max_win = 0.0f64;
        let mut max_loss = 0.0f64;

        for r in &results {
            total_pnl += r.pnl;
            match r.outcome.to_uppercase().as_str() {
                "WIN" => {
                    winning_trades += 1;
                    gross_profit += r.pnl;
                    if r.pnl > max_win {
                        max_win = r.pnl;
                    }
                }
                "LOSS" => {
                    losing_trades += 1;
                    gross_loss += -r.pnl;
                    if r.pnl < max_loss {
                        max_loss = r.pnl;
                    }
                }
                _ => {
                    void_trades += 1;
                }
            }
        }

        let non_void = winning_trades + losing_trades;
        let win_rate = if non_void > 0 {
            (winning_trades as f64 / non_void as f64) * 100.0
        } else {
            0.0
        };

        let profit_factor = if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else if gross_profit > 0.0 {
            999.99
        } else {
            0.0
        };

        let avg_trade_pnl = if total_trades > 0 {
            total_pnl / total_trades as f64
        } else {
            0.0
        };

        Ok(crate::types::TradeStatistics {
            total_trades,
            winning_trades,
            losing_trades,
            void_trades,
            win_rate,
            total_pnl,
            gross_profit,
            gross_loss,
            profit_factor,
            avg_trade_pnl,
            max_win,
            max_loss,
        })
    }

    // =========================================================================
    // Polymarket Accounts
    // =========================================================================
    pub async fn insert_account(&self, acc: &crate::types::PolymarketAccount) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO polymarket_accounts (
                id, label, api_key, api_secret, api_passphrase, wallet_address,
                proxy_wallet_address, is_active, balance_usdc, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&acc.id)
        .bind(&acc.label)
        .bind(&acc.api_key)
        .bind(&acc.api_secret)
        .bind(&acc.api_passphrase)
        .bind(&acc.wallet_address)
        .bind(&acc.proxy_wallet_address)
        .bind(if acc.is_active { 1 } else { 0 })
        .bind(acc.balance_usdc)
        .bind(acc.created_at)
        .bind(acc.updated_at)
        .execute(&self.pool)
        .await
        .context("Failed to insert polymarket account")?;
        Ok(())
    }

    pub async fn get_accounts(&self) -> Result<Vec<crate::types::PolymarketAccount>> {
        let rows = sqlx::query(
            r#"
            SELECT id, label, api_key, api_secret, api_passphrase, wallet_address,
                   proxy_wallet_address, is_active, balance_usdc, created_at, updated_at
            FROM polymarket_accounts
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch polymarket accounts")?;

        let mut accounts = Vec::new();
        for r in rows {
            let is_active_int: i64 = r.try_get("is_active")?;
            accounts.push(crate::types::PolymarketAccount {
                id: r.try_get("id")?,
                label: r.try_get("label")?,
                api_key: r.try_get("api_key")?,
                api_secret: r.try_get("api_secret")?,
                api_passphrase: r.try_get("api_passphrase")?,
                wallet_address: r.try_get("wallet_address")?,
                proxy_wallet_address: r.try_get("proxy_wallet_address")?,
                is_active: is_active_int == 1,
                balance_usdc: r.try_get("balance_usdc")?,
                created_at: r.try_get("created_at")?,
                updated_at: r.try_get("updated_at")?,
            });
        }
        Ok(accounts)
    }

    pub async fn get_active_account(&self) -> Result<Option<crate::types::PolymarketAccount>> {
        let row = sqlx::query(
            r#"
            SELECT id, label, api_key, api_secret, api_passphrase, wallet_address,
                   proxy_wallet_address, is_active, balance_usdc, created_at, updated_at
            FROM polymarket_accounts
            WHERE is_active = 1
            LIMIT 1
            "#
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch active polymarket account")?;

        if let Some(r) = row {
            let is_active_int: i64 = r.try_get("is_active")?;
            Ok(Some(crate::types::PolymarketAccount {
                id: r.try_get("id")?,
                label: r.try_get("label")?,
                api_key: r.try_get("api_key")?,
                api_secret: r.try_get("api_secret")?,
                api_passphrase: r.try_get("api_passphrase")?,
                wallet_address: r.try_get("wallet_address")?,
                proxy_wallet_address: r.try_get("proxy_wallet_address")?,
                is_active: is_active_int == 1,
                balance_usdc: r.try_get("balance_usdc")?,
                created_at: r.try_get("created_at")?,
                updated_at: r.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn set_active_account(&self, account_id: &str) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        // Deactivate all first
        sqlx::query("UPDATE polymarket_accounts SET is_active = 0, updated_at = ?")
            .bind(now)
            .execute(&self.pool)
            .await?;
        // Activate target
        sqlx::query("UPDATE polymarket_accounts SET is_active = 1, updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(account_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_account(&self, account_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM polymarket_accounts WHERE id = ?")
            .bind(account_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_account_balance(&self, account_id: &str, balance: f64) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        sqlx::query("UPDATE polymarket_accounts SET balance_usdc = ?, updated_at = ? WHERE id = ?")
            .bind(balance)
            .bind(now)
            .bind(account_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_account_proxy_wallet(&self, account_id: &str, proxy_wallet: &str) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        sqlx::query("UPDATE polymarket_accounts SET proxy_wallet_address = ?, updated_at = ? WHERE id = ?")
            .bind(proxy_wallet)
            .bind(now)
            .bind(account_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_account(&self, acc: &crate::types::PolymarketAccount) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        sqlx::query(
            r#"
            UPDATE polymarket_accounts
            SET label = ?, api_key = ?, api_secret = ?, api_passphrase = ?,
                wallet_address = ?, proxy_wallet_address = ?, balance_usdc = ?, updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(&acc.label)
        .bind(&acc.api_key)
        .bind(&acc.api_secret)
        .bind(&acc.api_passphrase)
        .bind(&acc.wallet_address)
        .bind(&acc.proxy_wallet_address)
        .bind(acc.balance_usdc)
        .bind(now)
        .bind(&acc.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // =========================================================================
    // Real Orders
    // =========================================================================
    pub async fn insert_real_order(&self, order: &crate::types::RealOrder) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO real_orders (
                id, account_id, clob_order_id, market_id, token_id, asset,
                side, outcome, order_type, price, size, filled_size, status,
                fee, pnl, error_message, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&order.id)
        .bind(&order.account_id)
        .bind(&order.clob_order_id)
        .bind(&order.market_id)
        .bind(&order.token_id)
        .bind(order.asset.to_string())
        .bind(&order.side)
        .bind(&order.outcome)
        .bind(&order.order_type)
        .bind(order.price)
        .bind(order.size)
        .bind(order.filled_size)
        .bind(&order.status)
        .bind(order.fee)
        .bind(order.pnl)
        .bind(&order.error_message)
        .bind(order.created_at)
        .bind(order.updated_at)
        .execute(&self.pool)
        .await
        .context("Failed to insert real order")?;
        Ok(())
    }

    pub async fn get_real_orders(&self, limit: i64) -> Result<Vec<crate::types::RealOrder>> {
        let rows = sqlx::query(
            r#"
            SELECT id, account_id, clob_order_id, market_id, token_id, asset,
                   side, outcome, order_type, price, size, filled_size, status,
                   fee, pnl, error_message, created_at, updated_at
            FROM real_orders
            ORDER BY created_at DESC
            LIMIT ?
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch real orders")?;

        let mut list = Vec::new();
        for r in rows {
            let asset_str: String = r.try_get("asset")?;
            let asset = asset_str.parse().unwrap_or(crate::types::Asset::BTC);
            list.push(crate::types::RealOrder {
                id: r.try_get("id")?,
                account_id: r.try_get("account_id")?,
                clob_order_id: r.try_get("clob_order_id")?,
                market_id: r.try_get("market_id")?,
                token_id: r.try_get("token_id")?,
                asset,
                side: r.try_get("side")?,
                outcome: r.try_get("outcome")?,
                order_type: r.try_get("order_type")?,
                price: r.try_get("price")?,
                size: r.try_get("size")?,
                filled_size: r.try_get("filled_size")?,
                status: r.try_get("status")?,
                fee: r.try_get("fee")?,
                pnl: r.try_get("pnl")?,
                error_message: r.try_get("error_message")?,
                created_at: r.try_get("created_at")?,
                updated_at: r.try_get("updated_at")?,
            });
        }
        Ok(list)
    }

    // =========================================================================
    // Self-Learning State & History
    // =========================================================================
    pub async fn save_learning_state(&self, state: &crate::types::LearningState) -> Result<()> {
        let weights_json = serde_json::to_string(&state.weights)?;
        let top_features_json = serde_json::to_string(&state.top_features)?;

        sqlx::query(
            r#"
            INSERT INTO learning_state (
                id, asset, version, weights_json, bias, platt_a, platt_b,
                learning_rate, total_samples_trained, rolling_accuracy,
                rolling_brier_score, top_features_json, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(asset) DO UPDATE SET
                version = excluded.version,
                weights_json = excluded.weights_json,
                bias = excluded.bias,
                platt_a = excluded.platt_a,
                platt_b = excluded.platt_b,
                learning_rate = excluded.learning_rate,
                total_samples_trained = excluded.total_samples_trained,
                rolling_accuracy = excluded.rolling_accuracy,
                rolling_brier_score = excluded.rolling_brier_score,
                top_features_json = excluded.top_features_json,
                updated_at = excluded.updated_at
            "#
        )
        .bind(&state.id)
        .bind(state.asset.to_string())
        .bind(state.version)
        .bind(weights_json)
        .bind(state.bias)
        .bind(state.platt_a)
        .bind(state.platt_b)
        .bind(state.learning_rate)
        .bind(state.total_samples_trained)
        .bind(state.rolling_accuracy)
        .bind(state.rolling_brier_score)
        .bind(top_features_json)
        .bind(state.updated_at)
        .execute(&self.pool)
        .await
        .context("Failed to save learning state")?;
        Ok(())
    }

    pub async fn get_learning_state(&self, asset: crate::types::Asset) -> Result<Option<crate::types::LearningState>> {
        let row = sqlx::query(
            r#"
            SELECT id, asset, version, weights_json, bias, platt_a, platt_b,
                   learning_rate, total_samples_trained, rolling_accuracy,
                   rolling_brier_score, top_features_json, updated_at
            FROM learning_state
            WHERE asset = ?
            "#
        )
        .bind(asset.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get learning state")?;

        if let Some(r) = row {
            let weights_str: String = r.try_get("weights_json")?;
            let weights: Vec<f64> = serde_json::from_str(&weights_str).unwrap_or_default();
            let top_features_str: Option<String> = r.try_get("top_features_json")?;
            let top_features = top_features_str
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            Ok(Some(crate::types::LearningState {
                id: r.try_get("id")?,
                asset,
                version: r.try_get("version")?,
                weights,
                bias: r.try_get("bias")?,
                platt_a: r.try_get("platt_a")?,
                platt_b: r.try_get("platt_b")?,
                learning_rate: r.try_get("learning_rate")?,
                total_samples_trained: r.try_get("total_samples_trained")?,
                rolling_accuracy: r.try_get("rolling_accuracy")?,
                rolling_brier_score: r.try_get("rolling_brier_score")?,
                top_features,
                updated_at: r.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn record_learning_history(
        &self,
        asset: crate::types::Asset,
        round_id: &str,
        predicted_prob: f64,
        actual_outcome: i64,
        loss: f64,
        weights_delta_norm: f64,
        brier_score: f64,
    ) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        sqlx::query(
            r#"
            INSERT INTO learning_history (
                asset, round_id, predicted_prob, actual_outcome,
                loss, weights_delta_norm, brier_score, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(asset.to_string())
        .bind(round_id)
        .bind(predicted_prob)
        .bind(actual_outcome)
        .bind(loss)
        .bind(weights_delta_norm)
        .bind(brier_score)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to record learning history")?;
        Ok(())
    }

    pub async fn get_learning_history(&self, asset: Option<crate::types::Asset>, limit: i64) -> Result<Vec<crate::types::LearningHistoryEntry>> {
        let rows = if let Some(a) = asset {
            sqlx::query(
                r#"
                SELECT id, asset, round_id, predicted_prob, actual_outcome,
                       loss, weights_delta_norm, brier_score, timestamp
                FROM learning_history
                WHERE asset = ?
                ORDER BY timestamp DESC
                LIMIT ?
                "#
            )
            .bind(a.to_string())
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                r#"
                SELECT id, asset, round_id, predicted_prob, actual_outcome,
                       loss, weights_delta_norm, brier_score, timestamp
                FROM learning_history
                ORDER BY timestamp DESC
                LIMIT ?
                "#
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        let mut list = Vec::new();
        for r in rows {
            let a_str: String = r.try_get("asset")?;
            let asset_parsed = a_str.parse().unwrap_or(crate::types::Asset::BTC);
            list.push(crate::types::LearningHistoryEntry {
                id: r.try_get("id")?,
                asset: asset_parsed,
                round_id: r.try_get("round_id")?,
                predicted_prob: r.try_get("predicted_prob")?,
                actual_outcome: r.try_get("actual_outcome")?,
                loss: r.try_get("loss")?,
                weights_delta_norm: r.try_get("weights_delta_norm")?,
                brier_score: r.try_get("brier_score")?,
                timestamp: r.try_get("timestamp")?,
            });
        }
        Ok(list)
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
  min_time_remaining_sec: 30
  max_time_remaining_sec: 240
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
