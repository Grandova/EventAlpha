use crate::config::StrategyConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterResult {
    Pass,
    Skip(String),
}

impl FilterResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, FilterResult::Pass)
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            FilterResult::Pass => None,
            FilterResult::Skip(r) => Some(r.as_str()),
        }
    }
}

pub struct HardFilterEngine;

impl HardFilterEngine {
    /// Check calibrated probability threshold (e.g. >= 0.70)
    pub fn check_probability(calibrated_p: f64, config: &StrategyConfig) -> FilterResult {
        if calibrated_p < config.min_probability {
            FilterResult::Skip(format!(
                "Calibrated probability {:.1}% < threshold {:.1}%",
                calibrated_p * 100.0,
                config.min_probability * 100.0
            ))
        } else {
            FilterResult::Pass
        }
    }

    /// Check net edge after fees and slippage (e.g. >= 0.05)
    pub fn check_net_edge(net_edge: f64, config: &StrategyConfig) -> FilterResult {
        if net_edge < config.min_net_edge {
            FilterResult::Skip(format!(
                "Net edge {:.2}% < required threshold {:.2}%",
                net_edge * 100.0,
                config.min_net_edge * 100.0
            ))
        } else {
            FilterResult::Pass
        }
    }

    /// Check maximum entry price ceiling (e.g. <= 0.85)
    pub fn check_entry_price(ask_price: f64, config: &StrategyConfig) -> FilterResult {
        if ask_price > config.max_entry_price {
            FilterResult::Skip(format!(
                "Ask price {:.2} > maximum allowed entry price {:.2}",
                ask_price, config.max_entry_price
            ))
        } else {
            FilterResult::Pass
        }
    }

    /// Check bid-ask spread (e.g. <= 0.04)
    pub fn check_spread(spread: f64, config: &StrategyConfig) -> FilterResult {
        if spread > config.max_spread {
            FilterResult::Skip(format!(
                "Spread {:.3} > maximum allowed spread {:.3}",
                spread, config.max_spread
            ))
        } else {
            FilterResult::Pass
        }
    }

    /// Check total available book liquidity (e.g. >= 300 USDC)
    pub fn check_liquidity(total_liquidity: f64, config: &StrategyConfig) -> FilterResult {
        if total_liquidity < config.min_liquidity {
            FilterResult::Skip(format!(
                "Total liquidity ${:.2} < required minimum ${:.2}",
                total_liquidity, config.min_liquidity
            ))
        } else {
            FilterResult::Pass
        }
    }

    /// Check execution time window (e.g. between 15s and 285s remaining)
    pub fn check_timing(remaining_seconds: i64, config: &StrategyConfig) -> FilterResult {
        if remaining_seconds < config.min_time_remaining_sec as i64 {
            FilterResult::Skip(format!(
                "Remaining time {}s < minimum execution limit {}s",
                remaining_seconds, config.min_time_remaining_sec
            ))
        } else if remaining_seconds > config.max_time_remaining_sec as i64 {
            FilterResult::Skip(format!(
                "Remaining time {}s > maximum allowed limit {}s (round too early)",
                remaining_seconds, config.max_time_remaining_sec
            ))
        } else {
            FilterResult::Pass
        }
    }

    /// Check exchange data freshness (fail-closed if stale)
    pub fn check_freshness(is_system_fresh: bool) -> FilterResult {
        if !is_system_fresh {
            FilterResult::Skip("One or more exchange data feeds are stale or disconnected".to_string())
        } else {
            FilterResult::Pass
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_strategy_config() -> StrategyConfig {
        StrategyConfig {
            strategy_version: "v1.0.0".to_string(),
            min_probability: 0.70,
            min_net_edge: 0.05,
            max_entry_price: 0.85,
            max_spread: 0.04,
            min_liquidity: 300.0,
            min_time_remaining_sec: 15,
            max_time_remaining_sec: 285,
            score_thresholds: crate::config::ScoreThresholds {
                skip_below: 60.0,
                low: 60.0,
                medium: 70.0,
                high: 80.0,
                very_high: 90.0,
            },
        }
    }

    #[test]
    fn test_hard_filters_pass_and_reject() {
        let cfg = test_strategy_config();

        // 1. Probability
        assert!(HardFilterEngine::check_probability(0.72, &cfg).is_pass());
        assert!(!HardFilterEngine::check_probability(0.68, &cfg).is_pass());

        // 2. Net Edge
        assert!(HardFilterEngine::check_net_edge(0.06, &cfg).is_pass());
        assert!(!HardFilterEngine::check_net_edge(0.04, &cfg).is_pass());

        // 3. Entry Price
        assert!(HardFilterEngine::check_entry_price(0.80, &cfg).is_pass());
        assert!(!HardFilterEngine::check_entry_price(0.88, &cfg).is_pass());

        // 4. Spread
        assert!(HardFilterEngine::check_spread(0.03, &cfg).is_pass());
        assert!(!HardFilterEngine::check_spread(0.05, &cfg).is_pass());

        // 5. Liquidity
        assert!(HardFilterEngine::check_liquidity(500.0, &cfg).is_pass());
        assert!(!HardFilterEngine::check_liquidity(100.0, &cfg).is_pass());

        // 6. Timing
        assert!(HardFilterEngine::check_timing(120, &cfg).is_pass());
        assert!(!HardFilterEngine::check_timing(10, &cfg).is_pass()); // too late
        assert!(!HardFilterEngine::check_timing(290, &cfg).is_pass()); // too early

        // 7. Freshness
        assert!(HardFilterEngine::check_freshness(true).is_pass());
        assert!(!HardFilterEngine::check_freshness(false).is_pass());
    }
}
