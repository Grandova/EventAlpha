use std::sync::Arc;
use tempfile::NamedTempFile;

use poly_quant_backend::config::AppConfig;
use poly_quant_backend::db::Database;
use poly_quant_backend::execution::LiveExecutionEngine;
use poly_quant_backend::features::FeatureSnapshot;
use poly_quant_backend::models::SelfLearningEngine;
use poly_quant_backend::polymarket::clob_client::PolymarketClobHttpClient;
use poly_quant_backend::types::{Asset, PolymarketAccount, Resolution, TradingMode};

fn test_config() -> AppConfig {
    let yaml = r#"
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
"#;
    let mut file = NamedTempFile::new().unwrap();
    use std::io::Write;
    file.write_all(yaml.as_bytes()).unwrap();
    AppConfig::load_from_path(file.path()).unwrap()
}

#[tokio::test]
async fn test_polymarket_account_management_flow() {
    let config = test_config();
    let db = Arc::new(Database::new(&config).await.unwrap());

    // 1. Initial accounts should be empty
    let initial_accs = db.get_accounts().await.unwrap();
    assert!(initial_accs.is_empty());

    // 2. Insert account
    let acc1 = PolymarketAccount {
        id: "acc-1".to_string(),
        label: "Main Safe".to_string(),
        api_key: "api-key-12345678".to_string(),
        api_secret: "secret-base64-encoded".to_string(),
        api_passphrase: "passphrase-secret".to_string(),
        wallet_address: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
        proxy_wallet_address: None,
        is_active: true,
        balance_usdc: 150.50,
        created_at: 1000,
        updated_at: 1000,
    };
    db.insert_account(&acc1).await.unwrap();

    let fetched = db.get_accounts().await.unwrap();
    assert_eq!(fetched.len(), 1);
    assert_eq!(fetched[0].label, "Main Safe");

    // 3. Active account query
    let active = db.get_active_account().await.unwrap();
    assert!(active.is_some());
    assert_eq!(active.unwrap().id, "acc-1");

    // 4. Update balance
    db.update_account_balance("acc-1", 200.75).await.unwrap();
    let updated = db.get_active_account().await.unwrap().unwrap();
    assert_eq!(updated.balance_usdc, 200.75);

    // 5. Deactivate / Delete
    db.delete_account("acc-1").await.unwrap();
    assert!(db.get_active_account().await.unwrap().is_none());
}

#[tokio::test]
async fn test_live_execution_mode_switching_guard() {
    let config = test_config();
    let db = Arc::new(Database::new(&config).await.unwrap());
    let clob = Arc::new(PolymarketClobHttpClient::default());
    let engine = LiveExecutionEngine::new(db.clone(), clob);

    // Initial mode is Paper
    assert_eq!(engine.get_mode().await, TradingMode::Paper);

    // Trying to switch to Live mode without an account must fail
    let err = engine.set_mode(TradingMode::Live).await;
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("No active Polymarket account found"));

    // Add an active account
    let acc = PolymarketAccount {
        id: "acc-polygon".to_string(),
        label: "Live Test Account".to_string(),
        api_key: "api-test".to_string(),
        api_secret: "secret-test".to_string(),
        api_passphrase: "pass-test".to_string(),
        wallet_address: "0x0000000000000000000000000000000000000001".to_string(),
        proxy_wallet_address: None,
        is_active: true,
        balance_usdc: 50.0,
        created_at: 1000,
        updated_at: 1000,
    };
    db.insert_account(&acc).await.unwrap();

    // Now switching to Live mode succeeds!
    let res = engine.set_mode(TradingMode::Live).await;
    assert!(res.is_ok());
    assert_eq!(engine.get_mode().await, TradingMode::Live);

    // Emergency halt immediately forces back to Paper mode
    engine.emergency_halt().await.unwrap();
    assert_eq!(engine.get_mode().await, TradingMode::Paper);
}

#[tokio::test]
async fn test_continuous_self_learning_evolution() {
    let config = test_config();
    let db = Arc::new(Database::new(&config).await.unwrap());
    let models = Arc::new(poly_quant_backend::models::ModelManager::new());
    let learner = SelfLearningEngine::new(models.clone(), db.clone());

    // 1. Initial learning status
    let initial_status = learner.get_status(Asset::BTC).await;
    assert!(initial_status.auto_learning_enabled);
    assert_eq!(initial_status.total_samples_learned, 0);

    // 2. Simulate 5-minute round settlement with ground truth
    let mut snap = FeatureSnapshot::new(Asset::BTC, "BTC_5M_ROUND_1", 1710000000000);
    snap.return_5s = 0.005;
    snap.distance_percent = 0.003;
    snap.poly_obi_top5 = 0.45;
    snap.trade_imbalance_5s = 0.60;

    // Ground truth: Market resolved UP
    let step1 = learner
        .on_round_resolved(Asset::BTC, "BTC_5M_ROUND_1", Resolution::Up, Some(&snap))
        .await;

    assert!(step1.is_some());
    let entry = step1.unwrap();
    assert_eq!(entry.round_id, "BTC_5M_ROUND_1");
    assert_eq!(entry.actual_outcome, 1);
    assert!(entry.weights_delta_norm > 0.0);

    // Check status after 1 sample
    let status_after = learner.get_status(Asset::BTC).await;
    assert_eq!(status_after.total_samples_learned, 1);

    // 3. Test persistence: learning state is saved in SQLite and can be restored!
    let restored_learner = SelfLearningEngine::new(models.clone(), db.clone());
    let did_restore = restored_learner.load_persisted_state(Asset::BTC).await;
    assert!(did_restore);

    let restored_status = restored_learner.get_status(Asset::BTC).await;
    assert_eq!(restored_status.total_samples_learned, 1);

    // Check learning history query
    let history = db.get_learning_history(Some(Asset::BTC), 10).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].round_id, "BTC_5M_ROUND_1");
}
