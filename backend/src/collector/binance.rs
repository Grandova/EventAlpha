use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};

use crate::collector::freshness::FreshnessTracker;
use crate::types::{Asset, Exchange, MarketTick, TradeTick};

pub const BINANCE_STREAM_URL: &str = "wss://stream.binance.com:9443/stream?streams=btcusdt@bookTicker/ethusdt@bookTicker/solusdt@bookTicker/btcusdt@trade/ethusdt@trade/solusdt@trade";

pub fn symbol_to_asset(symbol: &str) -> Option<Asset> {
    match symbol.to_uppercase().as_str() {
        "BTCUSDT" => Some(Asset::BTC),
        "ETHUSDT" => Some(Asset::ETH),
        "SOLUSDT" => Some(Asset::SOL),
        _ => None,
    }
}

pub fn parse_binance_message(
    text: &str,
    receive_ts_ms: i64,
) -> (Option<MarketTick>, Option<TradeTick>) {
    let Ok(val) = serde_json::from_str::<Value>(text) else {
        return (None, None);
    };

    let stream = val.get("stream").and_then(|s| s.as_str()).unwrap_or("");
    let Some(data) = val.get("data") else {
        return (None, None);
    };

    if stream.ends_with("@bookTicker") {
        let symbol = data.get("s").and_then(|s| s.as_str()).unwrap_or("");
        let Some(asset) = symbol_to_asset(symbol) else {
            return (None, None);
        };

        let bid = data
            .get("b")
            .and_then(|b| b.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let ask = data
            .get("a")
            .and_then(|a| a.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        if bid <= 0.0 || ask <= 0.0 {
            return (None, None);
        }

        let mid = (bid + ask) / 2.0;
        let latency_ms = 0; // BookTicker does not contain an exchange timestamp in stream wrapper

        let tick = MarketTick {
            exchange: Exchange::Binance,
            symbol: symbol.to_string(),
            asset,
            exchange_timestamp_ms: receive_ts_ms,
            receive_timestamp_ms: receive_ts_ms,
            latency_ms,
            bid,
            ask,
            mid,
            last: mid,
            volume_24h: 0.0,
        };
        (Some(tick), None)
    } else if stream.ends_with("@trade") {
        let symbol = data.get("s").and_then(|s| s.as_str()).unwrap_or("");
        let Some(asset) = symbol_to_asset(symbol) else {
            return (None, None);
        };

        let price = data
            .get("p")
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let size = data
            .get("q")
            .and_then(|q| q.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let exchange_ts = data
            .get("T")
            .and_then(|t| t.as_i64())
            .unwrap_or(receive_ts_ms);
        let is_buyer_maker = data
            .get("m")
            .and_then(|m| m.as_bool())
            .unwrap_or(false);

        let side = if is_buyer_maker { "sell" } else { "buy" };
        let latency_ms = (receive_ts_ms - exchange_ts).max(0);

        let trade = TradeTick {
            exchange: Exchange::Binance,
            symbol: symbol.to_string(),
            asset,
            exchange_timestamp_ms: exchange_ts,
            receive_timestamp_ms: receive_ts_ms,
            latency_ms,
            price,
            size,
            side: side.to_string(),
            is_aggressive: true,
        };
        (None, Some(trade))
    } else {
        (None, None)
    }
}

pub async fn run_binance_collector(
    market_tx: broadcast::Sender<MarketTick>,
    trade_tx: broadcast::Sender<TradeTick>,
    freshness: FreshnessTracker,
) {
    let mut backoff_secs = 1u64;

    loop {
        info!("Connecting to Binance WebSocket stream...");
        freshness.set_connected(Exchange::Binance, false);

        match connect_async(BINANCE_STREAM_URL).await {
            Ok((ws_stream, _)) => {
                info!("Successfully connected to Binance WebSocket.");
                freshness.set_connected(Exchange::Binance, true);
                backoff_secs = 1;

                let (mut write, mut read) = ws_stream.split();

                // Heartbeat ping interval
                let mut ping_interval = tokio::time::interval(Duration::from_secs(20));

                loop {
                    tokio::select! {
                        _ = ping_interval.tick() => {
                            if let Err(e) = write.send(Message::Ping(vec![].into())).await {
                                warn!("Failed to send Binance WebSocket Ping: {:?}", e);
                                break;
                            }
                        }
                        msg = read.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    let now_ms = Utc::now().timestamp_millis();
                                    let (m_tick, t_tick) = parse_binance_message(&text, now_ms);

                                    if let Some(tick) = m_tick {
                                        freshness.record_tick(
                                            Exchange::Binance,
                                            tick.asset,
                                            tick.exchange_timestamp_ms,
                                            now_ms,
                                        );
                                        let _ = market_tx.send(tick);
                                    }

                                    if let Some(trade) = t_tick {
                                        freshness.record_tick(
                                            Exchange::Binance,
                                            trade.asset,
                                            trade.exchange_timestamp_ms,
                                            now_ms,
                                        );
                                        let _ = trade_tx.send(trade);
                                    }
                                }
                                Some(Ok(Message::Ping(payload))) => {
                                    let _ = write.send(Message::Pong(payload)).await;
                                }
                                Some(Ok(Message::Close(_))) => {
                                    warn!("Binance WebSocket received Close frame");
                                    break;
                                }
                                Some(Err(e)) => {
                                    error!("Binance WebSocket stream error: {:?}", e);
                                    break;
                                }
                                None => {
                                    warn!("Binance WebSocket stream ended (None)");
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to connect to Binance WebSocket: {:?}. Retrying in {}s...", e, backoff_secs);
            }
        }

        freshness.set_connected(Exchange::Binance, false);
        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(15);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_binance_book_ticker() {
        let json = r#"
        {
            "stream": "btcusdt@bookTicker",
            "data": {
                "u": 400900217,
                "s": "BTCUSDT",
                "b": "67123.45000000",
                "B": "1.50000000",
                "a": "67124.50000000",
                "A": "2.30000000"
            }
        }
        "#;

        let (tick_opt, _) = parse_binance_message(json, 1700000000000);
        let tick = tick_opt.expect("Should parse bookTicker");
        assert_eq!(tick.exchange, Exchange::Binance);
        assert_eq!(tick.asset, Asset::BTC);
        assert_eq!(tick.bid, 67123.45);
        assert_eq!(tick.ask, 67124.50);
        assert_eq!(tick.mid, (67123.45 + 67124.50) / 2.0);
    }

    #[test]
    fn test_parse_binance_trade() {
        let json = r#"
        {
            "stream": "btcusdt@trade",
            "data": {
                "e": "trade",
                "E": 1700000000100,
                "s": "BTCUSDT",
                "t": 12345,
                "p": "67124.00",
                "q": "0.05",
                "T": 1700000000050,
                "m": false
            }
        }
        "#;

        let (_, trade_opt) = parse_binance_message(json, 1700000000200);
        let trade = trade_opt.expect("Should parse trade");
        assert_eq!(trade.exchange, Exchange::Binance);
        assert_eq!(trade.asset, Asset::BTC);
        assert_eq!(trade.price, 67124.0);
        assert_eq!(trade.size, 0.05);
        assert_eq!(trade.side, "buy");
        assert_eq!(trade.latency_ms, 150);
    }
}
