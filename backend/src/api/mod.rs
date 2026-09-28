use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::collector::freshness::FreshnessReport;
use crate::collector::{CollectorManager, PriceSummary};
use crate::composite::{CompositePriceEngine, CompositePriceSnapshot};
use crate::config::AppConfig;
use crate::db::Database;
use crate::polymarket::market_discovery::Polymarket5mMarket;
use crate::polymarket::orderbook::MarketBookSummary;
use crate::polymarket::resolution::MarketResolvedEvent;
use crate::polymarket::PolymarketManager;
use crate::safety::{SafetyGuard, SafetyStatus};
use crate::types::{Asset, BankrollState};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: Arc<Database>,
    pub collector: Arc<CollectorManager>,
    pub polymarket: Arc<PolymarketManager>,
    pub composite: Arc<CompositePriceEngine>,
    pub start_time_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub mode: String,
    pub uptime_secs: u64,
    pub timestamp_ms: i64,
    pub real_trading_enabled: bool,
    pub safety_status: SafetyStatus,
    pub exchange_freshness: FreshnessReport,
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDisplayInfo {
    #[serde(flatten)]
    pub market: Polymarket5mMarket,
    pub remaining_seconds: i64,
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/v1/health", get(handle_health))
        .route("/api/v1/safety", get(handle_safety))
        .route("/api/v1/config", get(handle_config))
        .route("/api/v1/paper/bankroll", get(handle_bankroll))
        .route("/api/v1/events", get(handle_events))
        .route("/api/v1/collector/status", get(handle_collector_status))
        .route("/api/v1/collector/prices", get(handle_collector_prices))
        .route("/api/v1/polymarket/markets", get(handle_polymarket_markets))
        .route("/api/v1/polymarket/book/{asset}", get(handle_polymarket_book))
        .route("/api/v1/polymarket/resolutions", get(handle_polymarket_resolutions))
        .route("/api/v1/composite/price/{asset}", get(handle_composite_price))
        .route("/api/v1/composite/all", get(handle_composite_all))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn handle_health(State(state): State<AppState>) -> Json<HealthResponse> {
    let now_ms = Utc::now().timestamp_millis();
    let uptime_secs = ((now_ms - state.start_time_ms) / 1000).max(0) as u64;

    let safety = SafetyGuard::enforce_paper_only(state.config.safety.real_trading_enabled)
        .unwrap_or_else(|e| SafetyStatus {
            real_trading_enabled: true,
            paper_trading_only: false,
            safety_lock_engaged: false,
            safety_signature: "CORRUPTED".to_string(),
            private_key_detected: false,
            message: e,
        });

    let freshness = state.collector.get_freshness_report();

    Json(HealthResponse {
        status: "ok".to_string(),
        service: "poly-quant-core".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        mode: state.config.mode.clone(),
        uptime_secs,
        timestamp_ms: now_ms,
        real_trading_enabled: state.config.safety.real_trading_enabled,
        safety_status: safety,
        exchange_freshness: freshness,
    })
}

async fn handle_safety(State(state): State<AppState>) -> Result<Json<SafetyStatus>, StatusCode> {
    match SafetyGuard::enforce_paper_only(state.config.safety.real_trading_enabled) {
        Ok(status) => Ok(Json(status)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn handle_config(State(state): State<AppState>) -> Json<AppConfig> {
    Json((*state.config).clone())
}

async fn handle_bankroll(
    State(state): State<AppState>,
) -> Result<Json<BankrollState>, StatusCode> {
    match state.db.get_or_init_bankroll(&state.config).await {
        Ok(bankroll) => Ok(Json(bankroll)),
        Err(e) => {
            tracing::error!("Error retrieving bankroll: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn handle_events(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    match state.db.get_recent_events(limit).await {
        Ok(events) => Ok(Json(events)),
        Err(e) => {
            tracing::error!("Error retrieving events: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn handle_collector_status(State(state): State<AppState>) -> Json<FreshnessReport> {
    Json(state.collector.get_freshness_report())
}

async fn handle_collector_prices(
    State(state): State<AppState>,
) -> Json<HashMap<String, HashMap<String, PriceSummary>>> {
    Json(state.collector.get_prices())
}

async fn handle_polymarket_markets(
    State(state): State<AppState>,
) -> Json<Vec<MarketDisplayInfo>> {
    let now_ms = Utc::now().timestamp_millis();
    let markets = state.polymarket.discovery().list_active_markets();
    let display_list: Vec<MarketDisplayInfo> = markets
        .into_iter()
        .map(|m| {
            let rem = m.remaining_seconds(now_ms);
            MarketDisplayInfo {
                market: m,
                remaining_seconds: rem,
            }
        })
        .collect();

    Json(display_list)
}

async fn handle_polymarket_book(
    State(state): State<AppState>,
    Path(asset_str): Path<String>,
) -> Result<Json<MarketBookSummary>, StatusCode> {
    let Ok(asset) = asset_str.parse::<Asset>() else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let active_market = state.polymarket.discovery().get_active_market(asset);
    let market_id = active_market
        .map(|m| m.id)
        .unwrap_or_else(|| format!("{}-5M-DEFAULT", asset));

    match state.polymarket.book_engine().get_market_summary(&market_id, asset) {
        Some(summary) => Ok(Json(summary)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn handle_polymarket_resolutions(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<MarketResolvedEvent>>, StatusCode> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    match state.polymarket.resolution().get_recently_resolved(limit).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => {
            tracing::error!("Failed to fetch resolutions: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn handle_composite_price(
    State(state): State<AppState>,
    Path(asset_str): Path<String>,
) -> Result<Json<CompositePriceSnapshot>, StatusCode> {
    let Ok(asset) = asset_str.parse::<Asset>() else {
        return Err(StatusCode::BAD_REQUEST);
    };

    match state.composite.get_latest_snapshot(asset) {
        Some(snapshot) => Ok(Json(snapshot)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn handle_composite_all(
    State(state): State<AppState>,
) -> Json<HashMap<String, CompositePriceSnapshot>> {
    Json(state.composite.get_all_snapshots())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn create_test_state() -> AppState {
        let yaml = r#"
mode: "paper"
safety:
  real_trading_enabled: false
  safety_signature: "PAPER_TRADING_ONLY_SAFETY_LOCK_ENGAGED"
server:
  host: "127.0.0.1"
  port: 8080
  cors_allowed_origins: []
database:
  url: "sqlite://:memory:"
  max_connections: 2
  min_connections: 1
  acquire_timeout_secs: 5
bankroll:
  initial: 10.0
  cap: 10.0
  minimum: 2.0
  mode: "capital_recovery"
position:
  mode: "fixed"
  stake: 1.0
  stake_percent: 0.10
  max_stake: 1.0
strategy:
  strategy_version: "v1.0.0"
  min_probability: 0.70
  min_net_edge: 0.05
  max_entry_price: 0.85
  max_spread: 0.04
  min_liquidity: 300.0
  min_time_remaining_sec: 15
  max_time_remaining_sec: 285
  score_thresholds:
    skip_below: 60.0
    low: 60.0
    medium: 70.0
    high: 80.0
    very_high: 90.0
risk:
  daily_loss_limit: 2.0
  max_drawdown: 0.20
  max_consecutive_losses: 5
  cooldown_minutes: 30
execution:
  latency_ms: 250
  orderbook_depth_fill: true
  fee_rate: 0.012
  slippage_rate: 0.005
freshness:
  stale_timeout_ms: 2000
  max_clock_skew_ms: 1000
assets: ["BTC", "ETH", "SOL"]
"#;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        file.write_all(yaml.as_bytes()).unwrap();

        let config = Arc::new(AppConfig::load_from_path(file.path()).unwrap());
        let db = Arc::new(Database::new(&config).await.unwrap());
        let collector = Arc::new(CollectorManager::new(&config));
        let polymarket = Arc::new(PolymarketManager::new(&config, db.clone()));
        let composite = Arc::new(CompositePriceEngine::new(collector.clone(), polymarket.discovery().clone()));

        AppState {
            config,
            db,
            collector,
            polymarket,
            composite,
            start_time_ms: Utc::now().timestamp_millis(),
        }
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = create_test_state().await;
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let health: HealthResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(health.status, "ok");
    }

    #[tokio::test]
    async fn test_polymarket_markets_endpoint() {
        let state = create_test_state().await;
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/polymarket/markets")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
