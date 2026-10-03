use serde::{Deserialize, Serialize};

use crate::types::Asset;

pub const FEATURE_NAMES: &[&str] = &[
    "composite_price",
    "return_1s",
    "return_3s",
    "return_5s",
    "return_10s",
    "return_30s",
    "return_60s",
    "realized_vol_5s",
    "realized_vol_10s",
    "realized_vol_30s",
    "realized_vol_60s",
    "velocity_5s",
    "velocity_15s",
    "acceleration_5s_15s",
    "distance_from_open",
    "distance_percent",
    "distance_to_vol_ratio",
    "remaining_seconds",
    "elapsed_seconds",
    "time_decay_factor",
    "spread_binance_okx",
    "spread_binance_bybit",
    "spread_binance_coinbase",
    "cvd_5s",
    "cvd_15s",
    "cvd_30s",
    "cvd_60s",
    "trade_imbalance_5s",
    "trade_imbalance_15s",
    "trade_imbalance_30s",
    "trade_imbalance_60s",
    "poly_obi_top5",
    "poly_obi_top10",
    "poly_obi_top20",
    "poly_spread",
    "poly_total_liquidity",
    "poly_implied_prob",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSnapshot {
    pub asset: Asset,
    pub market_id: String,
    pub timestamp_ms: i64,

    // CEX Price Dynamics & Returns
    pub composite_price: f64,
    pub return_1s: f64,
    pub return_3s: f64,
    pub return_5s: f64,
    pub return_10s: f64,
    pub return_30s: f64,
    pub return_60s: f64,

    // Realized Volatility
    pub realized_vol_5s: f64,
    pub realized_vol_10s: f64,
    pub realized_vol_30s: f64,
    pub realized_vol_60s: f64,

    // Velocity & Acceleration
    pub velocity_5s: f64,
    pub velocity_15s: f64,
    pub acceleration_5s_15s: f64,

    // 5M Round Relative Metrics
    pub distance_from_open: f64,
    pub distance_percent: f64,
    pub distance_to_vol_ratio: f64,
    pub remaining_seconds: f64,
    pub elapsed_seconds: f64,
    pub time_decay_factor: f64,

    // Cross-Exchange Microstructure
    pub spread_binance_okx: f64,
    pub spread_binance_bybit: f64,
    pub spread_binance_coinbase: f64,

    // Trade Flow & Cumulative Volume Delta (CVD)
    pub cvd_5s: f64,
    pub cvd_15s: f64,
    pub cvd_30s: f64,
    pub cvd_60s: f64,

    // Aggressive Trade Imbalance
    pub trade_imbalance_5s: f64,
    pub trade_imbalance_15s: f64,
    pub trade_imbalance_30s: f64,
    pub trade_imbalance_60s: f64,

    // Polymarket OrderBook Microstructure
    pub poly_obi_top5: f64,
    pub poly_obi_top10: f64,
    pub poly_obi_top20: f64,
    pub poly_spread: f64,
    pub poly_total_liquidity: f64,
    pub poly_implied_prob: f64,
}

impl FeatureSnapshot {
    pub fn new(asset: Asset, market_id: &str, timestamp_ms: i64) -> Self {
        Self {
            asset,
            market_id: market_id.to_string(),
            timestamp_ms,
            composite_price: 0.0,
            return_1s: 0.0,
            return_3s: 0.0,
            return_5s: 0.0,
            return_10s: 0.0,
            return_30s: 0.0,
            return_60s: 0.0,
            realized_vol_5s: 0.0,
            realized_vol_10s: 0.0,
            realized_vol_30s: 0.0,
            realized_vol_60s: 0.0,
            velocity_5s: 0.0,
            velocity_15s: 0.0,
            acceleration_5s_15s: 0.0,
            distance_from_open: 0.0,
            distance_percent: 0.0,
            distance_to_vol_ratio: 0.0,
            remaining_seconds: 150.0,
            elapsed_seconds: 150.0,
            time_decay_factor: 0.7,
            spread_binance_okx: 0.0,
            spread_binance_bybit: 0.0,
            spread_binance_coinbase: 0.0,
            cvd_5s: 0.0,
            cvd_15s: 0.0,
            cvd_30s: 0.0,
            cvd_60s: 0.0,
            trade_imbalance_5s: 0.0,
            trade_imbalance_15s: 0.0,
            trade_imbalance_30s: 0.0,
            trade_imbalance_60s: 0.0,
            poly_obi_top5: 0.0,
            poly_obi_top10: 0.0,
            poly_obi_top20: 0.0,
            poly_spread: 0.01,
            poly_total_liquidity: 1000.0,
            poly_implied_prob: 0.5,
        }
    }

    pub fn feature_names() -> &'static [&'static str] {
        FEATURE_NAMES
    }

    /// Converts all numerical features to an ordered vector for ML models / inference
    pub fn to_vector(&self) -> Vec<f64> {
        vec![
            self.composite_price,
            self.return_1s,
            self.return_3s,
            self.return_5s,
            self.return_10s,
            self.return_30s,
            self.return_60s,
            self.realized_vol_5s,
            self.realized_vol_10s,
            self.realized_vol_30s,
            self.realized_vol_60s,
            self.velocity_5s,
            self.velocity_15s,
            self.acceleration_5s_15s,
            self.distance_from_open,
            self.distance_percent,
            self.distance_to_vol_ratio,
            self.remaining_seconds,
            self.elapsed_seconds,
            self.time_decay_factor,
            self.spread_binance_okx,
            self.spread_binance_bybit,
            self.spread_binance_coinbase,
            self.cvd_5s,
            self.cvd_15s,
            self.cvd_30s,
            self.cvd_60s,
            self.trade_imbalance_5s,
            self.trade_imbalance_15s,
            self.trade_imbalance_30s,
            self.trade_imbalance_60s,
            self.poly_obi_top5,
            self.poly_obi_top10,
            self.poly_obi_top20,
            self.poly_spread,
            self.poly_total_liquidity,
            self.poly_implied_prob,
        ]
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_vector_dimension_match() {
        let snap = FeatureSnapshot {
            asset: Asset::BTC,
            market_id: "test-mkt".to_string(),
            timestamp_ms: 1000,
            composite_price: 65000.0,
            return_1s: 0.001,
            return_3s: 0.002,
            return_5s: 0.003,
            return_10s: 0.004,
            return_30s: 0.005,
            return_60s: 0.006,
            realized_vol_5s: 0.0005,
            realized_vol_10s: 0.0006,
            realized_vol_30s: 0.0007,
            realized_vol_60s: 0.0008,
            velocity_5s: 1.2,
            velocity_15s: 0.8,
            acceleration_5s_15s: 0.04,
            distance_from_open: 50.0,
            distance_percent: 0.076,
            distance_to_vol_ratio: 0.95,
            remaining_seconds: 180.0,
            elapsed_seconds: 120.0,
            time_decay_factor: 0.774,
            spread_binance_okx: 0.5,
            spread_binance_bybit: -0.2,
            spread_binance_coinbase: 0.1,
            cvd_5s: 10.5,
            cvd_15s: 25.0,
            cvd_30s: 40.0,
            cvd_60s: 55.0,
            trade_imbalance_5s: 0.25,
            trade_imbalance_15s: 0.30,
            trade_imbalance_30s: 0.20,
            trade_imbalance_60s: 0.15,
            poly_obi_top5: 0.12,
            poly_obi_top10: 0.15,
            poly_obi_top20: 0.18,
            poly_spread: 0.02,
            poly_total_liquidity: 5000.0,
            poly_implied_prob: 0.52,
        };

        let vec = snap.to_vector();
        let names = FeatureSnapshot::feature_names();
        assert_eq!(vec.len(), names.len());
        assert_eq!(vec.len(), 37);
    }
}
