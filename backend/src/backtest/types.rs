use serde::{Deserialize, Serialize};
use crate::types::{Asset, BankrollMode, PaperResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestRequest {
    pub start_time_ms: Option<i64>,
    pub end_time_ms: Option<i64>,
    pub assets: Option<Vec<Asset>>,
    pub initial_bankroll: Option<f64>,
    pub bankroll_cap: Option<f64>,
    pub mode: Option<BankrollMode>,
    pub stake: Option<f64>,
    pub min_prob: Option<f64>,
    pub min_net_edge: Option<f64>,
    pub fee_rate: Option<f64>,
    pub slippage_rate: Option<f64>,
}

impl Default for BacktestRequest {
    fn default() -> Self {
        Self {
            start_time_ms: None,
            end_time_ms: None,
            assets: Some(vec![Asset::BTC, Asset::ETH, Asset::SOL]),
            initial_bankroll: Some(10.0),
            bankroll_cap: Some(10.0),
            mode: Some(BankrollMode::CapitalRecovery),
            stake: Some(1.0),
            min_prob: Some(0.70),
            min_net_edge: Some(0.05),
            fee_rate: Some(0.012),
            slippage_rate: Some(0.005),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityPoint {
    pub timestamp_ms: i64,
    pub active_bankroll: f64,
    pub locked_profit: f64,
    pub total_equity: f64,
    pub drawdown_pct: f64,
    pub trade_pnl: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestMetrics {
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub void_trades: usize,
    pub win_rate: f64,
    pub gross_profit: f64,
    pub gross_loss: f64,
    pub net_pnl: f64,
    pub total_return_pct: f64,
    pub profit_factor: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub calmar_ratio: f64,
    pub max_drawdown_pct: f64,
    pub max_drawdown_usdc: f64,
    pub avg_trade_pnl: f64,
    pub avg_win_pnl: f64,
    pub avg_loss_pnl: f64,
    pub win_loss_payoff_ratio: f64,
    pub max_consecutive_wins: usize,
    pub max_consecutive_losses: usize,
    pub final_active_bankroll: f64,
    pub final_locked_profit: f64,
    pub final_total_equity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub backtest_id: String,
    pub config: BacktestRequest,
    pub metrics: BacktestMetrics,
    pub equity_curve: Vec<EquityPoint>,
    pub trades: Vec<PaperResult>,
    pub executed_at_ms: i64,
    pub execution_duration_ms: u64,
}
