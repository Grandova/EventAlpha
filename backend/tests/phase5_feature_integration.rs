use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use poly_quant_backend::api::{create_router, AppState};
use poly_quant_backend::collector::CollectorManager;
use poly_quant_backend::composite::CompositePriceEngine;
use poly_quant_backend::config::AppConfig;
use poly_quant_backend::db::Database;
use poly_quant_backend::features::{FeatureEngine, FEATURE_NAMES};
use poly_quant_backend::polymarket::PolymarketManager;
use poly_quant_backend::types::{Asset, Exchange, MarketSide, MarketTick, OrderBookLevel, TradeTick};
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

async fn setup_phase5() -> (
    axum::Router,
    Arc<CollectorManager>,
    Arc<PolymarketManager>,
    Arc<CompositePriceEngine>,
    Arc<FeatureEngine>,
    Arc<Database>,
) {
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
    let models = Arc::new(poly_quant_backend::models::ModelManager::new());
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

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        collector: collector.clone(),
        polymarket: polymarket.clone(),
        composite: composite.clone(),
        features: features.clone(),
        models,
        strategy,
        execution,
        start_time_ms: chrono::Utc::now().timestamp_millis(),
    };
    let app = create_router(state);
    (app, collector, polymarket, composite, features, db)
}

#[tokio::test]
async fn test_feature_vector_dimension_and_names() {
    let names = FEATURE_NAMES;
    assert_eq!(names.len(), 37);

    // Verify critical feature names are present
    assert!(names.contains(&"composite_price"));
    assert!(names.contains(&"return_5s"));
    assert!(names.contains(&"realized_vol_60s"));
    assert!(names.contains(&"velocity_5s"));
    assert!(names.contains(&"acceleration_5s_15s"));
    assert!(names.contains(&"distance_from_open"));
    assert!(names.contains(&"distance_percent"));
    assert!(names.contains(&"time_decay_factor"));
    assert!(names.contains(&"cvd_5s"));
    assert!(names.contains(&"trade_imbalance_5s"));
    assert!(names.contains(&"poly_obi_top5"));
    assert!(names.contains(&"poly_implied_prob"));
}

#[tokio::test]
async fn test_feature_calculation_and_components() {
    let (_app, collector, polymarket, _composite, features, _db) = setup_phase5().await;
    let now = chrono::Utc::now().timestamp_millis();

    // 1. Establish 5M market with open price 65000.0
    let market = polymarket
        .discovery()
        .ensure_active_market(Asset::BTC, now, Some(65000.0))
        .await
        .unwrap();

    // 2. Feed ticks to Binance & OKX
    collector.freshness_tracker().record_tick(Exchange::Binance, Asset::BTC, now - 10, now);
    collector.record_test_tick(MarketTick {
        exchange: Exchange::Binance,
        symbol: "BTCUSDT".to_string(),
        asset: Asset::BTC,
        exchange_timestamp_ms: now - 10,
        receive_timestamp_ms: now,
        latency_ms: 10,
        bid: 65199.0,
        ask: 65201.0,
        mid: 65200.0,
        last: 65200.0,
        volume_24h: 5000.0,
    });

    collector.freshness_tracker().record_tick(Exchange::Okx, Asset::BTC, now - 15, now);
    collector.record_test_tick(MarketTick {
        exchange: Exchange::Okx,
        symbol: "BTCUSDT".to_string(),
        asset: Asset::BTC,
        exchange_timestamp_ms: now - 15,
        receive_timestamp_ms: now,
        latency_ms: 15,
        bid: 65198.0,
        ask: 65202.0,
        mid: 65200.0,
        last: 65200.0,
        volume_24h: 3000.0,
    });

    // 3. Feed Polymarket OrderBook for the active market
    let up_bids = vec![
        OrderBookLevel { price: 0.52, size: 500.0 },
        OrderBookLevel { price: 0.51, size: 1000.0 },
    ];
    let up_asks = vec![
        OrderBookLevel { price: 0.54, size: 300.0 },
        OrderBookLevel { price: 0.55, size: 800.0 },
    ];
    let down_bids = vec![
        OrderBookLevel { price: 0.46, size: 400.0 },
    ];
    let down_asks = vec![
        OrderBookLevel { price: 0.48, size: 600.0 },
    ];
    polymarket.book_engine().update_book(
        &market.up_token_id,
        Asset::BTC,
        MarketSide::Up,
        up_bids,
        up_asks,
        now,
    );
    polymarket.book_engine().update_book(
        &market.down_token_id,
        Asset::BTC,
        MarketSide::Down,
        down_bids,
        down_asks,
        now,
    );

    // 4. Feed Trade Flow (Buy aggressive trade)
    features.flow_tracker().record_trade(&TradeTick {
        exchange: Exchange::Binance,
        symbol: "BTCUSDT".to_string(),
        asset: Asset::BTC,
        exchange_timestamp_ms: now - 500,
        receive_timestamp_ms: now - 500,
        latency_ms: 5,
        price: 65200.0,
        size: 5.5,
        side: "buy".to_string(),
        is_aggressive: true,
    });

    // 5. Calculate Features
    let snap = features
        .calculate_features(Asset::BTC, now)
        .expect("Features should be generated");

    assert_eq!(snap.asset, Asset::BTC);
    assert_eq!(snap.market_id, market.id);
    assert_eq!(snap.composite_price, 65200.0);

    // 5M open metrics
    assert_eq!(snap.distance_from_open, 200.0); // 65200 - 65000
    assert!((snap.distance_percent - (200.0 / 65000.0 * 100.0)).abs() < 1e-4);
    assert!(snap.remaining_seconds > 0.0);
    assert!(snap.time_decay_factor > 0.0 && snap.time_decay_factor <= 1.0);

    // Polymarket metrics
    assert!(snap.poly_obi_top5 > 0.0); // more bids (1500) than asks (1100)
    assert!((snap.poly_spread - 0.02).abs() < 1e-6); // 0.54 - 0.52
    assert!((snap.poly_implied_prob - 0.53).abs() < 1e-4); // mid of 0.52 and 0.54

    // Flow metrics
    assert_eq!(snap.cvd_5s, 5.5);
    assert_eq!(snap.trade_imbalance_5s, 1.0); // 100% buy

    // Vector export
    let vec = snap.to_vector();
    assert_eq!(vec.len(), 37);
    for val in &vec {
        assert!(!val.is_nan(), "Feature vector contains NaN!");
        assert!(!val.is_infinite(), "Feature vector contains Inf!");
    }
}

#[tokio::test]
async fn test_feature_api_endpoints() {
    let (app, collector, _polymarket, _composite, features, _db) = setup_phase5().await;
    let now = chrono::Utc::now().timestamp_millis();

    // 1. Check feature names endpoint
    let res_names = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/features/names")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_names.status(), StatusCode::OK);
    let bytes = res_names.into_body().collect().await.unwrap().to_bytes();
    let names: Vec<String> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(names.len(), 37);

    // 2. Feed tick and calculate snapshot
    collector.freshness_tracker().record_tick(Exchange::Binance, Asset::BTC, now, now);
    collector.record_test_tick(MarketTick {
        exchange: Exchange::Binance,
        symbol: "BTCUSDT".to_string(),
        asset: Asset::BTC,
        exchange_timestamp_ms: now - 5,
        receive_timestamp_ms: now,
        latency_ms: 5,
        bid: 65000.0,
        ask: 65001.0,
        mid: 65000.5,
        last: 65000.5,
        volume_24h: 1000.0,
    });

    let snap = features.calculate_features(Asset::BTC, now).unwrap();
    // Cache snapshot
    let _ = snap.to_json();

    // 3. Test get all features API
    let res_all = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/features/all")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_all.status(), StatusCode::OK);
}
