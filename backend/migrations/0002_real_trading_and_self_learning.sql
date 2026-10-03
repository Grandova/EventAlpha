-- =============================================================================
-- Migration 0002: Real Trading (Polymarket CLOB) & Continuous Self-Learning
-- =============================================================================

-- 1. Polymarket Accounts
CREATE TABLE IF NOT EXISTS polymarket_accounts (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    api_key TEXT NOT NULL,
    api_secret TEXT NOT NULL,
    api_passphrase TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    proxy_wallet_address TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    balance_usdc REAL NOT NULL DEFAULT 0.0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_accounts_active ON polymarket_accounts(is_active);

-- 2. Real CLOB Orders
CREATE TABLE IF NOT EXISTS real_orders (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    clob_order_id TEXT,
    market_id TEXT NOT NULL,
    token_id TEXT NOT NULL,
    asset TEXT NOT NULL,
    side TEXT NOT NULL,             -- BUY, SELL
    outcome TEXT NOT NULL,          -- UP, DOWN
    order_type TEXT NOT NULL,       -- FOK, GTC, IOC
    price REAL NOT NULL,
    size REAL NOT NULL,
    filled_size REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL,           -- PENDING, FILLED, CANCELLED, FAILED
    fee REAL NOT NULL DEFAULT 0.0,
    pnl REAL,
    error_message TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY(account_id) REFERENCES polymarket_accounts(id)
);
CREATE INDEX IF NOT EXISTS idx_real_orders_account ON real_orders(account_id);
CREATE INDEX IF NOT EXISTS idx_real_orders_status ON real_orders(status);
CREATE INDEX IF NOT EXISTS idx_real_orders_market ON real_orders(market_id);

-- 3. Self-Learning Model States (Persistent knowledge across system restarts)
CREATE TABLE IF NOT EXISTS learning_state (
    id TEXT PRIMARY KEY,
    asset TEXT NOT NULL UNIQUE,
    version INTEGER NOT NULL DEFAULT 1,
    weights_json TEXT NOT NULL,
    bias REAL NOT NULL DEFAULT 0.0,
    platt_a REAL NOT NULL DEFAULT 1.0,
    platt_b REAL NOT NULL DEFAULT 0.0,
    learning_rate REAL NOT NULL DEFAULT 0.01,
    total_samples_trained INTEGER NOT NULL DEFAULT 0,
    rolling_accuracy REAL NOT NULL DEFAULT 0.50,
    rolling_brier_score REAL NOT NULL DEFAULT 0.25,
    top_features_json TEXT,
    regime_stats_json TEXT,
    updated_at INTEGER NOT NULL
);

-- 4. Self-Learning Round-by-Round Evolution History
CREATE TABLE IF NOT EXISTS learning_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset TEXT NOT NULL,
    round_id TEXT NOT NULL,
    predicted_prob REAL NOT NULL,
    actual_outcome INTEGER NOT NULL, -- 1 for UP, 0 for DOWN
    loss REAL NOT NULL,
    weights_delta_norm REAL NOT NULL,
    brier_score REAL NOT NULL,
    timestamp INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_learning_history_asset_ts ON learning_history(asset, timestamp);
