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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScoreTier {
    Skip,
    Low,
    Medium,
    High,
    VeryHigh,
}

impl std::fmt::Display for ScoreTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScoreTier::Skip => write!(f, "SKIP"),
            ScoreTier::Low => write!(f, "LOW"),
            ScoreTier::Medium => write!(f, "MEDIUM"),
            ScoreTier::High => write!(f, "HIGH"),
            ScoreTier::VeryHigh => write!(f, "VERY_HIGH"),
        }
    }
}

/// Detailed point breakdown for auditability and transparency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub edge_points: f64,     // max 35
    pub prob_points: f64,     // max 25
    #[serde(default)]
    pub probability_points: f64, // alias for frontend compatibility
    pub obi_points: f64,      // max 20
    pub momentum_points: f64, // max 15
    pub timing_points: f64,   // max 5
    pub total_score: f64,     // 0 - 100
    pub tier: ScoreTier,
    #[serde(default)]
    pub score_tier: String,   // alias for frontend compatibility
}

/// Hard filter gate evaluation result for transparency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_name: String,
    pub passed: bool,
    pub value: String,
    pub threshold: Option<String>,
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
    #[serde(default)]
    pub gate_results: Vec<GateResult>,
    #[serde(default)]
    pub score_breakdown: Option<ScoreBreakdown>,
}

impl Default for PredictionSignal {
    fn default() -> Self {
        Self {
            prediction_id: String::new(),
            market_id: String::new(),
            asset: Asset::BTC,
            timestamp_ms: 0,
            model_version: String::new(),
            p_up: 0.5,
            p_down: 0.5,
            fair_value_up: 0.5,
            fair_value_down: 0.5,
            market_implied_up: None,
            gross_edge: 0.0,
            estimated_fee: 0.0,
            estimated_slippage: 0.0,
            net_edge: 0.0,
            signal_score: 0.0,
            confidence: ConfidenceLevel::Skip,
            action: SignalAction::Skip,
            decision_reason: String::new(),
            gate_results: Vec::new(),
            score_breakdown: None,
        }
    }
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
    #[serde(default)]
    pub is_halted: bool,
    pub halt_reason: Option<String>,
    pub daily_loss_current: f64,
    pub daily_loss_limit: f64,
    pub current_drawdown: f64,
    pub max_drawdown_limit: f64,
    pub peak_equity: f64,
    pub consecutive_losses: u32,
    pub max_consecutive_losses: u32,
    pub cooldown_until_ms: Option<i64>,
    #[serde(default)]
    pub is_in_cooldown: bool,
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
pub struct TradeStatistics {
    pub total_trades: i64,
    pub winning_trades: i64,
    pub losing_trades: i64,
    pub void_trades: i64,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub gross_profit: f64,
    pub gross_loss: f64,
    pub profit_factor: f64,
    pub avg_trade_pnl: f64,
    pub max_win: f64,
    pub max_loss: f64,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradingMode {
    Paper,
    Live,
}

impl Default for TradingMode {
    fn default() -> Self {
        TradingMode::Paper
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketAccount {
    pub id: String,
    pub label: String,
    pub api_key: String,
    pub api_secret: String,
    pub api_passphrase: String,
    pub wallet_address: String,
    pub proxy_wallet_address: Option<String>,
    pub is_active: bool,
    pub balance_usdc: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketAccountPublic {
    pub id: String,
    pub label: String,
    pub api_key_masked: String,
    pub wallet_address: String,
    pub proxy_wallet_address: Option<String>,
    pub is_active: bool,
    pub balance_usdc: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<&PolymarketAccount> for PolymarketAccountPublic {
    fn from(acc: &PolymarketAccount) -> Self {
        let masked_key = if acc.api_key.len() > 8 {
            format!("{}...{}", &acc.api_key[..4], &acc.api_key[acc.api_key.len()-4..])
        } else {
            "****".to_string()
        };
        Self {
            id: acc.id.clone(),
            label: acc.label.clone(),
            api_key_masked: masked_key,
            wallet_address: acc.wallet_address.clone(),
            proxy_wallet_address: acc.proxy_wallet_address.clone(),
            is_active: acc.is_active,
            balance_usdc: acc.balance_usdc,
            created_at: acc.created_at,
            updated_at: acc.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountRequest {
    pub label: String,
    pub api_key: String,
    pub api_secret: String,
    pub api_passphrase: String,
    pub wallet_address: String,
    pub proxy_wallet_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealOrder {
    pub id: String,
    pub account_id: String,
    pub clob_order_id: Option<String>,
    pub market_id: String,
    pub token_id: String,
    pub asset: Asset,
    pub side: String,
    pub outcome: String,
    pub order_type: String,
    pub price: f64,
    pub size: f64,
    pub filled_size: f64,
    pub status: String,
    pub fee: f64,
    pub pnl: Option<f64>,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningState {
    pub id: String,
    pub asset: Asset,
    pub version: i64,
    pub weights: Vec<f64>,
    pub bias: f64,
    pub platt_a: f64,
    pub platt_b: f64,
    pub learning_rate: f64,
    pub total_samples_trained: i64,
    pub rolling_accuracy: f64,
    pub rolling_brier_score: f64,
    pub top_features: Vec<(String, f64)>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningHistoryEntry {
    pub id: i64,
    pub asset: Asset,
    pub round_id: String,
    pub predicted_prob: f64,
    pub actual_outcome: i64,
    pub loss: f64,
    pub weights_delta_norm: f64,
    pub brier_score: f64,
    pub timestamp: i64,
}

