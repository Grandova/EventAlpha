use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

use crate::collector::CollectorManager;
use crate::polymarket::market_discovery::MarketDiscoveryEngine;
use crate::types::{Asset, Exchange};

const MAX_HISTORY_SECS: i64 = 120;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositePriceSnapshot {
    pub asset: Asset,
    pub timestamp_ms: i64,
    pub composite_price: f64,
    pub open_price: Option<f64>,
    pub distance_from_open: Option<f64>,
    pub distance_percent: Option<f64>,
    pub distance_to_vol_ratio: Option<f64>,
    pub realized_vol_5s: f64,
    pub realized_vol_10s: f64,
    pub realized_vol_30s: f64,
    pub realized_vol_60s: f64,
    pub return_1s: f64,
    pub return_3s: f64,
    pub return_5s: f64,
    pub return_10s: f64,
    pub return_30s: f64,
    pub return_60s: f64,
    pub spread_binance_okx: Option<f64>,
    pub spread_binance_bybit: Option<f64>,
    pub spread_binance_coinbase: Option<f64>,
    pub leading_exchange: Option<Exchange>,
    pub active_exchanges_count: usize,
}

#[derive(Clone)]
pub struct CompositePriceEngine {
    collector: Arc<CollectorManager>,
    discovery: Arc<MarketDiscoveryEngine>,
    // Rolling history of (timestamp_ms, composite_price) per asset
    price_history: Arc<DashMap<Asset, VecDeque<(i64, f64)>>>,
    // Latest snapshot per asset
    latest_snapshots: Arc<DashMap<Asset, CompositePriceSnapshot>>,
    snapshot_tx: broadcast::Sender<CompositePriceSnapshot>,
}

impl CompositePriceEngine {
    pub fn new(collector: Arc<CollectorManager>, discovery: Arc<MarketDiscoveryEngine>) -> Self {
        let (snapshot_tx, _) = broadcast::channel(1024);
        Self {
            collector,
            discovery,
            price_history: Arc::new(DashMap::new()),
            latest_snapshots: Arc::new(DashMap::new()),
            snapshot_tx,
        }
    }

    /// Background loop calculating composite prices every 100ms
    pub fn start(&self) {
        info!("Starting CompositePriceEngine with multi-exchange weighting and lead/lag analysis...");

        let engine = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_millis(100));
            let assets = [Asset::BTC, Asset::ETH, Asset::SOL];

            loop {
                ticker.tick().await;
                let now_ms = Utc::now().timestamp_millis();

                for &asset in &assets {
                    if let Some(snapshot) = engine.calculate_snapshot(asset, now_ms) {
                        engine.latest_snapshots.insert(asset, snapshot.clone());
                        let _ = engine.snapshot_tx.send(snapshot);
                    }
                }
            }
        });
    }

    /// Calculate real-time CompositePriceSnapshot for an asset
    pub fn calculate_snapshot(&self, asset: Asset, now_ms: i64) -> Option<CompositePriceSnapshot> {
        let target_exchanges = [
            (Exchange::Binance, 0.40f64),
            (Exchange::Okx, 0.25f64),
            (Exchange::Coinbase, 0.25f64),
            (Exchange::Bybit, 0.10f64),
        ];

        let mut available_prices: Vec<(Exchange, f64, f64)> = Vec::new(); // (Exchange, mid_price, base_weight)
        let mut exchange_mids: HashMap<Exchange, f64> = HashMap::new();

        for &(ex, base_weight) in &target_exchanges {
            if !self.collector.is_stale(ex, asset) {
                if let Some(tick) = self.collector.get_latest_tick(ex, asset) {
                    if tick.mid > 0.0 {
                        available_prices.push((ex, tick.mid, base_weight));
                        exchange_mids.insert(ex, tick.mid);
                    }
                }
            }
        }

        if available_prices.is_empty() {
            // No fresh ticks available from any exchange (Fail-Closed)
            return None;
        }

        // Re-normalize weights among fresh exchanges
        let total_weight: f64 = available_prices.iter().map(|(_, _, w)| *w).sum();
        let composite_price: f64 = available_prices
            .iter()
            .map(|(_, price, w)| price * (w / total_weight))
            .sum();

        // Update rolling history
        let mut hist = self.price_history.entry(asset).or_default();
        hist.push_back((now_ms, composite_price));
        let cutoff_ms = now_ms - (MAX_HISTORY_SECS * 1000);
        while let Some(&(ts, _)) = hist.front() {
            if ts < cutoff_ms {
                hist.pop_front();
            } else {
                break;
            }
        }

        // Compute multi-horizon returns
        let return_1s = Self::calc_return(&hist, now_ms, 1.0);
        let return_3s = Self::calc_return(&hist, now_ms, 3.0);
        let return_5s = Self::calc_return(&hist, now_ms, 5.0);
        let return_10s = Self::calc_return(&hist, now_ms, 10.0);
        let return_30s = Self::calc_return(&hist, now_ms, 30.0);
        let return_60s = Self::calc_return(&hist, now_ms, 60.0);

        // Compute multi-horizon realized volatility (standard deviation of returns)
        let realized_vol_5s = Self::calc_realized_vol(&hist, now_ms, 5.0);
        let realized_vol_10s = Self::calc_realized_vol(&hist, now_ms, 10.0);
        let realized_vol_30s = Self::calc_realized_vol(&hist, now_ms, 30.0);
        let realized_vol_60s = Self::calc_realized_vol(&hist, now_ms, 60.0);

        // Cross-exchange spreads against Binance
        let binance_mid = exchange_mids.get(&Exchange::Binance).copied();
        let spread_binance_okx = match (binance_mid, exchange_mids.get(&Exchange::Okx)) {
            (Some(b), Some(o)) => Some(b - o),
            _ => None,
        };
        let spread_binance_bybit = match (binance_mid, exchange_mids.get(&Exchange::Bybit)) {
            (Some(b), Some(by)) => Some(b - by),
            _ => None,
        };
        let spread_binance_coinbase = match (binance_mid, exchange_mids.get(&Exchange::Coinbase)) {
            (Some(b), Some(c)) => Some(b - c),
            _ => None,
        };

        // Leading exchange identification: exchange with fastest response matching trend
        let leading_exchange = Self::identify_leading_exchange(&self.collector, asset);

        // Active 5M round benchmark distance
        let active_market = self.discovery.get_active_market(asset);
        let open_price = active_market.and_then(|m| m.open_price);

        let (distance_from_open, distance_percent, distance_to_vol_ratio) = match open_price {
            Some(open) if open > 0.0 => {
                let dist = composite_price - open;
                let pct = (dist / open) * 100.0;
                let vol = realized_vol_60s.max(0.0001);
                let ratio = (pct / 100.0) / vol;
                (Some(dist), Some(pct), Some(ratio))
            }
            _ => (None, None, None),
        };

        Some(CompositePriceSnapshot {
            asset,
            timestamp_ms: now_ms,
            composite_price,
            open_price,
            distance_from_open,
            distance_percent,
            distance_to_vol_ratio,
            realized_vol_5s,
            realized_vol_10s,
            realized_vol_30s,
            realized_vol_60s,
            return_1s,
            return_3s,
            return_5s,
            return_10s,
            return_30s,
            return_60s,
            spread_binance_okx,
            spread_binance_bybit,
            spread_binance_coinbase,
            leading_exchange,
            active_exchanges_count: available_prices.len(),
        })
    }

    /// Calculate return over `window_secs`
    pub fn calc_return(history: &VecDeque<(i64, f64)>, now_ms: i64, window_secs: f64) -> f64 {
        if history.len() < 2 {
            return 0.0;
        }

        let target_ts = now_ms - (window_secs * 1000.0) as i64;
        let latest_price = history.back().map(|(_, p)| *p).unwrap_or(0.0);

        // Find price closest to target_ts
        let past_price = history
            .iter()
            .take_while(|(ts, _)| *ts <= target_ts)
            .last()
            .map(|(_, p)| *p)
            .or_else(|| history.front().map(|(_, p)| *p))
            .unwrap_or(latest_price);

        if past_price > 0.0 {
            (latest_price - past_price) / past_price
        } else {
            0.0
        }
    }

    /// Calculate realized volatility (sample standard deviation of 1-second log returns)
    pub fn calc_realized_vol(history: &VecDeque<(i64, f64)>, now_ms: i64, window_secs: f64) -> f64 {
        let cutoff_ms = now_ms - (window_secs * 1000.0) as i64;
        let window_samples: Vec<f64> = history
            .iter()
            .filter(|(ts, _)| *ts >= cutoff_ms)
            .map(|(_, p)| *p)
            .collect();

        if window_samples.len() < 3 {
            return 0.0;
        }

        let mut log_returns = Vec::with_capacity(window_samples.len() - 1);
        for i in 1..window_samples.len() {
            let prev = window_samples[i - 1];
            let curr = window_samples[i];
            if prev > 0.0 && curr > 0.0 {
                log_returns.push((curr / prev).ln());
            }
        }

        if log_returns.is_empty() {
            return 0.0;
        }

        let mean: f64 = log_returns.iter().sum::<f64>() / log_returns.len() as f64;
        let variance: f64 = log_returns
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / (log_returns.len() as f64 - 1.0).max(1.0);

        variance.sqrt()
    }

    /// Identify leading exchange based on latency and momentum
    fn identify_leading_exchange(collector: &CollectorManager, asset: Asset) -> Option<Exchange> {
        let exchanges = [Exchange::Binance, Exchange::Okx, Exchange::Coinbase, Exchange::Bybit];
        let mut min_latency = i64::MAX;
        let mut leading = None;

        for &ex in &exchanges {
            if !collector.is_stale(ex, asset) {
                if let Some(tick) = collector.get_latest_tick(ex, asset) {
                    if tick.latency_ms < min_latency {
                        min_latency = tick.latency_ms;
                        leading = Some(ex);
                    }
                }
            }
        }

        leading
    }

    pub fn get_latest_snapshot(&self, asset: Asset) -> Option<CompositePriceSnapshot> {
        self.latest_snapshots.get(&asset).map(|s| s.clone())
    }

    pub fn get_all_snapshots(&self) -> HashMap<String, CompositePriceSnapshot> {
        let mut map = HashMap::new();
        for entry in self.latest_snapshots.iter() {
            map.insert(entry.key().to_string(), entry.value().clone());
        }
        map
    }

    pub fn record_snapshot(&self, snapshot: CompositePriceSnapshot) {
        self.latest_snapshots.insert(snapshot.asset, snapshot.clone());
        let _ = self.snapshot_tx.send(snapshot);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CompositePriceSnapshot> {
        self.snapshot_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_return() {
        let mut hist = VecDeque::new();
        let now = 10000;
        hist.push_back((now - 5000, 100.0));
        hist.push_back((now - 3000, 102.0));
        hist.push_back((now - 1000, 105.0));
        hist.push_back((now, 106.0));

        let ret_3s = CompositePriceEngine::calc_return(&hist, now, 3.0);
        // past price ~ 102.0, latest = 106.0 => (106 - 102) / 102 = ~0.0392
        assert!((ret_3s - 0.0392).abs() < 0.005);
    }

    #[test]
    fn test_calc_realized_vol() {
        let mut hist = VecDeque::new();
        let now = 10000;
        hist.push_back((now - 4000, 100.0));
        hist.push_back((now - 3000, 101.0));
        hist.push_back((now - 2000, 99.5));
        hist.push_back((now - 1000, 100.5));
        hist.push_back((now, 102.0));

        let vol = CompositePriceEngine::calc_realized_vol(&hist, now, 5.0);
        assert!(vol > 0.0);
    }
}
