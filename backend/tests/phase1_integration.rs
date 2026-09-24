use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use poly_quant_backend::api::{create_router, AppState, HealthResponse};
use poly_quant_backend::config::AppConfig;
use poly_quant_backend::db::Database;
use poly_quant_backend::safety::{SafetyGuard, SafetyStatus};
use poly_quant_backend::types::BankrollState;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tower::ServiceExt;

fn build_test_yaml() -> String {
    r#"
mode: "paper"
safety:
  real_trading_enabled: false
  safety_signature: "PAPER_TRADING_ONLY_SAFETY_LOCK_ENGAGED"
server:
  host: "127.0.0.1"
  port: 8080
  cors_allowed_origins: ["http://localhost:5173"]
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
    .to_string()
}

async fn setup_test_app() -> (axum::Router, Arc<Database>, Arc<AppConfig>) {
    let mut tmp = NamedTempFile::new().unwrap();
    use std::io::Write;
    tmp.write_all(build_test_yaml().as_bytes()).unwrap();

    let config = Arc::new(AppConfig::load_from_path(tmp.path()).unwrap());
    let db = Arc::new(Database::new(&config).await.unwrap());
    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        start_time_ms: chrono::Utc::now().timestamp_millis(),
    };
    let router = create_router(state);
    (router, db, config)
}

#[tokio::test]
async fn test_phase1_full_api_health_and_safety_flow() {
    let (app, _db, _config) = setup_test_app().await;

    // 1. Health endpoint test
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let health: HealthResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(health.status, "ok");
    assert_eq!(health.mode, "paper");
    assert!(!health.real_trading_enabled);
    assert!(health.safety_status.safety_lock_engaged);

    // 2. Safety endpoint test
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/safety")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let safety: SafetyStatus = serde_json::from_slice(&bytes).unwrap();
    assert!(!safety.real_trading_enabled);
    assert!(safety.paper_trading_only);
    assert_eq!(
        safety.safety_signature,
        "PAPER_TRADING_ONLY_SAFETY_LOCK_ENGAGED"
    );

    // 3. Bankroll endpoint test
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/paper/bankroll")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let bankroll: BankrollState = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(bankroll.initial_bankroll, 10.0);
    assert_eq!(bankroll.active_bankroll, 10.0);
    assert_eq!(bankroll.bankroll_cap, 10.0);
    assert_eq!(bankroll.locked_profit, 0.0);
    assert!(!bankroll.is_trading_halted);

    // 4. Config inspection endpoint test
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let config_res: AppConfig = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(config_res.strategy.strategy_version, "v1.0.0");
    assert_eq!(config_res.assets, vec!["BTC", "ETH", "SOL"]);
}

#[tokio::test]
async fn test_phase1_database_tables_and_event_auditing() {
    let (_app, db, _config) = setup_test_app().await;

    // Record an audit log event
    db.record_system_event(
        "TEST_AUDIT",
        "WARN",
        "RISK_ENGINE",
        "Audit test trigger",
        Some(r#"{"test": true}"#),
    )
    .await
    .expect("Must record event");

    let events = db.get_recent_events(10).await.unwrap();
    assert!(!events.is_empty());
    assert_eq!(events[0].event_type, "TEST_AUDIT");
    assert_eq!(events[0].severity, "WARN");
    assert_eq!(events[0].component, "RISK_ENGINE");
}

#[test]
fn test_phase1_strict_safety_rejection() {
    // Verify that attempting to instantiate or validate with real trading enabled triggers panic
    let result = std::panic::catch_unwind(|| {
        let _ = SafetyGuard::enforce_paper_only(true);
    });
    assert!(result.is_err(), "SafetyGuard MUST panic when real_trading_enabled is true");
}
