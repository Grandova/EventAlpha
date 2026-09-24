use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

use crate::db::Database;
use crate::types::{Asset, Resolution};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketResolvedEvent {
    pub market_id: String,
    pub asset: Asset,
    pub open_price: f64,
    pub final_price: f64,
    pub price_change: f64,
    pub price_change_pct: f64,
    pub resolution: Resolution,
    pub resolved_at_ms: i64,
}

#[derive(Clone)]
pub struct ResolutionEngine {
    db: Arc<Database>,
    resolution_tx: broadcast::Sender<MarketResolvedEvent>,
}

impl ResolutionEngine {
    pub fn new(db: Arc<Database>) -> Self {
        let (resolution_tx, _) = broadcast::channel(1024);
        Self { db, resolution_tx }
    }

    pub fn subscribe_resolutions(&self) -> broadcast::Receiver<MarketResolvedEvent> {
        self.resolution_tx.subscribe()
    }

    /// Settle a market given its open and final benchmark settlement prices
    pub async fn resolve_market(
        &self,
        market_id: &str,
        asset: Asset,
        open_price: f64,
        final_price: f64,
        now_ms: i64,
    ) -> Result<MarketResolvedEvent> {
        let resolution = if final_price > open_price {
            Resolution::Up
        } else if final_price < open_price {
            Resolution::Down
        } else {
            Resolution::Void
        };

        let price_change = final_price - open_price;
        let price_change_pct = if open_price > 0.0 {
            (price_change / open_price) * 100.0
        } else {
            0.0
        };

        let res_str = match resolution {
            Resolution::Up => "UP",
            Resolution::Down => "DOWN",
            Resolution::Void => "VOID",
        };

        let start_time_ms = now_ms - 300_000;
        let slug = format!("{}-settled", market_id);

        sqlx::query(
            r#"
            INSERT INTO markets (id, condition_id, asset, slug, question, start_time, end_time, open_price, final_price, status, resolution, created_at, updated_at)
            VALUES (?, '0x_settle', ?, ?, '5M Up/Down Market', ?, ?, ?, ?, 'resolved', ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                final_price = excluded.final_price,
                resolution = excluded.resolution,
                status = 'resolved',
                updated_at = excluded.updated_at
            "#,
        )
        .bind(market_id)
        .bind(asset.to_string())
        .bind(&slug)
        .bind(start_time_ms)
        .bind(now_ms)
        .bind(open_price)
        .bind(final_price)
        .bind(res_str)
        .bind(now_ms)
        .bind(now_ms)
        .execute(self.db.pool())
        .await
        .context("Failed to upsert market resolution in DB")?;

        let event = MarketResolvedEvent {
            market_id: market_id.to_string(),
            asset,
            open_price,
            final_price,
            price_change,
            price_change_pct,
            resolution,
            resolved_at_ms: now_ms,
        };

        let payload = serde_json::to_string(&event).unwrap_or_default();
        self.db
            .record_system_event(
                "MARKET_RESOLVED",
                "INFO",
                "RESOLUTION_ENGINE",
                &format!("Market {} settled as {:?}", market_id, resolution),
                Some(&payload),
            )
            .await?;

        info!(
            "Resolved Market: {} | Open: {:.2} | Final: {:.2} | Diff: {:+.2} ({:+.3}%) | Result: {:?}",
            market_id, open_price, final_price, price_change, price_change_pct, resolution
        );

        let _ = self.resolution_tx.send(event.clone());
        Ok(event)
    }

    /// Query recently resolved markets from database
    pub async fn get_recently_resolved(&self, limit: i64) -> Result<Vec<MarketResolvedEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT id, asset, open_price, final_price, resolution, updated_at
            FROM markets
            WHERE status = 'resolved'
            ORDER BY updated_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(self.db.pool())
        .await
        .context("Failed to fetch resolved markets")?;

        use sqlx::Row;
        let mut results = Vec::new();
        for r in rows {
            let asset_str: String = r.get("asset");
            let asset = asset_str.parse::<Asset>().unwrap_or(Asset::BTC);
            let open_price: f64 = r.get("open_price");
            let final_price: f64 = r.get("final_price");
            let res_str: String = r.get("resolution");
            let resolution = match res_str.as_str() {
                "UP" => Resolution::Up,
                "DOWN" => Resolution::Down,
                _ => Resolution::Void,
            };

            let price_change = final_price - open_price;
            let price_change_pct = if open_price > 0.0 {
                (price_change / open_price) * 100.0
            } else {
                0.0
            };

            results.push(MarketResolvedEvent {
                market_id: r.get("id"),
                asset,
                open_price,
                final_price,
                price_change,
                price_change_pct,
                resolution,
                resolved_at_ms: r.get("updated_at"),
            });
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use tempfile::NamedTempFile;
    use std::io::Write;

    async fn create_test_db() -> Arc<Database> {
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
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();
        let config = AppConfig::load_from_path(tmp.path()).unwrap();
        Arc::new(Database::new(&config).await.unwrap())
    }

    #[tokio::test]
    async fn test_market_settlement_rules() {
        let db = create_test_db().await;
        let engine = ResolutionEngine::new(db.clone());

        // Insert a test market
        let market_id = "BTC-5M-TEST-SETTLE";
        sqlx::query(
            "INSERT INTO markets (id, condition_id, asset, start_time, end_time, open_price, status, created_at, updated_at) VALUES (?, '0x1', 'BTC', 1000, 2000, 65000.0, 'active', 1000, 1000)"
        )
        .bind(market_id)
        .execute(db.pool())
        .await
        .unwrap();

        // Resolve where final > open => UP
        let event = engine.resolve_market(market_id, Asset::BTC, 65000.0, 65120.0, 2001).await.unwrap();
        assert_eq!(event.resolution, Resolution::Up);
        assert!(event.price_change > 0.0);

        let resolved = engine.get_recently_resolved(5).await.unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].resolution, Resolution::Up);
        assert_eq!(resolved[0].final_price, 65120.0);
    }
}
