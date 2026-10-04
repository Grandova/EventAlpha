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
use poly_quant_backend::features::FeatureEngine;
use poly_quant_backend::models::{ConfidenceLevel as ModelConfidence, ModelManager, ModelPrediction};
use poly_quant_backend::polymarket::PolymarketManager;
use poly_quant_backend::strategy::filter::HardFilterEngine;
use poly_quant_backend::strategy::scoring::{ScoreTier, SignalScorer};
use poly_quant_backend::strategy::StrategyEngine;
use poly_quant_backend::types::{
    Asset, ConfidenceLevel, DecisionLog, PolymarketTick, PredictionSignal, SignalAction,
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

async fn setup_strategy_test_app() -> (axum::Router, Arc<StrategyEngine>, Arc<Database>, Arc<AppConfig>) {
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
    let execution = Arc::new(poly_quant_backend::execution::PaperExecutionEngine::new(
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
        strategy: strategy.clone(),
        execution,
        live_risk: risk.clone(),
        risk,
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
    (app, strategy, db, config)
}

#[tokio::test]
async fn test_hard_filter_seven_gates_comprehensive() {
    let (_, _, _, config) = setup_strategy_test_app().await;
    let cfg = &config.strategy;

    // Gate 1: Calibrated Probability Threshold (>= 70%)
    assert!(HardFilterEngine::check_probability(0.70, cfg).is_pass());
    assert!(HardFilterEngine::check_probability(0.85, cfg).is_pass());
    assert!(!HardFilterEngine::check_probability(0.699, cfg).is_pass());

    // Gate 2: Net Edge Threshold (>= 5%)
    assert!(HardFilterEngine::check_net_edge(0.05, cfg).is_pass());
    assert!(HardFilterEngine::check_net_edge(0.12, cfg).is_pass());
    assert!(!HardFilterEngine::check_net_edge(0.049, cfg).is_pass());

    // Gate 3: Entry Price Ceiling (<= 0.85)
    assert!(HardFilterEngine::check_entry_price(0.85, cfg).is_pass());
    assert!(HardFilterEngine::check_entry_price(0.55, cfg).is_pass());
    assert!(!HardFilterEngine::check_entry_price(0.851, cfg).is_pass());

    // Gate 4: Maximum Spread (<= 0.04)
    assert!(HardFilterEngine::check_spread(0.04, cfg).is_pass());
    assert!(HardFilterEngine::check_spread(0.015, cfg).is_pass());
    assert!(!HardFilterEngine::check_spread(0.045, cfg).is_pass());

    // Gate 5: Minimum Book Liquidity (>= $300 USDC)
    assert!(HardFilterEngine::check_liquidity(300.0, cfg).is_pass());
    assert!(HardFilterEngine::check_liquidity(1500.0, cfg).is_pass());
    assert!(!HardFilterEngine::check_liquidity(299.9, cfg).is_pass());

    // Gate 6: Timing Execution Window (15s <= t <= 285s)
    assert!(HardFilterEngine::check_timing(15, cfg).is_pass());
    assert!(HardFilterEngine::check_timing(150, cfg).is_pass());
    assert!(HardFilterEngine::check_timing(285, cfg).is_pass());
    assert!(!HardFilterEngine::check_timing(14, cfg).is_pass()); // too late
    assert!(!HardFilterEngine::check_timing(286, cfg).is_pass()); // too early

    // Gate 7: Exchange Data Freshness (Fail-Closed)
    assert!(HardFilterEngine::check_freshness(true).is_pass());
    assert!(!HardFilterEngine::check_freshness(false).is_pass());
}

#[tokio::test]
async fn test_opportunity_scoring_breakdown_and_tiers() {
    let (_, _, _, config) = setup_strategy_test_app().await;
    let thresholds = &config.strategy.score_thresholds;

    // 1. High-tier Opportunity (12% net edge, 82% prob, 0.45 OBI, 0.35 momentum, 120s remaining)
    let high = SignalScorer::compute_score(
        0.12, 0.82, 0.45, 0.35, 120, 0.05, 0.70, thresholds,
    );
    assert!(high.total_score >= 80.0, "Score was {}", high.total_score);
    assert_eq!(high.tier, ScoreTier::High);
    assert_eq!(high.timing_points, 5.0); // sweet spot

    // 2. Maximum possible score scenario
    let max = SignalScorer::compute_score(
        0.20, 0.95, 0.80, 0.60, 150, 0.05, 0.70, thresholds,
    );
    assert!((max.total_score - 100.0).abs() < 1e-4);
    assert_eq!(max.tier, ScoreTier::VeryHigh);

    // 3. Sub-par Opportunity below skip threshold
    let low = SignalScorer::compute_score(
        0.01, 0.52, -0.40, -0.30, 20, 0.05, 0.70, thresholds,
    );
    assert!(low.total_score < 60.0);
    assert_eq!(low.tier, ScoreTier::Skip);
}

#[tokio::test]
async fn test_strategy_evaluation_up_and_down_signals() {
    let (_, strategy, _, _) = setup_strategy_test_app().await;

    let now_ms = chrono::Utc::now().timestamp_millis();

    // 1. BUY UP Opportunity
    let up_pred = ModelPrediction {
        market_id: "mkt_btc_test_up".to_string(),
        asset: Asset::BTC,
        timestamp_ms: now_ms,
        model_version: "ensemble_v1.0".to_string(),
        raw_p_up: 0.80,
        raw_p_down: 0.20,
        calibrated_p_up: 0.78,
        calibrated_p_down: 0.22,
        confidence: ModelConfidence::High,
        top_contributions: vec![],
    };

    let poly_tick_up = PolymarketTick {
        market_id: "mkt_btc_test_up".to_string(),
        asset: Asset::BTC,
        timestamp_ms: now_ms,
        up_bid: Some(0.60),
        up_ask: Some(0.65), // entry price = 0.65
        down_bid: Some(0.30),
        down_ask: Some(0.35),
        up_mid: Some(0.625),
        down_mid: Some(0.325),
        spread: Some(0.02),
        volume_24h: Some(1000.0),
        liquidity: Some(800.0),
    };

    let (up_sig, up_dec) = strategy.evaluate(
        &up_pred,
        Some(&poly_tick_up),
        None,
        0.002, // +0.2% return
        true,
        150, // sweet spot
    );

    assert_eq!(up_sig.action, SignalAction::BuyUp);
    assert_eq!(up_dec.final_action, SignalAction::BuyUp);
    assert!((up_sig.gross_edge - (0.78 - 0.65)).abs() < 1e-4); // 0.13
    assert!((up_sig.estimated_fee - 0.012).abs() < 1e-4);
    assert!((up_sig.estimated_slippage - 0.005).abs() < 1e-4);
    assert!((up_sig.net_edge - (0.13 - 0.012 - 0.005)).abs() < 1e-4); // ~0.113
    assert!(up_sig.signal_score >= 60.0);

    // 2. BUY DOWN Opportunity
    let down_pred = ModelPrediction {
        market_id: "mkt_eth_test_down".to_string(),
        asset: Asset::ETH,
        timestamp_ms: now_ms,
        model_version: "ensemble_v1.0".to_string(),
        raw_p_up: 0.18,
        raw_p_down: 0.82,
        calibrated_p_up: 0.20,
        calibrated_p_down: 0.80,
        confidence: ModelConfidence::High,
        top_contributions: vec![],
    };

    let poly_tick_down = PolymarketTick {
        market_id: "mkt_eth_test_down".to_string(),
        asset: Asset::ETH,
        timestamp_ms: now_ms,
        up_bid: Some(0.30),
        up_ask: Some(0.35),
        down_bid: Some(0.62),
        down_ask: Some(0.68), // entry price = 0.68
        up_mid: Some(0.325),
        down_mid: Some(0.65),
        spread: Some(0.02),
        volume_24h: Some(1500.0),
        liquidity: Some(1200.0),
    };

    let (down_sig, down_dec) = strategy.evaluate(
        &down_pred,
        Some(&poly_tick_down),
        None,
        -0.003, // -0.3% return (agrees with down)
        true,
        180,
    );

    assert_eq!(down_sig.action, SignalAction::BuyDown);
    assert_eq!(down_dec.final_action, SignalAction::BuyDown);
    assert!((down_sig.gross_edge - (0.80 - 0.68)).abs() < 1e-4); // 0.12
    assert!((down_sig.net_edge - (0.12 - 0.012 - 0.005)).abs() < 1e-4); // ~0.103
    assert!(down_sig.signal_score >= 60.0);

    // 3. Stale Data => Fail-Closed SKIP
    let (stale_sig, stale_dec) = strategy.evaluate(
        &up_pred,
        Some(&poly_tick_up),
        None,
        0.002,
        false, // STALE!
        150,
    );
    assert_eq!(stale_sig.action, SignalAction::Skip);
    assert_eq!(stale_dec.final_action, SignalAction::Skip);
    assert!(stale_sig.decision_reason.contains("stale or disconnected"));
}

#[tokio::test]
async fn test_db_persistence_and_retrieval_of_signals() {
    let (_, _, db, _) = setup_strategy_test_app().await;

    let now_ms = chrono::Utc::now().timestamp_millis();
    let test_signal = PredictionSignal {
        prediction_id: "pred_db_test_001".to_string(),
        market_id: "mkt_sol_test_db".to_string(),
        asset: Asset::SOL,
        timestamp_ms: now_ms,
        model_version: "ensemble_v1.0".to_string(),
        p_up: 0.76,
        p_down: 0.24,
        fair_value_up: 0.76,
        fair_value_down: 0.24,
        market_implied_up: Some(0.62),
        gross_edge: 0.14,
        estimated_fee: 0.012,
        estimated_slippage: 0.005,
        net_edge: 0.123,
        signal_score: 84.5,
        confidence: ConfidenceLevel::High,
        action: SignalAction::BuyUp,
        decision_reason: "PASSED all filters".to_string(),
    };

    // Insert into DB
    db.insert_prediction(&test_signal).await.unwrap();

    // Query back
    let list = db.get_recent_predictions(10).await.unwrap();
    assert_eq!(list.len(), 1);
    let retrieved = &list[0];
    assert_eq!(retrieved.prediction_id, "pred_db_test_001");
    assert_eq!(retrieved.market_id, "mkt_sol_test_db");
    assert_eq!(retrieved.asset, Asset::SOL);
    assert_eq!(retrieved.action, SignalAction::BuyUp);
    assert_eq!(retrieved.confidence, ConfidenceLevel::High);
    assert!((retrieved.signal_score - 84.5).abs() < 1e-4);
    assert!((retrieved.net_edge - 0.123).abs() < 1e-4);
}

#[tokio::test]
async fn test_strategy_api_endpoints() {
    let (app, strategy, _, _) = setup_strategy_test_app().await;

    let now_ms = chrono::Utc::now().timestamp_millis();

    // 1. Initially no signals evaluated => 404
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/strategy/signals/latest/BTC")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 2. Process a prediction through strategy
    let pred = ModelPrediction {
        market_id: "mkt_btc_api_test".to_string(),
        asset: Asset::BTC,
        timestamp_ms: now_ms,
        model_version: "ensemble_v1.0".to_string(),
        raw_p_up: 0.82,
        raw_p_down: 0.18,
        calibrated_p_up: 0.80,
        calibrated_p_down: 0.20,
        confidence: ModelConfidence::High,
        top_contributions: vec![],
    };

    let poly_tick = PolymarketTick {
        market_id: "mkt_btc_api_test".to_string(),
        asset: Asset::BTC,
        timestamp_ms: now_ms,
        up_bid: Some(0.62),
        up_ask: Some(0.66),
        down_bid: Some(0.30),
        down_ask: Some(0.34),
        up_mid: Some(0.64),
        down_mid: Some(0.32),
        spread: Some(0.02),
        volume_24h: Some(2000.0),
        liquidity: Some(1500.0),
    };

    let (sig, dec) = strategy
        .process_prediction(&pred, Some(&poly_tick), None, 0.001, true, 120)
        .await;

    assert_eq!(sig.action, SignalAction::BuyUp);
    assert_eq!(dec.final_action, SignalAction::BuyUp);

    // 3. Query latest signal for BTC
    let res_btc = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/strategy/signals/latest/BTC")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_btc.status(), StatusCode::OK);
    let bytes = res_btc.into_body().collect().await.unwrap().to_bytes();
    let btc_signal: PredictionSignal = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(btc_signal.asset, Asset::BTC);
    assert_eq!(btc_signal.action, SignalAction::BuyUp);
    assert_eq!(btc_signal.market_id, "mkt_btc_api_test");

    // 4. Query all latest signals
    let res_all = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/strategy/signals/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_all.status(), StatusCode::OK);
    let bytes_all = res_all.into_body().collect().await.unwrap().to_bytes();
    let map: std::collections::HashMap<String, PredictionSignal> =
        serde_json::from_slice(&bytes_all).unwrap();
    assert_eq!(map.len(), 1);
    assert!(map.contains_key("BTC"));

    // 5. Query recent decisions
    let res_decisions = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/strategy/decisions/recent?limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_decisions.status(), StatusCode::OK);
    let bytes_dec = res_decisions.into_body().collect().await.unwrap().to_bytes();
    let decisions: Vec<DecisionLog> = serde_json::from_slice(&bytes_dec).unwrap();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].market_id, "mkt_btc_api_test");
    assert_eq!(decisions[0].final_action, SignalAction::BuyUp);
}

#[tokio::test]
async fn test_enabled_assets_api_and_skip_logic() {
    let (app, strategy, _, _) = setup_strategy_test_app().await;

    // 1. Initial enabled assets should include BTC, ETH, SOL
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/strategy/assets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["assets"].as_array().unwrap().len(), 3);

    // 2. Disable ETH and SOL, keep only BTC
    let update_body = serde_json::json!({
        "assets": ["BTC"]
    });
    let res_set = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/strategy/assets")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_set.status(), StatusCode::OK);

    // 3. Evaluate ETH prediction - should be SKIPPED because it is disabled
    let eth_pred = ModelPrediction {
        market_id: "mkt_eth_test".to_string(),
        asset: Asset::ETH,
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        model_version: "ensemble_v1.0".to_string(),
        raw_p_up: 0.85,
        raw_p_down: 0.15,
        calibrated_p_up: 0.82,
        calibrated_p_down: 0.18,
        confidence: ModelConfidence::High,
        top_contributions: vec![],
    };
    let (eth_sig, eth_dec) = strategy.evaluate(&eth_pred, None, None, 0.002, true, 150);
    assert_eq!(eth_sig.action, SignalAction::Skip);
    assert_eq!(eth_dec.final_action, SignalAction::Skip);
    assert!(eth_sig.decision_reason.contains("自动交易已在设置中禁用"));

    // 4. Evaluate BTC prediction - should NOT be skipped by asset filter
    let btc_pred = ModelPrediction {
        market_id: "mkt_btc_test".to_string(),
        asset: Asset::BTC,
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        model_version: "ensemble_v1.0".to_string(),
        raw_p_up: 0.85,
        raw_p_down: 0.15,
        calibrated_p_up: 0.82,
        calibrated_p_down: 0.18,
        confidence: ModelConfidence::High,
        top_contributions: vec![],
    };
    let (btc_sig, _) = strategy.evaluate(&btc_pred, None, None, 0.002, true, 150);
    assert_ne!(btc_sig.decision_reason, "标的 BTC 自动交易已在设置中禁用");
}
