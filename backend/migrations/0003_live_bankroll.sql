-- =============================================================================
-- Migration 0003: Isolated Live Trading Bankroll & Risk Management Schema
-- =============================================================================

-- 1. Live Bankroll History (Dedicated ledger for real Polymarket capital)
CREATE TABLE IF NOT EXISTS live_bankroll_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    active_bankroll REAL NOT NULL,
    locked_profit REAL NOT NULL,
    total_equity REAL NOT NULL,
    change_amount REAL NOT NULL,
    reason TEXT NOT NULL,
    trade_id TEXT,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_live_bankroll_history_ts ON live_bankroll_history(timestamp);

-- 2. Live Risk & Circuit Breaker Configuration
CREATE TABLE IF NOT EXISTS live_risk_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    daily_loss_limit REAL NOT NULL DEFAULT 5.0,
    max_consecutive_losses INTEGER NOT NULL DEFAULT 3,
    max_drawdown REAL NOT NULL DEFAULT 0.20,
    cooldown_minutes INTEGER NOT NULL DEFAULT 15,
    minimum_bankroll REAL NOT NULL DEFAULT 0.50,
    updated_at INTEGER NOT NULL
);

INSERT OR IGNORE INTO live_risk_config (
    id, daily_loss_limit, max_consecutive_losses, max_drawdown, cooldown_minutes, minimum_bankroll, updated_at
) VALUES (1, 5.0, 3, 0.20, 15, 0.50, 0);
