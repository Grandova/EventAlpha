use super::types::{BacktestMetrics, EquityPoint};
use crate::types::PaperResult;

/// Annualization factor for 5-minute trading periods:
/// 365 days * 24 hours * 12 (5-min intervals per hour) = 105,120 periods/year
pub const ANNUALIZATION_FACTOR_5M: f64 = 324.222; // sqrt(105,120)

/// Compute complete quantitative metrics from trades and equity curve
pub fn compute_backtest_metrics(
    trades: &[PaperResult],
    equity_curve: &[EquityPoint],
    initial_bankroll: f64,
) -> BacktestMetrics {
    let total_trades = trades.len();
    if total_trades == 0 {
        return BacktestMetrics {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            void_trades: 0,
            win_rate: 0.0,
            gross_profit: 0.0,
            gross_loss: 0.0,
            net_pnl: 0.0,
            total_return_pct: 0.0,
            profit_factor: 0.0,
            sharpe_ratio: 0.0,
            sortino_ratio: 0.0,
            calmar_ratio: 0.0,
            max_drawdown_pct: 0.0,
            max_drawdown_usdc: 0.0,
            avg_trade_pnl: 0.0,
            avg_win_pnl: 0.0,
            avg_loss_pnl: 0.0,
            win_loss_payoff_ratio: 0.0,
            max_consecutive_wins: 0,
            max_consecutive_losses: 0,
            final_active_bankroll: initial_bankroll,
            final_locked_profit: 0.0,
            final_total_equity: initial_bankroll,
        };
    }

    let mut winning_trades = 0usize;
    let mut losing_trades = 0usize;
    let mut void_trades = 0usize;

    let mut gross_profit = 0.0f64;
    let mut gross_loss = 0.0f64;
    let mut net_pnl = 0.0f64;

    let mut returns = Vec::with_capacity(total_trades);
    let mut outcomes = Vec::with_capacity(total_trades);

    for t in trades {
        net_pnl += t.pnl;
        let ret = if t.stake > 0.0 { t.pnl / t.stake } else { 0.0 };
        returns.push(ret);
        outcomes.push(t.outcome.to_uppercase());

        match t.outcome.to_uppercase().as_str() {
            "WIN" => {
                winning_trades += 1;
                gross_profit += t.pnl;
            }
            "LOSS" => {
                losing_trades += 1;
                gross_loss += -t.pnl;
            }
            _ => {
                void_trades += 1;
            }
        }
    }

    let non_void = winning_trades + losing_trades;
    let win_rate = if non_void > 0 {
        (winning_trades as f64 / non_void as f64) * 100.0
    } else {
        0.0
    };

    let profit_factor = if gross_loss > 0.0 {
        gross_profit / gross_loss
    } else if gross_profit > 0.0 {
        999.99
    } else {
        0.0
    };

    let total_return_pct = if initial_bankroll > 0.0 {
        (net_pnl / initial_bankroll) * 100.0
    } else {
        0.0
    };

    let avg_trade_pnl = net_pnl / total_trades as f64;
    let avg_win_pnl = if winning_trades > 0 {
        gross_profit / winning_trades as f64
    } else {
        0.0
    };
    let avg_loss_pnl = if losing_trades > 0 {
        gross_loss / losing_trades as f64
    } else {
        0.0
    };

    let win_loss_payoff_ratio = if avg_loss_pnl > 0.0 {
        avg_win_pnl / avg_loss_pnl
    } else if avg_win_pnl > 0.0 {
        999.99
    } else {
        0.0
    };

    // Calculate streaks
    let (max_consecutive_wins, max_consecutive_losses) = calculate_streaks(&outcomes);

    // Calculate Max Drawdown from Equity Curve
    let (max_drawdown_pct, max_drawdown_usdc) = calculate_drawdown(equity_curve, initial_bankroll);

    // Calculate Sharpe & Sortino
    let sharpe_ratio = calculate_sharpe(&returns);
    let sortino_ratio = calculate_sortino(&returns);

    // Calculate Calmar Ratio
    let calmar_ratio = calculate_calmar(total_return_pct, max_drawdown_pct);

    let (final_active, final_locked, final_total) = if let Some(last) = equity_curve.last() {
        (last.active_bankroll, last.locked_profit, last.total_equity)
    } else {
        (initial_bankroll, 0.0, initial_bankroll)
    };

    BacktestMetrics {
        total_trades,
        winning_trades,
        losing_trades,
        void_trades,
        win_rate,
        gross_profit,
        gross_loss,
        net_pnl,
        total_return_pct,
        profit_factor,
        sharpe_ratio,
        sortino_ratio,
        calmar_ratio,
        max_drawdown_pct,
        max_drawdown_usdc,
        avg_trade_pnl,
        avg_win_pnl,
        avg_loss_pnl,
        win_loss_payoff_ratio,
        max_consecutive_wins,
        max_consecutive_losses,
        final_active_bankroll: final_active,
        final_locked_profit: final_locked,
        final_total_equity: final_total,
    }
}

/// Calculate annualized Sharpe ratio
pub fn calculate_sharpe(returns: &[f64]) -> f64 {
    if returns.len() < 2 {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let std_dev = variance.sqrt();

    if std_dev > 1e-9 {
        // Annualize using trade-frequency scaling
        let ann_scale = (252.0 * 288.0f64 / n.max(1.0)).sqrt().clamp(1.0, 50.0);
        (mean / std_dev) * ann_scale
    } else {
        0.0
    }
}

/// Calculate annualized Sortino ratio (downside deviation only)
pub fn calculate_sortino(returns: &[f64]) -> f64 {
    if returns.len() < 2 {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;

    let downside_variance = returns
        .iter()
        .filter(|&&r| r < 0.0)
        .map(|r| r.powi(2))
        .sum::<f64>()
        / n;

    let downside_dev = downside_variance.sqrt();

    if downside_dev > 1e-9 {
        let ann_scale = (252.0 * 288.0f64 / n.max(1.0)).sqrt().clamp(1.0, 50.0);
        (mean / downside_dev) * ann_scale
    } else if mean > 0.0 {
        99.99 // No downside volatility with positive mean
    } else {
        0.0
    }
}

/// Calculate Calmar ratio
pub fn calculate_calmar(total_return_pct: f64, max_drawdown_pct: f64) -> f64 {
    if max_drawdown_pct > 1e-9 {
        (total_return_pct / (max_drawdown_pct * 100.0)).max(0.0)
    } else if total_return_pct > 0.0 {
        999.99
    } else {
        0.0
    }
}

/// Calculate maximum drawdown in percentage and USDC from equity curve
pub fn calculate_drawdown(curve: &[EquityPoint], initial_bankroll: f64) -> (f64, f64) {
    if curve.is_empty() {
        return (0.0, 0.0);
    }

    let mut peak = initial_bankroll;
    let mut max_dd_pct = 0.0f64;
    let mut max_dd_usdc = 0.0f64;

    for point in curve {
        if point.total_equity > peak {
            peak = point.total_equity;
        }

        let dd_usdc = peak - point.total_equity;
        let dd_pct = if peak > 0.0 { dd_usdc / peak } else { 0.0 };

        if dd_pct > max_dd_pct {
            max_dd_pct = dd_pct;
        }
        if dd_usdc > max_dd_usdc {
            max_dd_usdc = dd_usdc;
        }
    }

    (max_dd_pct, max_dd_usdc)
}

/// Calculate maximum consecutive winning and losing streaks
pub fn calculate_streaks(outcomes: &[String]) -> (usize, usize) {
    let mut max_wins = 0;
    let mut current_wins = 0;
    let mut max_losses = 0;
    let mut current_losses = 0;

    for outcome in outcomes {
        match outcome.as_str() {
            "WIN" => {
                current_wins += 1;
                current_losses = 0;
                if current_wins > max_wins {
                    max_wins = current_wins;
                }
            }
            "LOSS" => {
                current_losses += 1;
                current_wins = 0;
                if current_losses > max_losses {
                    max_losses = current_losses;
                }
            }
            _ => {
                current_wins = 0;
                current_losses = 0;
            }
        }
    }

    (max_wins, max_losses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_streaks() {
        let outcomes = vec![
            "WIN".to_string(),
            "WIN".to_string(),
            "LOSS".to_string(),
            "LOSS".to_string(),
            "LOSS".to_string(),
            "WIN".to_string(),
        ];
        let (wins, losses) = calculate_streaks(&outcomes);
        assert_eq!(wins, 2);
        assert_eq!(losses, 3);
    }

    #[test]
    fn test_calculate_drawdown() {
        let curve = vec![
            EquityPoint {
                timestamp_ms: 1,
                active_bankroll: 10.0,
                locked_profit: 0.0,
                total_equity: 10.0,
                drawdown_pct: 0.0,
                trade_pnl: None,
            },
            EquityPoint {
                timestamp_ms: 2,
                active_bankroll: 12.0,
                locked_profit: 0.0,
                total_equity: 12.0,
                drawdown_pct: 0.0,
                trade_pnl: Some(2.0),
            },
            EquityPoint {
                timestamp_ms: 3,
                active_bankroll: 9.0,
                locked_profit: 0.0,
                total_equity: 9.0,
                drawdown_pct: 0.25,
                trade_pnl: Some(-3.0),
            },
        ];

        let (dd_pct, dd_usdc) = calculate_drawdown(&curve, 10.0);
        assert_eq!(dd_usdc, 3.0);
        assert_eq!(dd_pct, 3.0 / 12.0); // 0.25
    }
}
