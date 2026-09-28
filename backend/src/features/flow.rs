use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::types::{Asset, TradeTick};

const MAX_TRADE_WINDOW_SECS: i64 = 120;

#[derive(Debug, Clone)]
struct TradeRecord {
    pub timestamp_ms: i64,
    pub size: f64,
    pub is_buy: bool,
}

#[derive(Clone)]
pub struct TradeFlowTracker {
    // Rolling deque of trades per asset
    trades: Arc<DashMap<Asset, VecDeque<TradeRecord>>>,
}

impl Default for TradeFlowTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TradeFlowTracker {
    pub fn new() -> Self {
        Self {
            trades: Arc::new(DashMap::new()),
        }
    }

    /// Record an incoming public trade
    pub fn record_trade(&self, tick: &TradeTick) {
        let is_buy = tick.side.to_lowercase() == "buy";
        let record = TradeRecord {
            timestamp_ms: tick.receive_timestamp_ms,
            size: tick.size,
            is_buy,
        };

        let mut queue = self.trades.entry(tick.asset).or_default();
        queue.push_back(record);

        // Prune older than MAX_TRADE_WINDOW_SECS
        let cutoff = tick.receive_timestamp_ms - (MAX_TRADE_WINDOW_SECS * 1000);
        while let Some(front) = queue.front() {
            if front.timestamp_ms < cutoff {
                queue.pop_front();
            } else {
                break;
            }
        }
    }

    /// Cumulative Volume Delta (CVD) = Buy Volume - Sell Volume over window_secs
    pub fn calc_cvd(&self, asset: Asset, now_ms: i64, window_secs: f64) -> f64 {
        let cutoff = now_ms - (window_secs * 1000.0) as i64;
        let Some(queue) = self.trades.get(&asset) else {
            return 0.0;
        };

        let mut cvd = 0.0;
        for t in queue.iter().rev() {
            if t.timestamp_ms < cutoff {
                break;
            }
            if t.is_buy {
                cvd += t.size;
            } else {
                cvd -= t.size;
            }
        }
        cvd
    }

    /// Aggressive Trade Imbalance = (Buy Vol - Sell Vol) / (Buy Vol + Sell Vol) in [-1.0, 1.0]
    pub fn calc_imbalance(&self, asset: Asset, now_ms: i64, window_secs: f64) -> f64 {
        let cutoff = now_ms - (window_secs * 1000.0) as i64;
        let Some(queue) = self.trades.get(&asset) else {
            return 0.0;
        };

        let mut buy_vol = 0.0;
        let mut sell_vol = 0.0;

        for t in queue.iter().rev() {
            if t.timestamp_ms < cutoff {
                break;
            }
            if t.is_buy {
                buy_vol += t.size;
            } else {
                sell_vol += t.size;
            }
        }

        let total = buy_vol + sell_vol;
        if total > 1e-6 {
            (buy_vol - sell_vol) / total
        } else {
            0.0
        }
    }

    /// Total traded volume over window_secs
    pub fn calc_volume(&self, asset: Asset, now_ms: i64, window_secs: f64) -> f64 {
        let cutoff = now_ms - (window_secs * 1000.0) as i64;
        let Some(queue) = self.trades.get(&asset) else {
            return 0.0;
        };

        let mut total = 0.0;
        for t in queue.iter().rev() {
            if t.timestamp_ms < cutoff {
                break;
            }
            total += t.size;
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Exchange;

    #[test]
    fn test_trade_flow_cvd_and_imbalance() {
        let tracker = TradeFlowTracker::new();
        let now = 100000i64;

        let mk_trade = |ts, size, side: &str| TradeTick {
            exchange: Exchange::Binance,
            symbol: "BTCUSDT".to_string(),
            asset: Asset::BTC,
            exchange_timestamp_ms: ts - 10,
            receive_timestamp_ms: ts,
            latency_ms: 10,
            price: 65000.0,
            size,
            side: side.to_string(),
            is_aggressive: true,
        };

        // 3 buys of size 2.0 (total buy = 6.0)
        tracker.record_trade(&mk_trade(now - 3000, 2.0, "buy"));
        tracker.record_trade(&mk_trade(now - 2000, 2.0, "buy"));
        tracker.record_trade(&mk_trade(now - 1000, 2.0, "buy"));

        // 1 sell of size 2.0 (total sell = 2.0)
        tracker.record_trade(&mk_trade(now - 500, 2.0, "sell"));

        // CVD over 5s: 6.0 - 2.0 = 4.0
        let cvd_5s = tracker.calc_cvd(Asset::BTC, now, 5.0);
        assert_eq!(cvd_5s, 4.0);

        // Imbalance: (6.0 - 2.0) / (6.0 + 2.0) = 4.0 / 8.0 = 0.50
        let imb_5s = tracker.calc_imbalance(Asset::BTC, now, 5.0);
        assert!((imb_5s - 0.50).abs() < 1e-6);

        // Volume: 8.0
        let vol_5s = tracker.calc_volume(Asset::BTC, now, 5.0);
        assert_eq!(vol_5s, 8.0);
    }
}
