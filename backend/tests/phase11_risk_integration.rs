use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::io::Write;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tower::ServiceExt;

use poly_quant_backend::api::{create_router, AppState};
use poly_quant_backend::collector::CollectorManager;
use poly_quant_backend::composite::CompositePriceEngine;
use poly_quant_backend::config::AppConfig;
use poly_quant_backend::db::Database;
use poly_quant_backend::execution::PaperExecutionEngine;
use poly_quant_backend::features::FeatureEngine;
use poly_quant_backend::models::ModelManager;
use poly_quant_backend::polymarket::PolymarketManager;
use poly_quant_backend::risk::RiskManager;
use poly_quant_backend::strategy::StrategyEngine;
use poly_quant_backend::types::{BankrollHistoryEntry, BankrollMode, BankrollState, RiskStatus};

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
  latency_ms: 0
  orderbook_depth_fill: true
  fee_rate: 0.012
  slippage_rate: 0.005
freshness:
  stale_timeout_ms: 2000
  max_clock_skew_ms: 1000
assets: ["BTC", "ETH", "SOL"]
"#
}

async fn setup_risk_test_app() -> (axum::Router, Arc<RiskManager>, Arc<Database>, Arc<AppConfig>) {
    let mut file = NamedTempFile::new().unwrap();
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

    let backtest = Arc::new(poly_quant_backend::backtest::BacktestEngine::new(
        db.clone(),
        models.clone(),
    ));
    let replay = Arc::new(poly_quant_backend::replay::ReplayEngine::new(
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
        models: models.clone(),
        strategy,
        execution,
        risk: risk.clone(),
        live_risk: risk.clone(),
        backtest,
        replay,
        live_execution: std::sync::Arc::new(poly_quant_backend::execution::LiveExecutionEngine::new(
            db.clone(),
            std::sync::Arc::new(poly_quant_backend::polymarket::PolymarketClobHttpClient::default()),
        )),
        self_learning: std::sync::Arc::new(poly_quant_backend::models::SelfLearningEngine::new(
            models.clone(),
            db.clone(),
        )),
        sessions: std::sync::Arc::new(dashmap::DashMap::new()),
        start_time_ms: chrono::Utc::now().timestamp_millis(),
    };
    let app = create_router(state);
    (app, risk, db, config)
}

#[tokio::test]
async fn test_mode_b_capital_recovery_and_profit_locking() {
    let (_, risk, db, _) = setup_risk_test_app().await;

    // Initial check: active 10.0, locked 0.0, total 10.0
    let b0 = risk.get_bankroll_state().await;
    assert_eq!(b0.active_bankroll, 10.0);
    assert_eq!(b0.locked_profit, 0.0);
    assert_eq!(b0.total_equity, 10.0);

    // Trade 1: Reserve stake 1.0 -> active drops to 9.0
    assert!(risk.can_open_position(1.0).await.is_ok());
    risk.reserve_stake(1.0, "ORDER_FILLED", Some("ord_1")).await.unwrap();

    let b1 = risk.get_bankroll_state().await;
    assert_eq!(b1.active_bankroll, 9.0);

    // Trade 1 resolves as a LOSS: returned stake = 0.0, net_profit = -1.0
    risk.process_settlement(0.0, -1.0, "LOSS", Some("pos_1")).await.unwrap();

    let b2 = risk.get_bankroll_state().await;
    assert_eq!(b2.active_bankroll, 9.0);
    assert_eq!(b2.locked_profit, 0.0);
    assert_eq!(b2.total_equity, 9.0);

    // Trade 2: Reserve stake 1.0 -> active drops from 9.0 to 8.0
    assert!(risk.can_open_position(1.0).await.is_ok());
    risk.reserve_stake(1.0, "ORDER_FILLED", Some("ord_2")).await.unwrap();

    let b3 = risk.get_bankroll_state().await;
    assert_eq!(b3.active_bankroll, 8.0);

    // Trade 2 resolves as a WIN: stake 1.0 returned, net profit = +1.5
    // In Mode B: deficit to bankroll_cap (10.0) is: 10.0 - (8.0 + 1.0) = 1.0.
    // So 1.0 net profit replenishes active bankroll to 10.0!
    // Remaining 0.5 net profit is locked into locked_profit!
    risk.process_settlement(1.0, 1.5, "WIN", Some("pos_2")).await.unwrap();

    let b4 = risk.get_bankroll_state().await;
    assert_eq!(b4.active_bankroll, 10.0); // Replenished up to cap of 10.0
    assert_eq!(b4.locked_profit, 0.5);   // Excess locked!
    assert_eq!(b4.total_equity, 10.5);

    // Verify DB audit log entries
    let history = db.get_bankroll_history(10).await.unwrap();
    assert!(history.len() >= 4); // INITIAL_DEPOSIT, STAKE_RESERVED x2, SETTLEMENT_LOSS, SETTLEMENT_WIN
}

#[tokio::test]
async fn test_mode_a_profit_isolation() {
    let (_, risk, _, _) = setup_risk_test_app().await;

    // Switch to Mode A (Profit Isolation)
    risk.set_mode(BankrollMode::ProfitIsolation).await;

    // Initial: active 10.0, reserve 1.0 -> active 9.0
    risk.reserve_stake(1.0, "ORDER_FILLED", Some("ord_a1")).await.unwrap();
    let b1 = risk.get_bankroll_state().await;
    assert_eq!(b1.active_bankroll, 9.0);

    // Win trade: returned stake 1.0, net profit +0.8
    // In Mode A: active gets back the stake (1.0 -> 10.0), 100% of profit (0.8) goes directly to locked_profit
    risk.process_settlement(1.0, 0.8, "WIN", Some("pos_a1")).await.unwrap();

    let b2 = risk.get_bankroll_state().await;
    assert_eq!(b2.active_bankroll, 10.0);
    assert_eq!(b2.locked_profit, 0.8);
    assert_eq!(b2.total_equity, 10.8);
}

#[tokio::test]
async fn test_circuit_breaker_minimum_bankroll_floor() {
    let (_, risk, _, _) = setup_risk_test_app().await;

    // Active bankroll is 10.0, floor is 2.0
    // Drain active bankroll to 2.5 by reserving 7.5
    risk.reserve_stake(7.5, "TEST_DRAIN", Some("ord_drain")).await.unwrap();
    let b = risk.get_bankroll_state().await;
    assert_eq!(b.active_bankroll, 2.5);

    // Now requesting stake 1.0 would leave active bankroll at 1.5 < 2.0 (minimum floor)
    let res = risk.can_open_position(1.0).await;
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.contains("minimum bankroll floor"));
}

#[tokio::test]
async fn test_circuit_breaker_daily_loss_limit() {
    let (_, risk, _, _) = setup_risk_test_app().await;

    // Daily loss limit is 2.0 USDC
    // Loss 1: -0.8
    risk.reserve_stake(0.8, "ORDER_FILLED", Some("ord_dl1")).await.unwrap();
    risk.process_settlement(0.0, -0.8, "LOSS", Some("pos_dl1")).await.unwrap();
    assert!(risk.can_open_position(0.5).await.is_ok());

    // Loss 2: -1.2 (cumulative daily loss = 2.0)
    risk.reserve_stake(1.2, "ORDER_FILLED", Some("ord_dl2")).await.unwrap();
    risk.process_settlement(0.0, -1.2, "LOSS", Some("pos_dl2")).await.unwrap();

    // Now daily loss = 2.0, hitting the limit
    let res = risk.can_open_position(0.5).await;
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Daily loss limit reached"));
}

#[tokio::test]
async fn test_circuit_breaker_consecutive_losses_cooldown() {
    let (_, risk, _, _) = setup_risk_test_app().await;

    // Consecutive loss limit is 5
    for i in 1..=4 {
        let ord_id = format!("ord_loss_{}", i);
        let pos_id = format!("pos_loss_{}", i);
        risk.reserve_stake(0.1, "ORDER_FILLED", Some(&ord_id)).await.unwrap();
        risk.process_settlement(0.0, -0.1, "LOSS", Some(&pos_id)).await.unwrap();
        assert!(risk.can_open_position(0.1).await.is_ok());
    }

    // 5th consecutive loss
    risk.reserve_stake(0.1, "ORDER_FILLED", Some("ord_loss_5")).await.unwrap();
    risk.process_settlement(0.0, -0.1, "LOSS", Some("pos_loss_5")).await.unwrap();

    // Now circuit breaker should trigger cooldown
    let res = risk.can_open_position(0.1).await;
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Risk cooldown active"));
}

#[tokio::test]
async fn test_risk_status_and_bankroll_history_rest_api() {
    let (app, risk, _, _) = setup_risk_test_app().await;

    // Generate a trade
    risk.reserve_stake(1.0, "ORDER_FILLED", Some("ord_api")).await.unwrap();
    risk.process_settlement(1.0, 1.2, "WIN", Some("pos_api")).await.unwrap();

    // 1. GET /api/v1/risk/status
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/risk/status")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let risk_status: RiskStatus = serde_json::from_slice(&body_bytes).unwrap();
    assert!(!risk_status.is_trading_halted);
    assert_eq!(risk_status.consecutive_losses, 0);

    // 2. GET /api/v1/paper/bankroll
    let req_b = Request::builder()
        .method("GET")
        .uri("/api/v1/paper/bankroll")
        .body(Body::empty())
        .unwrap();

    let res_b = app.clone().oneshot(req_b).await.unwrap();
    assert_eq!(res_b.status(), StatusCode::OK);
    let b_bytes = res_b.into_body().collect().await.unwrap().to_bytes();
    let bankroll: BankrollState = serde_json::from_slice(&b_bytes).unwrap();
    assert_eq!(bankroll.active_bankroll, 10.0);
    assert_eq!(bankroll.locked_profit, 1.2);

    // 3. GET /api/v1/paper/bankroll/history
    let req2 = Request::builder()
        .method("GET")
        .uri("/api/v1/paper/bankroll/history")
        .body(Body::empty())
        .unwrap();

    let res2 = app.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
    let body_bytes2 = res2.into_body().collect().await.unwrap().to_bytes();
    let history: Vec<BankrollHistoryEntry> = serde_json::from_slice(&body_bytes2).unwrap();
    assert!(history.len() >= 3);
    assert_eq!(history.first().unwrap().reason, "SETTLEMENT_WIN");
}

#[tokio::test]
async fn test_risk_config_update_and_unhalt_api() {
    let (app, risk, _, _) = setup_risk_test_app().await;

    // Trigger daily loss limit (initial limit is 2.0)
    risk.reserve_stake(1.0, "ORDER_FILLED", Some("ord_t1")).await.unwrap();
    risk.process_settlement(0.0, -1.0, "LOSS", Some("pos_t1")).await.unwrap();
    risk.reserve_stake(1.0, "ORDER_FILLED", Some("ord_t2")).await.unwrap();
    risk.process_settlement(0.0, -1.0, "LOSS", Some("pos_t2")).await.unwrap();

    // 1. Verify initially halted
    let status = risk.get_risk_status().await;
    assert!(status.is_trading_halted);
    assert_eq!(status.daily_loss_limit, 2.0);

    // 2. POST /api/v1/risk/config - expand limit to 10.0
    let update_body = serde_json::json!({
        "daily_loss_limit": 10.0,
        "max_consecutive_losses": 8
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/risk/config")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Expanding daily loss limit should have auto-unhalted since 2.0 < 10.0
    let status_after = risk.get_risk_status().await;
    assert_eq!(status_after.daily_loss_limit, 10.0);
    assert_eq!(status_after.max_consecutive_losses, 8);
    assert!(!status_after.is_trading_halted);

    // 3. Test manual POST /api/v1/risk/unhalt
    let req_unhalt = Request::builder()
        .method("POST")
        .uri("/api/v1/risk/unhalt")
        .body(Body::empty())
        .unwrap();

    let res_unhalt = app.oneshot(req_unhalt).await.unwrap();
    assert_eq!(res_unhalt.status(), StatusCode::OK);

    let status_final = risk.get_risk_status().await;
    assert!(!status_final.is_trading_halted);
    assert_eq!(status_final.daily_loss_current, 0.0);
    assert_eq!(status_final.consecutive_losses, 0);
}
