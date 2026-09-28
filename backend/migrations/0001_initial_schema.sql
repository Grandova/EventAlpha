-- =============================================================================
-- Migration 0001: Initial Database Schema for PolyQuant
-- =============================================================================

-- 1. Markets
CREATE TABLE IF NOT EXISTS markets (
    id TEXT PRIMARY KEY,
    condition_id TEXT NOT NULL,
    asset TEXT NOT NULL,
    slug TEXT,
    question TEXT,
    start_time INTEGER NOT NULL,
    end_time INTEGER NOT NULL,
    open_price REAL,
    final_price REAL,
    status TEXT NOT NULL DEFAULT 'active',
    resolution TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_markets_asset_status ON markets(asset, status);
CREATE INDEX IF NOT EXISTS idx_markets_end_time ON markets(end_time);
CREATE INDEX IF NOT EXISTS idx_markets_start_end ON markets(start_time, end_time);

-- 2. Market Ticks (Polymarket orderbook & pricing ticks)
CREATE TABLE IF NOT EXISTS market_ticks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    market_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    up_bid REAL,
    up_ask REAL,
    down_bid REAL,
    down_ask REAL,
    up_mid REAL,
    down_mid REAL,
    spread REAL,
    volume REAL,
    liquidity REAL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(market_id) REFERENCES markets(id)
);
CREATE INDEX IF NOT EXISTS idx_market_ticks_mid_ts ON market_ticks(market_id, timestamp);

-- 3. Exchange Ticks (Binance, OKX, Bybit, Coinbase)
CREATE TABLE IF NOT EXISTS exchange_ticks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    exchange TEXT NOT NULL,
    symbol TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    bid REAL NOT NULL,
    ask REAL NOT NULL,
    mid REAL NOT NULL,
    last REAL NOT NULL,
    volume REAL NOT NULL,
    latency_ms INTEGER,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_exchange_ticks_sym_ts ON exchange_ticks(exchange, symbol, timestamp);

-- 4. Order Book Snapshots
CREATE TABLE IF NOT EXISTS orderbook_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    market_or_symbol TEXT NOT NULL,
    exchange TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    bids_json TEXT NOT NULL,
    asks_json TEXT NOT NULL,
    obi_top5 REAL,
    obi_top10 REAL,
    obi_top20 REAL,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_orderbook_ts ON orderbook_snapshots(exchange, market_or_symbol, timestamp);

-- 5. Trades
CREATE TABLE IF NOT EXISTS trades (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    exchange TEXT NOT NULL,
    symbol TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    price REAL NOT NULL,
    size REAL NOT NULL,
    side TEXT NOT NULL,
    is_aggressive INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_trades_sym_ts ON trades(exchange, symbol, timestamp);

-- 6. Features
CREATE TABLE IF NOT EXISTS features (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    market_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    feature_name TEXT NOT NULL,
    feature_vector_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(market_id) REFERENCES markets(id)
);
CREATE INDEX IF NOT EXISTS idx_features_mid_ts ON features(market_id, timestamp);

-- 7. Predictions & Signals
CREATE TABLE IF NOT EXISTS predictions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    prediction_id TEXT NOT NULL UNIQUE,
    market_id TEXT NOT NULL,
    asset TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    model_version TEXT NOT NULL,
    p_up REAL NOT NULL,
    p_down REAL NOT NULL,
    fair_value_up REAL NOT NULL,
    fair_value_down REAL NOT NULL,
    market_implied_up REAL,
    gross_edge REAL NOT NULL,
    estimated_fee REAL NOT NULL,
    estimated_slippage REAL NOT NULL,
    net_edge REAL NOT NULL,
    confidence TEXT NOT NULL,
    signal_score REAL NOT NULL,
    recommended_action TEXT NOT NULL,
    decision_reason TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(market_id) REFERENCES markets(id)
);
CREATE INDEX IF NOT EXISTS idx_predictions_mid_ts ON predictions(market_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_predictions_asset_ts ON predictions(asset, timestamp);
CREATE INDEX IF NOT EXISTS idx_predictions_action ON predictions(recommended_action, timestamp);

-- 8. Paper Orders
CREATE TABLE IF NOT EXISTS paper_orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id TEXT NOT NULL UNIQUE,
    market_id TEXT NOT NULL,
    asset TEXT NOT NULL,
    side TEXT NOT NULL,
    stake REAL NOT NULL,
    shares REAL NOT NULL,
    quote_price REAL NOT NULL,
    fill_price REAL NOT NULL,
    slippage REAL NOT NULL,
    fee REAL NOT NULL,
    status TEXT NOT NULL,
    signal_id TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(market_id) REFERENCES markets(id)
);
CREATE INDEX IF NOT EXISTS idx_paper_orders_mid ON paper_orders(market_id);
CREATE INDEX IF NOT EXISTS idx_paper_orders_status ON paper_orders(status, created_at);

-- 9. Paper Positions
CREATE TABLE IF NOT EXISTS paper_positions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id TEXT NOT NULL UNIQUE,
    market_id TEXT NOT NULL,
    asset TEXT NOT NULL,
    side TEXT NOT NULL,
    entry_time INTEGER NOT NULL,
    entry_price REAL NOT NULL,
    stake REAL NOT NULL,
    shares REAL NOT NULL,
    status TEXT NOT NULL,
    settled_at INTEGER,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(market_id) REFERENCES markets(id)
);
CREATE INDEX IF NOT EXISTS idx_paper_positions_status_mid ON paper_positions(status, market_id);
CREATE INDEX IF NOT EXISTS idx_paper_positions_asset ON paper_positions(asset);

-- 10. Paper Results (Settled trades with complete ledger balance attribution)
CREATE TABLE IF NOT EXISTS paper_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    result_id TEXT NOT NULL UNIQUE,
    order_id TEXT NOT NULL,
    market_id TEXT NOT NULL,
    asset TEXT NOT NULL,
    side TEXT NOT NULL,
    entry_time INTEGER NOT NULL,
    entry_price REAL NOT NULL,
    fill_price REAL NOT NULL,
    shares REAL NOT NULL,
    stake REAL NOT NULL,
    predicted_probability REAL NOT NULL,
    model_confidence TEXT NOT NULL,
    gross_edge REAL NOT NULL,
    net_edge REAL NOT NULL,
    fee REAL NOT NULL,
    slippage REAL NOT NULL,
    outcome TEXT NOT NULL,
    payout REAL NOT NULL,
    pnl REAL NOT NULL,
    bankroll_before REAL NOT NULL,
    bankroll_after REAL NOT NULL,
    locked_profit_before REAL NOT NULL,
    locked_profit_after REAL NOT NULL,
    strategy_version TEXT NOT NULL,
    model_version TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(market_id) REFERENCES markets(id),
    FOREIGN KEY(order_id) REFERENCES paper_orders(order_id)
);
CREATE INDEX IF NOT EXISTS idx_paper_results_asset ON paper_results(asset);
CREATE INDEX IF NOT EXISTS idx_paper_results_created ON paper_results(created_at);

-- 11. Bankroll History
CREATE TABLE IF NOT EXISTS bankroll_history (
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
CREATE INDEX IF NOT EXISTS idx_bankroll_history_ts ON bankroll_history(timestamp);

-- 12. Strategy Versions
CREATE TABLE IF NOT EXISTS strategy_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    version TEXT NOT NULL UNIQUE,
    parameters_json TEXT NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL
);

-- 13. Model Versions
CREATE TABLE IF NOT EXISTS model_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    version TEXT NOT NULL UNIQUE,
    asset TEXT NOT NULL,
    features_list TEXT NOT NULL,
    hyperparameters_json TEXT NOT NULL,
    metrics_json TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- 14. Backtest Runs
CREATE TABLE IF NOT EXISTS backtest_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL UNIQUE,
    asset TEXT NOT NULL,
    time_range TEXT NOT NULL,
    parameters_json TEXT NOT NULL,
    metrics_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- 15. System Events (Audit log, circuit breaker trips, fail-closed events)
CREATE TABLE IF NOT EXISTS system_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    component TEXT NOT NULL,
    message TEXT NOT NULL,
    payload_json TEXT,
    timestamp INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_system_events_ts ON system_events(timestamp);
