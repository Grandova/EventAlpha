use anyhow::{Context, Result};
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GammaMarketInfo {
    pub question: String,
    pub condition_id: String,
    pub slug: String,
    pub up_token_id: String,
    pub down_token_id: String,
    pub up_price: Option<f64>,
    pub down_price: Option<f64>,
}

/// Fetch real live 5-minute market metadata from Polymarket official Gamma API
pub async fn fetch_gamma_market_info(asset: Asset, start_epoch_sec: i64) -> Option<GammaMarketInfo> {
    let sym = asset.to_string().to_lowercase();
    let slug = format!("{}-updown-5m-{}", sym, start_epoch_sec);
    let url = format!("https://gamma-api.polymarket.com/events?slug={}", slug);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .build()
        .ok()?;

    let resp = client.get(&url).send().await.ok()?;
    let events: Vec<serde_json::Value> = resp.json().await.ok()?;
    let event = events.first()?;
    let markets = event.get("markets")?.as_array()?;
    let market = markets.first()?;

    let question = market.get("question")?.as_str()?.to_string();
    let condition_id = market.get("conditionId")?.as_str()?.to_string();
    let slug = market.get("slug")?.as_str()?.to_string();

    let tokens_str = market.get("clobTokenIds")?.as_str()?;
    let tokens: Vec<String> = serde_json::from_str(tokens_str).ok()?;
    let up_token_id = tokens.get(0)?.clone();
    let down_token_id = tokens.get(1)?.clone();

    let mut up_price = None;
    let mut down_price = None;
    if let Some(outcome_str) = market.get("outcomePrices").and_then(|v| v.as_str()) {
        if let Ok(prices) = serde_json::from_str::<Vec<String>>(outcome_str) {
            if let Some(p) = prices.get(0).and_then(|s| s.parse::<f64>().ok()) {
                up_price = Some(p);
            }
            if let Some(p) = prices.get(1).and_then(|s| s.parse::<f64>().ok()) {
                down_price = Some(p);
            }
        }
    }

    Some(GammaMarketInfo {
        question,
        condition_id,
        slug,
        up_token_id,
        down_token_id,
        up_price,
        down_price,
    })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolymarketCryptoPriceResponse {
    pub open_price: Option<f64>,
    pub close_price: Option<f64>,
    pub timestamp: Option<i64>,
    pub completed: Option<bool>,
    pub incomplete: Option<bool>,
    pub cached: Option<bool>,
}

/// Query the official Polymarket crypto price endpoint for 5m Chainlink TWAP benchmark.
/// This endpoint returns the exact `openPrice` (Price to Beat / 目标价格) and `closePrice`
/// as displayed on polymarket.com/zh/event/{slug}.
pub async fn fetch_polymarket_crypto_price(
    asset: Asset,
    start_epoch_sec: i64,
) -> Option<PolymarketCryptoPriceResponse> {
    let sym = match asset {
        Asset::BTC => "BTC",
        Asset::ETH => "ETH",
        Asset::SOL => "SOL",
    };
    let end_epoch_sec = start_epoch_sec + 300;
    let url = format!(
        "https://polymarket.com/api/crypto/crypto-price?symbol={}&eventStartTime={}&endDate={}&variant=fiveminute&twapEnabled=true&twapLookbackSeconds=60",
        sym, start_epoch_sec, end_epoch_sec
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(3000))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .build()
        .ok()?;

    let resp = client.get(&url).send().await.ok()?;
    if resp.status().is_success() {
        if let Ok(data) = resp.json::<PolymarketCryptoPriceResponse>().await {
            return Some(data);
        }
    }
    None
}

/// Query the deterministic 5-minute candle open price.
/// Priority 1: Official Polymarket crypto-price API (exact Chainlink TWAP benchmark / 目标价格).
/// Priority 2: Coinbase USD candle open.
/// Priority 3: Binance Kline fallback.
pub async fn fetch_candle_open_price(asset: Asset, start_epoch_sec: i64) -> Option<f64> {
    // 1. Official Polymarket Chainlink TWAP benchmark (Priority 1)
    if let Some(poly_price) = fetch_polymarket_crypto_price(asset, start_epoch_sec).await {
        if let Some(open) = poly_price.open_price {
            if open > 0.0 {
                return Some(open);
            }
        }
    }
    // If the current round openPrice hasn't populated yet, check if the previous round's closePrice is available
    if let Some(prev_price) = fetch_polymarket_crypto_price(asset, start_epoch_sec - 300).await {
        if let Some(close) = prev_price.close_price {
            if close > 0.0 {
                return Some(close);
            }
        }
    }

    let cb_symbol = match asset {
        Asset::BTC => "BTC-USD",
        Asset::ETH => "ETH-USD",
        Asset::SOL => "SOL-USD",
    };

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .user_agent("Mozilla/5.0")
        .build()
    {
        Ok(c) => c,
        Err(_) => return None,
    };

    // 2. Try Coinbase REST API candles (USD benchmark fallback)
    let cb_url = format!(
        "https://api.exchange.coinbase.com/products/{}/candles?granularity=300",
        cb_symbol
    );
    if let Ok(resp) = client.get(&cb_url).send().await {
        if let Ok(candles) = resp.json::<Vec<Vec<serde_json::Value>>>().await {
            for candle in candles {
                if candle.len() >= 4 {
                    if let Some(time_sec) = candle[0].as_i64() {
                        if time_sec == start_epoch_sec {
                            if let Some(open) = candle[3].as_f64() {
                                if open > 0.0 {
                                    return Some(open);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Fallback: Binance Kline
    let binance_sym = match asset {
        Asset::BTC => "BTCUSDT",
        Asset::ETH => "ETHUSDT",
        Asset::SOL => "SOLUSDT",
    };
    let binance_url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval=5m&startTime={}&limit=1",
        binance_sym,
        start_epoch_sec * 1000
    );
    if let Ok(resp) = client.get(&binance_url).send().await {
        if let Ok(klines) = resp.json::<Vec<Vec<serde_json::Value>>>().await {
            if let Some(kline) = klines.first() {
                if kline.len() >= 2 {
                    if let Some(open_str) = kline[1].as_str() {
                        if let Ok(open_val) = open_str.parse::<f64>() {
                            if open_val > 0.0 {
                                return Some(open_val);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketPriceHistoryPoint {
    pub timestamp: i64,
    pub value: f64,
}

/// Fetch official real-time price history points for this 5-minute round from Polymarket.
/// The last point represents the official Polymarket current price (Chainlink 60s TWAP).
pub async fn fetch_polymarket_price_history(
    asset: Asset,
    start_epoch_sec: i64,
) -> Option<Vec<PolymarketPriceHistoryPoint>> {
    let sym = match asset {
        Asset::BTC => "BTC",
        Asset::ETH => "ETH",
        Asset::SOL => "SOL",
    };
    let end_epoch_sec = start_epoch_sec + 300;
    let url = format!(
        "https://polymarket.com/api/crypto/price-history?symbol={}&eventStartTime={}&endDate={}&variant=fiveminute&twapEnabled=true&twapLookbackSeconds=60",
        sym, start_epoch_sec, end_epoch_sec
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(3000))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .build()
        .ok()?;

    let resp = client.get(&url).send().await.ok()?;
    if resp.status().is_success() {
        if let Ok(points) = resp.json::<Vec<PolymarketPriceHistoryPoint>>().await {
            return Some(points);
        }
    }
    None
}

#[derive(Clone)]
pub struct MarketDiscoveryEngine {
    active_markets: Arc<DashMap<Asset, Polymarket5mMarket>>,
    latest_poly_prices: Arc<DashMap<Asset, (f64, i64)>>,
    poly_history: Arc<DashMap<Asset, VecDeque<crate::composite::PriceHistoryPoint>>>,
    db: Arc<Database>,
}

impl MarketDiscoveryEngine {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            active_markets: Arc::new(DashMap::new()),
            latest_poly_prices: Arc::new(DashMap::new()),
            poly_history: Arc::new(DashMap::new()),
            db,
        }
    }

    pub fn set_latest_poly_price(&self, asset: Asset, price: f64, timestamp_ms: i64) {
        if price <= 0.0 {
            return;
        }
        self.latest_poly_prices.insert(asset, (price, timestamp_ms));

        let mut queue = self.poly_history.entry(asset).or_insert_with(VecDeque::new);
        let pt = crate::composite::PriceHistoryPoint {
            timestamp_ms,
            price,
        };
        if let Some(last) = queue.back_mut() {
            if timestamp_ms - last.timestamp_ms < 500 {
                last.price = price;
                last.timestamp_ms = timestamp_ms;
                return;
            }
        }
        queue.push_back(pt);
        if queue.len() > 300 {
            queue.pop_front();
        }
    }

    pub fn get_latest_poly_price(&self, asset: Asset) -> Option<f64> {
        self.latest_poly_prices.get(&asset).map(|e| e.0)
    }

    pub fn get_poly_price_history(&self, asset: Asset) -> Vec<crate::composite::PriceHistoryPoint> {
        self.poly_history
            .get(&asset)
            .map(|q| q.iter().cloned().collect())
            .unwrap_or_default()
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

        // Check if cached active market matches the current window and has valid real token
        if let Some(entry) = self.active_markets.get(&asset) {
            if entry.id == market_id && entry.status == MarketStatus::Active && !entry.up_token_id.ends_with("_UP") {
                return Ok(entry.clone());
            }
        }

        // 1. Fetch real Polymarket Gamma market info
        let gamma_info = fetch_gamma_market_info(asset, start_epoch_sec).await;

        // Query database to see if this round was already recorded
        let row_opt = sqlx::query(
            r#"
            SELECT id, condition_id, asset, slug, question, start_time, end_time, open_price, final_price, status, resolution, up_token_id, down_token_id, created_at, updated_at
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

            let mut up_token: String = row.try_get("up_token_id").unwrap_or_else(|_| format!("{}_{}_UP", market_id, asset));
            let mut down_token: String = row.try_get("down_token_id").unwrap_or_else(|_| format!("{}_{}_DOWN", market_id, asset));
            let mut question: String = row.get("question");
            let mut condition_id: String = row.get("condition_id");
            let mut slug: String = row.get("slug");

            // Upgrade placeholder tokens to real Gamma tokens if available
            if (up_token.ends_with("_UP") || up_token.is_empty()) && gamma_info.is_some() {
                let g = gamma_info.as_ref().unwrap();
                up_token = g.up_token_id.clone();
                down_token = g.down_token_id.clone();
                question = g.question.clone();
                condition_id = g.condition_id.clone();
                slug = g.slug.clone();

                let _ = sqlx::query(
                    "UPDATE markets SET question = ?, condition_id = ?, slug = ?, up_token_id = ?, down_token_id = ?, updated_at = ? WHERE id = ?"
                )
                .bind(&question)
                .bind(&condition_id)
                .bind(&slug)
                .bind(&up_token)
                .bind(&down_token)
                .bind(now_ms)
                .bind(&market_id)
                .execute(self.db.pool())
                .await;
            }

            let mut open_p: Option<f64> = row.get("open_price");
            // Check official Polymarket crypto-price API to align open_price (目标价格 / Price to Beat)
            let official_open = fetch_candle_open_price(asset, start_epoch_sec).await;
            if let Some(p) = official_open {
                let should_update = match open_p {
                    Some(cur) => (cur - p).abs() > 0.001,
                    None => true,
                };
                if should_update {
                    let _ = sqlx::query("UPDATE markets SET open_price = ?, updated_at = ? WHERE id = ?")
                        .bind(p)
                        .bind(now_ms)
                        .bind(&market_id)
                        .execute(self.db.pool())
                        .await;
                    open_p = Some(p);
                }
            } else if open_p.is_none() {
                if let Some(p) = current_reference_price {
                    let _ = sqlx::query("UPDATE markets SET open_price = ?, updated_at = ? WHERE id = ?")
                        .bind(p)
                        .bind(now_ms)
                        .bind(&market_id)
                        .execute(self.db.pool())
                        .await;
                    open_p = Some(p);
                }
            }

            let market = Polymarket5mMarket {
                id: row.get("id"),
                condition_id,
                asset,
                slug,
                question,
                start_time_ms: row.get("start_time"),
                end_time_ms: row.get("end_time"),
                open_price: open_p,
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

        // Create new 5-minute market round with Gamma metadata or fallback
        let (question, condition_id, slug, up_token, down_token) = if let Some(ref g) = gamma_info {
            (
                g.question.clone(),
                g.condition_id.clone(),
                g.slug.clone(),
                g.up_token_id.clone(),
                g.down_token_id.clone(),
            )
        } else {
            (
                format!("Will {} be Up or Down in the next 5 minutes?", asset),
                format!("0x{:x}", md5_or_hash(&market_id)),
                format!("{}-updown-5m-{}", asset.to_string().to_lowercase(), start_epoch_sec),
                format!("{}_{}_UP", market_id, asset),
                format!("{}_{}_DOWN", market_id, asset),
            )
        };

        // Resolve open price: Priority 1 is official Polymarket crypto price endpoint (Chainlink TWAP 目标价格)
        let initial_open_price = fetch_candle_open_price(asset, start_epoch_sec).await
            .or(current_reference_price);

        sqlx::query(
            r#"
            INSERT INTO markets (id, condition_id, asset, slug, question, start_time, end_time, open_price, final_price, status, resolution, up_token_id, down_token_id, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL, 'active', NULL, ?, ?, ?, ?)
            "#,
        )
        .bind(&market_id)
        .bind(&condition_id)
        .bind(asset.to_string())
        .bind(&slug)
        .bind(&question)
        .bind(window_start_ms)
        .bind(window_end_ms)
        .bind(initial_open_price)
        .bind(&up_token)
        .bind(&down_token)
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
            open_price: initial_open_price,
            final_price: None,
            status: MarketStatus::Active,
            resolution: None,
            up_token_id: up_token,
            down_token_id: down_token,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        };

        info!(
            "🎯 Real 5M Polymarket Initialized: {} | Token: {} | Open Price: {:?} | Remaining: {}s",
            market.question,
            market.up_token_id,
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

    /// Update open price of active market to official Polymarket benchmark price if deviated or missing
    pub async fn update_open_price(&self, asset: Asset, open_price: f64) -> Result<()> {
        if let Some(mut entry) = self.active_markets.get_mut(&asset) {
            let should_update = match entry.open_price {
                Some(existing) => (existing - open_price).abs() > 0.001,
                None => true,
            };
            if should_update {
                entry.open_price = Some(open_price);
                let now_ms = Utc::now().timestamp_millis();
                entry.updated_at_ms = now_ms;
                sqlx::query(
                    "UPDATE markets SET open_price = ?, updated_at = ? WHERE id = ?"
                )
                .bind(open_price)
                .bind(now_ms)
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
