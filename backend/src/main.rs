use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use poly_quant_backend::api::{create_router, AppState};
use poly_quant_backend::config::AppConfig;
use poly_quant_backend::db::Database;
use poly_quant_backend::safety::SafetyGuard;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    // 1. Initialize environment variables from .env if present
    dotenvy::dotenv().ok();

    // 2. Initialize tracing and structured logging
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,poly_quant_backend=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("============================================================");
    info!("   Polymarket 5-Minute Crypto Quant Simulation System       ");
    info!("   Mode: PAPER TRADING ONLY | Zero Real Money Risk          ");
    info!("============================================================");

    // 3. Load and validate configuration
    let config = Arc::new(
        AppConfig::load_default()
            .map_err(|e| {
                error!("Fatal configuration error: {:?}", e);
                e
            })?,
    );

    // 4. Kernel-level Safety Guard check
    SafetyGuard::enforce_paper_only(config.safety.real_trading_enabled)
        .expect("Safety guard check failed!");

    // 5. Initialize database connection pool & run initial migrations
    let db = Arc::new(Database::new(&config).await.map_err(|e| {
        error!("Fatal database initialization error: {:?}", e);
        e
    })?);

    // 6. Record startup event in the audit ledger
    let start_payload = serde_json::json!({
        "mode": config.mode,
        "strategy_version": config.strategy.strategy_version,
        "bankroll_initial": config.bankroll.initial,
        "bankroll_cap": config.bankroll.cap,
        "bankroll_mode": config.bankroll.mode,
        "real_trading_enabled": false
    });

    db.record_system_event(
        "SYSTEM_STARTUP",
        "INFO",
        "KERNEL",
        "PolyQuant backend booted in safe simulation mode",
        Some(&start_payload.to_string()),
    )
    .await?;

    // 7. Initialize or verify Bankroll
    let bankroll = db.get_or_init_bankroll(&config).await?;
    info!(
        "Active Bankroll: {:.2} USDC | Cap: {:.2} USDC | Locked Profit: {:.2} USDC | Mode: {:?}",
        bankroll.active_bankroll, bankroll.bankroll_cap, bankroll.locked_profit, bankroll.mode
    );

    // 8. Initialize and launch multi-exchange live market collectors
    let collector = Arc::new(poly_quant_backend::collector::CollectorManager::new(&config));
    collector.start(db.clone());

    // 9. Initialize and launch Polymarket 5-minute lifecycle & book engine
    let polymarket = Arc::new(poly_quant_backend::polymarket::PolymarketManager::new(&config, db.clone()));
    polymarket.start(collector.clone());

    // 10. Initialize and launch Composite Price Engine
    let composite = Arc::new(poly_quant_backend::composite::CompositePriceEngine::new(
        collector.clone(),
        polymarket.discovery().clone(),
    ));
    composite.start();

    // 11. Initialize and launch Real-Time Feature Engineering Engine
    let features = Arc::new(poly_quant_backend::features::FeatureEngine::new(
        composite.clone(),
        polymarket.clone(),
        collector.clone(),
        db.clone(),
    ));
    features.start();

    // 12. Initialize and launch ML Prediction & Probability Calibration Engine
    let models = Arc::new(poly_quant_backend::models::ModelManager::new());
    models.start(features.subscribe(), db.clone());

    // 13. Initialize and launch Strategy Engine & Opportunity Filtering
    let strategy = Arc::new(poly_quant_backend::strategy::StrategyEngine::new(
        config.strategy.clone(),
        config.execution.clone(),
        db.clone(),
    ));
    strategy.start(
        models.subscribe(),
        polymarket.book_engine(),
        composite.clone(),
        collector.freshness(),
    );

    // 14. Initialize Risk & Bankroll Manager
    let risk = Arc::new(poly_quant_backend::risk::RiskManager::new(
        config.bankroll.clone(),
        config.risk.clone(),
        bankroll,
        db.clone(),
    ));

    // 15. Initialize and launch Realistic Paper Execution Engine
    let execution = Arc::new(poly_quant_backend::execution::PaperExecutionEngine::new(
        config.execution.clone(),
        config.position.clone(),
        db.clone(),
        polymarket.book_engine(),
    ));
    execution.set_risk_manager(risk.clone()).await;
    execution.load_active_positions_from_db().await;
    execution.start(strategy.subscribe_signals());
    execution.start_resolution_listener(polymarket.subscribe_resolutions());
    execution.start_position_expiry_audit();

    // 16. Initialize Polymarket CLOB Client and Live Execution Engine
    let clob_http = Arc::new(poly_quant_backend::polymarket::PolymarketClobHttpClient::default());
    let live_execution = Arc::new(poly_quant_backend::execution::LiveExecutionEngine::new(
        db.clone(),
        clob_http.clone(),
    ));

    // Listen to trading signals for live execution (if live mode is activated)
    let live_exec_clone = live_execution.clone();
    let mut live_sig_rx = strategy.subscribe_signals();
    tokio::spawn(async move {
        while let Ok(sig) = live_sig_rx.recv().await {
            if sig.action != poly_quant_backend::types::SignalAction::Skip {
                let _ = live_exec_clone.execute_signal(&sig).await;
            }
        }
    });

    // 17. Initialize Continuous Self-Learning Engine
    let self_learning = Arc::new(poly_quant_backend::models::SelfLearningEngine::new(
        models.clone(),
        db.clone(),
    ));

    // Restore prior learned knowledge from SQLite across system restarts
    self_learning.load_persisted_state(poly_quant_backend::types::Asset::BTC).await;
    self_learning.load_persisted_state(poly_quant_backend::types::Asset::ETH).await;
    self_learning.load_persisted_state(poly_quant_backend::types::Asset::SOL).await;

    // Hook market resolution events to continuous self-learning online SGD updates
    let self_learning_clone = self_learning.clone();
    let features_clone = features.clone();
    let mut resolution_rx = polymarket.subscribe_resolutions();
    tokio::spawn(async move {
        while let Ok(res) = resolution_rx.recv().await {
            let snap = features_clone.get_latest_features(res.asset);
            self_learning_clone.on_round_resolved(
                res.asset,
                &res.market_id,
                res.resolution,
                snap.as_ref(),
            ).await;
        }
    });

    // 18. Initialize High-Performance Backtest Engine
    let backtest = Arc::new(poly_quant_backend::backtest::BacktestEngine::new(
        db.clone(),
        models.clone(),
    ));

    // 19. Initialize Historical Replay Engine
    let replay = Arc::new(poly_quant_backend::replay::ReplayEngine::new(
        db.clone(),
        models.clone(),
        strategy.clone(),
    ));

    let start_time_ms = Utc::now().timestamp_millis();
    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        collector: collector.clone(),
        polymarket: polymarket.clone(),
        composite: composite.clone(),
        features: features.clone(),
        models: models.clone(),
        strategy: strategy.clone(),
        execution: execution.clone(),
        live_execution: live_execution.clone(),
        self_learning: self_learning.clone(),
        risk: risk.clone(),
        backtest,
        replay,
        sessions: Arc::new(dashmap::DashMap::new()),
        start_time_ms,
    };

    // 16. Bind HTTP API server
    let app = create_router(state);
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("HTTP REST & Health API listening on http://{}", addr);
    info!("Health check available at: http://{}/api/v1/health", addr);
    info!("Safety status available at: http://{}/api/v1/safety", addr);
    info!("Bankroll status available at: http://{}/api/v1/paper/bankroll", addr);
    info!("Exchange status available at: http://{}/api/v1/collector/status", addr);
    info!("Live cross-exchange prices: http://{}/api/v1/collector/prices", addr);
    info!("Polymarket 5M markets: http://{}/api/v1/polymarket/markets", addr);
    info!("Polymarket Orderbook (BTC): http://{}/api/v1/polymarket/book/BTC", addr);
    info!("Composite Price (BTC): http://{}/api/v1/composite/price/BTC", addr);
    info!("All Composite Prices: http://{}/api/v1/composite/all", addr);
    info!("Polymarket Resolutions: http://{}/api/v1/polymarket/resolutions", addr);
    info!("Real-time Features (BTC): http://{}/api/v1/features/latest/BTC", addr);
    info!("All Real-time Features: http://{}/api/v1/features/all", addr);
    info!("Feature Names (37 dims): http://{}/api/v1/features/names", addr);
    info!("Dataset Summary: http://{}/api/v1/dataset/summary", addr);
    info!("Model Predictions (BTC): http://{}/api/v1/models/prediction/BTC", addr);
    info!("All Model Predictions: http://{}/api/v1/models/all", addr);
    info!("Strategy Signal (BTC): http://{}/api/v1/strategy/signals/latest/BTC", addr);
    info!("All Strategy Signals: http://{}/api/v1/strategy/signals/latest", addr);
    info!("Recent Decisions: http://{}/api/v1/strategy/decisions/recent", addr);
    info!("Active Paper Positions: http://{}/api/v1/paper/positions/active", addr);
    info!("Recent Paper Orders: http://{}/api/v1/paper/orders", addr);
    info!("Position History: http://{}/api/v1/paper/positions/history", addr);
    info!("Bankroll Ledger History: http://{}/api/v1/paper/bankroll/history", addr);
    info!("Risk Control Status: http://{}/api/v1/risk/status", addr);

    // 15. Run server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(db))
        .await?;

    info!("PolyQuant service shut down cleanly.");
    Ok(())
}

async fn shutdown_signal(db: Arc<Database>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, initiating graceful shutdown...");
        },
        _ = terminate => {
            info!("Received termination signal, initiating graceful shutdown...");
        },
    }

    if let Err(e) = db
        .record_system_event(
            "SYSTEM_SHUTDOWN",
            "INFO",
            "KERNEL",
            "PolyQuant backend initiating clean shutdown",
            None,
        )
        .await
    {
        error!("Failed to record shutdown event: {:?}", e);
    }

    // Flush SQLite WAL buffer completely into main database file
    info!("Flushing SQLite WAL checkpoint to disk...");
    let _ = sqlx::raw_sql("PRAGMA wal_checkpoint(TRUNCATE);")
        .execute(db.pool())
        .await;
}
