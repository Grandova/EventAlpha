use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use poly_quant_backend::api::{create_router, AppState};
use poly_quant_backend::collector::binance::parse_binance_message;
use poly_quant_backend::collector::bybit::parse_bybit_message;
use poly_quant_backend::collector::coinbase::parse_coinbase_message;
use poly_quant_backend::collector::freshness::{FreshnessReport, FreshnessTracker};
use poly_quant_backend::collector::okx::parse_okx_message;
use poly_quant_backend::collector::CollectorManager;
use poly_quant_backend::config::AppConfig;
use poly_quant_backend::db::Database;
use poly_quant_backend::types::{Asset, Exchange};
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

#[test]
fn test_cross_exchange_parsing_normalization() {
    let now = 1710000000100;

    // 1. Binance
    let binance_json = r#"{"stream":"btcusdt@bookTicker","data":{"s":"BTCUSDT","b":"65000.10","a":"65000.90"}}"#;
    let (b_tick, _) = parse_binance_message(binance_json, now);
    let bt = b_tick.expect("Binance tick");
    assert_eq!(bt.exchange, Exchange::Binance);
    assert_eq!(bt.asset, Asset::BTC);
    assert_eq!(bt.bid, 65000.10);
    assert_eq!(bt.ask, 65000.90);
    assert_eq!(bt.mid, 65000.50);

    // 2. OKX
    let okx_json = r#"{"arg":{"channel":"tickers","instId":"ETH-USDT"},"data":[{"bidPx":"3500.20","askPx":"3500.80","last":"3500.50","vol24h":"1000","ts":"1710000000050"}]}"#;
    let (o_tick, _) = parse_okx_message(okx_json, now);
    let ot = o_tick.expect("OKX tick");
    assert_eq!(ot.exchange, Exchange::Okx);
    assert_eq!(ot.asset, Asset::ETH);
    assert_eq!(ot.bid, 3500.20);
    assert_eq!(ot.ask, 3500.80);
    assert_eq!(ot.latency_ms, 50);

    // 3. Bybit
    let bybit_json = r#"{"topic":"tickers.SOLUSDT","ts":1710000000060,"data":{"symbol":"SOLUSDT","bid1Price":"145.20","ask1Price":"145.40","lastPrice":"145.30","volume24h":"500"}}"#;
    let (by_tick, _) = parse_bybit_message(bybit_json, now);
    let byt = by_tick.expect("Bybit tick");
    assert_eq!(byt.exchange, Exchange::Bybit);
    assert_eq!(byt.asset, Asset::SOL);
    assert_eq!(byt.bid, 145.20);
    assert_eq!(byt.ask, 145.40);
    assert_eq!(byt.latency_ms, 40);

    // 4. Coinbase
    let cb_json = r#"{"type":"ticker","product_id":"BTC-USD","best_bid":"65001.00","best_ask":"65002.00","price":"65001.50","volume_24h":"800","time":"2024-03-09T17:46:40.080000Z"}"#;
    let (cb_tick, _) = parse_coinbase_message(cb_json, now);
    let cbt = cb_tick.expect("Coinbase tick");
    assert_eq!(cbt.exchange, Exchange::Coinbase);
    assert_eq!(cbt.asset, Asset::BTC);
    assert_eq!(cbt.bid, 65001.00);
    assert_eq!(cbt.ask, 65002.00);
}

#[tokio::test]
async fn test_collector_status_and_prices_api() {
    let mut tmp = NamedTempFile::new().unwrap();
    use std::io::Write;
    tmp.write_all(test_yaml().as_bytes()).unwrap();

    let config = Arc::new(AppConfig::load_from_path(tmp.path()).unwrap());
    let db = Arc::new(Database::new(&config).await.unwrap());
    let collector = Arc::new(CollectorManager::new(&config));
    let polymarket = Arc::new(poly_quant_backend::polymarket::PolymarketManager::new(&config, db.clone()));
    let composite = Arc::new(poly_quant_backend::composite::CompositePriceEngine::new(
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
    let replay = Arc::new(poly_quant_backend::replay::ReplayEngine::new(
        db.clone(),
        models.clone(),
        strategy.clone(),
    ));

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        collector: collector.clone(),
        polymarket,
        composite,
        features,
        models: models.clone(),
        strategy,
        execution,
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

    // Query status API
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/collector/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let report: FreshnessReport = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(report.exchanges.len(), 4);

    // Query prices API
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/collector/prices")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn test_freshness_tracker_timeout_and_fail_closed() {
    let tracker = FreshnessTracker::new(2000); // 2000ms threshold
    let now = chrono::Utc::now().timestamp_millis();

    // 1. Initial state: No ticks => Stale
    assert!(tracker.is_stale(Exchange::Binance, Asset::BTC));
    assert!(!tracker.get_report().is_system_fresh);

    // 2. Fresh tick arrives
    tracker.record_tick(Exchange::Binance, Asset::BTC, now - 100, now);
    assert!(!tracker.is_stale(Exchange::Binance, Asset::BTC));

    // 3. Stale tick (3000ms ago)
    let stale_time = now - 3000;
    tracker.record_tick(Exchange::Okx, Asset::ETH, stale_time - 100, stale_time);
    assert!(tracker.is_stale(Exchange::Okx, Asset::ETH));
}
