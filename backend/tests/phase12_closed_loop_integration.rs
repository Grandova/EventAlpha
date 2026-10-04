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
use poly_quant_backend::polymarket::resolution::MarketResolvedEvent;
use poly_quant_backend::polymarket::PolymarketManager;
use poly_quant_backend::risk::RiskManager;
use poly_quant_backend::strategy::StrategyEngine;
use poly_quant_backend::types::{
    Asset, ConfidenceLevel, MarketSide, OrderBookLevel, PaperResult,
    PredictionSignal, Resolution, SignalAction, TradeStatistics,
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

async fn setup_closed_loop_test_app() -> (
    axum::Router,
    Arc<PaperExecutionEngine>,
    Arc<RiskManager>,
    Arc<PolymarketManager>,
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
    let risk = Arc::new(RiskManager::new(
        config.bankroll.clone(),
        config.risk.clone(),
        bankroll,
        db.clone(),
    ));
    let backtest = Arc::new(poly_quant_backend::backtest::BacktestEngine::new(
        db.clone(),
        models.clone(),
    ));
    let replay = Arc::new(poly_quant_backend::replay::ReplayEngine::new(
        db.clone(),
        models.clone(),
        strategy.clone(),
    ));
    execution.set_risk_manager(risk.clone()).await;

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        collector,
        polymarket: polymarket.clone(),
        composite,
        features,
        models: models.clone(),
        strategy,
        execution: execution.clone(),
        risk: risk.clone(),
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
    (app, execution, risk, polymarket, db)
}

#[tokio::test]
async fn test_closed_loop_winning_settlement() {
    let (_, execution, risk, polymarket, db) = setup_closed_loop_test_app().await;

    let market_id = "BTC-5M-WIN-001";
    let now_ms = chrono::Utc::now().timestamp_millis();

    // 1. Populate Polymarket orderbook for realistic fill
    polymarket.book_engine().update_book(
        market_id,
        Asset::BTC,
        MarketSide::Up,
        vec![OrderBookLevel { price: 0.50, size: 500.0 }],
        vec![OrderBookLevel { price: 0.50, size: 500.0 }],
        now_ms,
    );

    // 2. Execute a BUY_UP signal
    let signal = PredictionSignal {
        prediction_id: "pred_win_1".to_string(),
        market_id: market_id.to_string(),
        asset: Asset::BTC,
        timestamp_ms: now_ms,
        model_version: "v1.0.0".to_string(),
        p_up: 0.80,
        p_down: 0.20,
        fair_value_up: 0.80,
        fair_value_down: 0.20,
        market_implied_up: Some(0.50),
        gross_edge: 0.30,
        estimated_fee: 0.012,
        estimated_slippage: 0.005,
        net_edge: 0.283,
        signal_score: 95.0,
        confidence: ConfidenceLevel::High,
        action: SignalAction::BuyUp,
        decision_reason: "Strong positive edge".to_string(),
    };

    let fill_res = execution.execute_signal(&signal).await.unwrap();
    assert!(fill_res.is_some());
    let (order, position) = fill_res.unwrap();
    assert_eq!(order.status, "FILLED");
    assert_eq!(position.status, "OPEN");
    assert_eq!(position.side, MarketSide::Up);
    assert_eq!(position.stake, 1.0);
    assert_eq!(position.shares, 2.0); // 1.0 / 0.50 = 2.0 shares

    // Active bankroll deducted from 10.0 to 9.0
    let b_open = risk.get_bankroll_state().await;
    assert_eq!(b_open.active_bankroll, 9.0);

    // 3. Resolve market with Up outcome (Open: 60,000, Final: 60,200)
    let resolve_event = MarketResolvedEvent {
        market_id: market_id.to_string(),
        asset: Asset::BTC,
        open_price: 60000.0,
        final_price: 60200.0,
        price_change: 200.0,
        price_change_pct: 0.33,
        resolution: Resolution::Up,
        resolved_at_ms: now_ms + 300_000,
    };

    let settled_results = execution.handle_market_resolved(&resolve_event).await.unwrap();
    assert_eq!(settled_results.len(), 1);
    let res = &settled_results[0];
    assert_eq!(res.outcome, "WIN");
    assert_eq!(res.payout, 2.0); // 2 shares * $1.00 = $2.00
    assert_eq!(res.pnl, 1.0);    // $2.00 payout - $1.00 stake = +$1.00 net profit

    // Position is closed and no longer in active positions
    assert_eq!(execution.get_active_positions().len(), 0);

    // Bankroll in Mode B: active bankroll replenished from 9.0 to 10.0, total equity becomes 11.0!
    let b_settled = risk.get_bankroll_state().await;
    assert_eq!(b_settled.active_bankroll, 10.0);
    assert_eq!(b_settled.locked_profit, 1.0);
    assert_eq!(b_settled.total_equity, 11.0);

    // Verify DB persistence of paper result
    let db_results = db.get_paper_results(10).await.unwrap();
    assert_eq!(db_results.len(), 1);
    assert_eq!(db_results[0].outcome, "WIN");
    assert_eq!(db_results[0].pnl, 1.0);
}

#[tokio::test]
async fn test_closed_loop_losing_and_void_settlements() {
    let (_, execution, risk, polymarket, db) = setup_closed_loop_test_app().await;

    let market_loss = "ETH-5M-LOSS-002";
    let market_void = "SOL-5M-VOID-003";
    let now_ms = chrono::Utc::now().timestamp_millis();

    // Seed book ladders
    polymarket.book_engine().update_book(
        market_loss,
        Asset::ETH,
        MarketSide::Up,
        vec![OrderBookLevel { price: 0.50, size: 500.0 }],
        vec![OrderBookLevel { price: 0.50, size: 500.0 }],
        now_ms,
    );
    polymarket.book_engine().update_book(
        market_void,
        Asset::SOL,
        MarketSide::Down,
        vec![OrderBookLevel { price: 0.50, size: 500.0 }],
        vec![OrderBookLevel { price: 0.50, size: 500.0 }],
        now_ms,
    );

    // Trade 1 (ETH): Buy UP
    let sig_eth = PredictionSignal {
        prediction_id: "pred_eth".to_string(),
        market_id: market_loss.to_string(),
        asset: Asset::ETH,
        timestamp_ms: now_ms,
        model_version: "v1.0.0".to_string(),
        p_up: 0.75,
        p_down: 0.25,
        fair_value_up: 0.75,
        fair_value_down: 0.25,
        market_implied_up: Some(0.50),
        gross_edge: 0.25,
        estimated_fee: 0.012,
        estimated_slippage: 0.005,
        net_edge: 0.233,
        signal_score: 85.0,
        confidence: ConfidenceLevel::High,
        action: SignalAction::BuyUp,
        decision_reason: "ETH bullish".to_string(),
    };
    execution.execute_signal(&sig_eth).await.unwrap();

    // Trade 2 (SOL): Buy DOWN
    let sig_sol = PredictionSignal {
        prediction_id: "pred_sol".to_string(),
        market_id: market_void.to_string(),
        asset: Asset::SOL,
        timestamp_ms: now_ms,
        model_version: "v1.0.0".to_string(),
        p_up: 0.25,
        p_down: 0.75,
        fair_value_up: 0.25,
        fair_value_down: 0.75,
        market_implied_up: Some(0.50),
        gross_edge: 0.25,
        estimated_fee: 0.012,
        estimated_slippage: 0.005,
        net_edge: 0.233,
        signal_score: 85.0,
        confidence: ConfidenceLevel::High,
        action: SignalAction::BuyDown,
        decision_reason: "SOL bearish".to_string(),
    };
    execution.execute_signal(&sig_sol).await.unwrap();

    assert_eq!(execution.get_active_positions().len(), 2);

    // Resolve ETH as DOWN (LOSS for Up position)
    let eth_resolve = MarketResolvedEvent {
        market_id: market_loss.to_string(),
        asset: Asset::ETH,
        open_price: 3000.0,
        final_price: 2950.0,
        price_change: -50.0,
        price_change_pct: -1.67,
        resolution: Resolution::Down,
        resolved_at_ms: now_ms + 300_000,
    };
    let eth_settled = execution.handle_market_resolved(&eth_resolve).await.unwrap();
    assert_eq!(eth_settled.len(), 1);
    assert_eq!(eth_settled[0].outcome, "LOSS");
    assert_eq!(eth_settled[0].payout, 0.0);
    assert_eq!(eth_settled[0].pnl, -1.0);

    // Resolve SOL as VOID (Tie / void refund)
    let sol_resolve = MarketResolvedEvent {
        market_id: market_void.to_string(),
        asset: Asset::SOL,
        open_price: 150.0,
        final_price: 150.0,
        price_change: 0.0,
        price_change_pct: 0.0,
        resolution: Resolution::Void,
        resolved_at_ms: now_ms + 300_000,
    };
    let sol_settled = execution.handle_market_resolved(&sol_resolve).await.unwrap();
    assert_eq!(sol_settled.len(), 1);
    assert_eq!(sol_settled[0].outcome, "VOID");
    assert_eq!(sol_settled[0].payout, 1.0); // full refund
    assert_eq!(sol_settled[0].pnl, 0.0);

    // Verify all active positions closed
    assert_eq!(execution.get_active_positions().len(), 0);

    // Verify risk consecutive loss recorded for ETH
    let r_status = risk.get_risk_status().await;
    assert_eq!(r_status.consecutive_losses, 1);
    assert_eq!(r_status.daily_loss_current, 1.0);

    // Verify DB results count
    let all_res = db.get_paper_results(10).await.unwrap();
    assert_eq!(all_res.len(), 2);
}

#[tokio::test]
async fn test_trade_statistics_and_rest_endpoints() {
    let (app, execution, _, polymarket, _) = setup_closed_loop_test_app().await;

    let now_ms = chrono::Utc::now().timestamp_millis();

    // Run 1 Win and 1 Loss
    // Market 1: WIN
    polymarket.book_engine().update_book(
        "MKT-WIN",
        Asset::BTC,
        MarketSide::Up,
        vec![OrderBookLevel { price: 0.50, size: 500.0 }],
        vec![OrderBookLevel { price: 0.50, size: 500.0 }],
        now_ms,
    );
    let sig1 = PredictionSignal {
        prediction_id: "p1".to_string(),
        market_id: "MKT-WIN".to_string(),
        asset: Asset::BTC,
        timestamp_ms: now_ms,
        model_version: "v1.0.0".to_string(),
        p_up: 0.80,
        p_down: 0.20,
        fair_value_up: 0.80,
        fair_value_down: 0.20,
        market_implied_up: Some(0.50),
        gross_edge: 0.30,
        estimated_fee: 0.012,
        estimated_slippage: 0.005,
        net_edge: 0.283,
        signal_score: 90.0,
        confidence: ConfidenceLevel::High,
        action: SignalAction::BuyUp,
        decision_reason: "bull".to_string(),
    };
    execution.execute_signal(&sig1).await.unwrap();
    execution.handle_market_resolved(&MarketResolvedEvent {
        market_id: "MKT-WIN".to_string(),
        asset: Asset::BTC,
        open_price: 100.0,
        final_price: 105.0,
        price_change: 5.0,
        price_change_pct: 5.0,
        resolution: Resolution::Up,
        resolved_at_ms: now_ms + 1000,
    }).await.unwrap();

    // Market 2: LOSS
    polymarket.book_engine().update_book(
        "MKT-LOSS",
        Asset::ETH,
        MarketSide::Up,
        vec![OrderBookLevel { price: 0.50, size: 500.0 }],
        vec![OrderBookLevel { price: 0.50, size: 500.0 }],
        now_ms,
    );
    let sig2 = PredictionSignal {
        prediction_id: "p2".to_string(),
        market_id: "MKT-LOSS".to_string(),
        asset: Asset::ETH,
        timestamp_ms: now_ms,
        model_version: "v1.0.0".to_string(),
        p_up: 0.80,
        p_down: 0.20,
        fair_value_up: 0.80,
        fair_value_down: 0.20,
        market_implied_up: Some(0.50),
        gross_edge: 0.30,
        estimated_fee: 0.012,
        estimated_slippage: 0.005,
        net_edge: 0.283,
        signal_score: 90.0,
        confidence: ConfidenceLevel::High,
        action: SignalAction::BuyUp,
        decision_reason: "bull".to_string(),
    };
    execution.execute_signal(&sig2).await.unwrap();
    execution.handle_market_resolved(&MarketResolvedEvent {
        market_id: "MKT-LOSS".to_string(),
        asset: Asset::ETH,
        open_price: 100.0,
        final_price: 95.0,
        price_change: -5.0,
        price_change_pct: -5.0,
        resolution: Resolution::Down,
        resolved_at_ms: now_ms + 1000,
    }).await.unwrap();

    // 1. GET /api/v1/paper/results
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/paper/results?limit=10")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let results: Vec<PaperResult> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(results.len(), 2);

    // 2. GET /api/v1/paper/statistics
    let req2 = Request::builder()
        .method("GET")
        .uri("/api/v1/paper/statistics")
        .body(Body::empty())
        .unwrap();

    let res2 = app.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
    let body_bytes2 = res2.into_body().collect().await.unwrap().to_bytes();
    let stats: TradeStatistics = serde_json::from_slice(&body_bytes2).unwrap();
    assert_eq!(stats.total_trades, 2);
    assert_eq!(stats.winning_trades, 1);
    assert_eq!(stats.losing_trades, 1);
    assert_eq!(stats.win_rate, 50.0);
    assert_eq!(stats.gross_profit, 1.0);
    assert_eq!(stats.gross_loss, 1.0);
    assert_eq!(stats.profit_factor, 1.0);
    assert_eq!(stats.total_pnl, 0.0);
}
