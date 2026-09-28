use serde::{Deserialize, Serialize};
use crate::features::FeatureSnapshot;
use crate::models::ModelPrediction;
use crate::types::{Asset, PaperResult, PredictionSignal};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayStatus {
    Idle,
    Playing,
    Paused,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayConfig {
    pub asset: Asset,
    pub start_time_ms: Option<i64>,
    pub end_time_ms: Option<i64>,
    #[serde(default = "default_speed")]
    pub speed_multiplier: f64,
}

fn default_speed() -> f64 {
    1.0
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            asset: Asset::BTC,
            start_time_ms: None,
            end_time_ms: None,
            speed_multiplier: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayFrame {
    pub frame_index: usize,
    pub timestamp_ms: i64,
    pub asset: Asset,
    pub market_id: String,
    pub composite_price: f64,
    pub poly_up_bid: f64,
    pub poly_up_ask: f64,
    pub poly_down_bid: f64,
    pub poly_down_ask: f64,
    pub implied_prob_up: f64,
    pub features: Option<FeatureSnapshot>,
    pub prediction: Option<ModelPrediction>,
    pub signal: Option<PredictionSignal>,
    pub executed_trade: Option<PaperResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayStateResponse {
    pub status: ReplayStatus,
    pub config: Option<ReplayConfig>,
    pub total_frames: usize,
    pub current_frame_index: usize,
    pub current_timestamp_ms: Option<i64>,
    pub speed_multiplier: f64,
    pub current_frame: Option<ReplayFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeekRequest {
    pub target_frame_index: Option<usize>,
    pub target_timestamp_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedRequest {
    pub speed_multiplier: f64,
}
