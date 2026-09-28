use chrono::Utc;
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::config::{ExecutionConfig, PositionConfig};
use crate::db::Database;
use crate::polymarket::orderbook::PolymarketBookEngine;
use crate::safety::SafetyGuard;
use crate::types::{
    MarketSide, PaperOrder, PaperPosition, PredictionSignal, SignalAction,
};

#[derive(Clone)]
pub struct PaperExecutionEngine {
    execution_config: ExecutionConfig,
    position_config: PositionConfig,
    db: Arc<Database>,
    poly_books: Arc<PolymarketBookEngine>,
    // position_id -> PaperPosition
    active_positions: Arc<DashMap<String, PaperPosition>>,
    // Recent orders ring buffer
    order_history: Arc<RwLock<VecDeque<PaperOrder>>>,
    order_tx: broadcast::Sender<PaperOrder>,
    position_tx: broadcast::Sender<PaperPosition>,
    risk: Arc<RwLock<Option<Arc<crate::risk::RiskManager>>>>,
}

impl PaperExecutionEngine {
    pub fn new(
        execution_config: ExecutionConfig,
        position_config: PositionConfig,
        db: Arc<Database>,
        poly_books: Arc<PolymarketBookEngine>,
    ) -> Self {
        // Enforce paper trading only invariant
        SafetyGuard::enforce_paper_only(false).expect("SafetyGuard invariant violation!");

        let (order_tx, _) = broadcast::channel(1024);
        let (position_tx, _) = broadcast::channel(1024);

        Self {
            execution_config,
            position_config,
            db,
            poly_books,
            active_positions: Arc::new(DashMap::new()),
            order_history: Arc::new(RwLock::new(VecDeque::with_capacity(500))),
            order_tx,
            position_tx,
            risk: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_risk_manager(&self, risk: Arc<crate::risk::RiskManager>) {
        let mut r = self.risk.write().await;
        *r = Some(risk);
    }

    /// Execute a trading signal using realistic orderbook depth-walking and simulated latency
    pub async fn execute_signal(
        &self,
        signal: &PredictionSignal,
    ) -> Result<Option<(PaperOrder, PaperPosition)>, String> {
        // Enforce safety guard: zero real trading
        SafetyGuard::enforce_paper_only(false).map_err(|e| e.to_string())?;

        let target_side = match signal.action {
            SignalAction::BuyUp => MarketSide::Up,
            SignalAction::BuyDown => MarketSide::Down,
            SignalAction::Skip => return Ok(None),
        };

        let now_ms = Utc::now().timestamp_millis();

        // Check if there is already an active position for this market
        for entry in self.active_positions.iter() {
            if entry.value().market_id == signal.market_id && entry.value().status == "OPEN" {
                info!(
                    "Position already open for market {} (Asset: {}), skipping duplicate entry",
                    signal.market_id, signal.asset
                );
                return Ok(None);
            }
        }

        // Determine requested stake
        let requested_stake = self.position_config.stake.min(self.position_config.max_stake);
        if requested_stake <= 0.0 {
            return Err("Requested stake must be positive".to_string());
        }

        // Pre-trade Risk & Circuit Breaker Check
        {
            let r_guard = self.risk.read().await;
            if let Some(ref risk) = *r_guard {
                if let Err(reason) = risk.can_open_position(requested_stake).await {
                    warn!("🛑 RiskManager rejected order for {}: {}", signal.asset, reason);
                    return Ok(None);
                }
            }
        }

        // Simulate network / order transmission latency
        if self.execution_config.latency_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.execution_config.latency_ms)).await;
        }

        // Execute realistic Depth-Walking fill simulation against Polymarket orderbook
        let (quote_price, fill_price, total_shares, slippage, fill_status) = if self.execution_config.orderbook_depth_fill {
            match self.poly_books.simulate_buy_fill(signal.asset, target_side, requested_stake) {
                Ok(fill) => (
                    fill.quote_price,
                    fill.avg_fill_price,
                    fill.total_shares,
                    fill.slippage,
                    "FILLED".to_string(),
                ),
                Err(err) => {
                    warn!(
                        "Order fill simulation rejected for {} {:?}: {}",
                        signal.asset, target_side, err
                    );
                    let order_id = format!("ord_{}", Uuid::new_v4().simple());
                    let rejected_order = PaperOrder {
                        order_id,
                        market_id: signal.market_id.clone(),
                        asset: signal.asset,
                        side: target_side,
                        stake: requested_stake,
                        shares: 0.0,
                        quote_price: 0.0,
                        fill_price: 0.0,
                        slippage: 0.0,
                        fee: 0.0,
                        status: format!("REJECTED: {}", err),
                        signal_id: Some(signal.prediction_id.clone()),
                        timestamp_ms: now_ms,
                    };
                    let _ = self.db.insert_paper_order(&rejected_order).await;
                    let _ = self.order_tx.send(rejected_order);
                    return Ok(None);
                }
            }
        } else {
            // Simplified fallback fill
            let quote = match target_side {
                MarketSide::Up => signal.p_up.min(0.95),
                MarketSide::Down => signal.p_down.min(0.95),
            };
            let fill = (quote * (1.0 + self.execution_config.slippage_rate)).min(0.99);
            let shares = requested_stake / fill;
            let slip = (fill - quote).max(0.0);
            (quote, fill, shares, slip, "FILLED".to_string())
        };

        // Fee deduction: 1.2%
        let fee = requested_stake * self.execution_config.fee_rate;

        let order_id = format!("ord_{}", Uuid::new_v4().simple());
        let position_id = format!("pos_{}", Uuid::new_v4().simple());

        let order = PaperOrder {
            order_id: order_id.clone(),
            market_id: signal.market_id.clone(),
            asset: signal.asset,
            side: target_side,
            stake: requested_stake,
            shares: total_shares,
            quote_price,
            fill_price,
            slippage,
            fee,
            status: fill_status,
            signal_id: Some(signal.prediction_id.clone()),
            timestamp_ms: now_ms,
        };

        let position = PaperPosition {
            position_id: position_id.clone(),
            order_id: order_id.clone(),
            market_id: signal.market_id.clone(),
            asset: signal.asset,
            side: target_side,
            entry_time_ms: now_ms,
            entry_price: fill_price,
            stake: requested_stake,
            shares: total_shares,
            status: "OPEN".to_string(),
            settled_at_ms: None,
            created_at_ms: now_ms,
        };

        // Persist to database
        if let Err(e) = self.db.insert_paper_order(&order).await {
            error!("Failed to persist paper order: {:#}", e);
        }
        if let Err(e) = self.db.insert_paper_position(&position).await {
            error!("Failed to persist paper position: {:#}", e);
        }

        // Deduct stake from active bankroll
        {
            let r_guard = self.risk.read().await;
            if let Some(ref risk) = *r_guard {
                let _ = risk
                    .reserve_stake(order.stake, "PAPER_ORDER_OPEN", Some(&order.order_id))
                    .await;
            }
        }

        // Cache active position and order history
        self.active_positions.insert(position_id.clone(), position.clone());
        {
            let mut hist = self.order_history.write().await;
            if hist.len() >= 500 {
                hist.pop_front();
            }
            hist.push_back(order.clone());
        }

        // Broadcast events
        let _ = self.order_tx.send(order.clone());
        let _ = self.position_tx.send(position.clone());

        info!(
            "🎯 PAPER ORDER FILLED: {} {} | Stake: {:.2} USDC | Price: {:.4} (Slip: {:.4}) | Shares: {:.2} | Fee: {:.4} USDC",
            order.side, order.asset, order.stake, order.fill_price, order.slippage, order.shares, order.fee
        );

        Ok(Some((order, position)))
    }

    /// Background runner subscribing to StrategyEngine prediction signals
    pub fn start(&self, mut signal_rx: broadcast::Receiver<PredictionSignal>) {
        info!("Starting PaperExecutionEngine order listening and fill loop...");

        let engine = self.clone();
        tokio::spawn(async move {
            while let Ok(signal) = signal_rx.recv().await {
                if signal.action != SignalAction::Skip {
                    let _ = engine.execute_signal(&signal).await;
                }
            }
        });
    }

    pub fn get_active_positions(&self) -> Vec<PaperPosition> {
        self.active_positions
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub async fn get_recent_orders(&self, limit: usize) -> Vec<PaperOrder> {
        let hist = self.order_history.read().await;
        hist.iter().rev().take(limit).cloned().collect()
    }

    pub async fn get_position_history(&self, limit: i64) -> anyhow::Result<Vec<PaperPosition>> {
        self.db.get_positions_history(limit).await
    }

    pub fn close_position(&self, position_id: &str) {
        self.active_positions.remove(position_id);
    }

    pub fn subscribe_orders(&self) -> broadcast::Receiver<PaperOrder> {
        self.order_tx.subscribe()
    }

    pub fn subscribe_positions(&self) -> broadcast::Receiver<PaperPosition> {
        self.position_tx.subscribe()
    }
}
