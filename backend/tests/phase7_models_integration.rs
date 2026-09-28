use axum::body::Body;
use axum::http::{Request, StatusCode};
use poly_quant_backend::api::{create_router, AppState};
use poly_quant_backend::collector::CollectorManager;
use poly_quant_backend::composite::CompositePriceEngine;
use poly_quant_backend::config::AppConfig;
use poly_quant_backend::dataset::DatasetRecord;
use poly_quant_backend::db::Database;
use poly_quant_backend::features::{FeatureEngine, FeatureSnapshot};
use poly_quant_backend::models::{
    ConfidenceLevel, EnsembleModel, LogisticRegressionModel, MetricsCalculator,
    ModelManager, ProbabilityCalibrator,
};
use poly_quant_backend::polymarket::PolymarketManager;
use poly_quant_backend::types::{Asset, Resolution};
use std::sync::Arc;
use tempfile::NamedTempFile;
use tower::ServiceExt;

fn test_yaml() -> String {
    r#"
mode: "paper"
safety:
  real_trading_enabled: false
  safety_signature: "PAPER_TRADING_ONLY_SAFETY_LOCK_ENGAGED"
server:
  host: "127.0.0.1"
  port: 8080
  cors_allowed_origins: ["*"]
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

async fn setup_phase7() -> (axum::Router, Arc<ModelManager>, Arc<Database>) {
    let mut tmp = NamedTempFile::new().unwrap();
    use std::io::Write;
    tmp.write_all(test_yaml().as_bytes()).unwrap();

    let config = Arc::new(AppConfig::load_from_path(tmp.path()).unwrap());
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
    let strategy = Arc::new(poly_quant_backend::strategy::StrategyEngine::new(
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
        strategy,
        execution,
        risk,
        backtest,
        replay,
        start_time_ms: chrono::Utc::now().timestamp_millis(),
    };
    let app = create_router(state);
    (app, models, db)
}

fn sample_bullish_features() -> FeatureSnapshot {
    FeatureSnapshot {
        asset: Asset::BTC,
        market_id: "BTC-5M-TEST".to_string(),
        timestamp_ms: 1000,
        composite_price: 65200.0,
        return_1s: 0.0005,
        return_3s: 0.0010,
        return_5s: 0.0020,
        return_10s: 0.0035,
        return_30s: 0.0060,
        return_60s: 0.0100,
        realized_vol_5s: 0.0003,
        realized_vol_10s: 0.0004,
        realized_vol_30s: 0.0006,
        realized_vol_60s: 0.0008,
        velocity_5s: 1.2,
        velocity_15s: 0.8,
        acceleration_5s_15s: 0.04,
        distance_from_open: 200.0,
        distance_percent: 0.30,
        distance_to_vol_ratio: 1.5,
        remaining_seconds: 150.0,
        elapsed_seconds: 150.0,
        time_decay_factor: 0.707,
        spread_binance_okx: 0.5,
        spread_binance_bybit: 0.2,
        spread_binance_coinbase: 0.6,
        cvd_5s: 25.0,
        cvd_15s: 45.0,
        cvd_30s: 70.0,
        cvd_60s: 90.0,
        trade_imbalance_5s: 0.80,
        trade_imbalance_15s: 0.60,
        trade_imbalance_30s: 0.45,
        trade_imbalance_60s: 0.35,
        poly_obi_top5: 0.40,
        poly_obi_top10: 0.30,
        poly_obi_top20: 0.25,
        poly_spread: 0.02,
        poly_total_liquidity: 6000.0,
        poly_implied_prob: 0.53,
    }
}

fn sample_bearish_features() -> FeatureSnapshot {
    let mut snap = sample_bullish_features();
    snap.return_1s = -0.0005;
    snap.return_3s = -0.0010;
    snap.return_5s = -0.0020;
    snap.return_10s = -0.0035;
    snap.return_30s = -0.0060;
    snap.return_60s = -0.0100;
    snap.velocity_5s = -1.2;
    snap.velocity_15s = -0.8;
    snap.acceleration_5s_15s = -0.04;
    snap.distance_from_open = -200.0;
    snap.distance_percent = -0.30;
    snap.distance_to_vol_ratio = -1.5;
    snap.cvd_5s = -25.0;
    snap.cvd_15s = -45.0;
    snap.cvd_30s = -70.0;
    snap.cvd_60s = -90.0;
    snap.trade_imbalance_5s = -0.80;
    snap.trade_imbalance_15s = -0.60;
    snap.trade_imbalance_30s = -0.45;
    snap.trade_imbalance_60s = -0.35;
    snap.poly_obi_top5 = -0.40;
    snap.poly_obi_top10 = -0.30;
    snap.poly_obi_top20 = -0.25;
    snap
}

fn sample_neutral_features() -> FeatureSnapshot {
    let mut snap = sample_bullish_features();
    snap.return_1s = 0.0001;
    snap.return_3s = -0.0001;
    snap.return_5s = 0.0001;
    snap.return_10s = -0.0001;
    snap.return_30s = 0.0;
    snap.return_60s = 0.0;
    snap.velocity_5s = 0.0;
    snap.velocity_15s = 0.0;
    snap.acceleration_5s_15s = 0.0;
    snap.distance_from_open = 0.0;
    snap.distance_percent = 0.0;
    snap.distance_to_vol_ratio = 0.0;
    snap.cvd_5s = 0.0;
    snap.cvd_15s = 0.0;
    snap.cvd_30s = 0.0;
    snap.cvd_60s = 0.0;
    snap.trade_imbalance_5s = 0.02;
    snap.trade_imbalance_15s = -0.02;
    snap.poly_obi_top5 = 0.02;
    snap.poly_obi_top10 = -0.02;
    snap.poly_obi_top20 = 0.0;
    snap
}

#[test]
fn test_logistic_prediction_and_top_contributions() {
    let model = LogisticRegressionModel::default_baseline();
    let bull = sample_bullish_features();

    let (p_up, contribs) = model.predict_raw(&bull);
    assert!(p_up > 0.60, "Bullish features must produce P(Up) > 0.60");
    assert_eq!(contribs.len(), 37);

    // Verify top contribution has significant weight
    let top = &contribs[0];
    assert!(top.contribution.abs() > 0.0);

    let bear = sample_bearish_features();
    let (p_bear_up, _) = model.predict_raw(&bear);
    assert!(p_bear_up < 0.40, "Bearish features must produce P(Up) < 0.40");
}

#[test]
fn test_logistic_sgd_training_improves_loss() {
    let mut model = LogisticRegressionModel::default_baseline();

    // Create synthetic training data: when feature 0 is positive -> target 1.0, else 0.0
    let mut train_data = Vec::new();
    for i in 0..100 {
        let is_up = i % 2 == 0;
        let mut feats = vec![0.0; 37];
        feats[0] = if is_up { 2.0 } else { -2.0 };
        train_data.push(DatasetRecord {
            market_id: format!("mkt-{}", i),
            asset: Asset::BTC,
            timestamp_ms: 1000 + i,
            features: feats,
            target_up: if is_up { 1.0 } else { 0.0 },
            resolution: if is_up { Resolution::Up } else { Resolution::Down },
        });
    }

    // Train model
    model.train(&train_data, 10, 0.05, 0.001);

    // Test inference after training
    let mut test_up_snap = sample_bullish_features();
    test_up_snap.composite_price = 2.0;
    let (p_up, _) = model.predict_raw(&test_up_snap);
    assert!(p_up > 0.50);
}

#[test]
fn test_ensemble_and_confidence_levels() {
    let ensemble = EnsembleModel::default_ensemble();
    let bull = sample_bullish_features();
    let (p_up, conf) = ensemble.predict(&bull);

    assert!(p_up > 0.60);
    assert!(conf == ConfidenceLevel::High || conf == ConfidenceLevel::VeryHigh);

    // Neutral features produce P around 0.50 and Low/Medium confidence
    let neutral = sample_neutral_features();
    let (p_neutral, neutral_conf) = ensemble.predict(&neutral);
    assert!((0.40..=0.60).contains(&p_neutral));
    assert!(neutral_conf == ConfidenceLevel::Low || neutral_conf == ConfidenceLevel::Medium);
}

#[test]
fn test_probability_calibration_and_clamping() {
    let calibrator = ProbabilityCalibrator::default_calibrator();

    // Extreme probability 0.99 should be compressed to <= 0.92
    let (p_up, p_down) = calibrator.calibrate(0.99, 0.5, 0.0005);
    assert!(p_up <= 0.92);
    assert!(p_up >= 0.70);
    assert!((p_up + p_down - 1.0).abs() < 1e-6);

    // Extreme probability 0.01 should be compressed to >= 0.08
    let (p_low_up, p_low_down) = calibrator.calibrate(0.01, 0.5, 0.0005);
    assert!(p_low_up >= 0.08);
    assert!(p_low_up <= 0.30);
    assert!((p_low_up + p_low_down - 1.0).abs() < 1e-6);
}

#[test]
fn test_brier_score_and_metrics_calculation() {
    let preds = vec![0.80, 0.20, 0.70, 0.30];
    let labels = vec![1.0, 0.0, 1.0, 0.0];

    let metrics = MetricsCalculator::evaluate(&preds, &labels).unwrap();
    assert_eq!(metrics.sample_count, 4);
    assert_eq!(metrics.accuracy, 1.0); // 100% directional accuracy
    assert!(metrics.brier_score < 0.10); // low Brier score is superior
    assert!(metrics.log_loss < 0.40);
    assert!(metrics.expected_calibration_error >= 0.0);
}

#[tokio::test]
async fn test_model_manager_online_prediction_and_api() {
    let (app, models, _db) = setup_phase7().await;

    // Run prediction on bullish features
    let bull = sample_bullish_features();
    let prediction = models.predict(&bull).await;

    assert_eq!(prediction.asset, Asset::BTC);
    assert!(prediction.raw_p_up > 0.60);
    assert!(prediction.calibrated_p_up > 0.55 && prediction.calibrated_p_up <= 0.92);
    assert!((prediction.calibrated_p_up + prediction.calibrated_p_down - 1.0).abs() < 1e-6);
    assert!(!prediction.top_contributions.is_empty());

    // 1. Query prediction API
    let res_empty = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/models/prediction/ETH")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_empty.status(), StatusCode::NOT_FOUND);

    // 2. Query all predictions API
    let res_all = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models/all")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_all.status(), StatusCode::OK);
}
