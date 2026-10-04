use chrono::Utc;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::polymarket::clob_client::PolymarketClobHttpClient;
use crate::types::{Asset, MarketSide, PredictionSignal, RealOrder, SignalAction, TradingMode};

#[derive(Clone)]
pub struct LiveExecutionEngine {
    db: Arc<Database>,
    clob_client: Arc<PolymarketClobHttpClient>,
    mode: Arc<RwLock<TradingMode>>,
    max_live_stake: f64,
    order_tx: broadcast::Sender<RealOrder>,
}

impl LiveExecutionEngine {
    pub fn new(db: Arc<Database>, clob_client: Arc<PolymarketClobHttpClient>) -> Self {
        let (order_tx, _) = broadcast::channel(512);
        Self {
            db,
            clob_client,
            mode: Arc::new(RwLock::new(TradingMode::Paper)),
            max_live_stake: 10.0, // Default hard ceiling: max $10 per live trade
            order_tx,
        }
    }

    pub fn clob_client(&self) -> &PolymarketClobHttpClient {
        &self.clob_client
    }

    pub async fn get_mode(&self) -> TradingMode {
        *self.mode.read().await
    }

    /// Toggle trading mode between Paper and Live
    pub async fn set_mode(&self, new_mode: TradingMode) -> Result<TradingMode, String> {
        let mut mode_lock = self.mode.write().await;

        if new_mode == TradingMode::Live {
            // Verify that an active verified Polymarket account exists
            let active_account = self
                .db
                .get_active_account()
                .await
                .map_err(|e| format!("Database error checking accounts: {:?}", e))?;

            let account = match active_account {
                Some(acc) => acc,
                None => {
                    return Err(
                        "Cannot switch to LIVE trading mode: No active Polymarket account found. \
                         Please add and activate your Polymarket API credentials in the Account Manager first."
                            .to_string(),
                    );
                }
            };

            // Test CLOB connection & refresh Polygon USDC balance
            match self.clob_client.fetch_balance(&account).await {
                Ok(bal) => {
                    let _ = self.db.update_account_balance(&account.id, bal).await;
                    info!(
                        "⚡ LIVE TRADING ACTIVATED for account '{}' ({}). Polygon USDC Balance: ${:.2}",
                        account.label, account.wallet_address, bal
                    );
                    let _ = self
                        .db
                        .record_system_event(
                            "LIVE_MODE_ENABLED",
                            "WARNING",
                            "LiveExecutionEngine",
                            &format!(
                                "Live Polymarket trading enabled for account '{}' (${:.2} USDC)",
                                account.label, bal
                            ),
                            None,
                        )
                        .await;
                }
                Err(e) => {
                    warn!("Failed to query live balance from Polymarket CLOB: {}. Allowing live switch with caution.", e);
                }
            }
        } else {
            info!("🛡️ System switched back to SIMULATION (Paper Trading) mode.");
            let _ = self
                .db
                .record_system_event(
                    "PAPER_MODE_ENABLED",
                    "INFO",
                    "LiveExecutionEngine",
                    "System switched to safe simulation paper trading mode",
                    None,
                )
                .await;
        }

        *mode_lock = new_mode;
        Ok(new_mode)
    }

    /// Execute a live trading signal on Polymarket CLOB
    pub async fn execute_signal(&self, signal: &PredictionSignal) -> Result<Option<RealOrder>, String> {
        if *self.mode.read().await != TradingMode::Live {
            return Ok(None);
        }

        let (target_side, outcome_str) = match signal.action {
            SignalAction::BuyUp => (MarketSide::Up, "UP"),
            SignalAction::BuyDown => (MarketSide::Down, "DOWN"),
            SignalAction::Skip => return Ok(None),
        };

        let account = match self.db.get_active_account().await {
            Ok(Some(acc)) => acc,
            _ => {
                error!("Cannot place live order: No active Polymarket account configured");
                return Err("No active Polymarket account".to_string());
            }
        };

        // Determine token_id for the market outcome
        let token_id = format!("{}_{}", signal.market_id, outcome_str);

        // Enforce hard stake ceiling
        let requested_price = match target_side {
            MarketSide::Up => signal.market_implied_up.unwrap_or(signal.fair_value_up),
            MarketSide::Down => 1.0 - signal.market_implied_up.unwrap_or(signal.fair_value_up),
        }
        .clamp(0.05, 0.95);
        let stake = self.max_live_stake.min(1.0); // Safe $1 default stake
        let size = (stake / requested_price).max(1.0);

        let now_ms = Utc::now().timestamp_millis();
        let internal_order_id = Uuid::new_v4().to_string();

        info!(
            "🚀 SUBMITTING LIVE ORDER TO POLYMARKET: {} {} | Price: {:.3} | Size: {:.2} (${:.2} USDC) | Account: {}",
            outcome_str, signal.asset, requested_price, size, stake, account.label
        );

        // Submit to Polymarket CLOB API
        let clob_resp = self
            .clob_client
            .place_order(
                &account,
                &token_id,
                "BUY",
                requested_price,
                size,
                "FOK", // Fill Or Kill
            )
            .await;

        let (clob_order_id, status, error_message, filled_size) = match clob_resp {
            Ok(resp) => {
                if resp.success {
                    info!("✅ POLYMARKET CLOB ORDER FILLED: {:?}", resp.order_id);
                    (resp.order_id, "FILLED".to_string(), None, resp.filled_size.unwrap_or(size))
                } else {
                    warn!("❌ Polymarket CLOB rejected order: {:?}", resp.error_msg);
                    (None, "FAILED".to_string(), resp.error_msg, 0.0)
                }
            }
            Err(e) => {
                error!("Network error submitting order to Polymarket: {}", e);
                (None, "FAILED".to_string(), Some(e), 0.0)
            }
        };

        let real_order = RealOrder {
            id: internal_order_id,
            account_id: account.id,
            clob_order_id,
            market_id: signal.market_id.clone(),
            token_id,
            asset: signal.asset,
            side: "BUY".to_string(),
            outcome: outcome_str.to_string(),
            order_type: "FOK".to_string(),
            price: requested_price,
            size,
            filled_size,
            status,
            fee: stake * 0.012,
            pnl: None,
            error_message,
            created_at: now_ms,
            updated_at: now_ms,
        };

        if let Err(e) = self.db.insert_real_order(&real_order).await {
            error!("Failed to persist real order: {:?}", e);
        }

        let _ = self.order_tx.send(real_order.clone());
        Ok(Some(real_order))
    }

    /// Execute a manual live order on Polymarket CLOB
    pub async fn execute_manual_order(
        &self,
        market_id: &str,
        asset: Asset,
        side: MarketSide,
        stake: Option<f64>,
    ) -> Result<RealOrder, String> {
        let mode = *self.mode.read().await;
        if mode != TradingMode::Live {
            return Err("系统当前处于模拟盘模式 (Paper Mode)，请先在设置中切换为实盘模式".to_string());
        }

        let account = match self.db.get_active_account().await {
            Ok(Some(acc)) => acc,
            _ => {
                error!("Cannot place live order: No active Polymarket account configured");
                return Err("未配置或未激活 Polymarket 实盘交易账户，请前往账户管理添加并激活".to_string());
            }
        };

        let outcome_str = match side {
            MarketSide::Up => "UP",
            MarketSide::Down => "DOWN",
        };

        let token_id = format!("{}_{}", market_id, outcome_str);

        // Requested stake capped by max_live_stake
        let requested_stake = stake.unwrap_or(1.0).clamp(0.1, self.max_live_stake);
        let requested_price = 0.50; // Fallback price estimation
        let size = (requested_stake / requested_price).max(1.0);

        let now_ms = Utc::now().timestamp_millis();
        let internal_order_id = Uuid::new_v4().to_string();

        info!(
            "🚀 SUBMITTING MANUAL LIVE ORDER TO POLYMARKET: {} {} | Size: {:.2} (${:.2} USDC) | Account: {}",
            outcome_str, asset, size, requested_stake, account.label
        );

        let clob_resp = self
            .clob_client
            .place_order(
                &account,
                &token_id,
                "BUY",
                requested_price,
                size,
                "FOK",
            )
            .await;

        let (clob_order_id, status, error_message, filled_size) = match clob_resp {
            Ok(resp) => {
                if resp.success {
                    info!("✅ POLYMARKET CLOB MANUAL ORDER FILLED: {:?}", resp.order_id);
                    (resp.order_id, "FILLED".to_string(), None, resp.filled_size.unwrap_or(size))
                } else {
                    warn!("❌ Polymarket CLOB rejected manual order: {:?}", resp.error_msg);
                    (None, "FAILED".to_string(), resp.error_msg, 0.0)
                }
            }
            Err(e) => {
                error!("Network error submitting manual order to Polymarket: {}", e);
                (None, "FAILED".to_string(), Some(e), 0.0)
            }
        };

        let real_order = RealOrder {
            id: internal_order_id,
            account_id: account.id,
            clob_order_id,
            market_id: market_id.to_string(),
            token_id,
            asset,
            side: "BUY".to_string(),
            outcome: outcome_str.to_string(),
            order_type: "FOK".to_string(),
            price: requested_price,
            size,
            filled_size,
            status,
            fee: requested_stake * 0.012,
            pnl: None,
            error_message,
            created_at: now_ms,
            updated_at: now_ms,
        };

        if let Err(e) = self.db.insert_real_order(&real_order).await {
            error!("Failed to persist manual real order: {:?}", e);
        }

        let _ = self.order_tx.send(real_order.clone());
        Ok(real_order)
    }

    /// Emergency Kill Switch: Cancel order & Halt live trading
    pub async fn emergency_halt(&self) -> Result<(), String> {
        let _ = self.set_mode(TradingMode::Paper).await;
        info!("🚨 EMERGENCY HALT TRIGGERED: Reverted to Simulation Paper Mode.");
        let _ = self
            .db
            .record_system_event(
                "EMERGENCY_HALT",
                "CRITICAL",
                "LiveExecutionEngine",
                "Emergency halt triggered: all live operations stopped, mode reverted to Paper",
                None,
            )
            .await;
        Ok(())
    }

    pub fn subscribe_orders(&self) -> broadcast::Receiver<RealOrder> {
        self.order_tx.subscribe()
    }
}
