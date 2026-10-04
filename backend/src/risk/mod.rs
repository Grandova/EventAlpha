use chrono::Utc;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::config::{BankrollConfig, RiskConfig};
use crate::db::Database;
use crate::types::{BankrollHistoryEntry, BankrollMode, BankrollState, RiskStatus};

#[derive(Clone)]
pub struct RiskManager {
    bankroll_config: BankrollConfig,
    risk_config: RiskConfig,
    state: Arc<RwLock<BankrollState>>,
    cooldown_until_ms: Arc<RwLock<Option<i64>>>,
    db: Arc<Database>,
    bankroll_tx: broadcast::Sender<BankrollState>,
}

impl RiskManager {
    pub fn new(
        bankroll_config: BankrollConfig,
        risk_config: RiskConfig,
        initial_state: BankrollState,
        db: Arc<Database>,
    ) -> Self {
        let (bankroll_tx, _) = broadcast::channel(1024);
        Self {
            bankroll_config,
            risk_config,
            state: Arc::new(RwLock::new(initial_state)),
            cooldown_until_ms: Arc::new(RwLock::new(None)),
            db,
            bankroll_tx,
        }
    }

    /// Pre-trade risk check: verify halted status, cooldowns, drawdown, daily loss, and capital limits
    pub async fn can_open_position(&self, requested_stake: f64) -> Result<(), String> {
        let state = self.state.read().await;
        let now_ms = Utc::now().timestamp_millis();

        // 1. Trading Halted Circuit Breaker
        if state.is_trading_halted {
            return Err(format!(
                "Trading halted: {}",
                state.halt_reason.as_deref().unwrap_or("System safety halt")
            ));
        }

        // 2. Consecutive Loss Cooldown Check
        if let Some(cooldown_end) = *self.cooldown_until_ms.read().await {
            if now_ms < cooldown_end {
                let rem_sec = (cooldown_end - now_ms) / 1000;
                return Err(format!(
                    "Risk cooldown active: {}s remaining due to {} consecutive losses",
                    rem_sec, state.consecutive_losses
                ));
            }
        }

        // 3. Daily Loss Limit Check
        if state.daily_loss_current >= self.risk_config.daily_loss_limit {
            return Err(format!(
                "Daily loss limit reached: ${:.2} >= limit ${:.2}",
                state.daily_loss_current, self.risk_config.daily_loss_limit
            ));
        }

        // 4. Max Drawdown Check
        if state.current_drawdown >= self.risk_config.max_drawdown {
            return Err(format!(
                "Max drawdown limit breached: {:.1}% >= limit {:.1}%",
                state.current_drawdown * 100.0,
                self.risk_config.max_drawdown * 100.0
            ));
        }

        // 5. Active Bankroll Check
        if state.active_bankroll < requested_stake {
            return Err(format!(
                "Insufficient active bankroll: ${:.2} available < ${:.2} requested",
                state.active_bankroll, requested_stake
            ));
        }

        // 6. Minimum Bankroll Floor Protection
        if (state.active_bankroll - requested_stake) < state.minimum_bankroll {
            return Err(format!(
                "Order would breach minimum bankroll floor: (${:.2} - ${:.2}) < ${:.2}",
                state.active_bankroll, requested_stake, state.minimum_bankroll
            ));
        }

        Ok(())
    }

    /// Deduct active bankroll when an order is executed
    pub async fn reserve_stake(
        &self,
        stake: f64,
        reason: &str,
        trade_id: Option<&str>,
    ) -> Result<BankrollState, String> {
        let mut state = self.state.write().await;

        if state.active_bankroll < stake {
            return Err(format!(
                "Cannot reserve stake ${:.2}; active bankroll is ${:.2}",
                stake, state.active_bankroll
            ));
        }

        state.active_bankroll -= stake;
        state.total_equity = state.active_bankroll + state.locked_profit;

        // Check if floor was reached
        if state.active_bankroll <= state.minimum_bankroll {
            state.is_trading_halted = true;
            state.halt_reason = Some(format!(
                "Active bankroll ${:.2} <= minimum floor ${:.2}",
                state.active_bankroll, state.minimum_bankroll
            ));
            warn!("🚨 TRADING HALTED: {}", state.halt_reason.as_ref().unwrap());
        }

        // Record in SQLite ledger
        let _ = self
            .db
            .record_bankroll_entry(
                state.active_bankroll,
                state.locked_profit,
                state.total_equity,
                -stake,
                reason,
                trade_id,
            )
            .await;

        let snapshot = state.clone();
        let _ = self.bankroll_tx.send(snapshot.clone());
        Ok(snapshot)
    }

    /// Process settlement of a paper position with Mode B (Capital Recovery) or Mode A (Profit Isolation)
    pub async fn process_settlement(
        &self,
        returned_stake: f64,
        net_profit: f64,
        outcome: &str,
        trade_id: Option<&str>,
    ) -> Result<BankrollState, String> {
        let mut state = self.state.write().await;
        let now_ms = Utc::now().timestamp_millis();

        let net_pnl = returned_stake + net_profit - returned_stake; // net_profit

        if net_profit > 0.0 {
            // Winning Trade
            state.consecutive_losses = 0;

            match state.mode {
                BankrollMode::CapitalRecovery => {
                    // Mode B: Capital Recovery
                    // 1. Return stake to active bankroll
                    state.active_bankroll += returned_stake;

                    // 2. Use net profit to replenish active bankroll up to cap first
                    let deficit = (state.bankroll_cap - state.active_bankroll).max(0.0);
                    let to_replenish = net_profit.min(deficit);
                    let to_lock = (net_profit - to_replenish).max(0.0);

                    state.active_bankroll += to_replenish;
                    state.locked_profit += to_lock;

                    info!(
                        "💰 MODE B RECOVERY: Trade WIN (+${:.2}) | Replenished to Active: +${:.2} (Now: ${:.2}) | Locked to Profit: +${:.2} (Total Locked: ${:.2})",
                        net_profit, to_replenish, state.active_bankroll, to_lock, state.locked_profit
                    );
                }
                BankrollMode::ProfitIsolation => {
                    // Mode A: Profit Isolation
                    // Return stake to active, lock 100% of profit immediately
                    state.active_bankroll += returned_stake;
                    state.locked_profit += net_profit;

                    info!(
                        "🔒 MODE A PROFIT ISOLATION: Trade WIN (+${:.2}) | 100% Locked to Profit (Total Locked: ${:.2})",
                        net_profit, state.locked_profit
                    );
                }
            }
        } else if net_profit < 0.0 {
            // Losing Trade
            let loss = -net_profit;
            state.active_bankroll += returned_stake; // If any partial return
            state.consecutive_losses += 1;
            state.daily_loss_current += loss;

            warn!(
                "📉 Trade LOSS (-${:.2}) | Consecutive Losses: {} | Daily Loss: ${:.2}/${:.2}",
                loss, state.consecutive_losses, state.daily_loss_current, self.risk_config.daily_loss_limit
            );

            // Circuit Breaker: Consecutive Loss Cooldown
            if state.consecutive_losses >= self.risk_config.max_consecutive_losses {
                let cooldown_ms = now_ms + (self.risk_config.cooldown_minutes as i64 * 60 * 1000);
                *self.cooldown_until_ms.write().await = Some(cooldown_ms);
                warn!(
                    "⏸️ RISK CIRCUIT BREAKER: {} consecutive losses hit! Cooldown for {} minutes.",
                    state.consecutive_losses, self.risk_config.cooldown_minutes
                );
            }

            // Circuit Breaker: Daily Loss Limit
            if state.daily_loss_current >= self.risk_config.daily_loss_limit {
                state.is_trading_halted = true;
                if state.halt_reason.is_none() {
                    state.halt_reason = Some(format!(
                        "Daily loss limit reached: ${:.2} >= limit ${:.2}",
                        state.daily_loss_current, self.risk_config.daily_loss_limit
                    ));
                    warn!("🚨 TRADING HALTED: {}", state.halt_reason.as_ref().unwrap());
                }
            }

            // Circuit Breaker: Minimum Active Bankroll
            if state.active_bankroll <= state.minimum_bankroll {
                state.is_trading_halted = true;
                if state.halt_reason.is_none() {
                    state.halt_reason = Some(format!(
                        "Active bankroll ${:.2} <= minimum floor ${:.2}",
                        state.active_bankroll, state.minimum_bankroll
                    ));
                    warn!("🚨 TRADING HALTED: {}", state.halt_reason.as_ref().unwrap());
                }
            }
        } else {
            // Void / Breakeven
            state.active_bankroll += returned_stake;
        }

        // Recalculate Total Equity and Drawdown
        state.total_equity = state.active_bankroll + state.locked_profit;
        if state.total_equity > state.peak_equity {
            state.peak_equity = state.total_equity;
        }

        state.current_drawdown = if state.peak_equity > 0.0 {
            (state.peak_equity - state.total_equity) / state.peak_equity
        } else {
            0.0
        };

        // Circuit Breaker: Max Drawdown
        if state.current_drawdown >= self.risk_config.max_drawdown {
            state.is_trading_halted = true;
            if state.halt_reason.is_none() {
                state.halt_reason = Some(format!(
                    "Max drawdown {:.1}% breached limit {:.1}%",
                    state.current_drawdown * 100.0,
                    self.risk_config.max_drawdown * 100.0
                ));
                warn!("🚨 TRADING HALTED: {}", state.halt_reason.as_ref().unwrap());
            }
        }

        let change_amount = net_pnl;
        let reason_str = format!("SETTLEMENT_{}", outcome.to_uppercase());

        // Record in SQLite ledger
        let _ = self
            .db
            .record_bankroll_entry(
                state.active_bankroll,
                state.locked_profit,
                state.total_equity,
                change_amount,
                &reason_str,
                trade_id,
            )
            .await;

        let snapshot = state.clone();
        let _ = self.bankroll_tx.send(snapshot.clone());
        Ok(snapshot)
    }

    /// Reset daily loss counter (e.g. at 00:00 UTC)
    pub async fn reset_daily_loss(&self) {
        let mut state = self.state.write().await;
        state.daily_loss_current = 0.0;
        if state.is_trading_halted {
            if let Some(ref reason) = state.halt_reason {
                if reason.contains("Daily loss") {
                    state.is_trading_halted = false;
                    state.halt_reason = None;
                    info!("Daily loss reset: trading halt lifted.");
                }
            }
        }
    }

    /// Set bankroll management mode (Mode A / Mode B)
    pub async fn set_mode(&self, mode: BankrollMode) {
        let mut state = self.state.write().await;
        state.mode = mode;
    }

    /// Reset consecutive loss counter
    pub async fn reset_consecutive_losses(&self) {
        let mut state = self.state.write().await;
        state.consecutive_losses = 0;
        *self.cooldown_until_ms.write().await = None;
    }

    /// Reset trading halt status
    pub async fn reset_trading_halt(&self) {
        let mut state = self.state.write().await;
        state.is_trading_halted = false;
        state.halt_reason = None;
    }

    /// Manually adjust or set active bankroll funds, lifting any minimum floor halts
    pub async fn update_bankroll_funds(
        &self,
        active_bankroll: f64,
        bankroll_cap: Option<f64>,
        minimum_bankroll: Option<f64>,
    ) -> Result<BankrollState, String> {
        if active_bankroll <= 0.0 {
            return Err("资金金额必须大于 0".to_string());
        }

        let mut state = self.state.write().await;
        let cap = bankroll_cap.unwrap_or(active_bankroll).max(active_bankroll);
        let min_floor = minimum_bankroll.unwrap_or(0.0).min(active_bankroll * 0.5);

        let prev_active = state.active_bankroll;
        state.active_bankroll = active_bankroll;
        state.bankroll_cap = cap;
        state.minimum_bankroll = min_floor;
        state.total_equity = state.active_bankroll + state.locked_profit;
        state.peak_equity = state.peak_equity.max(state.total_equity);
        state.current_drawdown = 0.0;
        state.daily_loss_current = 0.0;
        state.consecutive_losses = 0;
        state.is_trading_halted = false;
        state.halt_reason = None;

        *self.cooldown_until_ms.write().await = None;

        let delta = active_bankroll - prev_active;
        let snapshot = state.clone();

        let _ = self
            .db
            .record_bankroll_entry(
                snapshot.active_bankroll,
                snapshot.locked_profit,
                snapshot.total_equity,
                delta,
                "MANUAL_SET_FUNDS",
                None,
            )
            .await;

        let _ = self.bankroll_tx.send(snapshot.clone());
        info!(
            "Bankroll manually updated to {:.2} USDC (Cap: {:.2} USDC, Min floor: {:.2} USDC)",
            active_bankroll, cap, min_floor
        );

        Ok(snapshot)
    }

    pub async fn get_bankroll_state(&self) -> BankrollState {
        self.state.read().await.clone()
    }

    pub async fn get_risk_status(&self) -> RiskStatus {
        let state = self.state.read().await;
        let cooldown = *self.cooldown_until_ms.read().await;

        RiskStatus {
            is_trading_halted: state.is_trading_halted,
            halt_reason: state.halt_reason.clone(),
            daily_loss_current: state.daily_loss_current,
            daily_loss_limit: self.risk_config.daily_loss_limit,
            current_drawdown: state.current_drawdown,
            max_drawdown_limit: self.risk_config.max_drawdown,
            peak_equity: state.peak_equity,
            consecutive_losses: state.consecutive_losses,
            max_consecutive_losses: self.risk_config.max_consecutive_losses,
            cooldown_until_ms: cooldown,
        }
    }

    pub async fn get_history(&self, limit: i64) -> anyhow::Result<Vec<BankrollHistoryEntry>> {
        self.db.get_bankroll_history(limit).await
    }

    pub fn bankroll_config(&self) -> &BankrollConfig {
        &self.bankroll_config
    }

    pub fn risk_config(&self) -> &RiskConfig {
        &self.risk_config
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BankrollState> {
        self.bankroll_tx.subscribe()
    }
}
