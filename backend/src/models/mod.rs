pub mod calibration;
pub mod ensemble;
pub mod logistic;
pub mod metrics;

use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock as SyncRwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::info;

use crate::db::Database;
use crate::features::FeatureSnapshot;
use crate::types::Asset;
pub use calibration::{CalibrationConfig, ProbabilityCalibrator};
pub use ensemble::{ConfidenceLevel, EnsembleModel, HeuristicRuleModel};
pub use logistic::{FeatureContribution, LogisticModelConfig, LogisticRegressionModel};
pub use metrics::{MetricsCalculator, ModelEvaluationMetrics};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPrediction {
    pub market_id: String,
    pub asset: Asset,
    pub timestamp_ms: i64,
    pub model_version: String,
    pub raw_p_up: f64,
    pub raw_p_down: f64,
    pub calibrated_p_up: f64,
    pub calibrated_p_down: f64,
    pub confidence: ConfidenceLevel,
    pub top_contributions: Vec<FeatureContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTrainRequest {
    pub epochs: Option<usize>,
    pub lr: Option<f64>,
    pub l2_reg: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTrainResult {
    pub success: bool,
    pub samples_trained: usize,
    pub accuracy: f64,
    pub brier_score: f64,
    pub log_loss: f64,
    pub expected_calibration_error: f64,
    pub model_version: String,
    pub top_features: Vec<(String, f64)>,
}

#[derive(Clone)]
pub struct ModelManager {
    logistic: Arc<SyncRwLock<LogisticRegressionModel>>,
    ensemble: Arc<EnsembleModel>,
    calibrator: Arc<ProbabilityCalibrator>,
    active_model: Arc<RwLock<String>>,
    latest_predictions: Arc<DashMap<Asset, ModelPrediction>>,
    prediction_tx: broadcast::Sender<ModelPrediction>,
}

impl ModelManager {
    pub fn new() -> Self {
        let (prediction_tx, _) = broadcast::channel(1024);
        Self {
            logistic: Arc::new(SyncRwLock::new(LogisticRegressionModel::default_baseline())),
            ensemble: Arc::new(EnsembleModel::default_ensemble()),
            calibrator: Arc::new(ProbabilityCalibrator::default_calibrator()),
            active_model: Arc::new(RwLock::new("ensemble".to_string())),
            latest_predictions: Arc::new(DashMap::new()),
            prediction_tx,
        }
    }

    /// Background task consuming incoming FeatureSnapshots, running inference and calibration
    pub fn start(&self, mut feature_rx: broadcast::Receiver<FeatureSnapshot>, _db: Arc<Database>) {
        info!("Starting ModelManager with online inference, ensemble logic, and probability calibration...");

        let manager = self.clone();
        tokio::spawn(async move {
            while let Ok(features) = feature_rx.recv().await {
                let prediction = manager.predict(&features).await;
                manager.latest_predictions.insert(features.asset, prediction.clone());
                let _ = manager.prediction_tx.send(prediction);
            }
        });
    }

    /// Run inference & calibration on a FeatureSnapshot
    pub async fn predict(&self, features: &FeatureSnapshot) -> ModelPrediction {
        let active = self.active_model.read().await;

        let (raw_p_up, confidence, version, contribs) = if *active == "logistic" {
            let model = self.logistic.read();
            let (p, c) = model.predict_raw(features);
            let conf = if (p - 0.50).abs() > 0.20 {
                ConfidenceLevel::High
            } else if (p - 0.50).abs() > 0.10 {
                ConfidenceLevel::Medium
            } else {
                ConfidenceLevel::Low
            };
            (p, conf, model.version().to_string(), c)
        } else {
            // Ensemble model (default)
            let (p, conf) = self.ensemble.predict(features);
            let (_, c) = self.logistic.read().predict_raw(features);
            (p, conf, self.ensemble.version().to_string(), c)
        };

        let raw_p_down = 1.0 - raw_p_up;

        // Apply Probability Calibration
        let (calibrated_p_up, calibrated_p_down) = self.calibrator.calibrate(
            raw_p_up,
            features.time_decay_factor,
            features.realized_vol_60s,
        );

        let top_contributions = contribs.into_iter().take(5).collect();

        ModelPrediction {
            market_id: features.market_id.clone(),
            asset: features.asset,
            timestamp_ms: features.timestamp_ms,
            model_version: version,
            raw_p_up,
            raw_p_down,
            calibrated_p_up,
            calibrated_p_down,
            confidence,
            top_contributions,
        }
    }

    pub fn train_logistic(
        &self,
        records: &[crate::dataset::DatasetRecord],
        epochs: usize,
        lr: f64,
        l2_reg: f64,
    ) -> Option<ModelTrainResult> {
        if records.is_empty() {
            return None;
        }

        let mut model = self.logistic.write();
        model.train(records, epochs, lr, l2_reg);

        let new_version = format!("v{}-sgd-trained", Utc::now().format("%Y%m%d%H%M%S"));
        model.config.version = new_version.clone();

        let mut predictions = Vec::with_capacity(records.len());
        let mut labels = Vec::with_capacity(records.len());

        for r in records {
            let p = model.predict_vector(&r.features);
            predictions.push(p);
            labels.push(r.target_up);
        }

        let metrics = MetricsCalculator::evaluate(&predictions, &labels)?;

        let mut weights_with_names: Vec<(String, f64)> = crate::features::FEATURE_NAMES
            .iter()
            .enumerate()
            .map(|(i, &name)| (name.to_string(), model.config.weights.get(i).copied().unwrap_or(0.0)))
            .collect();
        weights_with_names.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));

        Some(ModelTrainResult {
            success: true,
            samples_trained: records.len(),
            accuracy: metrics.accuracy,
            brier_score: metrics.brier_score,
            log_loss: metrics.log_loss,
            expected_calibration_error: metrics.expected_calibration_error,
            model_version: new_version,
            top_features: weights_with_names.into_iter().take(10).collect(),
        })
    }

    pub fn get_logistic_config(&self) -> LogisticModelConfig {
        self.logistic.read().config.clone()
    }

    pub fn get_latest_prediction(&self, asset: Asset) -> Option<ModelPrediction> {
        self.latest_predictions.get(&asset).map(|p| p.clone())
    }

    pub fn get_all_latest_predictions(&self) -> HashMap<String, ModelPrediction> {
        let mut map = HashMap::new();
        for entry in self.latest_predictions.iter() {
            map.insert(entry.key().to_string(), entry.value().clone());
        }
        map
    }

    pub fn predict_vector(&self, vec: &[f64]) -> f64 {
        self.logistic.read().predict_vector(vec)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ModelPrediction> {
        self.prediction_tx.subscribe()
    }
}
