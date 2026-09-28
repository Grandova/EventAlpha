use anyhow::{Context, Result};
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::db::Database;
use crate::types::{Asset, MarketStatus, Resolution};

pub const CYCLE_DURATION_SECS: i64 = 300; // 5 minutes = 300 seconds

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Polymarket5mMarket {
    pub id: String,
    pub condition_id: String,
    pub asset: Asset,
    pub slug: String,
    pub question: String,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub open_price: Option<f64>,
    pub final_price: Option<f64>,
    pub status: MarketStatus,
    pub resolution: Option<Resolution>,
    pub up_token_id: String,
    pub down_token_id: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl Polymarket5mMarket {
    pub fn remaining_seconds(&self, now_ms: i64) -> i64 {
        ((self.end_time_ms - now_ms) / 1000).max(0)
    }

    pub fn is_expired(&self, now_ms: i64) -> bool {
        now_ms >= self.end_time_ms
    }
}

#[derive(Clone)]
pub struct MarketDiscoveryEngine {
    active_markets: Arc<DashMap<Asset, Polymarket5mMarket>>,
    db: Arc<Database>,
}

impl MarketDiscoveryEngine {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            active_markets: Arc::new(DashMap::new()),
            db,
        }
    }

    /// Calculate epoch-aligned 5-minute round timestamps
    pub fn calculate_5m_window(now_ms: i64) -> (i64, i64) {
        let now_secs = now_ms / 1000;
        let start_secs = (now_secs / CYCLE_DURATION_SECS) * CYCLE_DURATION_SECS;
        let end_secs = start_secs + CYCLE_DURATION_SECS;
        (start_secs * 1000, end_secs * 1000)
    }

    /// Generate deterministic 5-minute market for asset if not yet existing
    pub async fn ensure_active_market(
        &self,
        asset: Asset,
        now_ms: i64,
        current_reference_price: Option<f64>,
    ) -> Result<Polymarket5mMarket> {
        let (window_start_ms, window_end_ms) = Self::calculate_5m_window(now_ms);
        let start_epoch_sec = window_start_ms / 1000;

        let market_id = format!("{}-5M-{}", asset, start_epoch_sec);

        // Check if cached active market matches the current window
        if let Some(entry) = self.active_markets.get(&asset) {
            if entry.id == market_id && entry.status == MarketStatus::Active {
                return Ok(entry.clone());
            }
        }

        // Query database to see if this round was already recorded
        let row_opt = sqlx::query(
            r#"
            SELECT id, condition_id, asset, slug, question, start_time, end_time, open_price, final_price, status, resolution, created_at, updated_at
            FROM markets
            WHERE id = ?
            "#,
        )
        .bind(&market_id)
        .fetch_optional(self.db.pool())
        .await
        .context("Failed to query market by id")?;

        if let Some(row) = row_opt {
            use sqlx::Row;
            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "resolved" => MarketStatus::Resolved,
                "expired" => MarketStatus::Expired,
                _ => MarketStatus::Active,
            };

            let resolution_str: Option<String> = row.get("resolution");
            let resolution = resolution_str.map(|r| match r.as_str() {
                "UP" => Resolution::Up,
                "DOWN" => Resolution::Down,
                _ => Resolution::Void,
            });

            let up_token = format!("{}_{}_UP", market_id, asset);
            let down_token = format!("{}_{}_DOWN", market_id, asset);

            let market = Polymarket5mMarket {
                id: row.get("id"),
                condition_id: row.get("condition_id"),
                asset,
                slug: row.get("slug"),
                question: row.get("question"),
                start_time_ms: row.get("start_time"),
                end_time_ms: row.get("end_time"),
                open_price: row.get("open_price"),
                final_price: row.get("final_price"),
                status,
                resolution,
                up_token_id: up_token,
                down_token_id: down_token,
                created_at_ms: row.get("created_at"),
                updated_at_ms: row.get("updated_at"),
            };

            self.active_markets.insert(asset, market.clone());
            return Ok(market);
        }

        // Create new 5-minute market round
        let slug = format!("{}-updown-5m-{}", asset.to_string().to_lowercase(), start_epoch_sec);
        let question = format!("Will {} be Up or Down in the next 5 minutes?", asset);
        let condition_id = format!("0x{:x}", md5_or_hash(&market_id));
        let up_token = format!("{}_{}_UP", market_id, asset);
        let down_token = format!("{}_{}_DOWN", market_id, asset);

        sqlx::query(
            r#"
            INSERT INTO markets (id, condition_id, asset, slug, question, start_time, end_time, open_price, final_price, status, resolution, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL, 'active', NULL, ?, ?)
            "#,
        )
        .bind(&market_id)
        .bind(&condition_id)
        .bind(asset.to_string())
        .bind(&slug)
        .bind(&question)
        .bind(window_start_ms)
        .bind(window_end_ms)
        .bind(current_reference_price)
        .bind(now_ms)
        .bind(now_ms)
        .execute(self.db.pool())
        .await
        .context("Failed to insert new market round")?;

        let market = Polymarket5mMarket {
            id: market_id,
            condition_id,
            asset,
            slug,
            question,
            start_time_ms: window_start_ms,
            end_time_ms: window_end_ms,
            open_price: current_reference_price,
            final_price: None,
            status: MarketStatus::Active,
            resolution: None,
            up_token_id: up_token,
            down_token_id: down_token,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        };

        info!(
            "Discovered/Initialized new 5M Market: {} | Open Price: {:?} | Remaining: {}s",
            market.id,
            market.open_price,
            market.remaining_seconds(now_ms)
        );

        self.active_markets.insert(asset, market.clone());
        Ok(market)
    }

    /// Retrieve active market for an asset
    pub fn get_active_market(&self, asset: Asset) -> Option<Polymarket5mMarket> {
        self.active_markets.get(&asset).map(|e| e.clone())
    }

    /// List all currently active markets
    pub fn list_active_markets(&self) -> Vec<Polymarket5mMarket> {
        self.active_markets.iter().map(|e| e.value().clone()).collect()
    }

    /// Update open price of active market if not yet recorded
    pub async fn set_open_price(&self, asset: Asset, open_price: f64) -> Result<()> {
        if let Some(mut entry) = self.active_markets.get_mut(&asset) {
            if entry.open_price.is_none() {
                entry.open_price = Some(open_price);
                sqlx::query(
                    "UPDATE markets SET open_price = ?, updated_at = ? WHERE id = ?"
                )
                .bind(open_price)
                .bind(Utc::now().timestamp_millis())
                .bind(&entry.id)
                .execute(self.db.pool())
                .await?;
            }
        }
        Ok(())
    }
}

fn md5_or_hash(input: &str) -> u128 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish() as u128
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_5m_window() {
        // e.g. 1700000123 seconds => 1700000123000 ms
        let now_ms = 1700000123000i64;
        let (start, end) = MarketDiscoveryEngine::calculate_5m_window(now_ms);

        assert_eq!(start % (300 * 1000), 0);
        assert_eq!(end - start, 300 * 1000);
        assert!(now_ms >= start && now_ms < end);
    }

    #[test]
    fn test_remaining_seconds() {
        let now_ms = 1700000000000i64;
        let end_ms = now_ms + 120_000; // 120 seconds left

        let market = Polymarket5mMarket {
            id: "BTC-5M-TEST".to_string(),
            condition_id: "0x123".to_string(),
            asset: Asset::BTC,
            slug: "btc-5m".to_string(),
            question: "BTC up or down?".to_string(),
            start_time_ms: now_ms - 180_000,
            end_time_ms: end_ms,
            open_price: Some(65000.0),
            final_price: None,
            status: MarketStatus::Active,
            resolution: None,
            up_token_id: "token_up".to_string(),
            down_token_id: "token_down".to_string(),
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        };

        assert_eq!(market.remaining_seconds(now_ms), 120);
        assert!(!market.is_expired(now_ms));
        assert!(market.is_expired(end_ms + 1000));
    }
}
