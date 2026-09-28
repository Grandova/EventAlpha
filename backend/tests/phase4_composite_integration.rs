use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use poly_quant_backend::api::{create_router, AppState};
use poly_quant_backend::collector::CollectorManager;
use poly_quant_backend::composite::{CompositePriceEngine, CompositePriceSnapshot};
use poly_quant_backend::config::AppConfig;
use poly_quant_backend::db::Database;
use poly_quant_backend::polymarket::PolymarketManager;
use poly_quant_backend::types::{Asset, Exchange, MarketTick};
use std::collections::VecDeque;
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

async fn setup_phase4() -> (
    axum::Router,
    Arc<CollectorManager>,
    Arc<PolymarketManager>,
    Arc<CompositePriceEngine>,
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
    let features = Arc::new(poly_quant_backend::features::FeatureEngine::new(
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

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        collector: collector.clone(),
        polymarket: polymarket.clone(),
        composite: composite.clone(),
        features,
        models,
        strategy,
        execution,
        risk,
        backtest,
        start_time_ms: chrono::Utc::now().timestamp_millis(),
    };
    let app = create_router(state);
    (app, collector, polymarket, composite)
}

#[tokio::test]
async fn test_composite_weighting_and_stale_exclusion() {
    let (_app, collector, _polymarket, composite) = setup_phase4().await;
    let now = chrono::Utc::now().timestamp_millis();

    // 1. Initially no ticks -> Fail closed (returns None)
    assert!(composite.calculate_snapshot(Asset::BTC, now).is_none());

    // 2. Feed ticks for all 4 exchanges:
    // Binance (40%): mid = 65000.0
    // OKX (25%): mid = 65010.0
    // Coinbase (25%): mid = 65020.0
    // Bybit (10%): mid = 65000.0
    collector.freshness_tracker().record_tick(Exchange::Binance, Asset::BTC, now - 10, now);
    collector.freshness_tracker().record_tick(Exchange::Okx, Asset::BTC, now - 20, now);
    collector.freshness_tracker().record_tick(Exchange::Coinbase, Asset::BTC, now - 30, now);
    collector.freshness_tracker().record_tick(Exchange::Bybit, Asset::BTC, now - 40, now);

    let mk_tick = |exchange, mid, latency| MarketTick {
        exchange,
        symbol: "BTCUSDT".to_string(),
        asset: Asset::BTC,
        exchange_timestamp_ms: now - latency,
        receive_timestamp_ms: now,
        latency_ms: latency,
        bid: mid - 0.5,
        ask: mid + 0.5,
        mid,
        last: mid,
        volume_24h: 1000.0,
    };

    collector.record_test_tick(mk_tick(Exchange::Binance, 65000.0, 10));
    collector.record_test_tick(mk_tick(Exchange::Okx, 65010.0, 20));
    collector.record_test_tick(mk_tick(Exchange::Coinbase, 65020.0, 30));
    collector.record_test_tick(mk_tick(Exchange::Bybit, 65000.0, 40));

    let snap = composite
        .calculate_snapshot(Asset::BTC, now)
        .expect("Snapshot should be computed");

    // Expected price: 65000 * 0.40 + 65010 * 0.25 + 65020 * 0.25 + 65000 * 0.10
    // = 26000 + 16252.5 + 16255 + 6500 = 65007.5
    assert!((snap.composite_price - 65007.5).abs() < 1e-4);
    assert_eq!(snap.active_exchanges_count, 4);
    assert_eq!(snap.leading_exchange, Some(Exchange::Binance)); // min latency = 10ms

    // Check cross-exchange spreads
    assert_eq!(snap.spread_binance_okx, Some(65000.0 - 65010.0)); // -10.0
    assert_eq!(snap.spread_binance_coinbase, Some(65000.0 - 65020.0)); // -20.0
    assert_eq!(snap.spread_binance_bybit, Some(65000.0 - 65000.0)); // 0.0

    // 3. Mark Bybit and Coinbase as stale (>2000ms ago)
    let stale_time = now - 5000;
    collector.freshness_tracker().record_tick(Exchange::Coinbase, Asset::BTC, stale_time - 100, stale_time);
    collector.freshness_tracker().record_tick(Exchange::Bybit, Asset::BTC, stale_time - 100, stale_time);

    let snap_stale = composite
        .calculate_snapshot(Asset::BTC, now)
        .expect("Snapshot with 2 fresh exchanges");

    assert_eq!(snap_stale.active_exchanges_count, 2);
    // Binance (base 0.40) + OKX (base 0.25), total weight = 0.65
    // Normalized: Binance = 0.40 / 0.65, OKX = 0.25 / 0.65
    let expected_reweighted = 65000.0 * (0.40 / 0.65) + 65010.0 * (0.25 / 0.65);
    assert!((snap_stale.composite_price - expected_reweighted).abs() < 1e-4);
}

#[tokio::test]
async fn test_distance_from_open_and_volatility() {
    let (_app, collector, polymarket, composite) = setup_phase4().await;
    let now = chrono::Utc::now().timestamp_millis();

    // Initialize 5M market with open price 64000.0
    polymarket
        .discovery()
        .ensure_active_market(Asset::BTC, now, Some(64000.0))
        .await
        .unwrap();

    // Feed current price at 64500.0
    collector.freshness_tracker().record_tick(Exchange::Binance, Asset::BTC, now, now);
    collector.record_test_tick(MarketTick {
        exchange: Exchange::Binance,
        symbol: "BTCUSDT".to_string(),
        asset: Asset::BTC,
        exchange_timestamp_ms: now - 15,
        receive_timestamp_ms: now,
        latency_ms: 15,
        bid: 64499.0,
        ask: 64501.0,
        mid: 64500.0,
        last: 64500.0,
        volume_24h: 1000.0,
    });

    let snap = composite.calculate_snapshot(Asset::BTC, now).unwrap();
    assert_eq!(snap.open_price, Some(64000.0));
    assert_eq!(snap.distance_from_open, Some(500.0)); // 64500 - 64000
    // distance_percent = (500 / 64000) * 100 = 0.78125%
    assert!((snap.distance_percent.unwrap() - 0.78125).abs() < 1e-4);
}

#[tokio::test]
async fn test_math_returns_and_realized_vol() {
    let mut history = VecDeque::new();
    let now = 100000i64;

    // Simulate steady price climb over 60 seconds
    for s in 0..=60 {
        let ts = now - (60 - s) * 1000;
        let price = 100.0 + (s as f64 * 0.1); // from 100.0 to 106.0
        history.push_back((ts, price));
    }

    let ret_1s = CompositePriceEngine::calc_return(&history, now, 1.0);
    let ret_10s = CompositePriceEngine::calc_return(&history, now, 10.0);
    let ret_60s = CompositePriceEngine::calc_return(&history, now, 60.0);

    assert!(ret_1s > 0.0);
    assert!(ret_10s > ret_1s);
    assert!(ret_60s > ret_10s);
    // 60s return: (106 - 100) / 100 = 0.06
    assert!((ret_60s - 0.06).abs() < 1e-3);

    let vol_10s = CompositePriceEngine::calc_realized_vol(&history, now, 10.0);
    let vol_60s = CompositePriceEngine::calc_realized_vol(&history, now, 60.0);
    assert!(vol_10s >= 0.0);
    assert!(vol_60s >= 0.0);
}

#[tokio::test]
async fn test_composite_api_endpoints() {
    let (app, collector, _polymarket, composite) = setup_phase4().await;
    let now = chrono::Utc::now().timestamp_millis();

    collector.freshness_tracker().record_tick(Exchange::Binance, Asset::BTC, now, now);
    collector.record_test_tick(MarketTick {
        exchange: Exchange::Binance,
        symbol: "BTCUSDT".to_string(),
        asset: Asset::BTC,
        exchange_timestamp_ms: now - 10,
        receive_timestamp_ms: now,
        latency_ms: 10,
        bid: 65000.0,
        ask: 65001.0,
        mid: 65000.5,
        last: 65000.5,
        volume_24h: 1000.0,
    });

    // Compute snapshot & record it
    let snap = composite.calculate_snapshot(Asset::BTC, now).unwrap();
    composite.record_snapshot(snap);

    // 1. Request when empty returns 404
    let res_empty = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/composite/price/ETH")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_empty.status(), StatusCode::NOT_FOUND);

    // 2. Request BTC snapshot returns 200 OK
    let res_btc = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/composite/price/BTC")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_btc.status(), StatusCode::OK);
    let bytes = res_btc.into_body().collect().await.unwrap().to_bytes();
    let btc_snap: CompositePriceSnapshot = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(btc_snap.asset, Asset::BTC);
    assert_eq!(btc_snap.composite_price, 65000.5);

    // 3. Request all snapshots returns map containing BTC
    let res_all = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/composite/all")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_all.status(), StatusCode::OK);
    let bytes = res_all.into_body().collect().await.unwrap().to_bytes();
    let all_snaps: std::collections::HashMap<String, CompositePriceSnapshot> =
        serde_json::from_slice(&bytes).unwrap();
    assert!(all_snaps.contains_key("BTC"));
}
