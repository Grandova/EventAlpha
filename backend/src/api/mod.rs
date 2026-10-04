pub mod auth;
pub mod ws;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::backtest::{BacktestEngine, BacktestRequest, BacktestResult};
use crate::collector::freshness::FreshnessReport;
use crate::collector::{CollectorManager, PriceSummary};
use crate::replay::{
    ReplayConfig, ReplayEngine, ReplayFrame, ReplayStateResponse, SeekRequest, SpeedRequest,
};
use crate::composite::{CompositePriceEngine, CompositePriceSnapshot};
use crate::config::AppConfig;
use crate::dataset::{DatasetExporter, DatasetSummary};
use crate::db::Database;
use crate::features::{FeatureEngine, FeatureSnapshot, FEATURE_NAMES};
use crate::models::{ModelManager, ModelPrediction};
use crate::polymarket::market_discovery::Polymarket5mMarket;
use crate::polymarket::orderbook::MarketBookSummary;
use crate::execution::PaperExecutionEngine;
use crate::polymarket::resolution::MarketResolvedEvent;
use crate::polymarket::PolymarketManager;
use crate::risk::RiskManager;
use crate::safety::{SafetyGuard, SafetyStatus};
use crate::strategy::StrategyEngine;
use crate::types::{
    Asset, BankrollHistoryEntry, BankrollState, DecisionLog, PaperOrder, PaperPosition,
    PaperResult, PredictionSignal, RiskStatus, TradeStatistics,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: Arc<Database>,
    pub collector: Arc<CollectorManager>,
    pub polymarket: Arc<PolymarketManager>,
    pub composite: Arc<CompositePriceEngine>,
    pub features: Arc<FeatureEngine>,
    pub models: Arc<ModelManager>,
    pub strategy: Arc<StrategyEngine>,
    pub execution: Arc<PaperExecutionEngine>,
    pub live_execution: Arc<crate::execution::LiveExecutionEngine>,
    pub self_learning: Arc<crate::models::SelfLearningEngine>,
    pub risk: Arc<RiskManager>,
    pub live_risk: Arc<RiskManager>,
    pub backtest: Arc<BacktestEngine>,
    pub replay: Arc<ReplayEngine>,
    pub sessions: Arc<dashmap::DashMap<String, auth::SessionInfo>>,
    pub start_time_ms: i64,
}

impl AppState {
    pub fn new(
        config: Arc<AppConfig>,
        db: Arc<Database>,
        collector: Arc<CollectorManager>,
        polymarket: Arc<PolymarketManager>,
        composite: Arc<CompositePriceEngine>,
        features: Arc<FeatureEngine>,
        models: Arc<ModelManager>,
        strategy: Arc<StrategyEngine>,
        execution: Arc<PaperExecutionEngine>,
        risk: Arc<RiskManager>,
        backtest: Arc<BacktestEngine>,
        replay: Arc<ReplayEngine>,
        start_time_ms: i64,
    ) -> Self {
        let clob_http = Arc::new(crate::polymarket::PolymarketClobHttpClient::default());
        let live_execution = Arc::new(crate::execution::LiveExecutionEngine::new(db.clone(), clob_http));
        let self_learning = Arc::new(crate::models::SelfLearningEngine::new(models.clone(), db.clone()));
        let live_risk = Arc::new(crate::risk::RiskManager::new_live(
            config.bankroll.clone(),
            config.risk.clone(),
            crate::types::BankrollState {
                initial_bankroll: 0.0,
                active_bankroll: 0.0,
                bankroll_cap: 10.0,
                locked_profit: 0.0,
                total_equity: 0.0,
                minimum_bankroll: 0.50,
                mode: crate::types::BankrollMode::CapitalRecovery,
                daily_loss_current: 0.0,
                consecutive_losses: 0,
                peak_equity: 0.0,
                current_drawdown: 0.0,
                is_trading_halted: false,
                halt_reason: None,
            },
            db.clone(),
        ));
        Self {
            config,
            db,
            collector,
            polymarket,
            composite,
            features,
            models,
            strategy,
            execution,
            live_execution,
            self_learning,
            risk,
            live_risk,
            backtest,
            replay,
            sessions: Arc::new(dashmap::DashMap::new()),
            start_time_ms,
        }
    }
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

    let router = Router::new()
        .route("/health", get(handle_health))
        .route("/api/v1/health", get(handle_health))
        .route("/api/v1/auth/login", post(auth::handle_login))
        .route("/api/v1/auth/me", get(auth::handle_auth_me))
        .route("/api/v1/auth/logout", post(auth::handle_logout))
        .route("/api/v1/safety", get(handle_safety))
        .route("/api/v1/config", get(handle_config))
        .route("/api/v1/paper/bankroll", get(handle_bankroll))
        .route("/api/v1/paper/bankroll/set", post(handle_bankroll_set))
        .route("/api/v1/paper/bankroll/unlock", post(handle_bankroll_unlock))
        .route("/api/v1/paper/bankroll/history", get(handle_bankroll_history))
        .route("/api/v1/live/bankroll", get(handle_live_bankroll))
        .route("/api/v1/live/bankroll/set", post(handle_live_bankroll_set))
        .route("/api/v1/live/bankroll/unlock", post(handle_live_bankroll_unlock))
        .route("/api/v1/live/bankroll/history", get(handle_live_bankroll_history))
        .route("/api/v1/risk/status", get(handle_risk_status))
        .route("/api/v1/risk/config", post(handle_risk_config_update))
        .route("/api/v1/risk/unhalt", post(handle_risk_unhalt))
        .route("/api/v1/live/risk/status", get(handle_live_risk_status))
        .route("/api/v1/live/risk/config", post(handle_live_risk_config_update))
        .route("/api/v1/live/risk/unhalt", post(handle_live_risk_unhalt))
        .route("/api/v1/events", get(handle_events))
        .route("/api/v1/collector/status", get(handle_collector_status))
        .route("/api/v1/collector/prices", get(handle_collector_prices))
        .route("/api/v1/polymarket/markets", get(handle_polymarket_markets))
        .route("/api/v1/polymarket/book/{asset}", get(handle_polymarket_book))
        .route("/api/v1/polymarket/resolutions", get(handle_polymarket_resolutions))
        .route("/api/v1/composite/price/{asset}", get(handle_composite_price))
        .route("/api/v1/composite/all", get(handle_composite_all))
        .route("/api/v1/features/latest/{asset}", get(handle_features_latest))
        .route("/api/v1/features/all", get(handle_features_all))
        .route("/api/v1/features/names", get(handle_features_names))
        .route("/api/v1/dataset/summary", get(handle_dataset_summary))
        .route("/api/v1/models/prediction/{asset}", get(handle_model_prediction))
        .route("/api/v1/models/all", get(handle_models_all))
        .route("/api/v1/strategy/signals/latest/{asset}", get(handle_strategy_signal_latest))
        .route("/api/v1/strategy/signals/latest", get(handle_strategy_signals_all))
        .route("/api/v1/strategy/decisions/recent", get(handle_strategy_decisions_recent))
        .route("/api/v1/paper/orders", get(handle_paper_orders))
        .route("/api/v1/paper/positions/active", get(handle_paper_positions_active))
        .route("/api/v1/paper/positions/history", get(handle_paper_positions_history))
        .route("/api/v1/paper/results", get(handle_paper_results))
        .route("/api/v1/paper/statistics", get(handle_paper_statistics))
        .route("/api/v1/backtest/run", post(handle_backtest_run))
        .route("/api/v1/backtest/latest", get(handle_backtest_latest))
        .route("/api/v1/backtest/history", get(handle_backtest_history))
        .route("/api/v1/replay/start", post(handle_replay_start))
        .route("/api/v1/replay/pause", post(handle_replay_pause))
        .route("/api/v1/replay/resume", post(handle_replay_resume))
        .route("/api/v1/replay/step", post(handle_replay_step))
        .route("/api/v1/replay/seek", post(handle_replay_seek))
        .route("/api/v1/replay/speed", post(handle_replay_speed))
        .route("/api/v1/replay/stop", post(handle_replay_stop))
        .route("/api/v1/replay/status", get(handle_replay_status))
        .route("/api/v1/replay/frames", get(handle_replay_frames))
        .route("/api/v1/ws", get(ws::handle_ws_upgrade))
        .route("/api/v1/strategy/config", get(handle_strategy_config_get).post(handle_strategy_config_update))
        .route("/api/v1/strategy/assets", get(handle_strategy_assets_get).post(handle_strategy_assets_set))
        .route("/api/v1/strategy/autotrade", get(handle_strategy_autotrade_get).post(handle_strategy_autotrade_set))
        .route("/api/v1/models/config", get(handle_model_config))
        .route("/api/v1/models/train", post(handle_model_train))
        .route("/api/v1/dataset/generate_synthetic", post(handle_dataset_generate_synthetic))
        // Polymarket Accounts & Live Trading
        .route("/api/v1/trading/mode", get(handle_trading_mode_get).post(handle_trading_mode_set))
        .route("/api/v1/accounts", get(handle_accounts_get).post(handle_accounts_create))
        .route("/api/v1/accounts/{id}/activate", post(handle_account_activate))
        .route("/api/v1/accounts/{id}", put(handle_account_update).delete(handle_account_delete))
        .route("/api/v1/accounts/{id}/balance", get(handle_account_balance))
        .route("/api/v1/accounts/{id}/calibrate-balance", post(handle_account_calibrate_balance))
        .route("/api/v1/real/orders", get(handle_real_orders_get))
        .route("/api/v1/real/emergency_halt", post(handle_real_emergency_halt))
        .route("/api/v1/trade/manual", post(handle_trade_manual))
        // Self-Learning & Auto-Evolution
        .route("/api/v1/learning/status", get(handle_learning_status))
        .route("/api/v1/learning/toggle", post(handle_learning_toggle))
        .route("/api/v1/learning/retrain", post(handle_learning_retrain))
        .route("/api/v1/learning/history", get(handle_learning_history));

    let candidates = [
        std::env::var("FRONTEND_DIST_PATH").ok(),
        Some("frontend/dist".to_string()),
        Some("../frontend/dist".to_string()),
        Some("../../frontend/dist".to_string()),
        Some("/opt/polyquant/frontend/dist".to_string()),
        Some("/var/www/polyquant/frontend/dist".to_string()),
    ];

    let mut dist_path: Option<std::path::PathBuf> = None;
    for cand in candidates.into_iter().flatten() {
        let p = std::path::Path::new(&cand);
        if p.exists() && p.join("index.html").exists() {
            dist_path = Some(p.to_path_buf());
            break;
        }
    }

    let router = if let Some(path) = dist_path {
        tracing::info!("Serving frontend SPA static assets from {:?}", path);
        let index_file = path.join("index.html");
        let index_file_root = index_file.clone();
        let index_file_html = index_file.clone();

        router
            .route(
                "/",
                get({
                    move || async move {
                        match tokio::fs::read_to_string(&index_file_root).await {
                            Ok(html) => (
                                StatusCode::OK,
                                [
                                    ("content-type", "text/html; charset=utf-8"),
                                    ("cache-control", "no-cache, no-store, must-revalidate, max-age=0"),
                                    ("pragma", "no-cache"),
                                    ("expires", "0"),
                                ],
                                html,
                            ).into_response(),
                            Err(_) => handle_root_fallback().await.into_response(),
                        }
                    }
                }),
            )
            .route(
                "/index.html",
                get({
                    move || async move {
                        match tokio::fs::read_to_string(&index_file_html).await {
                            Ok(html) => (
                                StatusCode::OK,
                                [
                                    ("content-type", "text/html; charset=utf-8"),
                                    ("cache-control", "no-cache, no-store, must-revalidate, max-age=0"),
                                    ("pragma", "no-cache"),
                                    ("expires", "0"),
                                ],
                                html,
                            ).into_response(),
                            Err(_) => handle_root_fallback().await.into_response(),
                        }
                    }
                }),
            )
            .fallback_service(
                ServeDir::new(&path)
                    .not_found_service(ServeFile::new(index_file)),
            )
    } else {
        router.route("/", get(handle_root_fallback))
    };

    router
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn handle_root_fallback() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/html; charset=utf-8")],
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>PolyQuant Engine</title></head><body style="font-family:sans-serif;padding:2rem;background:#0f172a;color:#f8fafc"><h2>PolyQuant 5M Quant Simulation Engine Online</h2><p>Mode: Safe Paper Trading</p><p>Check Health API: <a href="/api/v1/health" style="color:#38bdf8">/api/v1/health</a></p></body></html>"#,
    )
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

async fn handle_bankroll(State(state): State<AppState>) -> Json<BankrollState> {
    Json(state.risk.get_bankroll_state().await)
}

#[derive(Debug, Deserialize)]
pub struct SetBankrollRequest {
    pub active_bankroll: f64,
    pub bankroll_cap: Option<f64>,
    pub minimum_bankroll: Option<f64>,
}

async fn handle_bankroll_set(
    State(state): State<AppState>,
    Json(req): Json<SetBankrollRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state
        .risk
        .update_bankroll_funds(req.active_bankroll, req.bankroll_cap, req.minimum_bankroll)
        .await
    {
        Ok(new_state) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": format!("模拟资金成功设置为 ${:.2} USDC", new_state.active_bankroll),
                "bankroll": new_state
            })),
        ),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "message": err
            })),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct UnlockProfitRequest {
    pub amount: Option<f64>,
}

async fn handle_bankroll_unlock(
    State(state): State<AppState>,
    Json(req): Json<UnlockProfitRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.risk.unlock_profit_to_active(req.amount).await {
        Ok(new_state) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": format!("成功将锁定金库中的利润提取至可用本金！当前可用本金: ${:.2} USDC", new_state.active_bankroll),
                "bankroll": new_state
            })),
        ),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "message": err
            })),
        ),
    }
}

async fn handle_bankroll_history(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<BankrollHistoryEntry>>, StatusCode> {
    let limit = query.limit.unwrap_or(50);
    state
        .risk
        .get_history(limit)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn handle_risk_status(State(state): State<AppState>) -> Json<RiskStatus> {
    Json(state.risk.get_risk_status().await)
}

#[derive(Debug, Deserialize)]
pub struct UpdateRiskConfigRequest {
    pub daily_loss_limit: Option<f64>,
    pub max_consecutive_losses: Option<u32>,
    pub max_drawdown: Option<f64>,
    pub cooldown_minutes: Option<u32>,
}

async fn handle_risk_config_update(
    State(state): State<AppState>,
    Json(req): Json<UpdateRiskConfigRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    state
        .risk
        .update_risk_limits(
            req.daily_loss_limit,
            req.max_consecutive_losses,
            req.max_drawdown,
            req.cooldown_minutes,
        )
        .await;

    let updated = state.risk.get_risk_status().await;
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": "风控限额设置已更新",
            "risk": updated
        })),
    )
}

async fn handle_risk_unhalt(
    State(state): State<AppState>,
) -> (StatusCode, Json<serde_json::Value>) {
    state.risk.unhalt_trading().await;
    let updated = state.risk.get_risk_status().await;
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": "风控熔断已解除，交易已恢复",
            "risk": updated
        })),
    )
}

// =============================================================================
// Isolated LIVE Trading Bankroll & Risk Handlers
// =============================================================================

async fn handle_live_bankroll(State(state): State<AppState>) -> Json<BankrollState> {
    Json(state.live_risk.get_bankroll_state().await)
}

async fn handle_live_bankroll_set(
    State(state): State<AppState>,
    Json(req): Json<SetBankrollRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state
        .live_risk
        .update_bankroll_funds(req.active_bankroll, req.bankroll_cap, req.minimum_bankroll)
        .await
    {
        Ok(new_state) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": format!("实盘资金配置已更新！可用资金: ${:.2} USDC", new_state.active_bankroll),
                "bankroll": new_state
            })),
        ),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "message": err
            })),
        ),
    }
}

async fn handle_live_bankroll_unlock(
    State(state): State<AppState>,
    Json(req): Json<UnlockProfitRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.live_risk.unlock_profit_to_active(req.amount).await {
        Ok(new_state) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": format!("成功将实盘锁定利润提取至可用资金！当前可用资金: ${:.2} USDC", new_state.active_bankroll),
                "bankroll": new_state
            })),
        ),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "message": err
            })),
        ),
    }
}

async fn handle_live_bankroll_history(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<BankrollHistoryEntry>>, StatusCode> {
    let limit = query.limit.unwrap_or(50);
    state
        .live_risk
        .get_history(limit)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn handle_live_risk_status(State(state): State<AppState>) -> Json<RiskStatus> {
    Json(state.live_risk.get_risk_status().await)
}

async fn handle_live_risk_config_update(
    State(state): State<AppState>,
    Json(req): Json<UpdateRiskConfigRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    state
        .live_risk
        .update_risk_limits(
            req.daily_loss_limit,
            req.max_consecutive_losses,
            req.max_drawdown,
            req.cooldown_minutes,
        )
        .await;

    let updated = state.live_risk.get_risk_status().await;
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": "实盘风控限额设置已更新",
            "risk": updated
        })),
    )
}

async fn handle_live_risk_unhalt(
    State(state): State<AppState>,
) -> (StatusCode, Json<serde_json::Value>) {
    state.live_risk.unhalt_trading().await;
    let updated = state.live_risk.get_risk_status().await;
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": "实盘风控熔断已解除，实盘交易已恢复",
            "risk": updated
        })),
    )
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

async fn handle_features_latest(
    State(state): State<AppState>,
    Path(asset_str): Path<String>,
) -> Result<Json<FeatureSnapshot>, StatusCode> {
    let Ok(asset) = asset_str.parse::<Asset>() else {
        return Err(StatusCode::BAD_REQUEST);
    };

    match state.features.get_latest_features(asset) {
        Some(snapshot) => Ok(Json(snapshot)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn handle_features_all(
    State(state): State<AppState>,
) -> Json<HashMap<String, FeatureSnapshot>> {
    Json(state.features.get_all_latest_features())
}

async fn handle_features_names() -> Json<Vec<&'static str>> {
    Json(FEATURE_NAMES.to_vec())
}

async fn handle_dataset_summary(
    State(state): State<AppState>,
) -> Result<Json<DatasetSummary>, StatusCode> {
    match DatasetExporter::get_dataset_summary(&state.db).await {
        Ok(summary) => Ok(Json(summary)),
        Err(e) => {
            tracing::error!("Failed to fetch dataset summary: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn handle_model_prediction(
    State(state): State<AppState>,
    Path(asset_str): Path<String>,
) -> Result<Json<ModelPrediction>, StatusCode> {
    let Ok(asset) = asset_str.parse::<Asset>() else {
        return Err(StatusCode::BAD_REQUEST);
    };

    match state.models.get_latest_prediction(asset) {
        Some(prediction) => Ok(Json(prediction)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn handle_models_all(
    State(state): State<AppState>,
) -> Json<HashMap<String, ModelPrediction>> {
    Json(state.models.get_all_latest_predictions())
}

async fn handle_strategy_signal_latest(
    State(state): State<AppState>,
    Path(asset_str): Path<String>,
) -> Result<Json<PredictionSignal>, StatusCode> {
    let Ok(asset) = asset_str.parse::<Asset>() else {
        return Err(StatusCode::BAD_REQUEST);
    };

    match state.strategy.get_latest_signal(asset) {
        Some(sig) => Ok(Json(sig)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn handle_strategy_signals_all(
    State(state): State<AppState>,
) -> Json<HashMap<String, PredictionSignal>> {
    Json(state.strategy.get_all_latest_signals())
}

async fn handle_strategy_decisions_recent(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> Json<Vec<DecisionLog>> {
    let limit = query.limit.unwrap_or(50) as usize;
    Json(state.strategy.get_recent_decisions(limit).await)
}

async fn handle_paper_orders(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> Json<Vec<PaperOrder>> {
    let limit = query.limit.unwrap_or(50) as usize;
    Json(state.execution.get_recent_orders(limit).await)
}

async fn handle_paper_positions_active(
    State(state): State<AppState>,
) -> Json<Vec<PaperPosition>> {
    Json(state.execution.get_active_positions())
}

async fn handle_paper_positions_history(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<PaperPosition>>, StatusCode> {
    let limit = query.limit.unwrap_or(50);
    state
        .execution
        .get_position_history(limit)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn handle_paper_results(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<PaperResult>>, StatusCode> {
    let limit = query.limit.unwrap_or(50);
    state
        .execution
        .get_paper_results(limit)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn handle_paper_statistics(
    State(state): State<AppState>,
) -> Result<Json<TradeStatistics>, StatusCode> {
    state
        .execution
        .get_trade_statistics()
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn handle_backtest_run(
    State(state): State<AppState>,
    Json(req): Json<BacktestRequest>,
) -> Result<Json<BacktestResult>, (StatusCode, String)> {
    state
        .backtest
        .run_backtest(req)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

async fn handle_backtest_latest(
    State(state): State<AppState>,
) -> Result<Json<BacktestResult>, StatusCode> {
    match state.backtest.get_latest_backtest().await {
        Some(res) => Ok(Json(res)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn handle_backtest_history(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> Json<Vec<BacktestResult>> {
    let limit = query.limit.unwrap_or(20) as usize;
    Json(state.backtest.get_backtest_history(limit).await)
}

#[derive(Debug, Deserialize)]
pub struct FramesQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

async fn handle_replay_start(
    State(state): State<AppState>,
    Json(config): Json<ReplayConfig>,
) -> Result<Json<ReplayStateResponse>, (StatusCode, String)> {
    match state.replay.start(config).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

async fn handle_replay_pause(State(state): State<AppState>) -> Json<ReplayStateResponse> {
    Json(state.replay.pause().await)
}

async fn handle_replay_resume(State(state): State<AppState>) -> Json<ReplayStateResponse> {
    Json(state.replay.resume().await)
}

async fn handle_replay_step(State(state): State<AppState>) -> (StatusCode, Json<Option<ReplayFrame>>) {
    let frame = state.replay.step().await;
    (StatusCode::OK, Json(frame))
}

async fn handle_replay_seek(
    State(state): State<AppState>,
    Json(req): Json<SeekRequest>,
) -> (StatusCode, Json<Option<ReplayFrame>>) {
    let frame = state.replay.seek(req).await;
    (StatusCode::OK, Json(frame))
}

async fn handle_replay_speed(
    State(state): State<AppState>,
    Json(req): Json<SpeedRequest>,
) -> Json<serde_json::Value> {
    let speed = state.replay.set_speed(req.speed_multiplier).await;
    Json(serde_json::json!({ "speed_multiplier": speed }))
}

async fn handle_replay_stop(State(state): State<AppState>) -> Json<serde_json::Value> {
    state.replay.stop().await;
    Json(serde_json::json!({ "status": "stopped" }))
}

async fn handle_replay_status(State(state): State<AppState>) -> Json<ReplayStateResponse> {
    Json(state.replay.get_state().await)
}

async fn handle_replay_frames(
    State(state): State<AppState>,
    Query(q): Query<FramesQuery>,
) -> Json<Vec<ReplayFrame>> {
    let offset = q.offset.unwrap_or(0);
    let limit = q.limit.unwrap_or(100);
    Json(state.replay.get_frames(offset, limit).await)
}

#[derive(Debug, Deserialize)]
pub struct GenerateSyntheticRequest {
    pub rounds: Option<usize>,
}

async fn handle_strategy_config_get(
    State(state): State<AppState>,
) -> Json<crate::config::StrategyConfig> {
    Json(state.strategy.get_config())
}

async fn handle_strategy_config_update(
    State(state): State<AppState>,
    Json(new_config): Json<crate::config::StrategyConfig>,
) -> Json<crate::config::StrategyConfig> {
    state.strategy.update_config(new_config.clone());
    Json(new_config)
}

#[derive(Debug, Deserialize)]
pub struct SetEnabledAssetsRequest {
    pub assets: Vec<Asset>,
}

async fn handle_strategy_assets_get(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let assets = state.strategy.get_enabled_assets();
    Json(serde_json::json!({
        "success": true,
        "assets": assets
    }))
}

async fn handle_strategy_assets_set(
    State(state): State<AppState>,
    Json(req): Json<SetEnabledAssetsRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    state.strategy.set_enabled_assets(req.assets);
    let current = state.strategy.get_enabled_assets();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": format!("自动交易标的已更新: {:?}", current),
            "assets": current
        })),
    )
}

#[derive(Debug, Deserialize)]
pub struct AutoTradeToggleRequest {
    pub enabled: Option<bool>,
    pub mode: Option<String>,
    pub paper_enabled: Option<bool>,
    pub live_enabled: Option<bool>,
}

async fn handle_strategy_autotrade_get(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mode = state.live_execution.get_mode().await;
    let paper_enabled = state.execution.is_auto_trading_enabled();
    let live_enabled = state.live_execution.is_auto_trading_enabled();
    let current_enabled = match mode {
        crate::types::TradingMode::Live => live_enabled,
        crate::types::TradingMode::Paper => paper_enabled,
    };

    Json(serde_json::json!({
        "enabled": current_enabled,
        "paper_enabled": paper_enabled,
        "live_enabled": live_enabled,
        "mode": format!("{:?}", mode).to_lowercase(),
    }))
}

async fn handle_strategy_autotrade_set(
    State(state): State<AppState>,
    Json(req): Json<AutoTradeToggleRequest>,
) -> Json<serde_json::Value> {
    if let Some(p) = req.paper_enabled {
        state.execution.set_auto_trading_enabled(p);
    }
    if let Some(l) = req.live_enabled {
        state.live_execution.set_auto_trading_enabled(l);
    }

    if let Some(enabled) = req.enabled {
        let current_mode = state.live_execution.get_mode().await;
        let target_mode = req.mode.as_deref().unwrap_or(match current_mode {
            crate::types::TradingMode::Live => "live",
            crate::types::TradingMode::Paper => "paper",
        });

        if target_mode.eq_ignore_ascii_case("live") {
            state.live_execution.set_auto_trading_enabled(enabled);
        } else {
            state.execution.set_auto_trading_enabled(enabled);
        }
    }

    let mode = state.live_execution.get_mode().await;
    let paper_enabled = state.execution.is_auto_trading_enabled();
    let live_enabled = state.live_execution.is_auto_trading_enabled();
    let current_enabled = match mode {
        crate::types::TradingMode::Live => live_enabled,
        crate::types::TradingMode::Paper => paper_enabled,
    };

    Json(serde_json::json!({
        "success": true,
        "enabled": current_enabled,
        "paper_enabled": paper_enabled,
        "live_enabled": live_enabled,
        "message": "自动交易设置已更新"
    }))
}

async fn handle_model_config(
    State(state): State<AppState>,
) -> Json<crate::models::LogisticModelConfig> {
    Json(state.models.get_logistic_config())
}

async fn handle_model_train(
    State(state): State<AppState>,
    Json(req): Json<crate::models::ModelTrainRequest>,
) -> Result<Json<crate::models::ModelTrainResult>, StatusCode> {
    let records = DatasetExporter::load_from_db(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let epochs = req.epochs.unwrap_or(20);
    let lr = req.lr.unwrap_or(0.05);
    let l2 = req.l2_reg.unwrap_or(0.001);

    match state.models.train_logistic(&records, epochs, lr, l2) {
        Some(res) => Ok(Json(res)),
        None => Err(StatusCode::BAD_REQUEST),
    }
}

async fn handle_dataset_generate_synthetic(
    State(state): State<AppState>,
    Json(req): Json<GenerateSyntheticRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let rounds = req.rounds.unwrap_or(50);
    let (markets, features) = DatasetExporter::generate_synthetic_rounds(&state.db, rounds)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "rounds_generated": markets / 3,
        "markets_created": markets,
        "features_created": features,
        "message": format!("Successfully generated {} synthetic rounds across BTC, ETH, and SOL", markets / 3)
    })))
}

#[derive(Debug, Deserialize)]
pub struct SetTradingModeRequest {
    pub mode: String, // "paper" or "live"
}

#[derive(Debug, Deserialize)]
pub struct RetrainRequest {
    pub asset: Option<String>,
    pub epochs: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ToggleLearningRequest {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct AssetQuery {
    pub asset: Option<String>,
    pub limit: Option<i64>,
}

async fn handle_trading_mode_get(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mode = state.live_execution.get_mode().await;
    let active_account = state
        .db
        .get_active_account()
        .await
        .ok()
        .flatten()
        .map(|acc| crate::types::PolymarketAccountPublic::from(&acc));
    Json(serde_json::json!({
        "mode": format!("{:?}", mode).to_lowercase(),
        "is_live": mode == crate::types::TradingMode::Live,
        "active_account": active_account
    }))
}

async fn handle_trading_mode_set(
    State(state): State<AppState>,
    Json(payload): Json<SetTradingModeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let new_mode = match payload.mode.to_lowercase().as_str() {
        "live" => crate::types::TradingMode::Live,
        _ => crate::types::TradingMode::Paper,
    };

    match state.live_execution.set_mode(new_mode).await {
        Ok(mode) => Ok(Json(serde_json::json!({
            "success": true,
            "mode": format!("{:?}", mode).to_lowercase()
        }))),
        Err(err) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "error": err
            })),
        )),
    }
}

async fn handle_accounts_get(State(state): State<AppState>) -> Json<Vec<crate::types::PolymarketAccountPublic>> {
    let accs = state.db.get_accounts().await.unwrap_or_default();
    let public_accs: Vec<_> = accs.iter().map(crate::types::PolymarketAccountPublic::from).collect();
    Json(public_accs)
}

async fn handle_accounts_create(
    State(state): State<AppState>,
    Json(req): Json<crate::types::CreateAccountRequest>,
) -> Result<Json<crate::types::PolymarketAccountPublic>, (StatusCode, String)> {
    let now = Utc::now().timestamp_millis();
    let id = uuid::Uuid::new_v4().to_string();

    let account = crate::types::PolymarketAccount {
        id,
        label: req.label,
        api_key: req.api_key,
        api_secret: req.api_secret,
        api_passphrase: req.api_passphrase,
        wallet_address: req.wallet_address,
        proxy_wallet_address: req.proxy_wallet_address,
        is_active: true,
        balance_usdc: 0.0,
        created_at: now,
        updated_at: now,
    };

    // If active, deactivate others
    let _ = state.db.set_active_account(&account.id).await;

    if let Err(e) = state.db.insert_account(&account).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save account: {:?}", e)));
    }

    Ok(Json(crate::types::PolymarketAccountPublic::from(&account)))
}

async fn handle_account_activate(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let success = state.db.set_active_account(&id).await.is_ok();
    if success {
        if let Ok(Some(acc)) = state.db.get_active_account().await {
            state.live_risk.sync_live_balance(acc.balance_usdc).await;
        }
    }
    Json(serde_json::json!({ "success": success }))
}

async fn handle_account_delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let success = state.db.delete_account(&id).await.is_ok();
    Json(serde_json::json!({ "success": success }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccountRequest {
    pub label: Option<String>,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub api_passphrase: Option<String>,
    pub wallet_address: Option<String>,
    pub proxy_wallet_address: Option<String>,
    pub balance_usdc: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct CalibrateBalanceRequest {
    pub balance_usdc: f64,
}

async fn handle_account_update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAccountRequest>,
) -> Result<Json<crate::types::PolymarketAccountPublic>, (StatusCode, String)> {
    let accounts = state.db.get_accounts().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut account = accounts.into_iter().find(|a| a.id == id).ok_or_else(|| (StatusCode::NOT_FOUND, "Account not found".to_string()))?;

    if let Some(l) = req.label {
        if !l.trim().is_empty() {
            account.label = l.trim().to_string();
        }
    }
    if let Some(k) = req.api_key {
        if !k.trim().is_empty() {
            account.api_key = k.trim().to_string();
        }
    }
    if let Some(s) = req.api_secret {
        account.api_secret = s.trim().to_string();
    }
    if let Some(p) = req.api_passphrase {
        account.api_passphrase = p.trim().to_string();
    }
    if let Some(w) = req.wallet_address {
        if !w.trim().is_empty() {
            account.wallet_address = w.trim().to_string();
        }
    }
    if let Some(proxy) = req.proxy_wallet_address {
        let p_trimmed = proxy.trim();
        account.proxy_wallet_address = if p_trimmed.is_empty() {
            None
        } else {
            Some(p_trimmed.to_string())
        };
    }
    if let Some(bal) = req.balance_usdc {
        if bal >= 0.0 {
            account.balance_usdc = bal;
            state.live_risk.sync_live_balance(bal).await;
        }
    }

    if let Err(e) = state.db.update_account(&account).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to update account: {:?}", e)));
    }

    Ok(Json(crate::types::PolymarketAccountPublic::from(&account)))
}

async fn handle_account_calibrate_balance(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CalibrateBalanceRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let accounts = state.db.get_accounts().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let account = accounts.into_iter().find(|a| a.id == id).ok_or_else(|| (StatusCode::NOT_FOUND, "Account not found".to_string()))?;

    let new_bal = req.balance_usdc.max(0.0);
    state.db.update_account_balance(&account.id, new_bal).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    state.live_risk.sync_live_balance(new_bal).await;

    Ok(Json(serde_json::json!({
        "account_id": id,
        "balance_usdc": new_bal,
        "success": true
    })))
}

async fn handle_account_balance(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let accounts = state.db.get_accounts().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut account = accounts.into_iter().find(|a| a.id == id).ok_or_else(|| (StatusCode::NOT_FOUND, "Account not found".to_string()))?;

    // Try auto-resolving proxy wallet if empty
    if account.proxy_wallet_address.as_deref().unwrap_or("").trim().is_empty() {
        if let Some(resolved) = state.live_execution.clob_client().resolve_proxy_wallet(&account.wallet_address).await {
            let _ = state.db.update_account_proxy_wallet(&account.id, &resolved).await;
            account.proxy_wallet_address = Some(resolved);
        }
    }

    let balance = state.live_execution.clob_client().fetch_balance(&account).await.unwrap_or(account.balance_usdc);
    let _ = state.db.update_account_balance(&account.id, balance).await;
    state.live_risk.sync_live_balance(balance).await;

    Ok(Json(serde_json::json!({
        "account_id": id,
        "balance_usdc": balance,
        "proxy_wallet_address": account.proxy_wallet_address
    })))
}

async fn handle_real_orders_get(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> Json<Vec<crate::types::RealOrder>> {
    let limit = query.limit.unwrap_or(100);
    let orders = state.db.get_real_orders(limit).await.unwrap_or_default();
    Json(orders)
}

async fn handle_real_emergency_halt(State(state): State<AppState>) -> Json<serde_json::Value> {
    let res = state.live_execution.emergency_halt().await;
    Json(serde_json::json!({
        "success": res.is_ok(),
        "mode": "paper",
        "message": "Emergency halt triggered. Real trading halted immediately."
    }))
}

#[derive(Debug, Deserialize)]
pub struct ManualTradeRequest {
    pub market_id: String,
    pub asset: Asset,
    pub side: crate::types::MarketSide,
    pub stake: Option<f64>,
    pub mode: Option<String>,
}

async fn handle_trade_manual(
    State(state): State<AppState>,
    Json(req): Json<ManualTradeRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let current_mode = state.live_execution.get_mode().await;
    let target_mode = req.mode.as_deref().unwrap_or(match current_mode {
        crate::types::TradingMode::Live => "live",
        crate::types::TradingMode::Paper => "paper",
    });

    if target_mode.eq_ignore_ascii_case("live") {
        match state
            .live_execution
            .execute_manual_order(&req.market_id, req.asset, req.side, req.stake)
            .await
        {
            Ok(real_order) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "mode": "live",
                    "order": real_order,
                    "message": format!("实盘手动订单已提交至 Polymarket CLOB: 状态 {}", real_order.status)
                })),
            ),
            Err(err) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "mode": "live",
                    "message": err
                })),
            ),
        }
    } else {
        match state
            .execution
            .execute_manual_order(&req.market_id, req.asset, req.side, req.stake)
            .await
        {
            Ok((order, position)) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "mode": "paper",
                    "order": order,
                    "position": position,
                    "message": format!("模拟盘手动订单已撮合成交！成交价: {:.3} USDC", order.fill_price)
                })),
            ),
            Err(err) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "mode": "paper",
                    "message": err
                })),
            ),
        }
    }
}

async fn handle_learning_status(
    State(state): State<AppState>,
    Query(query): Query<AssetQuery>,
) -> Json<crate::models::LearningStatusResponse> {
    let asset = query.asset.and_then(|a| a.parse().ok()).unwrap_or(Asset::BTC);
    let status = state.self_learning.get_status(asset).await;
    Json(status)
}

async fn handle_learning_toggle(
    State(state): State<AppState>,
    Json(req): Json<ToggleLearningRequest>,
) -> Json<serde_json::Value> {
    state.self_learning.set_enabled(req.enabled);
    Json(serde_json::json!({
        "success": true,
        "auto_learning_enabled": req.enabled
    }))
}

async fn handle_learning_retrain(
    State(state): State<AppState>,
    Json(req): Json<RetrainRequest>,
) -> Json<serde_json::Value> {
    let asset = req.asset.and_then(|a| a.parse().ok()).unwrap_or(Asset::BTC);
    let epochs = req.epochs.unwrap_or(15);
    let res = state.self_learning.trigger_batch_retrain(asset, epochs).await;
    Json(serde_json::json!({
        "success": res.is_some(),
        "result": res
    }))
}

async fn handle_learning_history(
    State(state): State<AppState>,
    Query(query): Query<AssetQuery>,
) -> Json<Vec<crate::types::LearningHistoryEntry>> {
    let asset = query.asset.and_then(|a| a.parse().ok());
    let limit = query.limit.unwrap_or(50);
    let history = state.db.get_learning_history(asset, limit).await.unwrap_or_default();
    Json(history)
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
        let features = Arc::new(FeatureEngine::new(
            composite.clone(),
            polymarket.clone(),
            collector.clone(),
            db.clone(),
        ));
        let models = Arc::new(ModelManager::new());
        let strategy = Arc::new(StrategyEngine::new(
            config.strategy.clone(),
            config.execution.clone(),
            db.clone(),
        ));
        let execution = Arc::new(PaperExecutionEngine::new(
            config.execution.clone(),
            config.position.clone(),
            db.clone(),
            polymarket.book_engine(),
        ));

        let bankroll_state = db.get_or_init_bankroll(&config).await.unwrap();
        let risk = Arc::new(RiskManager::new(
            config.bankroll.clone(),
            config.risk.clone(),
            bankroll_state.clone(),
            db.clone(),
        ));

        let backtest = Arc::new(BacktestEngine::new(db.clone(), models.clone()));
        let replay = Arc::new(ReplayEngine::new(db.clone(), models.clone(), strategy.clone()));

        let clob_http = Arc::new(crate::polymarket::PolymarketClobHttpClient::default());
        let live_execution = Arc::new(crate::execution::LiveExecutionEngine::new(db.clone(), clob_http));
        let self_learning = Arc::new(crate::models::SelfLearningEngine::new(models.clone(), db.clone()));

        let live_risk = Arc::new(RiskManager::new_live(
            config.bankroll.clone(),
            config.risk.clone(),
            bankroll_state.clone(),
            db.clone(),
        ));
        let sessions = Arc::new(dashmap::DashMap::new());

        AppState {
            config,
            db,
            collector,
            polymarket,
            composite,
            features,
            models,
            strategy,
            execution,
            live_execution,
            self_learning,
            risk,
            live_risk,
            backtest,
            replay,
            sessions,
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
