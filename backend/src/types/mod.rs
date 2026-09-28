use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Asset {
    BTC,
    ETH,
    SOL,
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Asset::BTC => write!(f, "BTC"),
            Asset::ETH => write!(f, "ETH"),
            Asset::SOL => write!(f, "SOL"),
        }
    }
}

impl std::str::FromStr for Asset {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "BTC" => Ok(Asset::BTC),
            "ETH" => Ok(Asset::ETH),
            "SOL" => Ok(Asset::SOL),
            other => Err(format!("Unknown asset: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MarketSide {
    Up,
    Down,
}

impl std::fmt::Display for MarketSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketSide::Up => write!(f, "UP"),
            MarketSide::Down => write!(f, "DOWN"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Resolution {
    Up,
    Down,
    Void,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarketStatus {
    Active,
    Resolved,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Exchange {
    Binance,
    Okx,
    Bybit,
    Coinbase,
    Polymarket,
}

impl std::fmt::Display for Exchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Exchange::Binance => write!(f, "binance"),
            Exchange::Okx => write!(f, "okx"),
            Exchange::Bybit => write!(f, "bybit"),
            Exchange::Coinbase => write!(f, "coinbase"),
            Exchange::Polymarket => write!(f, "polymarket"),
        }
    }
}

/// Unified tick across exchanges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTick {
    pub exchange: Exchange,
    pub symbol: String,
    pub asset: Asset,
    pub exchange_timestamp_ms: i64,
    pub receive_timestamp_ms: i64,
    pub latency_ms: i64,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub last: f64,
    pub volume_24h: f64,
}

/// Polymarket 5-minute Market Specific Tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketTick {
    pub market_id: String,
    pub asset: Asset,
    pub timestamp_ms: i64,
    pub up_bid: Option<f64>,
    pub up_ask: Option<f64>,
    pub down_bid: Option<f64>,
    pub down_ask: Option<f64>,
    pub up_mid: Option<f64>,
    pub down_mid: Option<f64>,
    pub spread: Option<f64>,
    pub volume_24h: Option<f64>,
    pub liquidity: Option<f64>,
}

/// Single level in an orderbook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: f64,
    pub size: f64,
}

/// Orderbook depth snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub exchange: Exchange,
    pub symbol: String,
    pub timestamp_ms: i64,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub obi_top5: f64,
    pub obi_top10: f64,
    pub obi_top20: f64,
}

/// Trade tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeTick {
    pub exchange: Exchange,
    pub symbol: String,
    pub asset: Asset,
    pub exchange_timestamp_ms: i64,
    pub receive_timestamp_ms: i64,
    pub latency_ms: i64,
    pub price: f64,
    pub size: f64,
    pub side: String, // "buy" or "sell"
    pub is_aggressive: bool,
}

/// Real-time health and connectivity metrics for an exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeHealth {
    pub exchange: Exchange,
    pub connected: bool,
    pub last_update_ms: i64,
    pub latency_ms: i64,
    pub ticks_received: u64,
    pub is_stale: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConfidenceLevel {
    Skip,
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignalAction {
    BuyUp,
    BuyDown,
    Skip,
}

impl std::fmt::Display for SignalAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalAction::BuyUp => write!(f, "BUY UP"),
            SignalAction::BuyDown => write!(f, "BUY DOWN"),
            SignalAction::Skip => write!(f, "SKIP"),
        }
    }
}

/// Strategy prediction and edge assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionSignal {
    pub prediction_id: String,
    pub market_id: String,
    pub asset: Asset,
    pub timestamp_ms: i64,
    pub model_version: String,
    pub p_up: f64,
    pub p_down: f64,
    pub fair_value_up: f64,
    pub fair_value_down: f64,
    pub market_implied_up: Option<f64>,
    pub gross_edge: f64,
    pub estimated_fee: f64,
    pub estimated_slippage: f64,
    pub net_edge: f64,
    pub signal_score: f64, // 0 - 100
    pub confidence: ConfidenceLevel,
    pub action: SignalAction,
    pub decision_reason: String,
}

/// Decision log for complete transparency on every evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionLog {
    pub timestamp_ms: i64,
    pub market_id: String,
    pub asset: Asset,
    pub p_up: f64,
    pub p_down: f64,
    pub up_ask: Option<f64>,
    pub down_ask: Option<f64>,
    pub gross_edge: f64,
    pub fee: f64,
    pub slippage: f64,
    pub net_edge: f64,
    pub liquidity_check: String,
    pub spread_check: String,
    pub time_check: String,
    pub risk_check: String,
    pub final_action: SignalAction,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BankrollMode {
    CapitalRecovery, // Mode B: Excess profit locked, losses replenished first
    ProfitIsolation, // Mode A: 100% net profit locked immediately
}

/// Active Bankroll and Locked Profit State
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankrollState {
    pub initial_bankroll: f64,
    pub active_bankroll: f64,
    pub bankroll_cap: f64,
    pub locked_profit: f64,
    pub total_equity: f64,
    pub minimum_bankroll: f64,
    pub mode: BankrollMode,
    pub daily_loss_current: f64,
    pub consecutive_losses: u32,
    pub peak_equity: f64,
    pub current_drawdown: f64,
    pub is_trading_halted: bool,
    pub halt_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankrollHistoryEntry {
    pub id: i64,
    pub timestamp_ms: i64,
    pub active_bankroll: f64,
    pub locked_profit: f64,
    pub total_equity: f64,
    pub change_amount: f64,
    pub reason: String,
    pub trade_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskStatus {
    pub is_trading_halted: bool,
    pub halt_reason: Option<String>,
    pub daily_loss_current: f64,
    pub daily_loss_limit: f64,
    pub current_drawdown: f64,
    pub max_drawdown_limit: f64,
    pub peak_equity: f64,
    pub consecutive_losses: u32,
    pub max_consecutive_losses: u32,
    pub cooldown_until_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperOrder {
    pub order_id: String,
    pub market_id: String,
    pub asset: Asset,
    pub side: MarketSide,
    pub stake: f64,
    pub shares: f64,
    pub quote_price: f64,
    pub fill_price: f64,
    pub slippage: f64,
    pub fee: f64,
    pub status: String,
    pub signal_id: Option<String>,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperPosition {
    pub position_id: String,
    pub order_id: String,
    pub market_id: String,
    pub asset: Asset,
    pub side: MarketSide,
    pub entry_time_ms: i64,
    pub entry_price: f64,
    pub stake: f64,
    pub shares: f64,
    pub status: String, // "OPEN", "CLOSED"
    pub settled_at_ms: Option<i64>,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperResult {
    pub result_id: String,
    pub order_id: String,
    pub market_id: String,
    pub asset: Asset,
    pub side: MarketSide,
    pub entry_time_ms: i64,
    pub entry_price: f64,
    pub fill_price: f64,
    pub shares: f64,
    pub stake: f64,
    pub predicted_probability: f64,
    pub model_confidence: String,
    pub gross_edge: f64,
    pub net_edge: f64,
    pub fee: f64,
    pub slippage: f64,
    pub outcome: String, // WIN, LOSS, VOID
    pub payout: f64,
    pub pnl: f64,
    pub bankroll_before: f64,
    pub bankroll_after: f64,
    pub locked_profit_before: f64,
    pub locked_profit_after: f64,
    pub strategy_version: String,
    pub model_version: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub event_type: String,
    pub severity: String,
    pub component: String,
    pub message: String,
    pub payload_json: Option<String>,
    pub timestamp_ms: i64,
}
