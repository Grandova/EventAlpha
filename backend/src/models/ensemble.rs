use serde::{Deserialize, Serialize};

use crate::features::FeatureSnapshot;
use crate::models::logistic::LogisticRegressionModel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ConfidenceLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

impl std::fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfidenceLevel::Low => write!(f, "LOW"),
            ConfidenceLevel::Medium => write!(f, "MEDIUM"),
            ConfidenceLevel::High => write!(f, "HIGH"),
            ConfidenceLevel::VeryHigh => write!(f, "VERY_HIGH"),
        }
    }
}

pub struct HeuristicRuleModel;

impl HeuristicRuleModel {
    /// Heuristic rule prediction based on multi-signal directional alignment
    pub fn predict(features: &FeatureSnapshot) -> (f64, ConfidenceLevel) {
        let mut up_signals = 0;
        let mut down_signals = 0;

        // Signal 1: Distance from 5M open
        if features.distance_percent > 0.05 {
            up_signals += 1;
        } else if features.distance_percent < -0.05 {
            down_signals += 1;
        }

        // Signal 2: Short term return
        if features.return_5s > 0.0005 {
            up_signals += 1;
        } else if features.return_5s < -0.0005 {
            down_signals += 1;
        }

        // Signal 3: Polymarket Orderbook Imbalance (OBI)
        if features.poly_obi_top5 > 0.10 {
            up_signals += 1;
        } else if features.poly_obi_top5 < -0.10 {
            down_signals += 1;
        }

        // Signal 4: Trade Flow Imbalance
        if features.trade_imbalance_5s > 0.20 {
            up_signals += 1;
        } else if features.trade_imbalance_5s < -0.20 {
            down_signals += 1;
        }

        let total_signals = up_signals + down_signals;
        if total_signals == 0 {
            return (0.50, ConfidenceLevel::Low);
        }

        let score = (up_signals as f64 - down_signals as f64) / 4.0; // [-1.0, 1.0]
        let p_up = 0.50 + (score * 0.40); // [0.10, 0.90]

        let confidence = match (up_signals, down_signals) {
            (4, 0) | (0, 4) => ConfidenceLevel::VeryHigh,
            (3, 0) | (0, 3) => ConfidenceLevel::High,
            (3, 1) | (1, 3) | (2, 0) | (0, 2) => ConfidenceLevel::Medium,
            _ => ConfidenceLevel::Low,
        };

        (p_up, confidence)
    }
}

pub struct EnsembleModel {
    version: String,
    logistic: LogisticRegressionModel,
    logistic_weight: f64,
    heuristic_weight: f64,
}

impl EnsembleModel {
    pub fn new(version: String, logistic: LogisticRegressionModel, logistic_weight: f64, heuristic_weight: f64) -> Self {
        Self {
            version,
            logistic,
            logistic_weight,
            heuristic_weight,
        }
    }

    pub fn default_ensemble() -> Self {
        Self::new(
            "v1.0.0-ensemble".to_string(),
            LogisticRegressionModel::default_baseline(),
            0.60,
            0.40,
        )
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    /// Predict using weighted combination of Logistic ML and Heuristic Rules
    pub fn predict(&self, features: &FeatureSnapshot) -> (f64, ConfidenceLevel) {
        let (logistic_p, _) = self.logistic.predict_raw(features);
        let (heuristic_p, heuristic_conf) = HeuristicRuleModel::predict(features);

        let total_weight = self.logistic_weight + self.heuristic_weight;
        let ensemble_p = (logistic_p * self.logistic_weight + heuristic_p * self.heuristic_weight) / total_weight;

        // Model agreement determination
        let agreement = (logistic_p - 0.50).signum() == (heuristic_p - 0.50).signum();
        let prob_distance = (ensemble_p - 0.50).abs();

        let final_confidence = if agreement {
            if prob_distance > 0.20 && heuristic_conf == ConfidenceLevel::VeryHigh {
                ConfidenceLevel::VeryHigh
            } else if prob_distance > 0.12 {
                ConfidenceLevel::High
            } else {
                ConfidenceLevel::Medium
            }
        } else {
            ConfidenceLevel::Low
        };

        (ensemble_p, final_confidence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Asset;

    #[test]
    fn test_ensemble_alignment_and_confidence() {
        let ensemble = EnsembleModel::default_ensemble();
        assert_eq!(ensemble.version(), "v1.0.0-ensemble");

        // Strong bullish alignment
        let snap = crate::features::FeatureSnapshot {
            asset: Asset::BTC,
            market_id: "test".to_string(),
            timestamp_ms: 1000,
            composite_price: 65000.0,
            return_1s: 0.001,
            return_3s: 0.002,
            return_5s: 0.005,
            return_10s: 0.008,
            return_30s: 0.010,
            return_60s: 0.015,
            realized_vol_5s: 0.0005,
            realized_vol_10s: 0.0006,
            realized_vol_30s: 0.0007,
            realized_vol_60s: 0.0008,
            velocity_5s: 1.5,
            velocity_15s: 0.8,
            acceleration_5s_15s: 0.07,
            distance_from_open: 50.0,
            distance_percent: 0.10,
            distance_to_vol_ratio: 1.25,
            remaining_seconds: 180.0,
            elapsed_seconds: 120.0,
            time_decay_factor: 0.774,
            spread_binance_okx: 0.5,
            spread_binance_bybit: 0.2,
            spread_binance_coinbase: 0.4,
            cvd_5s: 20.0,
            cvd_15s: 35.0,
            cvd_30s: 50.0,
            cvd_60s: 65.0,
            trade_imbalance_5s: 0.70,
            trade_imbalance_15s: 0.50,
            trade_imbalance_30s: 0.40,
            trade_imbalance_60s: 0.30,
            poly_obi_top5: 0.45,
            poly_obi_top10: 0.35,
            poly_obi_top20: 0.25,
            poly_spread: 0.02,
            poly_total_liquidity: 5000.0,
            poly_implied_prob: 0.52,
        };

        let (p_up, conf) = ensemble.predict(&snap);
        assert!(p_up > 0.65);
        assert!(conf == ConfidenceLevel::High || conf == ConfidenceLevel::VeryHigh);
    }
}
