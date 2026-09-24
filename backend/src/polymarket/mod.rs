pub mod clob_client;
pub mod market_discovery;
pub mod orderbook;
pub mod resolution;

use chrono::Utc;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

use crate::collector::CollectorManager;
use crate::config::AppConfig;
use crate::db::Database;
use crate::types::{Asset, PolymarketTick};
use market_discovery::MarketDiscoveryEngine;
use orderbook::PolymarketBookEngine;
use resolution::{MarketResolvedEvent, ResolutionEngine};

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
                    // Get current reference spot price from collectors (e.g. Binance / OKX)
                    let current_spot_price = collector
                        .get_latest_tick(crate::types::Exchange::Binance, asset)
                        .or_else(|| collector.get_latest_tick(crate::types::Exchange::Okx, asset))
                        .map(|t| t.mid);

                    // 1. Ensure market is initialized for current window
                    if let Ok(mut market) = discovery
                        .ensure_active_market(asset, now_ms, current_spot_price)
                        .await
                    {
                        // Record open price if spot price just became available
                        if market.open_price.is_none() {
                            if let Some(spot) = current_spot_price {
                                let _ = discovery.set_open_price(asset, spot).await;
                                market.open_price = Some(spot);
                            }
                        }

                        // 2. Check if current market expired => Trigger settlement
                        if market.is_expired(now_ms) {
                            if let (Some(open_p), Some(final_p)) = (market.open_price, current_spot_price) {
                                let _ = resolution
                                    .resolve_market(&market.id, asset, open_p, final_p, now_ms)
                                    .await;

                                // Roll forward to new active market round
                                let _ = discovery.ensure_active_market(asset, now_ms, Some(final_p)).await;
                            }
                        }

                        // 3. Maintain / seed orderbook implied probabilities
                        let implied_p = match (market.open_price, current_spot_price) {
                            (Some(open), Some(curr)) => {
                                // Heuristic implied probability based on distance and volatility
                                let diff_pct = (curr - open) / open;
                                (0.50 + diff_pct * 50.0).clamp(0.05, 0.95)
                            }
                            _ => 0.50,
                        };

                        // Ensure orderbook ladder is populated
                        if book_engine.get_market_summary(&market.id, asset).is_none() {
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

    pub fn discovery(&self) -> &MarketDiscoveryEngine {
        &self.discovery
    }

    pub fn book_engine(&self) -> &PolymarketBookEngine {
        &self.book_engine
    }

    pub fn resolution(&self) -> &ResolutionEngine {
        &self.resolution
    }

    pub fn subscribe_ticks(&self) -> broadcast::Receiver<PolymarketTick> {
        self.poly_tick_tx.subscribe()
    }

    pub fn subscribe_resolutions(&self) -> broadcast::Receiver<MarketResolvedEvent> {
        self.resolution.subscribe_resolutions()
    }
}
