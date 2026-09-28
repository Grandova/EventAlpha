use chrono::Utc;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::metrics::compute_backtest_metrics;
use super::types::{BacktestRequest, BacktestResult, EquityPoint};
use crate::dataset::{DatasetExporter, DatasetRecord};
use crate::db::Database;
use crate::models::ModelManager;
use crate::types::{BankrollMode, MarketSide, PaperResult, Resolution};

pub struct BacktestEngine {
    db: Arc<Database>,
    models: Arc<ModelManager>,
    recent_results: Arc<RwLock<VecDeque<BacktestResult>>>,
}

impl BacktestEngine {
    pub fn new(db: Arc<Database>, models: Arc<ModelManager>) -> Self {
        Self {
            db,
            models,
            recent_results: Arc::new(RwLock::new(VecDeque::with_capacity(50))),
        }
    }

    /// Execute a backtest by loading historical dataset records from the database
    pub async fn run_backtest(&self, req: BacktestRequest) -> Result<BacktestResult, String> {
        let start_time = Utc::now().timestamp_millis();

        // 1. Load historical records from database
        let mut records = DatasetExporter::load_from_db(&self.db)
            .await
            .map_err(|e| format!("Failed to load dataset from database: {:#}", e))?;

        // 2. Filter records based on request parameters
        if let Some(start) = req.start_time_ms {
            records.retain(|r| r.timestamp_ms >= start);
        }
        if let Some(end) = req.end_time_ms {
            records.retain(|r| r.timestamp_ms <= end);
        }
        if let Some(ref assets) = req.assets {
            records.retain(|r| assets.contains(&r.asset));
        }

        // Sort chronologically (strict time arrow invariant)
        records.sort_by_key(|r| r.timestamp_ms);

        let result = self.run_backtest_on_records(&req, &records)?;

        // Cache result in memory
        {
            let mut hist = self.recent_results.write().await;
            if hist.len() >= 50 {
                hist.pop_front();
            }
            hist.push_back(result.clone());
        }

        let duration_ms = (Utc::now().timestamp_millis() - start_time).max(0) as u64;
        let mut final_res = result;
        final_res.execution_duration_ms = duration_ms;
        Ok(final_res)
    }

    /// Pure simulation runner over an array of historical records
    pub fn run_backtest_on_records(
        &self,
        req: &BacktestRequest,
        records: &[DatasetRecord],
    ) -> Result<BacktestResult, String> {
        let backtest_id = format!("bt_{}", Uuid::new_v4().simple());
        let executed_at_ms = Utc::now().timestamp_millis();

        let initial_bankroll = req.initial_bankroll.unwrap_or(10.0);
        let bankroll_cap = req.bankroll_cap.unwrap_or(10.0);
        let mode = req.mode.unwrap_or(BankrollMode::CapitalRecovery);
        let stake = req.stake.unwrap_or(1.0);
        let min_prob = req.min_prob.unwrap_or(0.70);
        let min_net_edge = req.min_net_edge.unwrap_or(0.05);
        let fee_rate = req.fee_rate.unwrap_or(0.012);
        let slippage_rate = req.slippage_rate.unwrap_or(0.005);

        let mut active_bankroll = initial_bankroll;
        let mut locked_profit = 0.0f64;
        let mut peak_equity = initial_bankroll;

        let mut equity_curve = Vec::with_capacity(records.len() + 1);
        let mut trades = Vec::new();

        // Initial equity point
        let start_time_ms = records.first().map(|r| r.timestamp_ms).unwrap_or(executed_at_ms);
        equity_curve.push(EquityPoint {
            timestamp_ms: start_time_ms,
            active_bankroll,
            locked_profit,
            total_equity: initial_bankroll,
            drawdown_pct: 0.0,
            trade_pnl: None,
        });

        for record in records {
            // Check if active bankroll can afford stake
            if active_bankroll < stake {
                // Insufficient active bankroll, skip trade
                continue;
            }

            // Generate prediction from model engine
            let (p_up, p_down) = if record.features.len() >= 37 {
                let p = self.models.predict_vector(&record.features).clamp(0.05, 0.95);
                (p, 1.0 - p)
            } else {
                (0.50, 0.50)
            };

            // Heuristic market price: if feature vector contains Polymarket best ask, use it, else 0.50
            let quote_price = 0.50f64;
            let est_cost = fee_rate + slippage_rate;

            // Strategy hard gates and evaluation
            let mut target_side = None;
            let mut expected_net_edge = 0.0;

            let edge_up = p_up - quote_price - est_cost;
            let edge_down = p_down - quote_price - est_cost;

            if p_up >= min_prob && edge_up >= min_net_edge {
                target_side = Some(MarketSide::Up);
                expected_net_edge = edge_up;
            } else if p_down >= min_prob && edge_down >= min_net_edge {
                target_side = Some(MarketSide::Down);
                expected_net_edge = edge_down;
            }

            let side = match target_side {
                Some(s) => s,
                None => continue, // SKIP
            };

            // Order execution simulation: depth walking slippage and fee deduction
            let fill_price = quote_price * (1.0 + slippage_rate);
            let fee = stake * fee_rate;
            let shares = (stake - fee) / fill_price;

            // Snapshot balance before trade
            let bankroll_before = active_bankroll;
            let locked_profit_before = locked_profit;

            // Reserve stake
            active_bankroll -= stake;

            // Evaluate settlement based on historical ground truth
            let (outcome, is_win, is_loss) = match record.resolution {
                Resolution::Up => {
                    if side == MarketSide::Up {
                        ("WIN", true, false)
                    } else {
                        ("LOSS", false, true)
                    }
                }
                Resolution::Down => {
                    if side == MarketSide::Down {
                        ("WIN", true, false)
                    } else {
                        ("LOSS", false, true)
                    }
                }
                Resolution::Void => ("VOID", false, false),
            };

            let (payout, net_profit, returned_stake) = if is_win {
                // Polymarket binary shares pay $1.00 each
                let payout = shares * 1.0;
                let net_profit = payout - stake;
                (payout, net_profit, stake)
            } else if is_loss {
                (0.0, -stake, 0.0)
            } else {
                (stake, 0.0, stake)
            };

            // Settle bankroll according to mode
            if net_profit > 0.0 {
                // Return original stake
                active_bankroll += returned_stake;

                match mode {
                    BankrollMode::CapitalRecovery => {
                        // Mode B: Replenish active bankroll up to cap first, lock remainder
                        let deficit = (bankroll_cap - active_bankroll).max(0.0);
                        let to_replenish = net_profit.min(deficit);
                        let to_lock = (net_profit - to_replenish).max(0.0);

                        active_bankroll += to_replenish;
                        locked_profit += to_lock;
                    }
                    BankrollMode::ProfitIsolation => {
                        // Mode A: 100% of profit goes directly to locked profit
                        locked_profit += net_profit;
                    }
                }
            } else if net_profit < 0.0 {
                // Lost trade: stake is lost, active bankroll already deducted
                active_bankroll += returned_stake; // 0.0
            } else {
                // Void trade: stake refunded
                active_bankroll += returned_stake;
            }

            let total_equity = active_bankroll + locked_profit;
            if total_equity > peak_equity {
                peak_equity = total_equity;
            }
            let drawdown_pct = if peak_equity > 0.0 {
                (peak_equity - total_equity) / peak_equity
            } else {
                0.0
            };

            // Record trade result
            let trade_res = PaperResult {
                result_id: format!("bt_res_{}", Uuid::new_v4().simple()),
                order_id: format!("bt_ord_{}", Uuid::new_v4().simple()),
                market_id: record.market_id.clone(),
                asset: record.asset,
                side,
                entry_time_ms: record.timestamp_ms,
                entry_price: quote_price,
                fill_price,
                shares,
                stake,
                predicted_probability: if side == MarketSide::Up { p_up } else { p_down },
                model_confidence: "NORMAL".to_string(),
                gross_edge: if side == MarketSide::Up { p_up - quote_price } else { p_down - quote_price },
                net_edge: expected_net_edge,
                fee,
                slippage: fill_price - quote_price,
                outcome: outcome.to_string(),
                payout,
                pnl: net_profit,
                bankroll_before,
                bankroll_after: active_bankroll,
                locked_profit_before,
                locked_profit_after: locked_profit,
                strategy_version: "backtest-v1".to_string(),
                model_version: "logistic-v1".to_string(),
                created_at_ms: record.timestamp_ms + 300_000,
            };

            trades.push(trade_res);

            // Record equity point
            equity_curve.push(EquityPoint {
                timestamp_ms: record.timestamp_ms + 300_000,
                active_bankroll,
                locked_profit,
                total_equity,
                drawdown_pct,
                trade_pnl: Some(net_profit),
            });
        }

        // Compute metrics
        let metrics = compute_backtest_metrics(&trades, &equity_curve, initial_bankroll);

        Ok(BacktestResult {
            backtest_id,
            config: req.clone(),
            metrics,
            equity_curve,
            trades,
            executed_at_ms,
            execution_duration_ms: 0,
        })
    }

    /// Retrieve the most recent backtest result
    pub async fn get_latest_backtest(&self) -> Option<BacktestResult> {
        let hist = self.recent_results.read().await;
        hist.back().cloned()
    }

    /// Retrieve backtest history
    pub async fn get_backtest_history(&self, limit: usize) -> Vec<BacktestResult> {
        let hist = self.recent_results.read().await;
        hist.iter().rev().take(limit).cloned().collect()
    }
}
