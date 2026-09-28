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
