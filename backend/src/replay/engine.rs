use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use uuid::Uuid;

use super::types::{ReplayConfig, ReplayFrame, ReplayStateResponse, ReplayStatus, SeekRequest};
use crate::dataset::DatasetExporter;
use crate::db::Database;
use crate::models::{ConfidenceLevel, ModelManager, ModelPrediction};
use crate::strategy::StrategyEngine;
use crate::types::{Asset, MarketSide, PaperResult, SignalAction};

pub struct ReplayEngine {
    db: Arc<Database>,
    models: Arc<ModelManager>,
    strategy: Arc<StrategyEngine>,
    status: Arc<RwLock<ReplayStatus>>,
    config: Arc<RwLock<Option<ReplayConfig>>>,
    frames: Arc<RwLock<Vec<ReplayFrame>>>,
    current_index: Arc<AtomicUsize>,
    speed: Arc<RwLock<f64>>,
    broadcast_tx: broadcast::Sender<ReplayFrame>,
    playback_handle: Arc<Mutex<Option<tokio::task::AbortHandle>>>,
}

impl ReplayEngine {
    pub fn new(
        db: Arc<Database>,
        models: Arc<ModelManager>,
        strategy: Arc<StrategyEngine>,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1024);
        Self {
            db,
            models,
            strategy,
            status: Arc::new(RwLock::new(ReplayStatus::Idle)),
            config: Arc::new(RwLock::new(None)),
            frames: Arc::new(RwLock::new(Vec::new())),
            current_index: Arc::new(AtomicUsize::new(0)),
            speed: Arc::new(RwLock::new(1.0)),
            broadcast_tx,
            playback_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Load or synthesize historical frames based on config
    pub async fn load_frames(&self, config: &ReplayConfig) -> Result<usize, String> {
        let mut loaded_frames = Vec::new();

        // 1. Try loading real dataset records from DB
        if let Ok(records) = DatasetExporter::load_from_db(&self.db).await {
            for (idx, r) in records.iter().enumerate() {
                if r.asset != config.asset {
                    continue;
                }
                if let Some(start) = config.start_time_ms {
                    if r.timestamp_ms < start {
                        continue;
                    }
                }
                if let Some(end) = config.end_time_ms {
                    if r.timestamp_ms > end {
                        continue;
                    }
                }

                let p_up = self.models.predict_vector(&r.features).clamp(0.05, 0.95);
                let p_down = 1.0 - p_up;

                let conf = if (p_up - 0.50).abs() > 0.20 {
                    ConfidenceLevel::High
                } else if (p_up - 0.50).abs() > 0.10 {
                    ConfidenceLevel::Medium
                } else {
                    ConfidenceLevel::Low
                };

                let pred = ModelPrediction {
                    market_id: r.market_id.clone(),
                    asset: r.asset,
                    timestamp_ms: r.timestamp_ms,
                    model_version: "replay-v1".to_string(),
                    raw_p_up: p_up,
                    raw_p_down: p_down,
                    calibrated_p_up: p_up,
                    calibrated_p_down: p_down,
                    confidence: conf,
                    top_contributions: vec![],
                };

                let (sig, _) = self.strategy.evaluate(&pred, None, None, 0.001, true, 150);

                let comp_price = if !r.features.is_empty() {
                    r.features[0]
                } else {
                    60000.0
                };

                loaded_frames.push(ReplayFrame {
                    frame_index: idx,
                    timestamp_ms: r.timestamp_ms,
                    asset: r.asset,
                    market_id: r.market_id.clone(),
                    composite_price: comp_price,
                    poly_up_bid: 0.49,
                    poly_up_ask: 0.51,
                    poly_down_bid: 0.49,
                    poly_down_ask: 0.51,
                    implied_prob_up: 0.50,
                    features: None,
                    prediction: Some(pred),
                    signal: Some(sig),
                    executed_trade: None,
                });
            }
        }

        // 2. If no frames in DB for the asset/window, generate realistic simulation frames
        if loaded_frames.is_empty() {
            let base_time = config
                .start_time_ms
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() - 300_000);
            let base_price = match config.asset {
                Asset::BTC => 65000.0,
                Asset::ETH => 3500.0,
                Asset::SOL => 150.0,
            };

            for i in 0..60 {
                let ts = base_time + (i as i64 * 5_000); // 5-second interval
                let price_offset = (i as f64 * 0.1).sin() * (base_price * 0.001);
                let current_price = base_price + price_offset;

                let remaining_seconds = (300 - (i * 5)).max(5);
                let p_up = (0.50 + ((i as f64 - 30.0) / 100.0)).clamp(0.20, 0.85);
                let p_down = 1.0 - p_up;

                let conf = if (p_up - 0.50).abs() > 0.20 {
                    ConfidenceLevel::High
                } else {
                    ConfidenceLevel::Medium
                };

                let pred = ModelPrediction {
                    market_id: format!("{}-5M-REPLAY-SIM", config.asset),
                    asset: config.asset,
                    timestamp_ms: ts,
                    model_version: "replay-sim-v1".to_string(),
                    raw_p_up: p_up,
                    raw_p_down: p_down,
                    calibrated_p_up: p_up,
                    calibrated_p_down: p_down,
                    confidence: conf,
                    top_contributions: vec![],
                };

                let (sig, _) = self.strategy.evaluate(
                    &pred,
                    None,
                    None,
                    price_offset / base_price,
                    true,
                    remaining_seconds as i64,
                );

                let executed_trade = if sig.action == SignalAction::BuyUp {
                    Some(PaperResult {
                        result_id: format!("sim_res_{}", Uuid::new_v4().simple()),
                        order_id: format!("sim_ord_{}", Uuid::new_v4().simple()),
                        market_id: format!("{}-5M-REPLAY-SIM", config.asset),
                        asset: config.asset,
                        side: MarketSide::Up,
                        entry_time_ms: ts,
                        entry_price: 0.51,
                        fill_price: 0.512,
                        shares: 1.95,
                        stake: 1.0,
                        predicted_probability: p_up,
                        model_confidence: "HIGH".to_string(),
                        gross_edge: p_up - 0.51,
                        net_edge: p_up - 0.512 - 0.012,
                        fee: 0.012,
                        slippage: 0.002,
                        outcome: "PENDING".to_string(),
                        payout: 0.0,
                        pnl: 0.0,
                        bankroll_before: 10.0,
                        bankroll_after: 9.0,
                        locked_profit_before: 0.0,
                        locked_profit_after: 0.0,
                        strategy_version: "replay-v1".to_string(),
                        model_version: "replay-v1".to_string(),
                        created_at_ms: ts,
                    })
                } else {
                    None
                };

                loaded_frames.push(ReplayFrame {
                    frame_index: i,
                    timestamp_ms: ts,
                    asset: config.asset,
                    market_id: format!("{}-5M-REPLAY-SIM", config.asset),
                    composite_price: current_price,
                    poly_up_bid: (p_up - 0.02).clamp(0.01, 0.98),
                    poly_up_ask: (p_up + 0.02).clamp(0.02, 0.99),
                    poly_down_bid: (p_down - 0.02).clamp(0.01, 0.98),
                    poly_down_ask: (p_down + 0.02).clamp(0.02, 0.99),
                    implied_prob_up: p_up,
                    features: None,
                    prediction: Some(pred),
                    signal: Some(sig),
                    executed_trade,
                });
            }
        }

        let total = loaded_frames.len();
        {
            let mut f_guard = self.frames.write().await;
            *f_guard = loaded_frames;
        }
        self.current_index.store(0, Ordering::SeqCst);
        *self.config.write().await = Some(config.clone());
        *self.speed.write().await = config.speed_multiplier;

        Ok(total)
    }

    /// Start or restart replay playback
    pub async fn start(&self, config: ReplayConfig) -> Result<ReplayStateResponse, String> {
        self.stop().await;

        self.load_frames(&config).await?;
        *self.status.write().await = ReplayStatus::Playing;

        self.spawn_playback_task().await;
        Ok(self.get_state().await)
    }

    /// Pause active replay playback
    pub async fn pause(&self) -> ReplayStateResponse {
        *self.status.write().await = ReplayStatus::Paused;
        let mut handle = self.playback_handle.lock().await;
        if let Some(h) = handle.take() {
            h.abort();
        }
        self.get_state().await
    }

    /// Resume paused replay playback
    pub async fn resume(&self) -> ReplayStateResponse {
        *self.status.write().await = ReplayStatus::Playing;
        self.spawn_playback_task().await;
        self.get_state().await
    }

    /// Advance replay by a single frame (Step forward)
    pub async fn step(&self) -> Option<ReplayFrame> {
        let f_guard = self.frames.read().await;
        let total = f_guard.len();
        if total == 0 {
            return None;
        }

        let current = self.current_index.load(Ordering::SeqCst);
        if current >= total {
            *self.status.write().await = ReplayStatus::Completed;
            return f_guard.last().cloned();
        }

        let frame = f_guard[current].clone();
        let _ = self.broadcast_tx.send(frame.clone());

        let next = current + 1;
        self.current_index.store(next, Ordering::SeqCst);
        if next >= total {
            *self.status.write().await = ReplayStatus::Completed;
        }

        Some(frame)
    }

    /// Seek to a specific frame index or timestamp
    pub async fn seek(&self, req: SeekRequest) -> Option<ReplayFrame> {
        let f_guard = self.frames.read().await;
        let total = f_guard.len();
        if total == 0 {
            return None;
        }

        let target_idx = if let Some(idx) = req.target_frame_index {
            idx.min(total - 1)
        } else if let Some(ts) = req.target_timestamp_ms {
            f_guard
                .iter()
                .position(|f| f.timestamp_ms >= ts)
                .unwrap_or(total - 1)
        } else {
            0
        };

        self.current_index.store(target_idx, Ordering::SeqCst);
        let frame = f_guard[target_idx].clone();
        let _ = self.broadcast_tx.send(frame.clone());

        Some(frame)
    }

    /// Set replay playback speed multiplier
    pub async fn set_speed(&self, speed: f64) -> f64 {
        let spd = speed.clamp(0.1, 100.0);
        *self.speed.write().await = spd;
        spd
    }

    /// Stop replay and reset to idle
    pub async fn stop(&self) {
        let mut handle = self.playback_handle.lock().await;
        if let Some(h) = handle.take() {
            h.abort();
        }
        *self.status.write().await = ReplayStatus::Idle;
        self.current_index.store(0, Ordering::SeqCst);
    }

    /// Get current state snapshot
    pub async fn get_state(&self) -> ReplayStateResponse {
        let status = *self.status.read().await;
        let config = self.config.read().await.clone();
        let f_guard = self.frames.read().await;
        let total = f_guard.len();
        let cur_idx = self.current_index.load(Ordering::SeqCst);
        let speed = *self.speed.read().await;

        let (cur_ts, cur_frame) = if cur_idx < total {
            (
                Some(f_guard[cur_idx].timestamp_ms),
                Some(f_guard[cur_idx].clone()),
            )
        } else if let Some(last) = f_guard.last() {
            (Some(last.timestamp_ms), Some(last.clone()))
        } else {
            (None, None)
        };

        ReplayStateResponse {
            status,
            config,
            total_frames: total,
            current_frame_index: cur_idx,
            current_timestamp_ms: cur_ts,
            speed_multiplier: speed,
            current_frame: cur_frame,
        }
    }

    /// Get all or range of loaded frames for charting/timeline
    pub async fn get_frames(&self, offset: usize, limit: usize) -> Vec<ReplayFrame> {
        let f_guard = self.frames.read().await;
        f_guard.iter().skip(offset).take(limit).cloned().collect()
    }

    /// Subscribe to real-time replayed frames
    pub fn subscribe(&self) -> broadcast::Receiver<ReplayFrame> {
        self.broadcast_tx.subscribe()
    }

    /// Internal background playback task
    async fn spawn_playback_task(&self) {
        let mut handle_guard = self.playback_handle.lock().await;
        if let Some(h) = handle_guard.take() {
            h.abort();
        }

        let status_arc = self.status.clone();
        let frames_arc = self.frames.clone();
        let cur_idx_arc = self.current_index.clone();
        let speed_arc = self.speed.clone();
        let tx = self.broadcast_tx.clone();

        let join_handle = tokio::spawn(async move {
            loop {
                // Check status
                {
                    let s = *status_arc.read().await;
                    if s != ReplayStatus::Playing {
                        break;
                    }
                }

                let frames = frames_arc.read().await;
                let total = frames.len();
                let idx = cur_idx_arc.load(Ordering::SeqCst);

                if idx >= total {
                    *status_arc.write().await = ReplayStatus::Completed;
                    break;
                }

                let frame = frames[idx].clone();
                let _ = tx.send(frame.clone());

                let next_idx = idx + 1;
                cur_idx_arc.store(next_idx, Ordering::SeqCst);

                if next_idx >= total {
                    *status_arc.write().await = ReplayStatus::Completed;
                    break;
                }

                // Calculate realistic inter-frame delay scaled by speed
                let dt_ms = (frames[next_idx].timestamp_ms - frame.timestamp_ms).clamp(10, 5000);
                drop(frames);

                let speed = *speed_arc.read().await;
                let delay_ms = if speed > 0.0 {
                    (dt_ms as f64 / speed) as u64
                } else {
                    100
                };

                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms.max(10))).await;
            }
        });

        *handle_guard = Some(join_handle.abort_handle());
    }
}
