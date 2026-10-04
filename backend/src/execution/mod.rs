pub mod live;
pub use live::LiveExecutionEngine;

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
use crate::polymarket::resolution::MarketResolvedEvent;
use crate::safety::SafetyGuard;
use crate::types::{
    MarketSide, PaperOrder, PaperPosition, PaperResult, PredictionSignal, Resolution,
    SignalAction, TradeStatistics,
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
    result_tx: broadcast::Sender<PaperResult>,
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
        let (result_tx, _) = broadcast::channel(1024);

        Self {
            execution_config,
            position_config,
            db,
            poly_books,
            active_positions: Arc::new(DashMap::new()),
            order_history: Arc::new(RwLock::new(VecDeque::with_capacity(500))),
            order_tx,
            position_tx,
            result_tx,
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

    pub fn subscribe_results(&self) -> broadcast::Receiver<PaperResult> {
        self.result_tx.subscribe()
    }

    /// Settle a single active position based on market resolution outcome
    pub async fn settle_position(
        &self,
        position_id: &str,
        resolution: Resolution,
        resolved_at_ms: i64,
    ) -> Result<Option<PaperResult>, String> {
        let position = match self.active_positions.get(position_id) {
            Some(p) => p.clone(),
            None => match self.db.get_position(position_id).await {
                Ok(Some(p)) => p,
                _ => return Ok(None),
            },
        };

        if position.status != "OPEN" {
            return Ok(None);
        }

        let (outcome_str, is_win, is_loss) = match resolution {
            Resolution::Up => {
                if position.side == MarketSide::Up {
                    ("WIN", true, false)
                } else {
                    ("LOSS", false, true)
                }
            }
            Resolution::Down => {
                if position.side == MarketSide::Down {
                    ("WIN", true, false)
                } else {
                    ("LOSS", false, true)
                }
            }
            Resolution::Void => ("VOID", false, false),
        };

        let (payout, net_profit, returned_stake) = if is_win {
            // Polymarket binary options pay $1.00 USDC per share upon winning
            let payout = position.shares * 1.0;
            let net_profit = payout - position.stake;
            let returned_stake = position.stake;
            (payout, net_profit, returned_stake)
        } else if is_loss {
            (0.0, -position.stake, 0.0)
        } else {
            // Void / Tie: full refund of initial stake
            (position.stake, 0.0, position.stake)
        };

        // Call RiskManager to update ledger, replenishing active bankroll or locking profit
        let (bankroll_before, bankroll_after, locked_profit_before, locked_profit_after) = {
            let r_guard = self.risk.read().await;
            if let Some(ref risk) = *r_guard {
                let b_before = risk.get_bankroll_state().await;
                let b_after = risk
                    .process_settlement(
                        returned_stake,
                        net_profit,
                        outcome_str,
                        Some(&position.position_id),
                    )
                    .await
                    .unwrap_or(b_before.clone());
                (
                    b_before.active_bankroll,
                    b_after.active_bankroll,
                    b_before.locked_profit,
                    b_after.locked_profit,
                )
            } else {
                (10.0, 10.0, 0.0, 0.0)
            }
        };

        // Update position in SQLite
        let status_str = format!("RESOLVED_{}", outcome_str);
        let _ = self
            .db
            .update_position_status(&position.position_id, &status_str, resolved_at_ms)
            .await;

        // Remove from in-memory active positions
        self.active_positions.remove(position_id);

        let mut settled_pos = position.clone();
        settled_pos.status = status_str.clone();
        settled_pos.settled_at_ms = Some(resolved_at_ms);

        // Broadcast position update
        let _ = self.position_tx.send(settled_pos.clone());

        let result = PaperResult {
            result_id: format!("res_{}", Uuid::new_v4().simple()),
            order_id: position.order_id.clone(),
            market_id: position.market_id.clone(),
            asset: position.asset,
            side: position.side,
            entry_time_ms: position.entry_time_ms,
            entry_price: position.entry_price,
            fill_price: position.entry_price,
            shares: position.shares,
            stake: position.stake,
            predicted_probability: 0.0,
            model_confidence: "NORMAL".to_string(),
            gross_edge: 0.0,
            net_edge: 0.0,
            fee: 0.0,
            slippage: 0.0,
            outcome: outcome_str.to_string(),
            payout,
            pnl: net_profit,
            bankroll_before,
            bankroll_after,
            locked_profit_before,
            locked_profit_after,
            strategy_version: "v1.0.0".to_string(),
            model_version: "v1.0.0".to_string(),
            created_at_ms: resolved_at_ms,
        };

        // Persist result to SQLite
        if let Err(e) = self.db.insert_paper_result(&result).await {
            error!("Failed to persist paper result: {:#}", e);
        }

        // Broadcast result event
        let _ = self.result_tx.send(result.clone());

        info!(
            "🏆 PAPER SETTLEMENT: {} {} {:?} | Outcome: {} | PnL: ${:+.2} | Payout: ${:.2} | Active Bankroll: ${:.2} -> ${:.2} | Locked: ${:.2} -> ${:.2}",
            position.asset, position.side, position.position_id, outcome_str, net_profit, payout,
            bankroll_before, bankroll_after, locked_profit_before, locked_profit_after
        );

        Ok(Some(result))
    }

    /// Settle all active positions associated with a resolved market
    pub async fn handle_market_resolved(
        &self,
        event: &MarketResolvedEvent,
    ) -> Result<Vec<PaperResult>, String> {
        let mut matching_pos_ids: Vec<String> = self
            .active_positions
            .iter()
            .filter(|entry| entry.value().market_id == event.market_id && entry.value().status == "OPEN")
            .map(|entry| entry.key().clone())
            .collect();

        // Also check database for any OPEN positions belonging to this market
        if let Ok(db_positions) = self.db.get_active_positions().await {
            for p in db_positions {
                if p.market_id == event.market_id && !matching_pos_ids.contains(&p.position_id) {
                    matching_pos_ids.push(p.position_id);
                }
            }
        }

        let mut results = Vec::new();
        for pos_id in matching_pos_ids {
            if let Ok(Some(res)) = self
                .settle_position(&pos_id, event.resolution, event.resolved_at_ms)
                .await
            {
                results.push(res);
            }
        }
        Ok(results)
    }

    /// Load persisted active open positions from SQLite into memory on system launch
    pub async fn load_active_positions_from_db(&self) {
        if let Ok(positions) = self.db.get_active_positions().await {
            let count = positions.len();
            for p in positions {
                self.active_positions.insert(p.position_id.clone(), p);
            }
            if count > 0 {
                info!("Loaded {} active open positions from database into memory.", count);
            }
        }
    }

    /// Background listener for Polymarket market resolution events
    pub fn start_resolution_listener(
        &self,
        mut resolution_rx: broadcast::Receiver<MarketResolvedEvent>,
    ) {
        info!("Starting PaperExecutionEngine real-time resolution listener...");
        let engine = self.clone();
        tokio::spawn(async move {
            while let Ok(event) = resolution_rx.recv().await {
                let _ = engine.handle_market_resolved(&event).await;
            }
        });
    }

    /// Periodically audits open positions to ensure none are left un-settled past 5 minutes
    pub fn start_position_expiry_audit(&self) {
        let engine = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(5));
            loop {
                ticker.tick().await;
                let now_ms = Utc::now().timestamp_millis();
                if let Ok(open_positions) = engine.db.get_active_positions().await {
                    for pos in open_positions {
                        // If position has been open longer than 5 minutes + 10s grace
                        if now_ms >= pos.entry_time_ms + 310_000 {
                            use sqlx::Row;
                            let market_row = sqlx::query(
                                "SELECT status, resolution, open_price, final_price FROM markets WHERE id = ?"
                            )
                            .bind(&pos.market_id)
                            .fetch_optional(engine.db.pool())
                            .await;

                            if let Ok(Some(r)) = market_row {
                                let status: String = r.get("status");
                                if status == "resolved" {
                                    let res_str: Option<String> = r.get("resolution");
                                    let res = match res_str.as_deref() {
                                        Some("UP") => Resolution::Up,
                                        Some("DOWN") => Resolution::Down,
                                        _ => Resolution::Void,
                                    };
                                    info!("Audit auto-settling position {} for resolved market {} as {:?}", pos.position_id, pos.market_id, res);
                                    let _ = engine.settle_position(&pos.position_id, res, now_ms).await;
                                } else {
                                    let open_p: Option<f64> = r.get("open_price");
                                    let final_p: Option<f64> = r.get("final_price");
                                    let open_val = open_p.unwrap_or(pos.entry_price);
                                    let final_val = final_p.unwrap_or(open_val);
                                    let res = if final_val > open_val {
                                        Resolution::Up
                                    } else if final_val < open_val {
                                        Resolution::Down
                                    } else {
                                        Resolution::Void
                                    };
                                    info!("Audit auto-resolving expired market {} and settling position {} as {:?}", pos.market_id, pos.position_id, res);
                                    let _ = engine.settle_position(&pos.position_id, res, now_ms).await;
                                }
                            } else {
                                info!("Audit refunding orphaned position {} as VOID", pos.position_id);
                                let _ = engine.settle_position(&pos.position_id, Resolution::Void, now_ms).await;
                            }
                        }
                    }
                }
            }
        });
    }

    pub async fn get_paper_results(&self, limit: i64) -> anyhow::Result<Vec<PaperResult>> {
        self.db.get_paper_results(limit).await
    }

    pub async fn get_trade_statistics(&self) -> anyhow::Result<TradeStatistics> {
        self.db.get_trade_statistics().await
    }
}
