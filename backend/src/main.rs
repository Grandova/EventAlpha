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

#[tokio::main]
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

    let start_time_ms = Utc::now().timestamp_millis();
    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        start_time_ms,
    };

    // 8. Bind HTTP API server
    let app = create_router(state);
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("HTTP REST & Health API listening on http://{}", addr);
    info!("Health check available at: http://{}/api/v1/health", addr);
    info!("Safety status available at: http://{}/api/v1/safety", addr);
    info!("Bankroll status available at: http://{}/api/v1/paper/bankroll", addr);

    // 9. Run server with graceful shutdown
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
}
