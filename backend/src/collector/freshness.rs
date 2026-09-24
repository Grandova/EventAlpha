use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::types::{Asset, Exchange, ExchangeHealth};

#[derive(Debug, Clone)]
struct FeedState {
    last_update_ms: i64,
    last_exchange_ts_ms: i64,
    latency_ms: i64,
    ticks_count: u64,
    connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessReport {
    pub is_system_fresh: bool,
    pub stale_timeout_ms: u64,
    pub exchanges: Vec<ExchangeHealth>,
    pub timestamp_ms: i64,
}

#[derive(Clone)]
pub struct FreshnessTracker {
    feeds: Arc<DashMap<(Exchange, Asset), FeedState>>,
    stale_timeout_ms: u64,
}

impl FreshnessTracker {
    pub fn new(stale_timeout_ms: u64) -> Self {
        Self {
            feeds: Arc::new(DashMap::new()),
            stale_timeout_ms,
        }
    }

    /// Record receipt of a tick from an exchange
    pub fn record_tick(
        &self,
        exchange: Exchange,
        asset: Asset,
        exchange_ts_ms: i64,
        receive_ts_ms: i64,
    ) {
        let latency_ms = (receive_ts_ms - exchange_ts_ms).max(0);
        self.feeds
            .entry((exchange, asset))
            .and_modify(|s| {
                s.last_update_ms = receive_ts_ms;
                s.last_exchange_ts_ms = exchange_ts_ms;
                s.latency_ms = latency_ms;
                s.ticks_count += 1;
                s.connected = true;
            })
            .or_insert(FeedState {
                last_update_ms: receive_ts_ms,
                last_exchange_ts_ms: exchange_ts_ms,
                latency_ms,
                ticks_count: 1,
                connected: true,
            });
    }

    /// Update connection state when WebSocket disconnects or reconnects
    pub fn set_connected(&self, exchange: Exchange, connected: bool) {
        for mut item in self.feeds.iter_mut() {
            if item.key().0 == exchange {
                item.value_mut().connected = connected;
            }
        }
    }

    /// Check if a specific (exchange, asset) feed is stale
    pub fn is_stale(&self, exchange: Exchange, asset: Asset) -> bool {
        let now = Utc::now().timestamp_millis();
        match self.feeds.get(&(exchange, asset)) {
            Some(entry) => {
                if !entry.connected {
                    return true;
                }
                (now - entry.last_update_ms) > self.stale_timeout_ms as i64
            }
            None => true, // No ticks received yet is considered stale / unready
        }
    }

    /// Generate an overall health report across all tracked exchanges
    pub fn get_report(&self) -> FreshnessReport {
        let now = Utc::now().timestamp_millis();
        let target_exchanges = [
            Exchange::Binance,
            Exchange::Okx,
            Exchange::Bybit,
            Exchange::Coinbase,
        ];

        let mut exchange_healths = Vec::new();
        let mut all_fresh = true;

        for &ex in &target_exchanges {
            // Find ticks received for this exchange
            let mut ticks_sum = 0u64;
            let mut latest_update = 0i64;
            let mut avg_latency = 0i64;
            let mut count = 0;
            let mut connected = false;

            for entry in self.feeds.iter() {
                if entry.key().0 == ex {
                    ticks_sum += entry.value().ticks_count;
                    latest_update = latest_update.max(entry.value().last_update_ms);
                    avg_latency += entry.value().latency_ms;
                    count += 1;
                    if entry.value().connected {
                        connected = true;
                    }
                }
            }

            let latency = if count > 0 { avg_latency / count } else { 0 };
            let is_stale = latest_update == 0 || (now - latest_update) > self.stale_timeout_ms as i64;
            if is_stale {
                all_fresh = false;
            }

            exchange_healths.push(ExchangeHealth {
                exchange: ex,
                connected: connected && !is_stale,
                last_update_ms: latest_update,
                latency_ms: latency,
                ticks_received: ticks_sum,
                is_stale,
            });
        }

        FreshnessReport {
            is_system_fresh: all_fresh,
            stale_timeout_ms: self.stale_timeout_ms,
            exchanges: exchange_healths,
            timestamp_ms: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freshness_tracker_stale_detection() {
        let tracker = FreshnessTracker::new(2000);
        // Initially stale
        assert!(tracker.is_stale(Exchange::Binance, Asset::BTC));

        let now = Utc::now().timestamp_millis();
        tracker.record_tick(Exchange::Binance, Asset::BTC, now - 50, now);

        // Immediately after recording, it is fresh
        assert!(!tracker.is_stale(Exchange::Binance, Asset::BTC));

        // Simulated stale
        let old_time = now - 3000;
        tracker.record_tick(Exchange::Binance, Asset::BTC, old_time - 50, old_time);
        assert!(tracker.is_stale(Exchange::Binance, Asset::BTC));
    }
}
