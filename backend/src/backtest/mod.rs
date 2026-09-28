pub mod engine;
pub mod metrics;
pub mod types;

pub use engine::BacktestEngine;
pub use metrics::{
    calculate_calmar, calculate_drawdown, calculate_sharpe, calculate_sortino, calculate_streaks,
    compute_backtest_metrics,
};
pub use types::{BacktestMetrics, BacktestRequest, BacktestResult, EquityPoint};
