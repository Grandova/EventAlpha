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
use poly_quant_backend::config::AppConfig;
use poly_quant_backend::db::Database;
use poly_quant_backend::execution::PaperExecutionEngine;
use poly_quant_backend::features::FeatureEngine;
use poly_quant_backend::models::ModelManager;
use poly_quant_backend::polymarket::PolymarketManager;
use poly_quant_backend::replay::{
    ReplayConfig, ReplayEngine, ReplayFrame, ReplayStateResponse, ReplayStatus, SeekRequest,
    SpeedRequest,
};
use poly_quant_backend::risk::RiskManager;
use poly_quant_backend::strategy::StrategyEngine;
use poly_quant_backend::types::Asset;

fn test_config_yaml() -> &'static str {
    r#"
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
  max_connections: 4
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
"#
}

async fn setup_replay_test_app() -> (axum::Router, Arc<ReplayEngine>, Arc<Database>) {
    let mut file = NamedTempFile::new().unwrap();
    use std::io::Write;
    file.write_all(test_config_yaml().as_bytes()).unwrap();

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

    let bankroll = db.get_or_init_bankroll(&config).await.unwrap();
    let risk = Arc::new(RiskManager::new(
        config.bankroll.clone(),
        config.risk.clone(),
        bankroll,
        db.clone(),
    ));
    execution.set_risk_manager(risk.clone()).await;

    let backtest = Arc::new(BacktestEngine::new(db.clone(), models.clone()));
    let replay = Arc::new(ReplayEngine::new(
        db.clone(),
        models.clone(),
        strategy.clone(),
    ));

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        collector,
        polymarket,
        composite,
        features,
        models,
        strategy,
        execution,
        risk,
        backtest,
        replay: replay.clone(),
        start_time_ms: chrono::Utc::now().timestamp_millis(),
    };
    let app = create_router(state);
    (app, replay, db)
}

#[tokio::test]
async fn test_replay_load_frames_and_step_playback() {
    let (_, replay, _) = setup_replay_test_app().await;

    // 1. Initial State is Idle
    let state_0 = replay.get_state().await;
    assert_eq!(state_0.status, ReplayStatus::Idle);
    assert_eq!(state_0.total_frames, 0);

    // 2. Start replay on BTC
    let cfg = ReplayConfig {
        asset: Asset::BTC,
        start_time_ms: Some(1700000000000),
        end_time_ms: Some(1700000300000),
        speed_multiplier: 5.0,
    };
    let state_start = replay.start(cfg).await.unwrap();
    assert_eq!(state_start.status, ReplayStatus::Playing);
    assert!(state_start.total_frames >= 50);

    // 3. Pause
    let state_pause = replay.pause().await;
    assert_eq!(state_pause.status, ReplayStatus::Paused);

    // 4. Step forward 1 frame
    let frame_1 = replay.step().await.expect("Expected frame on step");
    assert_eq!(frame_1.asset, Asset::BTC);
    assert!(frame_1.composite_price > 0.0);
    assert!(frame_1.prediction.is_some());
    assert!(frame_1.signal.is_some());

    // 5. Seek to frame index 20
    let frame_seek = replay
        .seek(SeekRequest {
            target_frame_index: Some(20),
            target_timestamp_ms: None,
        })
        .await
        .expect("Expected frame on seek");
    assert_eq!(frame_seek.frame_index, 20);

    let state_seek = replay.get_state().await;
    assert_eq!(state_seek.current_frame_index, 20);

    // 6. Set speed
    let new_speed = replay.set_speed(15.0).await;
    assert_eq!(new_speed, 15.0);

    // 7. Stop replay
    replay.stop().await;
    let state_stop = replay.get_state().await;
    assert_eq!(state_stop.status, ReplayStatus::Idle);
    assert_eq!(state_stop.current_frame_index, 0);
}

#[tokio::test]
async fn test_replay_broadcast_subscription() {
    let (_, replay, _) = setup_replay_test_app().await;

    let cfg = ReplayConfig {
        asset: Asset::ETH,
        speed_multiplier: 1.0,
        ..Default::default()
    };
    let _ = replay.start(cfg).await.unwrap();
    replay.pause().await;

    let mut rx = replay.subscribe();

    // Step 3 times and collect broadcast frames
    let f1 = replay.step().await.unwrap();
    let f2 = replay.step().await.unwrap();
    let f3 = replay.step().await.unwrap();

    let recv1 = rx.recv().await.unwrap();
    let recv2 = rx.recv().await.unwrap();
    let recv3 = rx.recv().await.unwrap();

    assert_eq!(recv1.timestamp_ms, f1.timestamp_ms);
    assert_eq!(recv2.timestamp_ms, f2.timestamp_ms);
    assert_eq!(recv3.timestamp_ms, f3.timestamp_ms);

    replay.stop().await;
}

#[tokio::test]
async fn test_replay_rest_api_endpoints() {
    let (app, _, _) = setup_replay_test_app().await;

    // 1. POST /api/v1/replay/start
    let req_body = ReplayConfig {
        asset: Asset::SOL,
        speed_multiplier: 2.0,
        ..Default::default()
    };
    let json_bytes = serde_json::to_vec(&req_body).unwrap();

    let res_start = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/replay/start")
                .header("content-type", "application/json")
                .body(Body::from(json_bytes))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res_start.status(), StatusCode::OK);
    let bytes = res_start.into_body().collect().await.unwrap().to_bytes();
    let state_start: ReplayStateResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(state_start.status, ReplayStatus::Playing);
    assert!(state_start.total_frames > 0);

    // 2. GET /api/v1/replay/status
    let res_status = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/replay/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_status.status(), StatusCode::OK);

    // 3. POST /api/v1/replay/pause
    let res_pause = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/replay/pause")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_pause.status(), StatusCode::OK);

    // 4. POST /api/v1/replay/step
    let res_step = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/replay/step")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_step.status(), StatusCode::OK);
    let step_bytes = res_step.into_body().collect().await.unwrap().to_bytes();
    let opt_frame: Option<ReplayFrame> = serde_json::from_slice(&step_bytes).unwrap();
    assert!(opt_frame.is_some());

    // 5. POST /api/v1/replay/seek
    let seek_body = SeekRequest {
        target_frame_index: Some(5),
        target_timestamp_ms: None,
    };
    let res_seek = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/replay/seek")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&seek_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_seek.status(), StatusCode::OK);

    // 6. POST /api/v1/replay/speed
    let speed_body = SpeedRequest {
        speed_multiplier: 10.0,
    };
    let res_speed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/replay/speed")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&speed_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_speed.status(), StatusCode::OK);

    // 7. GET /api/v1/replay/frames
    let res_frames = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/replay/frames?offset=0&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_frames.status(), StatusCode::OK);
    let frames_bytes = res_frames.into_body().collect().await.unwrap().to_bytes();
    let frames: Vec<ReplayFrame> = serde_json::from_slice(&frames_bytes).unwrap();
    assert_eq!(frames.len(), 10);

    // 8. POST /api/v1/replay/stop
    let res_stop = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/replay/stop")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_stop.status(), StatusCode::OK);
}
