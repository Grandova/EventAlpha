use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::io::Write;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tower::ServiceExt;

use poly_quant_backend::api::{create_router, AppState};
use poly_quant_backend::backtest::{BacktestEngine, BacktestRequest, BacktestResult};
use poly_quant_backend::collector::CollectorManager;
use poly_quant_backend::composite::CompositePriceEngine;
use poly_quant_backend::config::AppConfig;
use poly_quant_backend::dataset::DatasetRecord;
use poly_quant_backend::db::Database;
use poly_quant_backend::execution::PaperExecutionEngine;
use poly_quant_backend::features::FeatureEngine;
use poly_quant_backend::models::ModelManager;
use poly_quant_backend::polymarket::PolymarketManager;
use poly_quant_backend::risk::RiskManager;
use poly_quant_backend::strategy::StrategyEngine;
use poly_quant_backend::types::{Asset, BankrollMode, Resolution};

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
  daily_loss_limit: 5.0
  max_drawdown: 0.50
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

async fn setup_backtest_test_app() -> (axum::Router, Arc<BacktestEngine>, Arc<Database>) {
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

    let backtest = Arc::new(BacktestEngine::new(db.clone(), models.clone()));

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
        backtest: backtest.clone(),
        start_time_ms: chrono::Utc::now().timestamp_millis(),
    };
    let app = create_router(state);
    (app, backtest, db)
}

fn create_sample_dataset_records(count: usize) -> Vec<DatasetRecord> {
    let mut records = Vec::with_capacity(count);
    let base_time = 1710000000000i64;

    for i in 0..count {
        let ts = base_time + (i as i64 * 300_000);
        let mut feats = vec![0.0; 37];

        // Alternating strong bullish and bearish signals
        let (resolution, target_up) = if i % 2 == 0 {
            // Bullish features
            feats[0] = 0.05;  // 5m return > 0
            feats[3] = 0.02;  // 15s return > 0
            feats[16] = 5.0;  // CVD 5m strongly positive
            feats[20] = 0.40; // OBI strongly positive
            (Resolution::Up, 1.0)
        } else {
            // Bearish features
            feats[0] = -0.05; // 5m return < 0
            feats[3] = -0.02; // 15s return < 0
            feats[16] = -5.0; // CVD 5m strongly negative
            feats[20] = -0.40;// OBI strongly negative
            (Resolution::Down, 0.0)
        };

        records.push(DatasetRecord {
            market_id: format!("mkt_sample_{}", i),
            asset: Asset::BTC,
            timestamp_ms: ts,
            features: feats,
            target_up,
            resolution,
        });
    }

    records
}

#[tokio::test]
async fn test_backtest_engine_simulation_and_metrics() {
    let (_, backtest, _) = setup_backtest_test_app().await;

    let records = create_sample_dataset_records(10);
    let req = BacktestRequest {
        start_time_ms: None,
        end_time_ms: None,
        assets: Some(vec![Asset::BTC]),
        initial_bankroll: Some(10.0),
        bankroll_cap: Some(10.0),
        mode: Some(BankrollMode::CapitalRecovery),
        stake: Some(1.0),
        min_prob: Some(0.60),
        min_net_edge: Some(0.04),
        fee_rate: Some(0.012),
        slippage_rate: Some(0.005),
    };

    let result = backtest.run_backtest_on_records(&req, &records).unwrap();

    assert!(!result.trades.is_empty(), "Expected backtest to generate trades");
    assert!(result.metrics.total_trades > 0);
    assert_eq!(result.metrics.total_trades, result.trades.len());

    // Equity curve should have initial point + point per trade
    assert_eq!(result.equity_curve.len(), result.trades.len() + 1);

    // Verify metrics exist and are well-formed
    assert!(result.metrics.win_rate >= 0.0 && result.metrics.win_rate <= 100.0);
    assert!(result.metrics.profit_factor >= 0.0);
    assert!(result.metrics.max_drawdown_pct >= 0.0 && result.metrics.max_drawdown_pct <= 1.0);
    assert_eq!(result.metrics.final_total_equity, result.equity_curve.last().unwrap().total_equity);
}

#[tokio::test]
async fn test_backtest_mode_b_vs_mode_a_comparison() {
    let (_, backtest, _) = setup_backtest_test_app().await;

    let records = create_sample_dataset_records(6);

    // 1. Backtest with Mode B (Capital Recovery)
    let req_b = BacktestRequest {
        mode: Some(BankrollMode::CapitalRecovery),
        initial_bankroll: Some(10.0),
        bankroll_cap: Some(10.0),
        stake: Some(1.0),
        min_prob: Some(0.60),
        min_net_edge: Some(0.04),
        ..Default::default()
    };
    let res_b = backtest.run_backtest_on_records(&req_b, &records).unwrap();

    // 2. Backtest with Mode A (Profit Isolation)
    let req_a = BacktestRequest {
        mode: Some(BankrollMode::ProfitIsolation),
        initial_bankroll: Some(10.0),
        bankroll_cap: Some(10.0),
        stake: Some(1.0),
        min_prob: Some(0.60),
        min_net_edge: Some(0.04),
        ..Default::default()
    };
    let res_a = backtest.run_backtest_on_records(&req_a, &records).unwrap();

    // Both backtests should have the exact same trades and net PnL
    assert_eq!(res_b.metrics.total_trades, res_a.metrics.total_trades);
    assert!((res_b.metrics.net_pnl - res_a.metrics.net_pnl).abs() < 1e-6);

    // In Mode A, all winning profit goes to locked_profit
    // In Mode B, active bankroll is replenished to 10.0 first
    if res_a.metrics.winning_trades > 0 {
        assert!(res_a.metrics.final_locked_profit >= 0.0);
    }
}

#[tokio::test]
async fn test_backtest_rest_api_endpoints() {
    let (app, _backtest, db) = setup_backtest_test_app().await;

    // Insert historical records into DB so /api/v1/backtest/run can load them
    let now_ms = chrono::Utc::now().timestamp_millis();
    for i in 0..5 {
        let mkt_id = format!("mkt_db_{}", i);
        let res_str = if i % 2 == 0 { "UP" } else { "DOWN" };
        let cond_id = format!("cond_{}", i);

        let _ = sqlx::query(
            r#"
            INSERT INTO markets (id, condition_id, asset, start_time, end_time, status, resolution, created_at, updated_at)
            VALUES (?, ?, 'BTC', ?, ?, 'resolved', ?, ?, ?)
            "#,
        )
        .bind(&mkt_id)
        .bind(&cond_id)
        .bind(now_ms - 10000 + (i as i64 * 300_000))
        .bind(now_ms - 10000 + ((i + 1) as i64 * 300_000))
        .bind(res_str)
        .bind(now_ms)
        .bind(now_ms)
        .execute(db.pool())
        .await;

        let vec_json = serde_json::to_string(&vec![0.1f64; 37]).unwrap();
        let _ = sqlx::query(
            r#"
            INSERT INTO features (market_id, timestamp, feature_name, feature_vector_json, created_at)
            VALUES (?, ?, 'snapshot_v1', ?, ?)
            "#,
        )
        .bind(&mkt_id)
        .bind(now_ms - 10000 + (i as i64 * 300_000))
        .bind(&vec_json)
        .bind(now_ms)
        .execute(db.pool())
        .await;
    }

    // 1. POST /api/v1/backtest/run
    let req_body = BacktestRequest {
        assets: Some(vec![Asset::BTC]),
        initial_bankroll: Some(10.0),
        min_prob: Some(0.50), // Low threshold to ensure execution
        min_net_edge: Some(0.01),
        ..Default::default()
    };
    let json_bytes = serde_json::to_vec(&req_body).unwrap();

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/backtest/run")
        .header("content-type", "application/json")
        .body(Body::from(json_bytes))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let bt_result: BacktestResult = serde_json::from_slice(&body_bytes).unwrap();
    assert!(!bt_result.backtest_id.is_empty());

    // 2. GET /api/v1/backtest/latest
    let req2 = Request::builder()
        .method("GET")
        .uri("/api/v1/backtest/latest")
        .body(Body::empty())
        .unwrap();

    let res2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
    let body_bytes2 = res2.into_body().collect().await.unwrap().to_bytes();
    let latest: BacktestResult = serde_json::from_slice(&body_bytes2).unwrap();
    assert_eq!(latest.backtest_id, bt_result.backtest_id);

    // 3. GET /api/v1/backtest/history
    let req3 = Request::builder()
        .method("GET")
        .uri("/api/v1/backtest/history?limit=10")
        .body(Body::empty())
        .unwrap();

    let res3 = app.oneshot(req3).await.unwrap();
    assert_eq!(res3.status(), StatusCode::OK);
    let body_bytes3 = res3.into_body().collect().await.unwrap().to_bytes();
    let history: Vec<BacktestResult> = serde_json::from_slice(&body_bytes3).unwrap();
    assert!(!history.is_empty());
    assert_eq!(history.first().unwrap().backtest_id, bt_result.backtest_id);
}
