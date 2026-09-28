use serde::{Deserialize, Serialize};

use crate::dataset::DatasetRecord;
use crate::features::{FeatureSnapshot, FEATURE_NAMES};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureContribution {
    pub feature_name: String,
    pub value: f64,
    pub weight: f64,
    pub contribution: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticModelConfig {
    pub version: String,
    pub weights: Vec<f64>,
    pub bias: f64,
    pub means: Vec<f64>,
    pub stds: Vec<f64>,
}

#[derive(Clone)]
pub struct LogisticRegressionModel {
    config: LogisticModelConfig,
}

impl LogisticRegressionModel {
    pub fn new(config: LogisticModelConfig) -> Self {
        assert_eq!(
            config.weights.len(),
            FEATURE_NAMES.len(),
            "Weights length must match feature dimensions"
        );
        Self { config }
    }

    /// Construct default pre-trained baseline model with expert quantitative priors
    pub fn default_baseline() -> Self {
        let n = FEATURE_NAMES.len();
        let mut weights = vec![0.0; n];

        // Assign quantitative prior weights according to feature dynamics
        for (i, &name) in FEATURE_NAMES.iter().enumerate() {
            weights[i] = match name {
                // Short-term momentum
                "return_1s" => 0.5,
                "return_3s" => 0.8,
                "return_5s" => 1.2,
                "return_10s" => 1.0,
                "return_30s" => 0.6,
                "return_60s" => 0.4,

                // Velocity and acceleration
                "velocity_5s" => 0.8,
                "velocity_15s" => 0.5,
                "acceleration_5s_15s" => 0.6,

                // 5M open relative displacement
                "distance_percent" => 1.5,
                "distance_to_vol_ratio" => 1.0,

                // Trade flow CVD and aggressive imbalance
                "trade_imbalance_5s" => 1.2,
                "trade_imbalance_15s" => 0.8,
                "cvd_5s" => 0.3,
                "cvd_15s" => 0.2,

                // Polymarket Orderbook Imbalance
                "poly_obi_top5" => 1.0,
                "poly_obi_top10" => 0.8,
                "poly_obi_top20" => 0.5,

                _ => 0.0,
            };
        }

        let means = vec![0.0; n];
        let stds = vec![1.0; n];

        Self {
            config: LogisticModelConfig {
                version: "v1.0.0-logistic-baseline".to_string(),
                weights,
                bias: 0.0,
                means,
                stds,
            },
        }
    }

    pub fn version(&self) -> &str {
        &self.config.version
    }

    /// Predict raw probability P(Up) in [0.0, 1.0] and local feature explanations
    pub fn predict_raw(&self, features: &FeatureSnapshot) -> (f64, Vec<FeatureContribution>) {
        let vec = features.to_vector();
        let mut z = self.config.bias;
        let mut contributions = Vec::with_capacity(vec.len());

        for i in 0..vec.len() {
            let val = vec[i];
            let mean = self.config.means.get(i).copied().unwrap_or(0.0);
            let std = self.config.stds.get(i).copied().unwrap_or(1.0).max(1e-6);
            let norm_x = (val - mean) / std;

            let w = self.config.weights.get(i).copied().unwrap_or(0.0);
            let contrib = w * norm_x;
            z += contrib;

            contributions.push(FeatureContribution {
                feature_name: FEATURE_NAMES[i].to_string(),
                value: val,
                weight: w,
                contribution: contrib,
            });
        }

        // Sigmoid activation: 1 / (1 + exp(-z))
        let p_up = 1.0 / (1.0 + (-z).exp());

        // Sort contributions by absolute magnitude descending
        contributions.sort_by(|a, b| {
            b.contribution
                .abs()
                .partial_cmp(&a.contribution.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        (p_up, contributions)
    }

    /// Predict raw probability P(Up) from a feature vector slice
    pub fn predict_vector(&self, vec: &[f64]) -> f64 {
        let mut z = self.config.bias;
        for i in 0..vec.len() {
            let val = vec[i];
            let mean = self.config.means.get(i).copied().unwrap_or(0.0);
            let std = self.config.stds.get(i).copied().unwrap_or(1.0).max(1e-6);
            let norm_x = (val - mean) / std;
            let w = self.config.weights.get(i).copied().unwrap_or(0.0);
            z += w * norm_x;
        }
        1.0 / (1.0 + (-z).exp())
    }

    /// Train model on historical walk-forward records using L2 regularized SGD
    pub fn train(&mut self, records: &[DatasetRecord], epochs: usize, lr: f64, l2_reg: f64) {
        if records.is_empty() {
            return;
        }

        let n_features = FEATURE_NAMES.len();

        // 1. Calculate feature means and stds for normalization
        let mut means = vec![0.0; n_features];
        let mut vars = vec![0.0; n_features];
        let count = records.len() as f64;

        for r in records {
            for (i, &v) in r.features.iter().enumerate().take(n_features) {
                means[i] += v;
            }
        }
        for m in &mut means {
            *m /= count;
        }

        for r in records {
            for (i, &v) in r.features.iter().enumerate().take(n_features) {
                vars[i] += (v - means[i]).powi(2);
            }
        }
        let stds: Vec<f64> = vars.iter().map(|v| (v / count).sqrt().max(1e-4)).collect();

        self.config.means = means;
        self.config.stds = stds;

        // 2. SGD Optimization with L2 penalty
        for _ in 0..epochs {
            for r in records {
                let mut z = self.config.bias;
                let mut norm_x = Vec::with_capacity(n_features);

                for i in 0..n_features {
                    let v = r.features.get(i).copied().unwrap_or(0.0);
                    let nx = (v - self.config.means[i]) / self.config.stds[i];
                    norm_x.push(nx);
                    z += self.config.weights[i] * nx;
                }

                let p = 1.0 / (1.0 + (-z).exp());
                let error = p - r.target_up; // Gradient: (p - y)

                // Update bias
                self.config.bias -= lr * error;

                // Update weights with L2 regularization
                for i in 0..n_features {
                    let grad = error * norm_x[i] + (l2_reg * self.config.weights[i]);
                    self.config.weights[i] -= lr * grad;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Asset;

    #[test]
    fn test_baseline_logistic_prediction_and_explanations() {
        let model = LogisticRegressionModel::default_baseline();
        assert_eq!(model.version(), "v1.0.0-logistic-baseline");

        // Positive momentum features
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
            distance_percent: 0.076,
            distance_to_vol_ratio: 0.95,
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
            trade_imbalance_5s: 0.60,
            trade_imbalance_15s: 0.40,
            trade_imbalance_30s: 0.30,
            trade_imbalance_60s: 0.20,
            poly_obi_top5: 0.35,
            poly_obi_top10: 0.25,
            poly_obi_top20: 0.20,
            poly_spread: 0.02,
            poly_total_liquidity: 5000.0,
            poly_implied_prob: 0.52,
        };

        let (p_up, contribs) = model.predict_raw(&snap);
        // Positive returns and positive OBI should produce P(Up) > 0.50
        assert!(p_up > 0.50);
        assert!(!contribs.is_empty());
        assert_eq!(contribs.len(), 37);

        // Strong downward momentum should produce P(Up) < 0.50
        let mut bear_snap = snap.clone();
        bear_snap.return_1s = -0.001;
        bear_snap.return_3s = -0.002;
        bear_snap.return_5s = -0.005;
        bear_snap.return_10s = -0.008;
        bear_snap.return_30s = -0.010;
        bear_snap.return_60s = -0.015;
        bear_snap.velocity_5s = -1.5;
        bear_snap.velocity_15s = -0.8;
        bear_snap.acceleration_5s_15s = -0.07;
        bear_snap.distance_from_open = -50.0;
        bear_snap.distance_percent = -0.076;
        bear_snap.distance_to_vol_ratio = -0.95;
        bear_snap.cvd_5s = -20.0;
        bear_snap.cvd_15s = -35.0;
        bear_snap.trade_imbalance_5s = -0.60;
        bear_snap.trade_imbalance_15s = -0.40;
        bear_snap.poly_obi_top5 = -0.35;
        bear_snap.poly_obi_top10 = -0.25;

        let (p_down_up, _) = model.predict_raw(&bear_snap);
        assert!(p_down_up < 0.50);
    }
}
