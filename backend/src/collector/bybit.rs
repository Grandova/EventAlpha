use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};

use crate::collector::freshness::FreshnessTracker;
use crate::types::{Asset, Exchange, MarketTick, TradeTick};

pub const BYBIT_WS_URL: &str = "wss://stream.bybit.com/v5/public/spot";

pub fn symbol_to_asset(symbol: &str) -> Option<Asset> {
    match symbol.to_uppercase().as_str() {
        "BTCUSDT" => Some(Asset::BTC),
        "ETHUSDT" => Some(Asset::ETH),
        "SOLUSDT" => Some(Asset::SOL),
        _ => None,
    }
}

pub fn parse_bybit_message(
    text: &str,
    receive_ts_ms: i64,
) -> (Option<MarketTick>, Option<TradeTick>) {
    let Ok(val) = serde_json::from_str::<Value>(text) else {
        return (None, None);
    };

    let topic = val.get("topic").and_then(|t| t.as_str()).unwrap_or("");
    let ts = val.get("ts").and_then(|t| t.as_i64()).unwrap_or(receive_ts_ms);

    if topic.starts_with("tickers.") {
        let Some(data) = val.get("data") else {
            return (None, None);
        };
        let symbol = data.get("symbol").and_then(|s| s.as_str()).unwrap_or("");
        let Some(asset) = symbol_to_asset(symbol) else {
            return (None, None);
        };

        let bid = data
            .get("bid1Price")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let ask = data
            .get("ask1Price")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let last = data
            .get("lastPrice")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let vol = data
            .get("volume24h")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        if bid <= 0.0 || ask <= 0.0 {
            return (None, None);
        }

        let mid = (bid + ask) / 2.0;
        let latency_ms = (receive_ts_ms - ts).max(0);

        let tick = MarketTick {
            exchange: Exchange::Bybit,
            symbol: symbol.to_string(),
            asset,
            exchange_timestamp_ms: ts,
            receive_timestamp_ms: receive_ts_ms,
            latency_ms,
            bid,
            ask,
            mid,
            last: if last > 0.0 { last } else { mid },
            volume_24h: vol,
        };
        (Some(tick), None)
    } else if topic.starts_with("publicTrade.") {
        let Some(data_arr) = val.get("data").and_then(|d| d.as_array()) else {
            return (None, None);
        };
        let Some(first) = data_arr.first() else {
            return (None, None);
        };

        let symbol = first.get("s").and_then(|s| s.as_str()).unwrap_or("");
        let Some(asset) = symbol_to_asset(symbol) else {
            return (None, None);
        };

        let price = first
            .get("p")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let size = first
            .get("v")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let side = first
            .get("S")
            .and_then(|s| s.as_str())
            .unwrap_or("Buy");
        let trade_ts = first
            .get("T")
            .and_then(|t| t.as_i64())
            .unwrap_or(ts);

        let latency_ms = (receive_ts_ms - trade_ts).max(0);

        let trade = TradeTick {
            exchange: Exchange::Bybit,
            symbol: symbol.to_string(),
            asset,
            exchange_timestamp_ms: trade_ts,
            receive_timestamp_ms: receive_ts_ms,
            latency_ms,
            price,
            size,
            side: side.to_lowercase(),
            is_aggressive: true,
        };
        (None, Some(trade))
    } else {
        (None, None)
    }
}

pub async fn run_bybit_collector(
    market_tx: broadcast::Sender<MarketTick>,
    trade_tx: broadcast::Sender<TradeTick>,
    freshness: FreshnessTracker,
) {
    let mut backoff_secs = 1u64;

    loop {
        info!("Connecting to Bybit WebSocket stream...");
        freshness.set_connected(Exchange::Bybit, false);

        match tokio::time::timeout(Duration::from_secs(5), connect_async(BYBIT_WS_URL)).await {
            Ok(Ok((ws_stream, _))) => {
                info!("Successfully connected to Bybit WebSocket.");
                freshness.set_connected(Exchange::Bybit, true);
                backoff_secs = 1;

                let (mut write, mut read) = ws_stream.split();

                // Send subscription command
                let sub_payload = serde_json::json!({
                    "op": "subscribe",
                    "args": [
                        "tickers.BTCUSDT",
                        "tickers.ETHUSDT",
                        "tickers.SOLUSDT",
                        "publicTrade.BTCUSDT",
                        "publicTrade.ETHUSDT",
                        "publicTrade.SOLUSDT"
                    ]
                });

                if let Err(e) = write.send(Message::Text(sub_payload.to_string().into())).await {
                    error!("Failed to send Bybit subscribe command: {:?}", e);
                    continue;
                }

                let mut ping_interval = tokio::time::interval(Duration::from_secs(20));

                loop {
                    tokio::select! {
                        _ = ping_interval.tick() => {
                            let ping = serde_json::json!({"op": "ping"});
                            if let Err(e) = write.send(Message::Text(ping.to_string().into())).await {
                                warn!("Failed to send Bybit ping: {:?}", e);
                                break;
                            }
                        }
                        msg = read.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    let now_ms = Utc::now().timestamp_millis();
                                    let (m_tick, t_tick) = parse_bybit_message(&text, now_ms);

                                    if let Some(tick) = m_tick {
                                        freshness.record_tick(
                                            Exchange::Bybit,
                                            tick.asset,
                                            tick.exchange_timestamp_ms,
                                            now_ms,
                                        );
                                        let _ = market_tx.send(tick);
                                    }

                                    if let Some(trade) = t_tick {
                                        freshness.record_tick(
                                            Exchange::Bybit,
                                            trade.asset,
                                            trade.exchange_timestamp_ms,
                                            now_ms,
                                        );
                                        let _ = trade_tx.send(trade);
                                    }
                                }
                                Some(Ok(Message::Close(_))) => {
                                    warn!("Bybit WebSocket received Close frame");
                                    break;
                                }
                                Some(Err(e)) => {
                                    error!("Bybit WebSocket error: {:?}", e);
                                    break;
                                }
                                None => {
                                    warn!("Bybit WebSocket stream closed");
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                error!("Failed to connect to Bybit WebSocket: {:?}. Retrying in {}s...", e, backoff_secs);
            }
            Err(_) => {
                warn!("Timeout (5s) connecting to Bybit WebSocket. Retrying in {}s...", backoff_secs);
            }
        }

        freshness.set_connected(Exchange::Bybit, false);
        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(15);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bybit_ticker() {
        let json = r#"
        {
            "topic": "tickers.BTCUSDT",
            "ts": 1700000000000,
            "type": "snapshot",
            "data": {
                "symbol": "BTCUSDT",
                "lastPrice": "67140.00",
                "bid1Price": "67139.50",
                "ask1Price": "67140.50",
                "volume24h": "5432.1"
            }
        }
        "#;

        let (tick_opt, _) = parse_bybit_message(json, 1700000000050);
        let tick = tick_opt.expect("Should parse Bybit ticker");
        assert_eq!(tick.exchange, Exchange::Bybit);
        assert_eq!(tick.asset, Asset::BTC);
        assert_eq!(tick.bid, 67139.50);
        assert_eq!(tick.ask, 67140.50);
        assert_eq!(tick.latency_ms, 50);
    }
}
