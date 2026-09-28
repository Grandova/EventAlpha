pub mod binance;
pub mod bybit;
pub mod coinbase;
pub mod freshness;
pub mod okx;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

use crate::config::AppConfig;
use crate::db::Database;
use crate::types::{Asset, Exchange, MarketTick, TradeTick};
use freshness::{FreshnessReport, FreshnessTracker};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceSummary {
    pub exchange: Exchange,
    pub asset: Asset,
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub last: f64,
    pub latency_ms: i64,
    pub timestamp_ms: i64,
}

#[derive(Clone)]
pub struct CollectorManager {
    freshness: FreshnessTracker,
    latest_ticks: Arc<DashMap<(Exchange, Asset), MarketTick>>,
    market_tx: broadcast::Sender<MarketTick>,
    trade_tx: broadcast::Sender<TradeTick>,
}

impl CollectorManager {
    pub fn new(config: &AppConfig) -> Self {
        let (market_tx, _) = broadcast::channel::<MarketTick>(4096);
        let (trade_tx, _) = broadcast::channel::<TradeTick>(4096);
        let freshness = FreshnessTracker::new(config.freshness.stale_timeout_ms);
        let latest_ticks = Arc::new(DashMap::new());

        Self {
            freshness,
            latest_ticks,
            market_tx,
            trade_tx,
        }
    }

    /// Spawn background collection tasks for all 4 exchanges
    pub fn start(&self, db: Arc<Database>) {
        info!("Starting CollectorManager with Binance, OKX, Bybit, Coinbase...");

        // 1. Spawn cache updater task
        let mut rx = self.market_tx.subscribe();
        let cache_ref = self.latest_ticks.clone();
        tokio::spawn(async move {
            while let Ok(tick) = rx.recv().await {
                cache_ref.insert((tick.exchange, tick.asset), tick);
            }
        });

        // 2. Spawn DB batch persistence task (asynchronously writing ticks in chunks)
        let mut db_rx = self.market_tx.subscribe();
        let db_clone = db.clone();
        tokio::spawn(async move {
            let mut buffer = Vec::with_capacity(50);
            let mut flush_timer = tokio::time::interval(std::time::Duration::from_millis(500));

            loop {
                tokio::select! {
                    Ok(tick) = db_rx.recv() => {
                        buffer.push(tick);
                        if buffer.len() >= 50 {
                            Self::flush_ticks_to_db(&db_clone, &buffer).await;
                            buffer.clear();
                        }
                    }
                    _ = flush_timer.tick() => {
                        if !buffer.is_empty() {
                            Self::flush_ticks_to_db(&db_clone, &buffer).await;
                            buffer.clear();
                        }
                    }
                }
            }
        });

        // 3. Spawn individual exchange collectors
        tokio::spawn(binance::run_binance_collector(
            self.market_tx.clone(),
            self.trade_tx.clone(),
            self.freshness.clone(),
        ));

        tokio::spawn(okx::run_okx_collector(
            self.market_tx.clone(),
            self.trade_tx.clone(),
            self.freshness.clone(),
        ));

        tokio::spawn(bybit::run_bybit_collector(
            self.market_tx.clone(),
            self.trade_tx.clone(),
            self.freshness.clone(),
        ));

        tokio::spawn(coinbase::run_coinbase_collector(
            self.market_tx.clone(),
            self.trade_tx.clone(),
            self.freshness.clone(),
        ));
    }

    async fn flush_ticks_to_db(db: &Database, ticks: &[MarketTick]) {
        for tick in ticks {
            let ex_str = tick.exchange.to_string();
            let _ = sqlx::query(
                r#"
                INSERT INTO exchange_ticks (exchange, symbol, timestamp, bid, ask, mid, last, volume, latency_ms, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&ex_str)
            .bind(&tick.symbol)
            .bind(tick.exchange_timestamp_ms)
            .bind(tick.bid)
            .bind(tick.ask)
            .bind(tick.mid)
            .bind(tick.last)
            .bind(tick.volume_24h)
            .bind(tick.latency_ms)
            .bind(tick.receive_timestamp_ms)
            .execute(db.pool())
            .await;
        }
    }

    pub fn subscribe_market_ticks(&self) -> broadcast::Receiver<MarketTick> {
        self.market_tx.subscribe()
    }

    pub fn subscribe_trade_ticks(&self) -> broadcast::Receiver<TradeTick> {
        self.trade_tx.subscribe()
    }

    pub fn get_freshness_report(&self) -> FreshnessReport {
        self.freshness.get_report()
    }

    pub fn is_stale(&self, exchange: Exchange, asset: Asset) -> bool {
        self.freshness.is_stale(exchange, asset)
    }

    pub fn get_latest_tick(&self, exchange: Exchange, asset: Asset) -> Option<MarketTick> {
        self.latest_ticks.get(&(exchange, asset)).map(|t| t.clone())
    }

    /// Retrieve grouped price matrix across all assets and exchanges
    pub fn get_prices(&self) -> HashMap<String, HashMap<String, PriceSummary>> {
        let mut result: HashMap<String, HashMap<String, PriceSummary>> = HashMap::new();

        for entry in self.latest_ticks.iter() {
            let tick = entry.value();
            let asset_key = tick.asset.to_string();
            let ex_key = tick.exchange.to_string();

            let summary = PriceSummary {
                exchange: tick.exchange,
                asset: tick.asset,
                symbol: tick.symbol.clone(),
                bid: tick.bid,
                ask: tick.ask,
                mid: tick.mid,
                last: tick.last,
                latency_ms: tick.latency_ms,
                timestamp_ms: tick.receive_timestamp_ms,
            };

            result.entry(asset_key).or_default().insert(ex_key, summary);
        }

        result
    }

    pub fn freshness_tracker(&self) -> &FreshnessTracker {
        &self.freshness
    }

    pub fn record_test_tick(&self, tick: MarketTick) {
        self.latest_ticks.insert((tick.exchange, tick.asset), tick);
    }
}
