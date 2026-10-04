pub mod flow;
pub mod snapshot;

use chrono::Utc;
use dashmap::DashMap;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

use crate::collector::CollectorManager;
use crate::composite::CompositePriceEngine;
use crate::db::Database;
use crate::polymarket::PolymarketManager;
use crate::types::Asset;
pub use flow::TradeFlowTracker;
pub use snapshot::{FeatureSnapshot, FEATURE_NAMES};

const MAX_PRICE_HISTORY_SECS: i64 = 60;

#[derive(Clone)]
pub struct FeatureEngine {
    composite: Arc<CompositePriceEngine>,
    polymarket: Arc<PolymarketManager>,
    flow_tracker: Arc<TradeFlowTracker>,
    collector: Arc<CollectorManager>,
    db: Arc<Database>,
    latest_features: Arc<DashMap<Asset, FeatureSnapshot>>,
    price_history: Arc<DashMap<Asset, VecDeque<(i64, f64)>>>,
    feature_tx: broadcast::Sender<FeatureSnapshot>,
}

impl FeatureEngine {
    pub fn new(
        composite: Arc<CompositePriceEngine>,
        polymarket: Arc<PolymarketManager>,
        collector: Arc<CollectorManager>,
        db: Arc<Database>,
    ) -> Self {
        let (feature_tx, _) = broadcast::channel(1024);
        Self {
            composite,
            polymarket,
            flow_tracker: Arc::new(TradeFlowTracker::new()),
            collector,
            db,
            latest_features: Arc::new(DashMap::new()),
            price_history: Arc::new(DashMap::new()),
            feature_tx,
        }
    }

    pub fn flow_tracker(&self) -> Arc<TradeFlowTracker> {
        self.flow_tracker.clone()
    }

    /// Start trade subscriber, periodic feature calculations, and DB persistence
    pub fn start(&self) {
        info!("Starting FeatureEngine with real-time OBI, CVD, Velocity, and Distance metrics...");

        // 1. Ingest trade flow from collectors
        let mut trade_rx = self.collector.subscribe_trade_ticks();
        let tracker = self.flow_tracker.clone();
        tokio::spawn(async move {
            while let Ok(tick) = trade_rx.recv().await {
                tracker.record_trade(&tick);
            }
        });

        // 2. Periodic feature calculation loop (every 100ms)
        let engine = self.clone();
        let db_clone = self.db.clone();
        let mut feature_sub = self.feature_tx.subscribe();

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_millis(100));
            let assets = [Asset::BTC, Asset::ETH, Asset::SOL];

            loop {
                ticker.tick().await;
                let now_ms = Utc::now().timestamp_millis();

                for &asset in &assets {
                    if let Some(features) = engine.calculate_features(asset, now_ms) {
                        engine.latest_features.insert(asset, features.clone());
                        let _ = engine.feature_tx.send(features);
                    }
                }
            }
        });

        // 3. Batch DB persistence task for feature snapshots (sampling every 1 sec to avoid excessive DB write churn)
        tokio::spawn(async move {
            let mut buffer = Vec::with_capacity(30);
            let mut flush_timer = tokio::time::interval(std::time::Duration::from_millis(1000));

            loop {
                tokio::select! {
                    Ok(feat) = feature_sub.recv() => {
                        buffer.push(feat);
                        if buffer.len() >= 30 {
                            Self::flush_features_to_db(&db_clone, &buffer).await;
                            buffer.clear();
                        }
                    }
                    _ = flush_timer.tick() => {
                        if !buffer.is_empty() {
                            Self::flush_features_to_db(&db_clone, &buffer).await;
                            buffer.clear();
                        }
                    }
                }
            }
        });
    }

    /// Calculate complete FeatureSnapshot for a specific asset at now_ms
    pub fn calculate_features(&self, asset: Asset, now_ms: i64) -> Option<FeatureSnapshot> {
        // 1. Must have valid fresh composite price
        let comp = self.composite.calculate_snapshot(asset, now_ms)?;

        // Update price history for velocity & acceleration
        let mut hist = self.price_history.entry(asset).or_default();
        hist.push_back((now_ms, comp.composite_price));
        let cutoff = now_ms - (MAX_PRICE_HISTORY_SECS * 1000);
        while let Some(&(ts, _)) = hist.front() {
            if ts < cutoff {
                hist.pop_front();
            } else {
                break;
            }
        }

        // Velocity over 5s: (P_now - P_5s_ago) / 5.0
        let price_5s_ago = Self::get_past_price(&hist, now_ms, 5.0).unwrap_or(comp.composite_price);
        let velocity_5s = (comp.composite_price - price_5s_ago) / 5.0;

        // Velocity over 15s: (P_now - P_15s_ago) / 15.0
        let price_15s_ago = Self::get_past_price(&hist, now_ms, 15.0).unwrap_or(comp.composite_price);
        let velocity_15s = (comp.composite_price - price_15s_ago) / 15.0;

        // Acceleration = (Velocity_5s - Velocity_15s) / 10.0
        let acceleration_5s_15s = (velocity_5s - velocity_15s) / 10.0;

        // 2. Active 5M round metrics
        let active_market = self.polymarket.discovery().get_active_market(asset);
        let market_id = active_market
            .as_ref()
            .map(|m| m.id.clone())
            .unwrap_or_else(|| format!("{}-5M-DEFAULT", asset));

        let remaining_seconds = active_market
            .as_ref()
            .map(|m| m.remaining_seconds(now_ms))
            .unwrap_or(150) as f64;
        let elapsed_seconds = (300.0 - remaining_seconds).clamp(0.0, 300.0);
        let time_decay_factor = (remaining_seconds.max(0.0) / 300.0).sqrt();

        // 3. Polymarket OrderBook Microstructure
        let book_summary = self
            .polymarket
            .book_engine()
            .get_market_summary(&market_id, asset);

        let (
            poly_obi_top5,
            poly_obi_top10,
            poly_obi_top20,
            poly_spread,
            poly_total_liquidity,
            poly_implied_prob,
        ) = match book_summary {
            Some(s) => (
                s.up_book.obi_top5,
                s.up_book.obi_top10,
                s.up_book.obi_top20,
                s.up_book.spread.unwrap_or(0.0),
                s.up_book.total_bid_depth_usdc + s.down_book.total_bid_depth_usdc,
                s.implied_prob_up,
            ),
            None => (0.0, 0.0, 0.0, 0.0, 0.0, 0.50),
        };

        // 4. Trade flow CVD and imbalances
        let cvd_5s = self.flow_tracker.calc_cvd(asset, now_ms, 5.0);
        let cvd_15s = self.flow_tracker.calc_cvd(asset, now_ms, 15.0);
        let cvd_30s = self.flow_tracker.calc_cvd(asset, now_ms, 30.0);
        let cvd_60s = self.flow_tracker.calc_cvd(asset, now_ms, 60.0);

        let trade_imbalance_5s = self.flow_tracker.calc_imbalance(asset, now_ms, 5.0);
        let trade_imbalance_15s = self.flow_tracker.calc_imbalance(asset, now_ms, 15.0);
        let trade_imbalance_30s = self.flow_tracker.calc_imbalance(asset, now_ms, 30.0);
        let trade_imbalance_60s = self.flow_tracker.calc_imbalance(asset, now_ms, 60.0);

        Some(FeatureSnapshot {
            asset,
            market_id,
            timestamp_ms: now_ms,
            composite_price: comp.composite_price,
            return_1s: comp.return_1s,
            return_3s: comp.return_3s,
            return_5s: comp.return_5s,
            return_10s: comp.return_10s,
            return_30s: comp.return_30s,
            return_60s: comp.return_60s,
            realized_vol_5s: comp.realized_vol_5s,
            realized_vol_10s: comp.realized_vol_10s,
            realized_vol_30s: comp.realized_vol_30s,
            realized_vol_60s: comp.realized_vol_60s,
            velocity_5s,
            velocity_15s,
            acceleration_5s_15s,
            distance_from_open: comp.distance_from_open.unwrap_or(0.0),
            distance_percent: comp.distance_percent.unwrap_or(0.0),
            distance_to_vol_ratio: comp.distance_to_vol_ratio.unwrap_or(0.0),
            remaining_seconds,
            elapsed_seconds,
            time_decay_factor,
            spread_binance_okx: comp.spread_binance_okx.unwrap_or(0.0),
            spread_binance_bybit: comp.spread_binance_bybit.unwrap_or(0.0),
            spread_binance_coinbase: comp.spread_binance_coinbase.unwrap_or(0.0),
            cvd_5s,
            cvd_15s,
            cvd_30s,
            cvd_60s,
            trade_imbalance_5s,
            trade_imbalance_15s,
            trade_imbalance_30s,
            trade_imbalance_60s,
            poly_obi_top5,
            poly_obi_top10,
            poly_obi_top20,
            poly_spread,
            poly_total_liquidity,
            poly_implied_prob,
        })
    }

    fn get_past_price(history: &VecDeque<(i64, f64)>, now_ms: i64, window_secs: f64) -> Option<f64> {
        let target = now_ms - (window_secs * 1000.0) as i64;
        history
            .iter()
            .take_while(|(ts, _)| *ts <= target)
            .last()
            .map(|(_, p)| *p)
            .or_else(|| history.front().map(|(_, p)| *p))
    }

    async fn flush_features_to_db(db: &Database, buffer: &[FeatureSnapshot]) {
        if buffer.is_empty() {
            return;
        }
        let mut tx = match db.pool().begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::warn!("Failed to begin transaction for features flush: {:?}", e);
                return;
            }
        };

        for snap in buffer {
            let json_vec = snap.to_json();
            let now = Utc::now().timestamp_millis();
            let _ = sqlx::query(
                r#"
                INSERT INTO features (market_id, timestamp, feature_name, feature_vector_json, created_at)
                VALUES (?, ?, 'snapshot_v1', ?, ?)
                "#,
            )
            .bind(&snap.market_id)
            .bind(snap.timestamp_ms)
            .bind(&json_vec)
            .bind(now)
            .execute(&mut *tx)
            .await;
        }

        if let Err(e) = tx.commit().await {
            tracing::warn!("Failed to commit transaction for features flush: {:?}", e);
        }
    }

    pub fn get_latest_features(&self, asset: Asset) -> Option<FeatureSnapshot> {
        self.latest_features.get(&asset).map(|s| s.clone())
    }

    pub fn get_all_latest_features(&self) -> HashMap<String, FeatureSnapshot> {
        let mut map = HashMap::new();
        for entry in self.latest_features.iter() {
            map.insert(entry.key().to_string(), entry.value().clone());
        }
        map
    }

    pub fn subscribe(&self) -> broadcast::Receiver<FeatureSnapshot> {
        self.feature_tx.subscribe()
    }
}
