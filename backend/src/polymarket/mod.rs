pub mod clob_client;
pub mod market_discovery;
pub mod orderbook;
pub mod resolution;

use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::info;

use crate::collector::CollectorManager;
use crate::config::AppConfig;
use crate::db::Database;
use crate::types::{Asset, MarketSide, OrderBookLevel, PolymarketTick};
pub use clob_client::{PolymarketClobHttpClient, ClobOrderResponse};
use market_discovery::MarketDiscoveryEngine;
use orderbook::PolymarketBookEngine;
use resolution::{MarketResolvedEvent, ResolutionEngine};

/// Fetch real live L2 orderbook for a Polymarket token ID directly from CLOB API
pub async fn fetch_clob_orderbook(
    client: &reqwest::Client,
    token_id: &str,
) -> Option<(Vec<OrderBookLevel>, Vec<OrderBookLevel>)> {
    if token_id.is_empty() || token_id.ends_with("_UP") || token_id.ends_with("_DOWN") {
        return None;
    }
    let url = format!("https://clob.polymarket.com/book?token_id={}", token_id);
    let resp = client.get(&url).send().await.ok()?;
    let val: serde_json::Value = resp.json().await.ok()?;

    let mut bids = Vec::new();
    if let Some(bids_arr) = val.get("bids").and_then(|b| b.as_array()) {
        for b in bids_arr {
            let price = b.get("price").and_then(|p| p.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
            let size = b.get("size").and_then(|s| s.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
            if price > 0.0 && size > 0.0 {
                bids.push(OrderBookLevel { price, size });
            }
        }
    }

    let mut asks = Vec::new();
    if let Some(asks_arr) = val.get("asks").and_then(|a| a.as_array()) {
        for a in asks_arr {
            let price = a.get("price").and_then(|p| p.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
            let size = a.get("size").and_then(|s| s.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
            if price > 0.0 && size > 0.0 {
                asks.push(OrderBookLevel { price, size });
            }
        }
    }

    if bids.is_empty() && asks.is_empty() {
        None
    } else {
        Some((bids, asks))
    }
}

#[derive(Clone)]
pub struct PolymarketManager {
    discovery: Arc<MarketDiscoveryEngine>,
    book_engine: Arc<PolymarketBookEngine>,
    resolution: Arc<ResolutionEngine>,
    poly_tick_tx: broadcast::Sender<PolymarketTick>,
}

impl PolymarketManager {
    pub fn new(_config: &AppConfig, db: Arc<Database>) -> Self {
        let discovery = Arc::new(MarketDiscoveryEngine::new(db.clone()));
        let book_engine = Arc::new(PolymarketBookEngine::new());
        let resolution = Arc::new(ResolutionEngine::new(db));
        let (poly_tick_tx, _) = broadcast::channel(1024);

        Self {
            discovery,
            book_engine,
            resolution,
            poly_tick_tx,
        }
    }

    pub fn start(&self, collector: Arc<CollectorManager>) {
        info!("Starting Polymarket 5-Minute Lifecycle & Orderbook Engine...");

        let discovery = self.discovery.clone();
        let book_engine = self.book_engine.clone();
        let resolution = self.resolution.clone();
        let poly_tx = self.poly_tick_tx.clone();

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(2500))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build()
            .unwrap_or_default();

        // 1. Spawn CLOB WebSocket client in background
        tokio::spawn(clob_client::run_polymarket_clob_collector(
            book_engine.clone(),
            vec![], // Active dynamic tokens
        ));

        // 2. Spawn 5-minute lifecycle round management task
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_millis(1000));
            let assets = [Asset::BTC, Asset::ETH, Asset::SOL];

            loop {
                ticker.tick().await;
                let now_ms = Utc::now().timestamp_millis();

                for &asset in &assets {
                    // Get current reference spot price from collectors (Prioritizing Coinbase USD for Polymarket oracle alignment)
                    let current_spot_price = collector
                        .get_latest_tick(crate::types::Exchange::Coinbase, asset)
                        .or_else(|| collector.get_latest_tick(crate::types::Exchange::Binance, asset))
                        .or_else(|| collector.get_latest_tick(crate::types::Exchange::Okx, asset))
                        .or_else(|| collector.get_latest_tick(crate::types::Exchange::Bybit, asset))
                        .map(|t| t.mid);

                    // 1. Check if the currently active market for this asset has expired BEFORE creating a new one!
                    if let Some(active_market) = discovery.get_active_market(asset) {
                        if active_market.is_expired(now_ms) && active_market.status == crate::types::MarketStatus::Active {
                            let start_epoch_sec = active_market.start_time_ms / 1000;
                            // Attempt to fetch official Polymarket crypto-price for resolution
                            let official_res = market_discovery::fetch_polymarket_crypto_price(asset, start_epoch_sec).await;

                            let open_p = official_res.as_ref().and_then(|r| r.open_price)
                                .or(active_market.open_price)
                                .unwrap_or_else(|| current_spot_price.unwrap_or(0.0));

                            // If official closePrice is present, use it. Otherwise, if next round's openPrice is available, that is identical to closePrice!
                            let next_open_res = market_discovery::fetch_polymarket_crypto_price(asset, start_epoch_sec + 300).await;
                            let final_p = official_res.as_ref().and_then(|r| r.close_price)
                                .or_else(|| next_open_res.as_ref().and_then(|r| r.open_price))
                                .or(current_spot_price)
                                .unwrap_or(open_p);

                            if open_p > 0.0 && final_p > 0.0 {
                                info!(
                                    "🔔 5M Round EXPIRED for {}: {} | Open: {:.2}, Final: {:.2} | Resolving market with official Chainlink TWAP...",
                                    asset, active_market.id, open_p, final_p
                                );
                                let _ = resolution
                                    .resolve_market(&active_market.id, asset, open_p, final_p, now_ms)
                                    .await;
                            }
                        }
                    }

                    // 2. Also sweep any expired active markets in SQLite DB (recovers unclosed rounds after restart)
                    if let Ok(expired_markets) = resolution.get_unsettled_expired_markets(asset, now_ms).await {
                        for (exp_id, maybe_open) in expired_markets {
                            let parsed_epoch = exp_id.split('-').last().and_then(|s| s.parse::<i64>().ok());
                            let official_res = if let Some(epoch_sec) = parsed_epoch {
                                market_discovery::fetch_polymarket_crypto_price(asset, epoch_sec).await
                            } else {
                                None
                            };

                            let open_p = official_res.as_ref().and_then(|r| r.open_price)
                                .or(maybe_open)
                                .unwrap_or_else(|| current_spot_price.unwrap_or(0.0));

                            let next_epoch = parsed_epoch.map(|e| e + 300);
                            let next_open_res = if let Some(ne) = next_epoch {
                                market_discovery::fetch_polymarket_crypto_price(asset, ne).await
                            } else {
                                None
                            };

                            let final_p = official_res.as_ref().and_then(|r| r.close_price)
                                .or_else(|| next_open_res.as_ref().and_then(|r| r.open_price))
                                .or(current_spot_price)
                                .unwrap_or(open_p);

                            if open_p > 0.0 && final_p > 0.0 {
                                info!(
                                    "🔔 Sweeping historical un-settled market {}: Open={:.2}, Final={:.2}",
                                    exp_id, open_p, final_p
                                );
                                let _ = resolution
                                    .resolve_market(&exp_id, asset, open_p, final_p, now_ms)
                                    .await;
                            }
                        }
                    }

                    // 3. Ensure market is initialized for current window
                    if let Ok(mut market) = discovery
                        .ensure_active_market(asset, now_ms, current_spot_price)
                        .await
                    {
                        // Ensure open price and live current price are continuously synchronized with official Polymarket benchmark (目标价格 & 当前价格)
                        let (window_start_ms, _) = market_discovery::MarketDiscoveryEngine::calculate_5m_window(now_ms);
                        let start_epoch_sec = window_start_ms / 1000;

                        if let Some(pts) = market_discovery::fetch_polymarket_price_history(asset, start_epoch_sec).await {
                            if let Some(first_pt) = pts.first() {
                                if first_pt.value > 0.0 {
                                    let _ = discovery.update_open_price(asset, first_pt.value).await;
                                    market.open_price = Some(first_pt.value);
                                }
                            }
                            if let Some(last_pt) = pts.last() {
                                if last_pt.value > 0.0 {
                                    discovery.set_latest_poly_price(asset, last_pt.value, last_pt.timestamp);
                                }
                            }
                        } else if let Some(official_open) = market_discovery::fetch_candle_open_price(asset, start_epoch_sec).await {
                            let needs_update = match market.open_price {
                                Some(existing) => (existing - official_open).abs() > 0.001,
                                None => true,
                            };
                            if needs_update {
                                let _ = discovery.update_open_price(asset, official_open).await;
                                market.open_price = Some(official_open);
                            }
                        } else if market.open_price.is_none() {
                            if let Some(spot) = current_spot_price {
                                let _ = discovery.set_open_price(asset, spot).await;
                                market.open_price = Some(spot);
                            }
                        }

                        // 4. Fetch REAL Polymarket CLOB orderbooks for UP & DOWN tokens concurrently
                        let (up_book_res, down_book_res) = tokio::join!(
                            fetch_clob_orderbook(&http_client, &market.up_token_id),
                            fetch_clob_orderbook(&http_client, &market.down_token_id)
                        );

                        let mut has_real_book = false;
                        if let Some((bids, asks)) = up_book_res {
                            book_engine.update_book(&market.up_token_id, asset, MarketSide::Up, bids, asks, now_ms);
                            has_real_book = true;
                        }
                        if let Some((bids, asks)) = down_book_res {
                            book_engine.update_book(&market.down_token_id, asset, MarketSide::Down, bids, asks, now_ms);
                            has_real_book = true;
                        }

                        // Fallback to theoretical ladder ONLY if CLOB returned no depth
                        if !has_real_book {
                            let implied_p = match (market.open_price, current_spot_price) {
                                (Some(open), Some(curr)) if open > 0.0 && curr.is_finite() && open.is_finite() => {
                                    let diff_pct = ((curr - open) / open).clamp(-0.5, 0.5);
                                    (0.50 + diff_pct * 50.0).clamp(0.05, 0.95)
                                }
                                _ => 0.50,
                            };
                            book_engine.seed_fallback_ladder(asset, implied_p, now_ms);
                        }

                        // 4. Construct and broadcast PolymarketTick
                        if let Some(summary) = book_engine.get_market_summary(&market.id, asset) {
                            let tick = PolymarketTick {
                                market_id: market.id.clone(),
                                asset,
                                timestamp_ms: now_ms,
                                up_bid: summary.up_book.best_bid,
                                up_ask: summary.up_book.best_ask,
                                down_bid: summary.down_book.best_bid,
                                down_ask: summary.down_book.best_ask,
                                up_mid: summary.up_book.mid,
                                down_mid: summary.down_book.mid,
                                spread: summary.up_book.spread,
                                volume_24h: Some(summary.up_book.total_bid_depth_usdc + summary.up_book.total_ask_depth_usdc),
                                liquidity: Some(summary.up_book.total_bid_depth_usdc + summary.down_book.total_bid_depth_usdc),
                            };
                            let _ = poly_tx.send(tick);
                        }
                    }
                }
            }
        });
    }

    pub fn discovery(&self) -> Arc<MarketDiscoveryEngine> {
        self.discovery.clone()
    }

    pub fn book_engine(&self) -> Arc<PolymarketBookEngine> {
        self.book_engine.clone()
    }

    pub fn resolution(&self) -> Arc<ResolutionEngine> {
        self.resolution.clone()
    }

    pub fn subscribe_ticks(&self) -> broadcast::Receiver<PolymarketTick> {
        self.poly_tick_tx.subscribe()
    }

    pub fn subscribe_resolutions(&self) -> broadcast::Receiver<MarketResolvedEvent> {
        self.resolution.subscribe_resolutions()
    }
}
