use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};

use crate::collector::freshness::FreshnessTracker;
use crate::types::{Asset, Exchange, MarketTick, TradeTick};

pub const OKX_WS_URL: &str = "wss://ws.okx.com:8443/ws/v5/public";

pub fn inst_to_asset(inst_id: &str) -> Option<Asset> {
    match inst_id.to_uppercase().as_str() {
        "BTC-USDT" => Some(Asset::BTC),
        "ETH-USDT" => Some(Asset::ETH),
        "SOL-USDT" => Some(Asset::SOL),
        _ => None,
    }
}

pub fn parse_okx_message(
    text: &str,
    receive_ts_ms: i64,
) -> (Option<MarketTick>, Option<TradeTick>) {
    if text == "pong" {
        return (None, None);
    }

    let Ok(val) = serde_json::from_str::<Value>(text) else {
        return (None, None);
    };

    let Some(arg) = val.get("arg") else {
        return (None, None);
    };
    let channel = arg.get("channel").and_then(|c| c.as_str()).unwrap_or("");
    let inst_id = arg.get("instId").and_then(|i| i.as_str()).unwrap_or("");
    let Some(asset) = inst_to_asset(inst_id) else {
        return (None, None);
    };

    let Some(data_arr) = val.get("data").and_then(|d| d.as_array()) else {
        return (None, None);
    };
    let Some(first) = data_arr.first() else {
        return (None, None);
    };

    if channel == "tickers" {
        let bid = first
            .get("bidPx")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let ask = first
            .get("askPx")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let last = first
            .get("last")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let vol = first
            .get("vol24h")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let exchange_ts = first
            .get("ts")
            .and_then(|t| t.as_str())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(receive_ts_ms);

        if bid <= 0.0 || ask <= 0.0 {
            return (None, None);
        }

        let mid = (bid + ask) / 2.0;
        let latency_ms = (receive_ts_ms - exchange_ts).max(0);

        let tick = MarketTick {
            exchange: Exchange::Okx,
            symbol: inst_id.to_string(),
            asset,
            exchange_timestamp_ms: exchange_ts,
            receive_timestamp_ms: receive_ts_ms,
            latency_ms,
            bid,
            ask,
            mid,
            last: if last > 0.0 { last } else { mid },
            volume_24h: vol,
        };
        (Some(tick), None)
    } else if channel == "trades" {
        let price = first
            .get("px")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let size = first
            .get("sz")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let side = first
            .get("side")
            .and_then(|s| s.as_str())
            .unwrap_or("buy");
        let exchange_ts = first
            .get("ts")
            .and_then(|t| t.as_str())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(receive_ts_ms);

        let latency_ms = (receive_ts_ms - exchange_ts).max(0);

        let trade = TradeTick {
            exchange: Exchange::Okx,
            symbol: inst_id.to_string(),
            asset,
            exchange_timestamp_ms: exchange_ts,
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

pub async fn run_okx_collector(
    market_tx: broadcast::Sender<MarketTick>,
    trade_tx: broadcast::Sender<TradeTick>,
    freshness: FreshnessTracker,
) {
    let mut backoff_secs = 1u64;

    loop {
        info!("Connecting to OKX WebSocket stream...");
        freshness.set_connected(Exchange::Okx, false);

        match tokio::time::timeout(Duration::from_secs(5), connect_async(OKX_WS_URL)).await {
            Ok(Ok((ws_stream, _))) => {
                info!("Successfully connected to OKX WebSocket.");
                freshness.set_connected(Exchange::Okx, true);
                backoff_secs = 1;

                let (mut write, mut read) = ws_stream.split();

                // Send subscription command
                let sub_payload = serde_json::json!({
                    "op": "subscribe",
                    "args": [
                        {"channel": "tickers", "instId": "BTC-USDT"},
                        {"channel": "tickers", "instId": "ETH-USDT"},
                        {"channel": "tickers", "instId": "SOL-USDT"},
                        {"channel": "trades", "instId": "BTC-USDT"},
                        {"channel": "trades", "instId": "ETH-USDT"},
                        {"channel": "trades", "instId": "SOL-USDT"}
                    ]
                });

                if let Err(e) = write.send(Message::Text(sub_payload.to_string().into())).await {
                    error!("Failed to send OKX subscribe command: {:?}", e);
                    continue;
                }

                let mut ping_interval = tokio::time::interval(Duration::from_secs(20));

                loop {
                    tokio::select! {
                        _ = ping_interval.tick() => {
                            if let Err(e) = write.send(Message::Text("ping".into())).await {
                                warn!("Failed to send OKX ping: {:?}", e);
                                break;
                            }
                        }
                        msg = read.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    let now_ms = Utc::now().timestamp_millis();
                                    let (m_tick, t_tick) = parse_okx_message(&text, now_ms);

                                    if let Some(tick) = m_tick {
                                        freshness.record_tick(
                                            Exchange::Okx,
                                            tick.asset,
                                            tick.exchange_timestamp_ms,
                                            now_ms,
                                        );
                                        let _ = market_tx.send(tick);
                                    }

                                    if let Some(trade) = t_tick {
                                        freshness.record_tick(
                                            Exchange::Okx,
                                            trade.asset,
                                            trade.exchange_timestamp_ms,
                                            now_ms,
                                        );
                                        let _ = trade_tx.send(trade);
                                    }
                                }
                                Some(Ok(Message::Close(_))) => {
                                    warn!("OKX WebSocket received Close frame");
                                    break;
                                }
                                Some(Err(e)) => {
                                    error!("OKX WebSocket error: {:?}", e);
                                    break;
                                }
                                None => {
                                    warn!("OKX WebSocket stream closed");
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                error!("Failed to connect to OKX WebSocket: {:?}. Retrying in {}s...", e, backoff_secs);
            }
            Err(_) => {
                warn!("Timeout (5s) connecting to OKX WebSocket. Retrying in {}s...", backoff_secs);
            }
        }

        freshness.set_connected(Exchange::Okx, false);
        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(15);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_okx_ticker() {
        let json = r#"
        {
            "arg": { "channel": "tickers", "instId": "BTC-USDT" },
            "data": [
                {
                    "instType": "SPOT",
                    "instId": "BTC-USDT",
                    "last": "67150.0",
                    "bidPx": "67149.0",
                    "askPx": "67151.0",
                    "vol24h": "9876.5",
                    "ts": "1700000000000"
                }
            ]
        }
        "#;

        let (tick_opt, _) = parse_okx_message(json, 1700000000080);
        let tick = tick_opt.expect("Should parse OKX ticker");
        assert_eq!(tick.exchange, Exchange::Okx);
        assert_eq!(tick.asset, Asset::BTC);
        assert_eq!(tick.bid, 67149.0);
        assert_eq!(tick.ask, 67151.0);
        assert_eq!(tick.mid, 67150.0);
        assert_eq!(tick.latency_ms, 80);
    }
}
