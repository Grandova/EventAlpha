use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tower::ServiceExt;

use poly_quant_backend::api::{create_router, AppState};
use poly_quant_backend::backtest::BacktestEngine;
use poly_quant_backend::collector::CollectorManager;
use poly_quant_backend::composite::CompositePriceEngine;
use poly_quant_backend::config::{AppConfig, StrategyConfig};
use poly_quant_backend::db::Database;
use poly_quant_backend::execution::PaperExecutionEngine;
use poly_quant_backend::features::FeatureEngine;
use poly_quant_backend::models::ModelManager;
use poly_quant_backend::polymarket::PolymarketManager;
use poly_quant_backend::replay::ReplayEngine;
use poly_quant_backend::risk::RiskManager;
use poly_quant_backend::strategy::StrategyEngine;

async fn setup_test_app() -> (axum::Router, AppState) {
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
  max_connections: 5
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

    let mut file = NamedTempFile::new().unwrap();
    use std::io::Write;
    file.write_all(yaml.as_bytes()).unwrap();

    let config = Arc::new(AppConfig::load_from_path(file.path()).unwrap());
    let db = Arc::new(Database::new(&config).await.unwrap());
    let collector = Arc::new(CollectorManager::new(&config));
    let polymarket = Arc::new(PolymarketManager::new(&config, db.clone()));
    let composite = Arc::new(CompositePriceEngine::new(
        collector.clone(),
        polymarket.discovery().clone(),
    ));
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
        bankroll_state,
        db.clone(),
    ));

    let backtest = Arc::new(BacktestEngine::new(db.clone(), models.clone()));
    let replay = Arc::new(ReplayEngine::new(
        db.clone(),
        models.clone(),
        strategy.clone(),
    ));

    let state = AppState {
        config,
        db,
        collector,
        polymarket,
        composite,
        features,
        models,
        strategy,
        execution,
        risk,
        backtest,
        replay,
        start_time_ms: chrono::Utc::now().timestamp_millis(),
    };

    let router = create_router(state.clone());
    (router, state)
}

#[tokio::test]
async fn test_strategy_parameter_tuning_and_hot_reload() {
    let (app, state) = setup_test_app().await;

    // 1. Initial config fetch
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/strategy/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let cfg: StrategyConfig = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(cfg.min_probability, 0.70);
    assert_eq!(cfg.min_net_edge, 0.05);

    // 2. Hot-reload new parameters via POST
    let mut updated_cfg = cfg.clone();
    updated_cfg.min_probability = 0.65;
    updated_cfg.min_net_edge = 0.03;
    updated_cfg.max_spread = 0.05;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/strategy/config")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&updated_cfg).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 3. Verify in-memory engine immediately reflects new config without restart
    let current_engine_cfg = state.strategy.get_config();
    assert_eq!(current_engine_cfg.min_probability, 0.65);
    assert_eq!(current_engine_cfg.min_net_edge, 0.03);
    assert_eq!(current_engine_cfg.max_spread, 0.05);
}

#[tokio::test]
async fn test_synthetic_dataset_generation_and_model_training_flow() {
    let (app, _state) = setup_test_app().await;

    // 1. Generate 20 synthetic historical rounds
    let gen_req = serde_json::json!({ "rounds": 20 });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/dataset/generate_synthetic")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&gen_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let gen_res: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(gen_res["status"], "success");
    assert_eq!(gen_res["rounds_generated"], 20);

    // 2. Check dataset summary
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/dataset/summary")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let summary: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(summary["total_samples"].as_i64().unwrap() > 0);

    // 3. Trigger model training via POST /api/v1/models/train
    let train_req = serde_json::json!({
        "epochs": 15,
        "lr": 0.05,
        "l2_reg": 0.001
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/models/train")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&train_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let train_res: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(train_res["success"], true);
    assert!(train_res["samples_trained"].as_i64().unwrap() > 0);
    assert!(train_res["accuracy"].as_f64().unwrap() >= 0.0);
    assert!(train_res["brier_score"].as_f64().unwrap() <= 1.0);
}
