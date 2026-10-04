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

        match tokio::time::timeout(Duration::from_secs(5), connect_async(POLYMARKET_CLOB_WS_URL)).await {
            Ok(Ok((ws_stream, _))) => {
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
            Ok(Err(e)) => {
                error!("Failed to connect to Polymarket CLOB WebSocket: {:?}. Retrying in {}s...", e, backoff_secs);
            }
            Err(_) => {
                warn!("Timeout (5s) connecting to Polymarket CLOB WebSocket. Retrying in {}s...", backoff_secs);
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

    #[test]
    fn test_clob_hmac_signature_generation() {
        let client = PolymarketClobHttpClient::new("https://clob.polymarket.com");
        let secret = "bXktc2VjcmV0LWtleS0xMjM0NQ=="; // base64 encoded
        let headers = client.generate_auth_headers(
            "test-api-key",
            secret,
            "test-passphrase",
            1710000000,
            "GET",
            "/balance-allowance",
            None,
        );
        assert!(headers.is_ok());
        let h = headers.unwrap();
        assert_eq!(h.get("poly-api-key").unwrap(), "test-api-key");
        assert_eq!(h.get("poly-timestamp").unwrap(), "1710000000");
        assert_eq!(h.get("poly-passphrase").unwrap(), "test-passphrase");
        assert!(h.contains_key("poly-signature"));
    }
}

// =============================================================================
// Polymarket CLOB HTTP Client for Real Live Trading
// =============================================================================

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClobOrderResponse {
    pub success: bool,
    pub order_id: Option<String>,
    pub status: String,
    pub filled_size: Option<f64>,
    pub avg_price: Option<f64>,
    pub error_msg: Option<String>,
}

#[derive(Clone)]
pub struct PolymarketClobHttpClient {
    base_url: String,
    http_client: reqwest::Client,
}

impl Default for PolymarketClobHttpClient {
    fn default() -> Self {
        Self::new("https://clob.polymarket.com")
    }
}

impl PolymarketClobHttpClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Generate Polymarket L2 HMAC-SHA256 signature headers
    pub fn generate_auth_headers(
        &self,
        api_key: &str,
        api_secret: &str,
        api_passphrase: &str,
        timestamp: u64,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Result<HeaderMap, String> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("poly-api-key"),
            HeaderValue::from_str(api_key).map_err(|e| e.to_string())?,
        );
        let _ = HeaderValue::from_str(api_key).map(|v| headers.insert(HeaderName::from_static("x-api-key"), v));

        // If secret is provided, generate full L2 HMAC-SHA256 signature
        if !api_secret.trim().is_empty() {
            let secret_bytes = BASE64.decode(api_secret).unwrap_or_else(|_| api_secret.as_bytes().to_vec());
            if let Ok(mut mac) = HmacSha256::new_from_slice(&secret_bytes) {
                let payload = format!("{}{}{}{}", timestamp, method.to_uppercase(), path, body.unwrap_or(""));
                mac.update(payload.as_bytes());
                let signature = BASE64.encode(mac.finalize().into_bytes());
                if let Ok(sig_val) = HeaderValue::from_str(&signature) {
                    headers.insert(HeaderName::from_static("poly-signature"), sig_val);
                }
            }
        }

        headers.insert(
            HeaderName::from_static("poly-timestamp"),
            HeaderValue::from_str(&timestamp.to_string()).map_err(|e| e.to_string())?,
        );

        if !api_passphrase.trim().is_empty() {
            if let Ok(pass_val) = HeaderValue::from_str(api_passphrase) {
                headers.insert(HeaderName::from_static("poly-passphrase"), pass_val);
            }
        }

        Ok(headers)
    }

    /// Try resolving proxy wallet address from EOA via Polymarket profile API
    pub async fn resolve_proxy_wallet(&self, eoa: &str) -> Option<String> {
        let clean_eoa = eoa.trim();
        if !clean_eoa.starts_with("0x") {
            return None;
        }
        let url = format!("https://polymarket.com/api/profile/userData?address={}", clean_eoa);
        if let Ok(resp) = self
            .http_client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(4))
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(proxy) = json.get("proxyWallet").and_then(|p| p.as_str()) {
                        if proxy.starts_with("0x") && !proxy.eq_ignore_ascii_case(clean_eoa) {
                            return Some(proxy.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// Query ERC-20 token balance via Polygon public RPC
    pub async fn fetch_polygon_erc20_balance(&self, token_address: &str, user_address: &str) -> Option<f64> {
        let clean_user = user_address.trim().to_lowercase();
        let clean_user = clean_user.strip_prefix("0x").unwrap_or(&clean_user);
        if clean_user.len() != 40 {
            return None;
        }
        let padded_addr = format!("{:0>64}", clean_user);
        let data = format!("0x70a08231{}", padded_addr); // balanceOf(address) selector: 0x70a08231

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [
                {
                    "to": token_address,
                    "data": data
                },
                "latest"
            ],
            "id": 1
        });

        let rpc_endpoints = [
            "https://polygon-bor-rpc.publicnode.com",
            "https://1rpc.io/matic",
            "https://polygon.drpc.org",
        ];

        for rpc in rpc_endpoints {
            if let Ok(resp) = self
                .http_client
                .post(rpc)
                .header("Content-Type", "application/json")
                .header("User-Agent", "Mozilla/5.0")
                .timeout(Duration::from_secs(3))
                .json(&payload)
                .send()
                .await
            {
                if resp.status().is_success() {
                    if let Ok(res_json) = resp.json::<serde_json::Value>().await {
                        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
                            let clean_hex = hex_str.strip_prefix("0x").unwrap_or(hex_str);
                            if let Ok(raw_u128) = u128::from_str_radix(clean_hex, 16) {
                                let balance = (raw_u128 as f64) / 1_000_000.0;
                                return Some(balance);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Fetch USDC Balance on Polygon for account
    pub async fn fetch_balance(&self, account: &crate::types::PolymarketAccount) -> Result<f64, String> {
        let mut target_addresses: Vec<String> = Vec::new();
        if let Some(ref proxy) = account.proxy_wallet_address {
            if !proxy.trim().is_empty() {
                target_addresses.push(proxy.trim().to_string());
            }
        }
        if !account.wallet_address.trim().is_empty() {
            let eoa = account.wallet_address.trim().to_string();
            if !target_addresses.contains(&eoa) {
                target_addresses.push(eoa);
            }
        }

        // If proxy is not set, try auto-resolving from EOA profile
        if account.proxy_wallet_address.as_deref().unwrap_or("").trim().is_empty() {
            if let Some(resolved) = self.resolve_proxy_wallet(&account.wallet_address).await {
                if !target_addresses.contains(&resolved) {
                    target_addresses.insert(0, resolved);
                }
            }
        }

        // 1. Try On-Chain ERC-20 balances on Polygon (Native USDC, USDC.e, pUSD)
        let tokens = [
            "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359", // Native USDC
            "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174", // Bridged USDC.e
            "0xc011a7e12a19f7b1f670d46f03b03f3342e82dfb", // PolyUSD (pUSD)
        ];

        let mut max_onchain_balance = 0.0;
        for addr in &target_addresses {
            let mut addr_sum = 0.0;
            for token in &tokens {
                if let Some(b) = self.fetch_polygon_erc20_balance(token, addr).await {
                    addr_sum += b;
                }
            }
            if addr_sum > max_onchain_balance {
                max_onchain_balance = addr_sum;
            }
        }

        if max_onchain_balance > 0.0 {
            return Ok(max_onchain_balance);
        }

        // 2. Try Polymarket public Data API (keyless, works for both proxy and EOA wallets)
        for addr in &target_addresses {
            let data_api_url = format!("https://data-api.polymarket.com/value?user={}", addr);
            if let Ok(resp) = self
                .http_client
                .get(&data_api_url)
                .timeout(std::time::Duration::from_secs(3))
                .send()
                .await
            {
                if resp.status().is_success() {
                    if let Ok(val_json) = resp.json::<serde_json::Value>().await {
                        let found_val = val_json
                            .get("value")
                            .and_then(|v| v.as_f64())
                            .or_else(|| {
                                val_json
                                    .as_array()
                                    .and_then(|arr| arr.first())
                                    .and_then(|item| item.get("value"))
                                    .and_then(|v| v.as_f64())
                            })
                            .or_else(|| val_json.as_f64());
                        if let Some(v) = found_val {
                            if v > 0.0 {
                                return Ok(v);
                            }
                        }
                    }
                }
            }
        }

        // 3. Try Polymarket CLOB /balance-allowance
        let path = "/balance-allowance?asset_type=COLLATERAL";
        let url = format!("{}{}", self.base_url, path);
        let timestamp = Utc::now().timestamp() as u64;

        if let Ok(headers) = self.generate_auth_headers(
            &account.api_key,
            &account.api_secret,
            &account.api_passphrase,
            timestamp,
            "GET",
            path,
            None,
        ) {
            if let Ok(resp) = self
                .http_client
                .get(&url)
                .headers(headers)
                .timeout(std::time::Duration::from_secs(4))
                .send()
                .await
            {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        let balance_str = json
                            .get("balance")
                            .and_then(|b| b.as_str())
                            .unwrap_or("0.0");
                        if let Ok(raw_val) = balance_str.parse::<f64>() {
                            let balance = if raw_val > 1000.0 && !balance_str.contains('.') {
                                raw_val / 1_000_000.0
                            } else {
                                raw_val
                            };
                            if balance > 0.0 {
                                return Ok(balance);
                            }
                        }
                    }
                }
            }
        }

        // Fallback to existing account balance if previously recorded
        Ok(account.balance_usdc)
    }

    /// Place a real order on Polymarket CLOB
    pub async fn place_order(
        &self,
        account: &crate::types::PolymarketAccount,
        token_id: &str,
        side: &str, // BUY or SELL
        price: f64,
        size: f64,
        order_type: &str, // FOK, GTC, IOC
    ) -> Result<ClobOrderResponse, String> {
        let path = "/order";
        let url = format!("{}{}", self.base_url, path);
        let timestamp = Utc::now().timestamp() as u64;

        let order_payload = serde_json::json!({
            "tokenID": token_id,
            "price": format!("{:.3}", price),
            "size": format!("{:.2}", size),
            "side": side.to_uppercase(),
            "orderType": order_type.to_uppercase(),
            "user": account.wallet_address,
        });
        let body_str = order_payload.to_string();

        let headers = self.generate_auth_headers(
            &account.api_key,
            &account.api_secret,
            &account.api_passphrase,
            timestamp,
            "POST",
            path,
            Some(&body_str),
        )?;

        let resp = self
            .http_client
            .post(&url)
            .headers(headers)
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await
            .map_err(|e| format!("Order submission failed: {:?}", e))?;

        let status_code = resp.status();
        let resp_text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read CLOB order response text: {:?}", e))?;

        if !status_code.is_success() {
            let error_msg = if let Ok(resp_json) = serde_json::from_str::<serde_json::Value>(&resp_text) {
                resp_json
                    .get("errorMsg")
                    .or_else(|| resp_json.get("message"))
                    .or_else(|| resp_json.get("error"))
                    .and_then(|m| m.as_str())
                    .unwrap_or(&resp_text)
                    .to_string()
            } else {
                format!("HTTP {}: {}", status_code, resp_text.trim())
            };
            return Ok(ClobOrderResponse {
                success: false,
                order_id: None,
                status: "FAILED".to_string(),
                filled_size: None,
                avg_price: None,
                error_msg: Some(error_msg),
            });
        }

        let resp_json: serde_json::Value = serde_json::from_str(&resp_text)
            .map_err(|e| format!("Failed to parse successful CLOB order response: {:?}", e))?;

        let order_id = resp_json
            .get("orderID")
            .or_else(|| resp_json.get("id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string());

        let status = resp_json
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("FILLED")
            .to_string();

        Ok(ClobOrderResponse {
            success: true,
            order_id,
            status,
            filled_size: Some(size),
            avg_price: Some(price),
            error_msg: None,
        })
    }

    /// Cancel a real open order on Polymarket CLOB
    pub async fn cancel_order(
        &self,
        account: &crate::types::PolymarketAccount,
        order_id: &str,
    ) -> Result<bool, String> {
        let path = "/order";
        let url = format!("{}{}", self.base_url, path);
        let timestamp = Utc::now().timestamp() as u64;

        let payload = serde_json::json!({ "orderID": order_id }).to_string();

        let headers = self.generate_auth_headers(
            &account.api_key,
            &account.api_secret,
            &account.api_passphrase,
            timestamp,
            "DELETE",
            path,
            Some(&payload),
        )?;

        let resp = self
            .http_client
            .delete(&url)
            .headers(headers)
            .header("Content-Type", "application/json")
            .body(payload)
            .send()
            .await
            .map_err(|e| format!("Cancel order failed: {:?}", e))?;

        Ok(resp.status().is_success())
    }
}

