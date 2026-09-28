use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use poly_quant_backend::api::{create_router, AppState};
use poly_quant_backend::collector::CollectorManager;
use poly_quant_backend::composite::CompositePriceEngine;
use poly_quant_backend::config::AppConfig;
use poly_quant_backend::dataset::{
    DatasetExporter, DatasetRecord, DatasetSummary, WalkForwardSplitter,
};
use poly_quant_backend::db::Database;
use poly_quant_backend::features::{FeatureEngine, FeatureSnapshot, FEATURE_NAMES};
use poly_quant_backend::polymarket::PolymarketManager;
use poly_quant_backend::types::{Asset, Resolution};
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use tempfile::{NamedTempFile, TempDir};
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

async fn setup_phase6() -> (axum::Router, Arc<Database>) {
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

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        collector,
        polymarket,
        composite,
        features,
        start_time_ms: chrono::Utc::now().timestamp_millis(),
    };
    let app = create_router(state);
    (app, db)
}

fn create_sample_snapshot(market_id: &str, timestamp_ms: i64) -> FeatureSnapshot {
    FeatureSnapshot {
        asset: Asset::BTC,
        market_id: market_id.to_string(),
        timestamp_ms,
        composite_price: 65000.0,
        return_1s: 0.0001,
        return_3s: 0.0002,
        return_5s: 0.0003,
        return_10s: 0.0005,
        return_30s: 0.0010,
        return_60s: 0.0015,
        realized_vol_5s: 0.0002,
        realized_vol_10s: 0.0003,
        realized_vol_30s: 0.0005,
        realized_vol_60s: 0.0008,
        velocity_5s: 0.5,
        velocity_15s: 0.3,
        acceleration_5s_15s: 0.02,
        distance_from_open: 25.0,
        distance_percent: 0.038,
        distance_to_vol_ratio: 0.48,
        remaining_seconds: 120.0,
        elapsed_seconds: 180.0,
        time_decay_factor: 0.632,
        spread_binance_okx: 0.2,
        spread_binance_bybit: -0.1,
        spread_binance_coinbase: 0.4,
        cvd_5s: 12.0,
        cvd_15s: 25.0,
        cvd_30s: 40.0,
        cvd_60s: 55.0,
        trade_imbalance_5s: 0.3,
        trade_imbalance_15s: 0.2,
        trade_imbalance_30s: 0.15,
        trade_imbalance_60s: 0.10,
        poly_obi_top5: 0.15,
        poly_obi_top10: 0.18,
        poly_obi_top20: 0.20,
        poly_spread: 0.02,
        poly_total_liquidity: 4500.0,
        poly_implied_prob: 0.54,
    }
}

#[tokio::test]
async fn test_database_load_and_csv_jsonl_export() {
    let (_app, db) = setup_phase6().await;
    let base_ts = 1710000000000i64;

    // 1. Populate database with 20 resolved markets and corresponding feature snapshots
    for i in 0..20 {
        let mkt_id = format!("BTC-5M-TEST-{}", i);
        let start_ts = base_ts + (i * 300_000);
        let end_ts = start_ts + 300_000;
        let res = if i % 2 == 0 { "UP" } else { "DOWN" };

        sqlx::query(
            r#"
            INSERT INTO markets (id, condition_id, asset, slug, question, start_time, end_time, open_price, final_price, status, resolution, created_at, updated_at)
            VALUES (?, ?, 'BTC', ?, 'Will BTC be Up or Down?', ?, ?, 65000.0, 65100.0, 'resolved', ?, ?, ?)
            "#,
        )
        .bind(&mkt_id)
        .bind(format!("0x{:x}", i))
        .bind(format!("btc-5m-{}", i))
        .bind(start_ts)
        .bind(end_ts)
        .bind(res)
        .bind(start_ts)
        .bind(end_ts)
        .execute(db.pool())
        .await
        .unwrap();

        // Insert feature snapshot at start_ts + 10_000
        let snap = create_sample_snapshot(&mkt_id, start_ts + 10_000);
        sqlx::query(
            r#"
            INSERT INTO features (market_id, timestamp, feature_name, feature_vector_json, created_at)
            VALUES (?, ?, 'snapshot_v1', ?, ?)
            "#,
        )
        .bind(&mkt_id)
        .bind(start_ts + 10_000)
        .bind(snap.to_json())
        .bind(start_ts + 10_000)
        .execute(db.pool())
        .await
        .unwrap();
    }

    // 2. Load dataset from DB
    let records = DatasetExporter::load_from_db(&db).await.unwrap();
    assert_eq!(records.len(), 20);
    assert_eq!(records[0].features.len(), FEATURE_NAMES.len());
    assert_eq!(records[0].resolution, Resolution::Up);
    assert_eq!(records[0].target_up, 1.0);
    assert_eq!(records[1].resolution, Resolution::Down);
    assert_eq!(records[1].target_up, 0.0);

    // 3. Export to CSV
    let tmp_dir = TempDir::new().unwrap();
    let csv_path = tmp_dir.path().join("dataset.csv");
    DatasetExporter::export_to_csv(&records, &csv_path).unwrap();

    assert!(csv_path.exists());
    let file = std::fs::File::open(&csv_path).unwrap();
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

    // 1 header + 20 rows = 21 lines
    assert_eq!(lines.len(), 21);
    assert!(lines[0].starts_with("timestamp_ms,market_id,asset,target_up,resolution"));
    assert!(lines[0].contains("poly_implied_prob"));

    // 4. Export to JSONL
    let jsonl_path = tmp_dir.path().join("dataset.jsonl");
    DatasetExporter::export_to_jsonl(&records, &jsonl_path).unwrap();

    assert!(jsonl_path.exists());
    let j_file = std::fs::File::open(&jsonl_path).unwrap();
    let j_reader = BufReader::new(j_file);
    let j_lines: Vec<String> = j_reader.lines().map(|l| l.unwrap()).collect();
    assert_eq!(j_lines.len(), 20);

    let parsed_record: DatasetRecord = serde_json::from_str(&j_lines[0]).unwrap();
    assert_eq!(parsed_record.market_id, "BTC-5M-TEST-0");
}

#[tokio::test]
async fn test_walk_forward_strict_no_leakage_and_metrics() {
    let mut records = Vec::new();
    for i in 0..100 {
        records.push(DatasetRecord {
            market_id: format!("mkt-{}", i),
            asset: Asset::BTC,
            timestamp_ms: 10000 + (i * 1000),
            features: vec![i as f64; 37],
            target_up: if i % 2 == 0 { 1.0 } else { 0.0 },
            resolution: if i % 2 == 0 { Resolution::Up } else { Resolution::Down },
        });
    }

    let splitter = WalkForwardSplitter::new(0.70, 0.15, 0.15);
    let split = splitter.split(records).unwrap();

    assert_eq!(split.train.len(), 70);
    assert_eq!(split.val.len(), 15);
    assert_eq!(split.test.len(), 15);

    // Assert strictly ordered and no overlapping boundaries
    assert!(split.train.last().unwrap().timestamp_ms <= split.val.first().unwrap().timestamp_ms);
    assert!(split.val.last().unwrap().timestamp_ms <= split.test.first().unwrap().timestamp_ms);

    assert_eq!(split.summary.total_samples, 100);
    assert_eq!(split.summary.up_count, 50);
    assert_eq!(split.summary.down_count, 50);
    assert_eq!(split.summary.up_ratio, 0.50);
    assert_eq!(split.summary.feature_dimension, 37);
    assert_eq!(split.summary.invalid_value_count, 0);
}

#[tokio::test]
async fn test_dataset_summary_api_endpoint() {
    let (app, db) = setup_phase6().await;

    // 1. Initial state: No samples
    let res_empty = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/dataset/summary")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res_empty.status(), StatusCode::OK);
    let bytes = res_empty.into_body().collect().await.unwrap().to_bytes();
    let summary: DatasetSummary = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(summary.total_samples, 0);

    // 2. Add resolved market & feature
    let mkt_id = "BTC-5M-API-TEST";
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        r#"
        INSERT INTO markets (id, condition_id, asset, slug, question, start_time, end_time, open_price, final_price, status, resolution, created_at, updated_at)
        VALUES (?, '0x1', 'BTC', 'btc-test', 'Question?', ?, ?, 65000.0, 65200.0, 'resolved', 'UP', ?, ?)
        "#,
    )
    .bind(mkt_id)
    .bind(now - 300_000)
    .bind(now)
    .bind(now - 300_000)
    .bind(now)
    .execute(db.pool())
    .await
    .unwrap();

    let snap = create_sample_snapshot(mkt_id, now - 150_000);
    sqlx::query(
        r#"
        INSERT INTO features (market_id, timestamp, feature_name, feature_vector_json, created_at)
        VALUES (?, ?, 'snapshot_v1', ?, ?)
        "#,
    )
    .bind(mkt_id)
    .bind(now - 150_000)
    .bind(snap.to_json())
    .bind(now - 150_000)
    .execute(db.pool())
    .await
    .unwrap();

    let res_populated = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/dataset/summary")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res_populated.status(), StatusCode::OK);
    let bytes = res_populated.into_body().collect().await.unwrap().to_bytes();
    let summary: DatasetSummary = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(summary.total_samples, 1);
    assert_eq!(summary.up_count, 1);
}
