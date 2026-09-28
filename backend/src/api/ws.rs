use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::time::{interval, Duration};
use tracing::debug;

use super::AppState;
use crate::collector::PriceSummary;
use crate::polymarket::resolution::MarketResolvedEvent;
use crate::types::{Asset, PredictionSignal};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum WsEvent {
    Heartbeat {
        timestamp_ms: i64,
        uptime_secs: u64,
        is_fresh: bool,
    },
    Ticker {
        asset: Asset,
        composite_price: f64,
        spot_prices: Vec<PriceSummary>,
    },
    Signal {
        signal: PredictionSignal,
    },
    Resolution {
        event: MarketResolvedEvent,
    },
    Bankroll {
        active: f64,
        locked: f64,
        total: f64,
    },
}

pub async fn handle_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    let mut signal_rx = state.strategy.subscribe_signals();
    let mut resolution_rx = state.polymarket.subscribe_resolutions();
    let mut ticker_timer = interval(Duration::from_millis(500));
    let mut heartbeat_timer = interval(Duration::from_millis(1000));
    let mut ping_timer = interval(Duration::from_secs(15));

    debug!("New WebSocket client connected to /api/v1/ws");

    // Spawn client reader to handle incoming messages and close frames
    let mut client_task = tokio::spawn(async move {
        while let Some(res) = receiver.next().await {
            match res {
                Ok(Message::Close(_)) | Err(_) => break,
                Ok(Message::Ping(_)) => {
                    // Axum automatically replies to pings
                }
                _ => {}
            }
        }
    });

    let mut send_task = tokio::spawn(async move {
        let assets = [Asset::BTC, Asset::ETH, Asset::SOL];

        loop {
            tokio::select! {
                // 0. Periodic ping (15s) to detect dead sockets immediately
                _ = ping_timer.tick() => {
                    if sender.send(Message::Ping(vec![].into())).await.is_err() {
                        return;
                    }
                }
                // 1. Periodic Ticker update (500ms)
                _ = ticker_timer.tick() => {
                    let now_ms = Utc::now().timestamp_millis();
                    let spot_prices = state.collector.get_prices();

                    for &asset in &assets {
                        if let Some(comp) = state.composite.calculate_snapshot(asset, now_ms) {
                            let asset_spots: Vec<PriceSummary> = spot_prices
                                .get(&asset.to_string())
                                .map(|m| m.values().cloned().collect())
                                .unwrap_or_default();

                            let event = WsEvent::Ticker {
                                asset,
                                composite_price: comp.composite_price,
                                spot_prices: asset_spots,
                            };
                            if let Ok(json) = serde_json::to_string(&event) {
                                if sender.send(Message::Text(json.into())).await.is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }

                // 2. Periodic Heartbeat & Bankroll update (1000ms)
                _ = heartbeat_timer.tick() => {
                    let now_ms = Utc::now().timestamp_millis();
                    let uptime_secs = ((now_ms - state.start_time_ms) / 1000).max(0) as u64;
                    let is_fresh = state.collector.get_freshness_report().is_system_fresh;

                    let hb = WsEvent::Heartbeat {
                        timestamp_ms: now_ms,
                        uptime_secs,
                        is_fresh,
                    };
                    if let Ok(json) = serde_json::to_string(&hb) {
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            return;
                        }
                    }

                    // Bankroll snapshot
                    let br = state.risk.get_bankroll_state().await;
                    let br_event = WsEvent::Bankroll {
                        active: br.active_bankroll,
                        locked: br.locked_profit,
                        total: br.total_equity,
                    };
                    if let Ok(json) = serde_json::to_string(&br_event) {
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            return;
                        }
                    }
                }

                // 3. New Strategy Signal event
                Ok(sig) = signal_rx.recv() => {
                    let event = WsEvent::Signal { signal: sig };
                    if let Ok(json) = serde_json::to_string(&event) {
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            return;
                        }
                    }
                }

                // 4. Market Resolution event
                Ok(res) = resolution_rx.recv() => {
                    let event = WsEvent::Resolution { event: res };
                    if let Ok(json) = serde_json::to_string(&event) {
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            return;
                        }
                    }
                }
            }
        }
    });

    // Wait for either send or receive to terminate
    tokio::select! {
        _ = (&mut client_task) => {
            send_task.abort();
        }
        _ = (&mut send_task) => {
            client_task.abort();
        }
    }

    debug!("WebSocket client disconnected from /api/v1/ws");
}
