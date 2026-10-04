pub mod filter;
pub mod scoring;

use chrono::Utc;
use dashmap::DashMap;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

use parking_lot::RwLock as SyncRwLock;

use crate::collector::FreshnessTracker;
use crate::composite::CompositePriceEngine;
use crate::config::{ExecutionConfig, StrategyConfig};
use crate::db::Database;
use crate::models::ModelPrediction;
use crate::polymarket::orderbook::PolymarketBookEngine;
use crate::types::{
    Asset, ConfidenceLevel, DecisionLog, OrderBookSnapshot, PolymarketTick, PredictionSignal,
    SignalAction,
};
pub use filter::{FilterResult, HardFilterEngine};
pub use scoring::{ScoreBreakdown, ScoreTier, SignalScorer};

#[derive(Clone)]
pub struct StrategyEngine {
    config: Arc<SyncRwLock<StrategyConfig>>,
    execution_config: ExecutionConfig,
    db: Arc<Database>,
    latest_signals: Arc<DashMap<Asset, PredictionSignal>>,
    recent_decisions: Arc<RwLock<VecDeque<DecisionLog>>>,
    signal_tx: broadcast::Sender<PredictionSignal>,
}

impl StrategyEngine {
    pub fn new(
        config: StrategyConfig,
        execution_config: ExecutionConfig,
        db: Arc<Database>,
    ) -> Self {
        let (signal_tx, _) = broadcast::channel(1024);
        Self {
            config: Arc::new(SyncRwLock::new(config)),
            execution_config,
            db,
            latest_signals: Arc::new(DashMap::new()),
            recent_decisions: Arc::new(RwLock::new(VecDeque::with_capacity(500))),
            signal_tx,
        }
    }

    pub fn get_config(&self) -> StrategyConfig {
        self.config.read().clone()
    }

    pub fn update_config(&self, new_config: StrategyConfig) {
        *self.config.write() = new_config;
    }

    /// Pure evaluation function without side effects, ideal for testing and real-time execution
    pub fn evaluate(
        &self,
        prediction: &ModelPrediction,
        polymarket_tick: Option<&PolymarketTick>,
        polymarket_book: Option<&OrderBookSnapshot>,
        composite_return_60s: f64,
        is_fresh: bool,
        remaining_seconds: i64,
    ) -> (PredictionSignal, DecisionLog) {
        let is_up = prediction.calibrated_p_up >= prediction.calibrated_p_down;
        let (calibrated_p, market_ask, directed_obi, directed_momentum) = if is_up {
            let ask = polymarket_tick
                .and_then(|t| t.up_ask)
                .unwrap_or(0.50);
            let obi = polymarket_book
                .map(|b| b.obi_top10)
                .unwrap_or(0.0);
            let mom = (composite_return_60s * 1000.0).clamp(-1.0, 1.0);
            (prediction.calibrated_p_up, ask, obi, mom)
        } else {
            let ask = polymarket_tick
                .and_then(|t| t.down_ask)
                .or_else(|| polymarket_tick.and_then(|t| t.up_bid).map(|bid| 1.0 - bid))
                .unwrap_or(0.50);
            let obi = polymarket_book
                .map(|b| -b.obi_top10)
                .unwrap_or(0.0);
            let mom = (-composite_return_60s * 1000.0).clamp(-1.0, 1.0);
            (prediction.calibrated_p_down, ask, obi, mom)
        };

        // Edge calculations
        let fair_value_up = prediction.calibrated_p_up;
        let fair_value_down = prediction.calibrated_p_down;
        let gross_edge = calibrated_p - market_ask;
        let estimated_fee = self.execution_config.fee_rate;
        let estimated_slippage = self.execution_config.slippage_rate;
        let net_edge = gross_edge - estimated_fee - estimated_slippage;

        // Market checks
        let spread = polymarket_tick.and_then(|t| t.spread).unwrap_or(0.02);
        let total_liquidity = polymarket_tick.and_then(|t| t.liquidity).unwrap_or(500.0);

        let cfg = self.config.read();

        // Run 7 Hard Filter Gates
        let fresh_res = HardFilterEngine::check_freshness(is_fresh);
        let time_res = HardFilterEngine::check_timing(remaining_seconds, &cfg);
        let prob_res = HardFilterEngine::check_probability(calibrated_p, &cfg);
        let entry_res = HardFilterEngine::check_entry_price(market_ask, &cfg);
        let spread_res = HardFilterEngine::check_spread(spread, &cfg);
        let liq_res = HardFilterEngine::check_liquidity(total_liquidity, &cfg);
        let edge_res = HardFilterEngine::check_net_edge(net_edge, &cfg);

        // Compute Opportunity Score
        let breakdown = SignalScorer::compute_score(
            net_edge,
            calibrated_p,
            directed_obi,
            directed_momentum,
            remaining_seconds,
            cfg.min_net_edge,
            cfg.min_probability,
            &cfg.score_thresholds,
        );

        // Check first filter rejection or evaluate final action
        let (action, decision_reason) = if !fresh_res.is_pass() {
            (SignalAction::Skip, fresh_res.reason().unwrap_or("Freshness check failed").to_string())
        } else if !time_res.is_pass() {
            (SignalAction::Skip, time_res.reason().unwrap_or("Timing check failed").to_string())
        } else if !prob_res.is_pass() {
            (SignalAction::Skip, prob_res.reason().unwrap_or("Probability threshold not met").to_string())
        } else if !entry_res.is_pass() {
            (SignalAction::Skip, entry_res.reason().unwrap_or("Entry price ceiling exceeded").to_string())
        } else if !spread_res.is_pass() {
            (SignalAction::Skip, spread_res.reason().unwrap_or("Spread too wide").to_string())
        } else if !liq_res.is_pass() {
            (SignalAction::Skip, liq_res.reason().unwrap_or("Insufficient liquidity").to_string())
        } else if !edge_res.is_pass() {
            (SignalAction::Skip, edge_res.reason().unwrap_or("Insufficient net edge").to_string())
        } else if breakdown.total_score < cfg.score_thresholds.skip_below {
            (
                SignalAction::Skip,
                format!(
                    "Score {:.1} < threshold {:.1}",
                    breakdown.total_score, cfg.score_thresholds.skip_below
                ),
            )
        } else {
            let act = if is_up {
                SignalAction::BuyUp
            } else {
                SignalAction::BuyDown
            };
            (
                act,
                format!(
                    "PASSED all filters & score {:.1} >= {:.1} (NetEdge: {:.2}%, P: {:.1}%)",
                    breakdown.total_score,
                    cfg.score_thresholds.skip_below,
                    net_edge * 100.0,
                    calibrated_p * 100.0
                ),
            )
        };

        let confidence = match breakdown.tier {
            ScoreTier::Skip => ConfidenceLevel::Skip,
            ScoreTier::Low => ConfidenceLevel::Low,
            ScoreTier::Medium => ConfidenceLevel::Medium,
            ScoreTier::High => ConfidenceLevel::High,
            ScoreTier::VeryHigh => ConfidenceLevel::VeryHigh,
        };

        let prediction_id = format!("pred_{}", Uuid::new_v4().simple());
        let market_implied_up = polymarket_tick.and_then(|t| t.up_mid).or(Some(0.50));

        let signal = PredictionSignal {
            prediction_id,
            market_id: prediction.market_id.clone(),
            asset: prediction.asset,
            timestamp_ms: prediction.timestamp_ms,
            model_version: prediction.model_version.clone(),
            p_up: prediction.calibrated_p_up,
            p_down: prediction.calibrated_p_down,
            fair_value_up,
            fair_value_down,
            market_implied_up,
            gross_edge,
            estimated_fee,
            estimated_slippage,
            net_edge,
            signal_score: breakdown.total_score,
            confidence,
            action,
            decision_reason: decision_reason.clone(),
        };

        let decision = DecisionLog {
            timestamp_ms: prediction.timestamp_ms,
            market_id: prediction.market_id.clone(),
            asset: prediction.asset,
            p_up: prediction.calibrated_p_up,
            p_down: prediction.calibrated_p_down,
            up_ask: polymarket_tick.and_then(|t| t.up_ask),
            down_ask: polymarket_tick.and_then(|t| t.down_ask),
            gross_edge,
            fee: estimated_fee,
            slippage: estimated_slippage,
            net_edge,
            liquidity_check: if liq_res.is_pass() { "PASS".to_string() } else { liq_res.reason().unwrap().to_string() },
            spread_check: if spread_res.is_pass() { "PASS".to_string() } else { spread_res.reason().unwrap().to_string() },
            time_check: if time_res.is_pass() { "PASS".to_string() } else { time_res.reason().unwrap().to_string() },
            risk_check: "PASS".to_string(),
            final_action: action,
            reason: decision_reason,
        };

        (signal, decision)
    }

    /// Process a new prediction: evaluate, cache in-memory, persist to database, and broadcast
    pub async fn process_prediction(
        &self,
        prediction: &ModelPrediction,
        polymarket_tick: Option<&PolymarketTick>,
        polymarket_book: Option<&OrderBookSnapshot>,
        composite_return_60s: f64,
        is_fresh: bool,
        remaining_seconds: i64,
    ) -> (PredictionSignal, DecisionLog) {
        let (signal, decision) = self.evaluate(
            prediction,
            polymarket_tick,
            polymarket_book,
            composite_return_60s,
            is_fresh,
            remaining_seconds,
        );

        // Update latest cache
        self.latest_signals.insert(prediction.asset, signal.clone());

        // Append to recent decisions ring-buffer
        {
            let mut recents = self.recent_decisions.write().await;
            if recents.len() >= 500 {
                recents.pop_front();
            }
            recents.push_back(decision.clone());
        }

        // Persist to database predictions table only for active signals or periodic 10s audit sampling
        if signal.action != SignalAction::Skip || signal.timestamp_ms % 10_000 < 150 {
            if let Err(e) = self.db.insert_prediction(&signal).await {
                warn!("Failed to persist prediction signal to database: {:#}", e);
            }
        }

        // Broadcast to consumers (e.g. Paper Trading Engine & WebSocket clients)
        let _ = self.signal_tx.send(signal.clone());

        (signal, decision)
    }

    /// Background task runner listening to ModelPrediction channel
    pub fn start(
        &self,
        mut prediction_rx: broadcast::Receiver<ModelPrediction>,
        poly_books: Arc<PolymarketBookEngine>,
        composite_engine: Arc<CompositePriceEngine>,
        freshness: FreshnessTracker,
    ) {
        info!("Starting StrategyEngine decision loop with hard filters and opportunity scoring...");

        let engine = self.clone();
        tokio::spawn(async move {
            while let Ok(pred) = prediction_rx.recv().await {
                let now_ms = Utc::now().timestamp_millis();
                let remaining_seconds = 300 - ((now_ms / 1000) % 300);

                let is_fresh = freshness.get_report().is_system_fresh;

                // Fetch latest Polymarket orderbook & summary
                let book_summary = poly_books.get_market_summary(&pred.market_id, pred.asset);
                let poly_tick = book_summary.as_ref().map(|s| PolymarketTick {
                    market_id: pred.market_id.clone(),
                    asset: pred.asset,
                    timestamp_ms: now_ms,
                    up_bid: s.up_book.best_bid,
                    up_ask: s.up_book.best_ask,
                    down_bid: s.down_book.best_bid,
                    down_ask: s.down_book.best_ask,
                    up_mid: s.up_book.mid,
                    down_mid: s.down_book.mid,
                    spread: s.up_book.spread,
                    volume_24h: Some(s.up_book.total_bid_depth_usdc + s.up_book.total_ask_depth_usdc),
                    liquidity: Some(s.up_book.total_bid_depth_usdc + s.down_book.total_bid_depth_usdc),
                });

                let composite_snap = composite_engine.get_latest_snapshot(pred.asset);
                let return_60s = composite_snap.map(|c| c.return_60s).unwrap_or(0.0);

                let book_snapshot = book_summary.as_ref().map(|s| OrderBookSnapshot {
                    exchange: crate::types::Exchange::Polymarket,
                    symbol: format!("{}-UP", pred.asset),
                    timestamp_ms: now_ms,
                    bids: s.up_book.bids.clone(),
                    asks: s.up_book.asks.clone(),
                    obi_top5: s.up_book.obi_top5,
                    obi_top10: s.up_book.obi_top10,
                    obi_top20: s.up_book.obi_top20,
                });

                let (sig, _) = engine
                    .process_prediction(
                        &pred,
                        poly_tick.as_ref(),
                        book_snapshot.as_ref(),
                        return_60s,
                        is_fresh,
                        remaining_seconds,
                    )
                    .await;

                if sig.action != SignalAction::Skip {
                    info!(
                        "⚡ STRATEGY SIGNAL GENERATED: {} on {} (Score: {:.1}, NetEdge: {:.2}%, P: {:.1}%)",
                        sig.action, sig.asset, sig.signal_score, sig.net_edge * 100.0,
                        if sig.action == SignalAction::BuyUp { sig.p_up * 100.0 } else { sig.p_down * 100.0 }
                    );
                } else {
                    debug!("Strategy SKIP for {}: {}", sig.asset, sig.decision_reason);
                }
            }
        });
    }

    pub fn get_latest_signal(&self, asset: Asset) -> Option<PredictionSignal> {
        self.latest_signals.get(&asset).map(|s| s.clone())
    }

    pub fn get_all_latest_signals(&self) -> HashMap<String, PredictionSignal> {
        let mut map = HashMap::new();
        for entry in self.latest_signals.iter() {
            map.insert(entry.key().to_string(), entry.value().clone());
        }
        map
    }

    pub async fn get_recent_decisions(&self, limit: usize) -> Vec<DecisionLog> {
        let recents = self.recent_decisions.read().await;
        recents.iter().rev().take(limit).cloned().collect()
    }

    pub fn subscribe_signals(&self) -> broadcast::Receiver<PredictionSignal> {
        self.signal_tx.subscribe()
    }
}
