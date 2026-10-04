use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};

use crate::polymarket::market_discovery::MarketDiscoveryEngine;
use crate::types::Asset;

pub const POLYMARKET_LIVE_WS_URL: &str = "wss://ws-live-data.polymarket.com";

pub fn symbol_to_asset(sym: &str) -> Option<Asset> {
    match sym.to_lowercase().as_str() {
        "btc/usd" | "btcusd" | "btc" => Some(Asset::BTC),
        "eth/usd" | "ethusd" | "eth" => Some(Asset::ETH),
        "sol/usd" | "solusd" | "sol" => Some(Asset::SOL),
        _ => None,
    }
}

pub fn parse_live_price_message(
    text: &str,
) -> Option<(Asset, f64, i64, &'static str)> {
    let val = serde_json::from_str::<Value>(text).ok()?;
    let topic = val.get("topic").and_then(|t| t.as_str()).unwrap_or("");

    // Prioritize 60s TWAP (official settlement benchmark) or Chainlink live spot feed
    let is_twap = topic == "crypto_prices_twap_sixty";
    let is_chainlink = topic == "crypto_prices_chainlink";
    if !is_twap && !is_chainlink {
        return None;
    }

    let payload = val.get("payload")?;
    let sym_str = payload.get("symbol").and_then(|s| s.as_str())?;
    let asset = symbol_to_asset(sym_str)?;

    let value = payload.get("value").and_then(|v| v.as_f64())?;
    let ts = payload
        .get("timestamp")
        .and_then(|t| t.as_i64())
        .unwrap_or_else(|| Utc::now().timestamp_millis());

    let source = if is_twap { "twap_60s" } else { "chainlink_spot" };
    Some((asset, value, ts, source))
}

pub async fn run_polymarket_live_feed(
    discovery: Arc<MarketDiscoveryEngine>,
) {
    let mut backoff_secs = 1u64;

    loop {
        info!("Connecting to Polymarket Official Live Data WebSocket ({}) ...", POLYMARKET_LIVE_WS_URL);

        match tokio::time::timeout(Duration::from_secs(6), connect_async(POLYMARKET_LIVE_WS_URL)).await {
            Ok(Ok((ws_stream, _))) => {
                info!("Successfully connected to Polymarket Official Live Data WebSocket.");
                backoff_secs = 1;

                let (mut write, mut read) = ws_stream.split();

                // Subscribe to official 60s TWAP benchmark and Chainlink spot for BTC, ETH, and SOL
                let sub_payload = serde_json::json!({
                    "action": "subscribe",
                    "subscriptions": [
                        { "topic": "crypto_prices_twap_sixty", "type": "update", "filters": "{\"symbol\":\"btc/usd\"}" },
                        { "topic": "crypto_prices_chainlink", "type": "update", "filters": "{\"symbol\":\"btc/usd\"}" },
                        { "topic": "crypto_prices_twap_sixty", "type": "update", "filters": "{\"symbol\":\"eth/usd\"}" },
                        { "topic": "crypto_prices_chainlink", "type": "update", "filters": "{\"symbol\":\"eth/usd\"}" },
                        { "topic": "crypto_prices_twap_sixty", "type": "update", "filters": "{\"symbol\":\"sol/usd\"}" },
                        { "topic": "crypto_prices_chainlink", "type": "update", "filters": "{\"symbol\":\"sol/usd\"}" }
                    ]
                });

                if let Err(e) = write.send(Message::Text(sub_payload.to_string().into())).await {
                    error!("Failed to send Polymarket live data subscribe command: {:?}", e);
                    continue;
                }

                let mut ping_interval = tokio::time::interval(Duration::from_secs(20));

                loop {
                    tokio::select! {
                        _ = ping_interval.tick() => {
                            if let Err(e) = write.send(Message::Text("PING".into())).await {
                                warn!("Failed to send Polymarket live WebSocket PING heartbeat: {:?}", e);
                                break;
                            }
                        }
                        msg = read.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    if text == "PONG" {
                                        continue;
                                    }
                                    if let Some((asset, price, ts, _source)) = parse_live_price_message(&text) {
                                        if price > 0.0 {
                                            discovery.set_latest_poly_price(asset, price, ts);
                                        }
                                    }
                                }
                                Some(Ok(Message::Close(_))) => {
                                    warn!("Polymarket live WebSocket received Close frame");
                                    break;
                                }
                                Some(Err(e)) => {
                                    warn!("Polymarket live WebSocket error: {:?}", e);
                                    break;
                                }
                                None => {
                                    warn!("Polymarket live WebSocket stream closed");
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                warn!("Failed to connect to Polymarket Live WebSocket: {:?}. Retrying in {}s...", e, backoff_secs);
            }
            Err(_) => {
                warn!("Timeout connecting to Polymarket Live WebSocket. Retrying in {}s...", backoff_secs);
            }
        }

        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(15);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_live_price_message() {
        let msg = r#"{
            "connection_id": "test_conn",
            "payload": {
                "symbol": "btc/usd",
                "timestamp": 1791144821000,
                "value": 85397.133,
                "window_s": 60
            },
            "timestamp": 1791144822135,
            "topic": "crypto_prices_twap_sixty",
            "type": "update"
        }"#;

        let parsed = parse_live_price_message(msg).expect("Should parse live price message");
        assert_eq!(parsed.0, Asset::BTC);
        assert_eq!(parsed.1, 85397.133);
        assert_eq!(parsed.2, 1791144821000);
        assert_eq!(parsed.3, "twap_60s");
    }
}
