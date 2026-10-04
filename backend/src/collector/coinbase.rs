use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};

use crate::collector::freshness::FreshnessTracker;
use crate::types::{Asset, Exchange, MarketTick, TradeTick};

pub const COINBASE_WS_URL: &str = "wss://ws-feed.exchange.coinbase.com";

pub fn product_to_asset(product_id: &str) -> Option<Asset> {
    match product_id.to_uppercase().as_str() {
        "BTC-USD" | "BTC-USDT" => Some(Asset::BTC),
        "ETH-USD" | "ETH-USDT" => Some(Asset::ETH),
        "SOL-USD" | "SOL-USDT" => Some(Asset::SOL),
        _ => None,
    }
}

pub fn parse_coinbase_message(
    text: &str,
    receive_ts_ms: i64,
) -> (Option<MarketTick>, Option<TradeTick>) {
    let Ok(val) = serde_json::from_str::<Value>(text) else {
        return (None, None);
    };

    let msg_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("");
    if msg_type != "ticker" {
        return (None, None);
    }

    let product_id = val.get("product_id").and_then(|p| p.as_str()).unwrap_or("");
    let Some(asset) = product_to_asset(product_id) else {
        return (None, None);
    };

    let bid = val
        .get("best_bid")
        .and_then(|b| b.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let ask = val
        .get("best_ask")
        .and_then(|a| a.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let last = val
        .get("price")
        .and_then(|p| p.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let volume = val
        .get("volume_24h")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    let exchange_ts = val
        .get("time")
        .and_then(|t| t.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(receive_ts_ms);

    let latency_ms = (receive_ts_ms - exchange_ts).max(0);

    let market_tick = if bid > 0.0 && ask > 0.0 {
        let mid = (bid + ask) / 2.0;
        Some(MarketTick {
            exchange: Exchange::Coinbase,
            symbol: product_id.to_string(),
            asset,
            exchange_timestamp_ms: exchange_ts,
            receive_timestamp_ms: receive_ts_ms,
            latency_ms,
            bid,
            ask,
            mid,
            last: if last > 0.0 { last } else { mid },
            volume_24h: volume,
        })
    } else {
        None
    };

    let last_size = val
        .get("last_size")
        .and_then(|s| s.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let side = val
        .get("side")
        .and_then(|s| s.as_str())
        .unwrap_or("buy");

    let trade_tick = if last > 0.0 && last_size > 0.0 {
        Some(TradeTick {
            exchange: Exchange::Coinbase,
            symbol: product_id.to_string(),
            asset,
            exchange_timestamp_ms: exchange_ts,
            receive_timestamp_ms: receive_ts_ms,
            latency_ms,
            price: last,
            size: last_size,
            side: side.to_lowercase(),
            is_aggressive: true,
        })
    } else {
        None
    };

    (market_tick, trade_tick)
}

pub async fn run_coinbase_collector(
    market_tx: broadcast::Sender<MarketTick>,
    trade_tx: broadcast::Sender<TradeTick>,
    freshness: FreshnessTracker,
) {
    let mut backoff_secs = 1u64;

    loop {
        info!("Connecting to Coinbase WebSocket stream...");
        freshness.set_connected(Exchange::Coinbase, false);

        match tokio::time::timeout(Duration::from_secs(5), connect_async(COINBASE_WS_URL)).await {
            Ok(Ok((ws_stream, _))) => {
                info!("Successfully connected to Coinbase WebSocket.");
                freshness.set_connected(Exchange::Coinbase, true);
                backoff_secs = 1;

                let (mut write, mut read) = ws_stream.split();

                // Send subscribe
                let sub_payload = serde_json::json!({
                    "type": "subscribe",
                    "product_ids": ["BTC-USD", "ETH-USD", "SOL-USD"],
                    "channels": ["ticker"]
                });

                if let Err(e) = write.send(Message::Text(sub_payload.to_string().into())).await {
                    error!("Failed to send Coinbase subscribe command: {:?}", e);
                    continue;
                }

                // Heartbeat / ping
                let mut ping_interval = tokio::time::interval(Duration::from_secs(30));

                loop {
                    tokio::select! {
                        _ = ping_interval.tick() => {
                            if let Err(e) = write.send(Message::Ping(vec![].into())).await {
                                warn!("Failed to send Coinbase WebSocket Ping: {:?}", e);
                                break;
                            }
                        }
                        msg = read.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    let now_ms = Utc::now().timestamp_millis();
                                    let (m_tick, t_tick) = parse_coinbase_message(&text, now_ms);

                                    if let Some(tick) = m_tick {
                                        freshness.record_tick(
                                            Exchange::Coinbase,
                                            tick.asset,
                                            tick.exchange_timestamp_ms,
                                            now_ms,
                                        );
                                        let _ = market_tx.send(tick);
                                    }

                                    if let Some(trade) = t_tick {
                                        freshness.record_tick(
                                            Exchange::Coinbase,
                                            trade.asset,
                                            trade.exchange_timestamp_ms,
                                            now_ms,
                                        );
                                        let _ = trade_tx.send(trade);
                                    }
                                }
                                Some(Ok(Message::Close(_))) => {
                                    warn!("Coinbase WebSocket received Close frame");
                                    break;
                                }
                                Some(Err(e)) => {
                                    error!("Coinbase WebSocket error: {:?}", e);
                                    break;
                                }
                                None => {
                                    warn!("Coinbase WebSocket stream closed");
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                error!("Failed to connect to Coinbase WebSocket: {:?}. Retrying in {}s...", e, backoff_secs);
            }
            Err(_) => {
                warn!("Timeout (5s) connecting to Coinbase WebSocket. Retrying in {}s...", backoff_secs);
            }
        }

        freshness.set_connected(Exchange::Coinbase, false);
        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(15);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_coinbase_ticker() {
        let json = r#"
        {
            "type": "ticker",
            "sequence": 123456,
            "product_id": "BTC-USD",
            "price": "67135.00",
            "best_bid": "67134.50",
            "best_ask": "67135.50",
            "side": "buy",
            "time": "2024-01-01T12:00:00.000000Z",
            "last_size": "0.12"
        }
        "#;

        let (tick_opt, trade_opt) = parse_coinbase_message(json, 1704110400050);
        let tick = tick_opt.expect("Should parse Coinbase ticker");
        assert_eq!(tick.exchange, Exchange::Coinbase);
        assert_eq!(tick.asset, Asset::BTC);
        assert_eq!(tick.bid, 67134.50);
        assert_eq!(tick.ask, 67135.50);

        let trade = trade_opt.expect("Should parse trade");
        assert_eq!(trade.price, 67135.00);
        assert_eq!(trade.size, 0.12);
        assert_eq!(trade.side, "buy");
    }
}
