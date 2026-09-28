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
use poly_quant_backend::polymarket::orderbook::PolymarketBookEngine;
use poly_quant_backend::polymarket::PolymarketManager;
use poly_quant_backend::strategy::StrategyEngine;
use poly_quant_backend::types::{
    Asset, ConfidenceLevel, MarketSide, OrderBookLevel, PaperOrder, PaperPosition,
    PredictionSignal, SignalAction,
};

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

async fn setup_execution_test_app() -> (
    axum::Router,
    Arc<PaperExecutionEngine>,
    Arc<PolymarketBookEngine>,
    Arc<Database>,
) {
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
    let risk = Arc::new(poly_quant_backend::risk::RiskManager::new(
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
        polymarket: polymarket.clone(),
        composite,
        features,
        models,
        strategy,
        execution: execution.clone(),
        risk,
        backtest,
        replay,
        start_time_ms: chrono::Utc::now().timestamp_millis(),
    };
    let app = create_router(state);
    (app, execution, polymarket.book_engine(), db)
}

#[tokio::test]
async fn test_paper_execution_depth_walking_fill() {
    let (_, execution, books, db) = setup_execution_test_app().await;

    let now_ms = chrono::Utc::now().timestamp_millis();

    // 1. Seed UP orderbook with multiple levels
    // Level 1: price 0.50, size 0.5 (capacity = 0.25 USDC)
    // Level 2: price 0.52, size 1.0 (capacity = 0.52 USDC)
    // Level 3: price 0.55, size 2.0 (capacity = 1.10 USDC)
    let bids = vec![OrderBookLevel { price: 0.48, size: 5.0 }];
    let asks = vec![
        OrderBookLevel { price: 0.50, size: 0.5 },
        OrderBookLevel { price: 0.52, size: 1.0 },
        OrderBookLevel { price: 0.55, size: 2.0 },
    ];
    books.update_book("tok_btc_up", Asset::BTC, MarketSide::Up, bids, asks, now_ms);

    // 2. Formulate BUY UP signal
    let signal = PredictionSignal {
        prediction_id: "pred_exec_001".to_string(),
        market_id: "mkt_btc_exec_test".to_string(),
        asset: Asset::BTC,
        timestamp_ms: now_ms,
        model_version: "ensemble_v1.0".to_string(),
        p_up: 0.78,
        p_down: 0.22,
        fair_value_up: 0.78,
        fair_value_down: 0.22,
        market_implied_up: Some(0.50),
        gross_edge: 0.28,
        estimated_fee: 0.012,
        estimated_slippage: 0.005,
        net_edge: 0.263,
        signal_score: 88.0,
        confidence: ConfidenceLevel::High,
        action: SignalAction::BuyUp,
        decision_reason: "PASSED".to_string(),
    };

    // 3. Execute paper order
    let res = execution.execute_signal(&signal).await.unwrap();
    assert!(res.is_some());
    let (order, position) = res.unwrap();

    // Check order properties
    assert_eq!(order.market_id, "mkt_btc_exec_test");
    assert_eq!(order.asset, Asset::BTC);
    assert_eq!(order.side, MarketSide::Up);
    assert_eq!(order.stake, 1.0);
    assert_eq!(order.status, "FILLED");
    assert_eq!(order.quote_price, 0.50); // best ask
    assert!((order.fee - 0.012).abs() < 1e-4);

    // Section 21 depth-walking verification:
    // Level 1: 0.25 USDC -> 0.5 shares
    // Level 2: 0.52 USDC -> 1.0 shares
    // Remainder 0.23 USDC at 0.55 -> 0.23 / 0.55 = 0.41818 shares
    // Total shares = 0.5 + 1.0 + 0.41818 = 1.91818
    assert!((order.shares - 1.91818).abs() < 1e-3);
    let expected_avg_price = 1.0 / order.shares;
    assert!((order.fill_price - expected_avg_price).abs() < 1e-4);
    assert!((order.slippage - (expected_avg_price - 0.50)).abs() < 1e-4);

    // Check position properties
    assert_eq!(position.status, "OPEN");
    assert_eq!(position.market_id, "mkt_btc_exec_test");
    assert_eq!(position.entry_price, order.fill_price);
    assert_eq!(position.shares, order.shares);
    assert_eq!(position.stake, 1.0);

    // Check database records
    let orders_db = db.get_recent_paper_orders(10).await.unwrap();
    assert_eq!(orders_db.len(), 1);
    assert_eq!(orders_db[0].order_id, order.order_id);

    let active_db = db.get_active_positions().await.unwrap();
    assert_eq!(active_db.len(), 1);
    assert_eq!(active_db[0].position_id, position.position_id);
}

#[tokio::test]
async fn test_insufficient_liquidity_rejection() {
    let (_, execution, books, db) = setup_execution_test_app().await;

    let now_ms = chrono::Utc::now().timestamp_millis();

    // Book has only 0.20 USDC capacity (0.40 price * 0.5 size = 0.20 USDC), but stake is 1.0 USDC
    let bids = vec![OrderBookLevel { price: 0.38, size: 1.0 }];
    let asks = vec![OrderBookLevel { price: 0.40, size: 0.5 }];
    books.update_book("tok_eth_up", Asset::ETH, MarketSide::Up, bids, asks, now_ms);

    let signal = PredictionSignal {
        prediction_id: "pred_eth_low_liq".to_string(),
        market_id: "mkt_eth_low_liq".to_string(),
        asset: Asset::ETH,
        timestamp_ms: now_ms,
        model_version: "ensemble_v1.0".to_string(),
        p_up: 0.75,
        p_down: 0.25,
        fair_value_up: 0.75,
        fair_value_down: 0.25,
        market_implied_up: Some(0.40),
        gross_edge: 0.35,
        estimated_fee: 0.012,
        estimated_slippage: 0.005,
        net_edge: 0.333,
        signal_score: 85.0,
        confidence: ConfidenceLevel::High,
        action: SignalAction::BuyUp,
        decision_reason: "PASSED".to_string(),
    };

    let res = execution.execute_signal(&signal).await.unwrap();
    assert!(res.is_none(), "Expected order to be rejected due to low book depth");

    // Verify rejection was recorded in paper_orders
    let orders = db.get_recent_paper_orders(10).await.unwrap();
    assert_eq!(orders.len(), 1);
    assert!(orders[0].status.contains("REJECTED"));
    assert!(orders[0].status.contains("Insufficient orderbook liquidity"));

    // Verify no open positions
    let active = execution.get_active_positions();
    assert!(active.is_empty());
}

#[tokio::test]
async fn test_duplicate_position_prevention() {
    let (_, execution, books, _) = setup_execution_test_app().await;

    let now_ms = chrono::Utc::now().timestamp_millis();

    let bids = vec![OrderBookLevel { price: 0.40, size: 10.0 }];
    let asks = vec![OrderBookLevel { price: 0.45, size: 10.0 }];
    books.update_book("tok_sol_up", Asset::SOL, MarketSide::Up, bids, asks, now_ms);

    let signal = PredictionSignal {
        prediction_id: "pred_sol_dup_1".to_string(),
        market_id: "mkt_sol_same_round".to_string(),
        asset: Asset::SOL,
        timestamp_ms: now_ms,
        model_version: "ensemble_v1.0".to_string(),
        p_up: 0.75,
        p_down: 0.25,
        fair_value_up: 0.75,
        fair_value_down: 0.25,
        market_implied_up: Some(0.45),
        gross_edge: 0.30,
        estimated_fee: 0.012,
        estimated_slippage: 0.005,
        net_edge: 0.283,
        signal_score: 85.0,
        confidence: ConfidenceLevel::High,
        action: SignalAction::BuyUp,
        decision_reason: "PASSED".to_string(),
    };

    // First execution succeeds
    let first = execution.execute_signal(&signal).await.unwrap();
    assert!(first.is_some());

    // Second execution with same market_id must be skipped
    let second = execution.execute_signal(&signal).await.unwrap();
    assert!(second.is_none(), "Duplicate position in same market round must be skipped");

    assert_eq!(execution.get_active_positions().len(), 1);
}

#[tokio::test]
async fn test_paper_orders_and_positions_api_endpoints() {
    let (app, execution, books, _) = setup_execution_test_app().await;

    let now_ms = chrono::Utc::now().timestamp_millis();

    let bids = vec![OrderBookLevel { price: 0.45, size: 10.0 }];
    let asks = vec![OrderBookLevel { price: 0.50, size: 10.0 }];
    books.update_book("tok_btc_api", Asset::BTC, MarketSide::Up, bids, asks, now_ms);

    let signal = PredictionSignal {
        prediction_id: "pred_api_test".to_string(),
        market_id: "mkt_btc_api_exec".to_string(),
        asset: Asset::BTC,
        timestamp_ms: now_ms,
        model_version: "ensemble_v1.0".to_string(),
        p_up: 0.76,
        p_down: 0.24,
        fair_value_up: 0.76,
        fair_value_down: 0.24,
        market_implied_up: Some(0.50),
        gross_edge: 0.26,
        estimated_fee: 0.012,
        estimated_slippage: 0.005,
        net_edge: 0.243,
        signal_score: 82.0,
        confidence: ConfidenceLevel::High,
        action: SignalAction::BuyUp,
        decision_reason: "PASSED".to_string(),
    };

    execution.execute_signal(&signal).await.unwrap();

    // 1. Query /api/v1/paper/orders
    let res_orders = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/paper/orders?limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_orders.status(), StatusCode::OK);
    let bytes_orders = res_orders.into_body().collect().await.unwrap().to_bytes();
    let orders: Vec<PaperOrder> = serde_json::from_slice(&bytes_orders).unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].market_id, "mkt_btc_api_exec");

    // 2. Query /api/v1/paper/positions/active
    let res_active = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/paper/positions/active")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_active.status(), StatusCode::OK);
    let bytes_active = res_active.into_body().collect().await.unwrap().to_bytes();
    let positions: Vec<PaperPosition> = serde_json::from_slice(&bytes_active).unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].status, "OPEN");

    // 3. Query /api/v1/paper/positions/history
    let res_history = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/paper/positions/history?limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_history.status(), StatusCode::OK);
    let bytes_history = res_history.into_body().collect().await.unwrap().to_bytes();
    let history: Vec<PaperPosition> = serde_json::from_slice(&bytes_history).unwrap();
    assert_eq!(history.len(), 1);
}
