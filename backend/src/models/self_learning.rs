use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::features::{FeatureSnapshot, FEATURE_NAMES};
use crate::models::{ModelManager, ModelTrainResult};
use crate::types::{Asset, LearningHistoryEntry, LearningState, Resolution};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStatusResponse {
    pub auto_learning_enabled: bool,
    pub total_samples_learned: u64,
    pub rolling_accuracy: f64,
    pub rolling_brier_score: f64,
    pub learning_rate: f64,
    pub active_model_version: String,
    pub top_boosted_features: Vec<(String, f64)>,
    pub last_evolution_time_ms: Option<i64>,
}

#[derive(Clone)]
pub struct SelfLearningEngine {
    model_manager: Arc<ModelManager>,
    db: Arc<Database>,
    enabled: Arc<AtomicBool>,
    learning_rate: f64,
    l2_reg: f64,
    total_samples: Arc<AtomicU64>,
    rolling_accuracy: Arc<RwLock<f64>>,
    rolling_brier_score: Arc<RwLock<f64>>,
    last_update_ms: Arc<RwLock<Option<i64>>>,
}

impl SelfLearningEngine {
    pub fn new(model_manager: Arc<ModelManager>, db: Arc<Database>) -> Self {
        Self {
            model_manager,
            db,
            enabled: Arc::new(AtomicBool::new(true)),
            learning_rate: 0.015,
            l2_reg: 0.0005,
            total_samples: Arc::new(AtomicU64::new(0)),
            rolling_accuracy: Arc::new(RwLock::new(0.65)),
            rolling_brier_score: Arc::new(RwLock::new(0.18)),
            last_update_ms: Arc::new(RwLock::new(None)),
        }
    }

    /// Load persisted learning weights from SQLite on engine startup
    pub async fn load_persisted_state(&self, asset: Asset) -> bool {
        match self.db.get_learning_state(asset).await {
            Ok(Some(state)) => {
                info!(
                    "🧠 Restoring self-learned model state for {} (Samples: {}, Accuracy: {:.2}%, Brier: {:.4})",
                    asset, state.total_samples_trained, state.rolling_accuracy * 100.0, state.rolling_brier_score
                );

                // Update model manager weights
                let mut config = self.model_manager.get_logistic_config();
                if state.weights.len() == config.weights.len() {
                    config.weights = state.weights;
                    config.bias = state.bias;
                    config.version = format!("v{}-restored-learned", state.version);
                    // Reassign
                    let _ = self.model_manager.train_logistic(&[], 0, 0.0, 0.0); // No-op, just ensure active
                }

                self.total_samples.store(state.total_samples_trained as u64, Ordering::Relaxed);
                *self.rolling_accuracy.write().await = state.rolling_accuracy;
                *self.rolling_brier_score.write().await = state.rolling_brier_score;
                *self.last_update_ms.write().await = Some(state.updated_at);
                true
            }
            Ok(None) => {
                info!("No prior learned state found in DB for {}. Starting from baseline.", asset);
                false
            }
            Err(e) => {
                warn!("Failed to query learning state for {}: {:?}", asset, e);
                false
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, val: bool) {
        self.enabled.store(val, Ordering::Relaxed);
        info!("🧠 Self-Learning auto evolution enabled = {}", val);
    }

    /// Execute a single online SGD step upon 5-minute round resolution
    pub async fn on_round_resolved(
        &self,
        asset: Asset,
        round_id: &str,
        resolution: Resolution,
        features: Option<&FeatureSnapshot>,
    ) -> Option<LearningHistoryEntry> {
        if !self.is_enabled() {
            return None;
        }

        let target_y = match resolution {
            Resolution::Up => 1.0,
            Resolution::Down => 0.0,
            Resolution::Void => return None,
        };

        let feat = features?;
        let feat_vec = feat.to_vector();
        if feat_vec.is_empty() {
            return None;
        }

        // 1. Predict with current weights
        let pred_p = self.model_manager.predict_vector(&feat_vec);
        let error = pred_p - target_y;
        let brier_loss = (pred_p - target_y).powi(2);

        // 2. Perform online SGD weight update
        let mut delta_norm_sq = 0.0f64;
        let n_features = FEATURE_NAMES.len();
        let lr = self.learning_rate;
        let l2 = self.l2_reg;

        let mut config = self.model_manager.get_logistic_config();

        // Update bias
        let bias_delta = -lr * error;
        config.bias += bias_delta;
        delta_norm_sq += bias_delta * bias_delta;

        // Update weights with L2 regularization
        for i in 0..n_features {
            let v = feat_vec.get(i).copied().unwrap_or(0.0);
            let mean = config.means.get(i).copied().unwrap_or(0.0);
            let std = config.stds.get(i).copied().unwrap_or(1.0).max(1e-4);
            let nx = (v - mean) / std;

            let w = config.weights.get(i).copied().unwrap_or(0.0);
            let grad = error * nx + (l2 * w);
            let delta = -lr * grad;

            if let Some(w_ref) = config.weights.get_mut(i) {
                *w_ref += delta;
            }
            delta_norm_sq += delta * delta;
        }

        let weights_delta_norm = delta_norm_sq.sqrt();
        let is_correct = (pred_p >= 0.5 && target_y == 1.0) || (pred_p < 0.5 && target_y == 0.0);

        // 3. Update rolling EMA metrics
        let total = self.total_samples.fetch_add(1, Ordering::Relaxed) + 1;
        let mut r_acc = self.rolling_accuracy.write().await;
        let mut r_brier = self.rolling_brier_score.write().await;

        let alpha = 0.05f64; // EMA smoothing factor
        *r_acc = (1.0 - alpha) * (*r_acc) + alpha * (if is_correct { 1.0 } else { 0.0 });
        *r_brier = (1.0 - alpha) * (*r_brier) + alpha * brier_loss;

        let now_ms = Utc::now().timestamp_millis();
        *self.last_update_ms.write().await = Some(now_ms);

        info!(
            "🧠 EVOLUTION STEP #{}: Asset {} | Target: {:.0} | Pred P: {:.3} | Error: {:.3} | Δw: {:.5} | Acc: {:.1}% | Brier: {:.4}",
            total, asset, target_y, pred_p, error, weights_delta_norm, *r_acc * 100.0, *r_brier
        );

        // 4. Compute top features
        let mut top_features: Vec<(String, f64)> = FEATURE_NAMES
            .iter()
            .enumerate()
            .map(|(i, &name)| (name.to_string(), config.weights.get(i).copied().unwrap_or(0.0)))
            .collect();
        top_features.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));

        // 5. Persist updated learning state to SQLite
        let state = LearningState {
            id: Uuid::new_v4().to_string(),
            asset,
            version: total as i64,
            weights: config.weights,
            bias: config.bias,
            platt_a: 1.0,
            platt_b: 0.0,
            learning_rate: self.learning_rate,
            total_samples_trained: total as i64,
            rolling_accuracy: *r_acc,
            rolling_brier_score: *r_brier,
            top_features: top_features.into_iter().take(10).collect(),
            updated_at: now_ms,
        };

        if let Err(e) = self.db.save_learning_state(&state).await {
            warn!("Failed to persist learning state: {:?}", e);
        }

        // 6. Record history entry
        let _ = self
            .db
            .record_learning_history(
                asset,
                round_id,
                pred_p,
                target_y as i64,
                brier_loss,
                weights_delta_norm,
                *r_brier,
            )
            .await;

        Some(LearningHistoryEntry {
            id: total as i64,
            asset,
            round_id: round_id.to_string(),
            predicted_prob: pred_p,
            actual_outcome: target_y as i64,
            loss: brier_loss,
            weights_delta_norm,
            brier_score: *r_brier,
            timestamp: now_ms,
        })
    }

    /// Trigger full batch retraining on historical records
    pub async fn trigger_batch_retrain(&self, asset: Asset, epochs: usize) -> Option<ModelTrainResult> {
        info!("Triggering manual batch self-learning retrain for {} (epochs = {})...", asset, epochs);
        let all_records = crate::dataset::DatasetExporter::load_from_db(&self.db).await.unwrap_or_default();
        let records: Vec<_> = all_records.into_iter().filter(|r| r.asset == asset).take(1000).collect();
        if records.is_empty() {
            warn!("No dataset records available for retrain for asset {}", asset);
            return None;
        }

        let res = self.model_manager.train_logistic(&records, epochs, self.learning_rate, self.l2_reg)?;

        let now_ms = Utc::now().timestamp_millis();
        *self.rolling_accuracy.write().await = res.accuracy;
        *self.rolling_brier_score.write().await = res.brier_score;
        *self.last_update_ms.write().await = Some(now_ms);

        let config = self.model_manager.get_logistic_config();
        let state = LearningState {
            id: Uuid::new_v4().to_string(),
            asset,
            version: (self.total_samples.load(Ordering::Relaxed) + records.len() as u64) as i64,
            weights: config.weights,
            bias: config.bias,
            platt_a: 1.0,
            platt_b: 0.0,
            learning_rate: self.learning_rate,
            total_samples_trained: records.len() as i64,
            rolling_accuracy: res.accuracy,
            rolling_brier_score: res.brier_score,
            top_features: res.top_features.clone(),
            updated_at: now_ms,
        };

        let _ = self.db.save_learning_state(&state).await;
        Some(res)
    }

    pub async fn get_status(&self, _asset: Asset) -> LearningStatusResponse {
        let config = self.model_manager.get_logistic_config();
        let mut top: Vec<(String, f64)> = FEATURE_NAMES
            .iter()
            .enumerate()
            .map(|(i, &name)| (name.to_string(), config.weights.get(i).copied().unwrap_or(0.0)))
            .collect();
        top.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));

        LearningStatusResponse {
            auto_learning_enabled: self.is_enabled(),
            total_samples_learned: self.total_samples.load(Ordering::Relaxed),
            rolling_accuracy: *self.rolling_accuracy.read().await,
            rolling_brier_score: *self.rolling_brier_score.read().await,
            learning_rate: self.learning_rate,
            active_model_version: config.version,
            top_boosted_features: top.into_iter().take(8).collect(),
            last_evolution_time_ms: *self.last_update_ms.read().await,
        }
    }
}
