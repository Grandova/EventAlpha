use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};

use crate::polymarket::orderbook::PolymarketBookEngine;
use crate::types::{Asset, MarketSide, OrderBookLevel};

pub const POLYMARKET_CLOB_WS_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";

pub fn parse_clob_book_message(
    val: &Value,
    asset: Asset,
    side: MarketSide,
    engine: &PolymarketBookEngine,
    now_ms: i64,
) -> bool {
    let event_type = val.get("event_type").and_then(|t| t.as_str()).unwrap_or("");
    let asset_id = val.get("asset_id").and_then(|a| a.as_str()).unwrap_or("");

    if event_type == "book" {
        let mut bids = Vec::new();
        if let Some(bids_arr) = val.get("bids").and_then(|b| b.as_array()) {
            for b in bids_arr {
                let price = b.get("price").and_then(|p| p.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                let size = b.get("size").and_then(|s| s.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                if price > 0.0 && size > 0.0 {
                    bids.push(OrderBookLevel { price, size });
                }
            }
        }

        let mut asks = Vec::new();
        if let Some(asks_arr) = val.get("asks").and_then(|a| a.as_array()) {
            for a in asks_arr {
                let price = a.get("price").and_then(|p| p.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                let size = a.get("size").and_then(|s| s.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                if price > 0.0 && size > 0.0 {
                    asks.push(OrderBookLevel { price, size });
                }
            }
        }

        engine.update_book(asset_id, asset, side, bids, asks, now_ms);
        true
    } else {
        false
    }
}

pub async fn run_polymarket_clob_collector(
    engine: Arc<PolymarketBookEngine>,
    token_subs: Vec<String>,
) {
    let mut backoff_secs = 1u64;

    loop {
        if token_subs.is_empty() {
            // Idle wait until specific on-chain tokens are passed
            tokio::time::sleep(Duration::from_secs(15)).await;
            continue;
        }

        info!("Connecting to Polymarket CLOB WebSocket: {}", POLYMARKET_CLOB_WS_URL);

        match connect_async(POLYMARKET_CLOB_WS_URL).await {
            Ok((ws_stream, _)) => {
                info!("Successfully connected to Polymarket CLOB WebSocket.");
                backoff_secs = 1;

                let (mut write, mut read) = ws_stream.split();

                // Send subscription payload if token subscriptions exist
                if !token_subs.is_empty() {
                    let sub_payload = serde_json::json!({
                        "assets_ids": token_subs,
                        "type": "market",
                        "custom_feature_enabled": true
                    });

                    if let Err(e) = write.send(Message::Text(sub_payload.to_string().into())).await {
                        error!("Failed to send Polymarket CLOB subscribe: {:?}", e);
                        continue;
                    }
                }

                // Heartbeat ping every 10s
                let mut ping_interval = tokio::time::interval(Duration::from_secs(10));

                loop {
                    tokio::select! {
                        _ = ping_interval.tick() => {
                            if let Err(e) = write.send(Message::Text("PING".into())).await {
                                warn!("Failed to send Polymarket CLOB PING: {:?}", e);
                                break;
                            }
                        }
                        msg = read.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    if text == "PONG" {
                                        continue;
                                    }
                                    let now_ms = Utc::now().timestamp_millis();
                                    if let Ok(val) = serde_json::from_str::<Value>(&text) {
                                        // Try parse as book message
                                        let asset_id = val.get("asset_id").and_then(|a| a.as_str()).unwrap_or("");
                                        let (asset, side) = if asset_id.contains("BTC") {
                                            (Asset::BTC, if asset_id.contains("UP") { MarketSide::Up } else { MarketSide::Down })
                                        } else if asset_id.contains("ETH") {
                                            (Asset::ETH, if asset_id.contains("UP") { MarketSide::Up } else { MarketSide::Down })
                                        } else {
                                            (Asset::SOL, if asset_id.contains("UP") { MarketSide::Up } else { MarketSide::Down })
                                        };
                                        parse_clob_book_message(&val, asset, side, &engine, now_ms);
                                    }
                                }
                                Some(Ok(Message::Close(_))) => {
                                    warn!("Polymarket CLOB WebSocket received Close frame");
                                    break;
                                }
                                Some(Err(e)) => {
                                    error!("Polymarket CLOB WebSocket error: {:?}", e);
                                    break;
                                }
                                None => {
                                    warn!("Polymarket CLOB WebSocket closed");
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to connect to Polymarket CLOB WebSocket: {:?}. Retrying in {}s...", e, backoff_secs);
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
    fn test_parse_clob_book_message() {
        let engine = PolymarketBookEngine::new();
        let json_text = r#"
        {
            "event_type": "book",
            "asset_id": "BTC_5M_UP",
            "bids": [{"price": "0.64", "size": "150.0"}, {"price": "0.63", "size": "300.0"}],
            "asks": [{"price": "0.65", "size": "200.0"}, {"price": "0.66", "size": "400.0"}],
            "timestamp": "1710000000"
        }
        "#;
        let val: Value = serde_json::from_str(json_text).unwrap();
        let parsed = parse_clob_book_message(&val, Asset::BTC, MarketSide::Up, &engine, 1710000000100);
        assert!(parsed);

        let book = engine.get_market_summary("BTC_5M_TEST", Asset::BTC);
        assert!(book.is_none()); // Down book not seeded yet

        // Seed Down book
        let down_json = r#"
        {
            "event_type": "book",
            "asset_id": "BTC_5M_DOWN",
            "bids": [{"price": "0.34", "size": "100.0"}],
            "asks": [{"price": "0.36", "size": "100.0"}],
            "timestamp": "1710000000"
        }
        "#;
        let val2: Value = serde_json::from_str(down_json).unwrap();
        parse_clob_book_message(&val2, Asset::BTC, MarketSide::Down, &engine, 1710000000100);

        let summary = engine.get_market_summary("BTC_5M_TEST", Asset::BTC).expect("Both books ready");
        assert_eq!(summary.up_book.best_bid, Some(0.64));
        assert_eq!(summary.up_book.best_ask, Some(0.65));
        assert_eq!(summary.implied_prob_up, 0.645);
    }
}
