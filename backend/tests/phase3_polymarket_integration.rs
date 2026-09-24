use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use poly_quant_backend::api::{create_router, AppState, MarketDisplayInfo};
use poly_quant_backend::collector::CollectorManager;
use poly_quant_backend::config::AppConfig;
use poly_quant_backend::db::Database;
use poly_quant_backend::polymarket::orderbook::MarketBookSummary;
use poly_quant_backend::polymarket::PolymarketManager;
use poly_quant_backend::types::{Asset, MarketSide, OrderBookLevel, Resolution};
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

async fn setup_phase3_app() -> (axum::Router, Arc<PolymarketManager>, Arc<Database>) {
    let mut tmp = NamedTempFile::new().unwrap();
    use std::io::Write;
    tmp.write_all(test_yaml().as_bytes()).unwrap();

    let config = Arc::new(AppConfig::load_from_path(tmp.path()).unwrap());
    let db = Arc::new(Database::new(&config).await.unwrap());
    let collector = Arc::new(CollectorManager::new(&config));
    let polymarket = Arc::new(PolymarketManager::new(&config, db.clone()));

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        collector,
        polymarket: polymarket.clone(),
        start_time_ms: chrono::Utc::now().timestamp_millis(),
    };
    let app = create_router(state);
    (app, polymarket, db)
}

#[tokio::test]
async fn test_phase3_market_discovery_and_idempotency() {
    let (_app, polymarket, _db) = setup_phase3_app().await;
    let now_ms = 1710000100000i64;

    // 1. Discover/Ensure active BTC market
    let market1 = polymarket
        .discovery()
        .ensure_active_market(Asset::BTC, now_ms, Some(65000.0))
        .await
        .unwrap();

    assert_eq!(market1.asset, Asset::BTC);
    assert_eq!(market1.open_price, Some(65000.0));
    assert!(market1.remaining_seconds(now_ms) > 0);

    // 2. Calling again in the same window returns the exact same market
    let market2 = polymarket
        .discovery()
        .ensure_active_market(Asset::BTC, now_ms + 5000, Some(65100.0))
        .await
        .unwrap();

    assert_eq!(market1.id, market2.id);
    assert_eq!(market2.open_price, Some(65000.0)); // original open price retained
}

#[tokio::test]
async fn test_phase3_section21_orderbook_fill_simulation() {
    let (_app, polymarket, _db) = setup_phase3_app().await;

    // Seed test orderbook for UP token:
    // Ask 1: 0.60 -> capacity 0.60 USDC (1.0 share)
    // Ask 2: 0.65 -> capacity 1.30 USDC (2.0 shares)
    let bids = vec![OrderBookLevel { price: 0.58, size: 5.0 }];
    let asks = vec![
        OrderBookLevel { price: 0.60, size: 1.0 },
        OrderBookLevel { price: 0.65, size: 2.0 },
    ];

    polymarket.book_engine().update_book(
        "BTC_5M_UP",
        Asset::BTC,
        MarketSide::Up,
        bids,
        asks,
        1710000000,
    );

    // Simulate buying 1.0 USDC of UP tokens:
    // - Consumes 0.60 USDC @ 0.60 = 1.0 share
    // - Remaining 0.40 USDC @ 0.65 = 0.61538 shares
    // Total shares = 1.61538 shares
    // Average Fill Price = 1.0 / 1.61538 = 0.619047 USDC
    // Best Ask was 0.60 => Slippage = 0.619047 - 0.60 = 0.019047 USDC
    let fill = polymarket
        .book_engine()
        .simulate_buy_fill(Asset::BTC, MarketSide::Up, 1.0)
        .expect("Should fill across depth");

    assert_eq!(fill.levels_consumed, 2);
    assert_eq!(fill.quote_price, 0.60);
    assert!((fill.avg_fill_price - 0.619).abs() < 0.005);
    assert!((fill.slippage - 0.019).abs() < 0.005);
    assert!((fill.total_shares - 1.615).abs() < 0.005);
}

#[tokio::test]
async fn test_phase3_resolution_and_settlement_flow() {
    let (_app, polymarket, _db) = setup_phase3_app().await;
    let now_ms = 1710000305000i64;

    // Settle market where Final > Open => UP wins
    let res = polymarket
        .resolution()
        .resolve_market("BTC-5M-TEST-1", Asset::BTC, 65000.0, 65200.0, now_ms)
        .await
        .unwrap();

    assert_eq!(res.resolution, Resolution::Up);
    assert_eq!(res.price_change, 200.0);
    assert!(res.price_change_pct > 0.0);

    // Settle market where Final < Open => DOWN wins
    let res_down = polymarket
        .resolution()
        .resolve_market("ETH-5M-TEST-1", Asset::ETH, 3500.0, 3480.0, now_ms)
        .await
        .unwrap();

    assert_eq!(res_down.resolution, Resolution::Down);
    assert_eq!(res_down.price_change, -20.0);

    // Query resolutions list
    let history = polymarket
        .resolution()
        .get_recently_resolved(10)
        .await
        .unwrap();
    assert_eq!(history.len(), 2);
}

#[tokio::test]
async fn test_phase3_polymarket_rest_api_endpoints() {
    let (app, polymarket, _db) = setup_phase3_app().await;
    let now_ms = chrono::Utc::now().timestamp_millis();

    // Seed an active market and orderbook
    polymarket
        .discovery()
        .ensure_active_market(Asset::BTC, now_ms, Some(65000.0))
        .await
        .unwrap();
    polymarket.book_engine().seed_fallback_ladder(Asset::BTC, 0.65, now_ms);

    // 1. GET /api/v1/polymarket/markets
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/polymarket/markets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let markets: Vec<MarketDisplayInfo> = serde_json::from_slice(&bytes).unwrap();
    assert!(!markets.is_empty());
    assert_eq!(markets[0].market.asset, Asset::BTC);

    // 2. GET /api/v1/polymarket/book/BTC
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/polymarket/book/BTC")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let book_summary: MarketBookSummary = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(book_summary.asset, Asset::BTC);
    assert!(book_summary.implied_prob_up > 0.0);
    assert!(book_summary.implied_prob_down > 0.0);

    // 3. GET /api/v1/polymarket/resolutions
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/polymarket/resolutions?limit=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}
