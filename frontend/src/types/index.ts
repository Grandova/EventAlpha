export type Asset = 'BTC' | 'ETH' | 'SOL';

export interface SafetyStatus {
  real_trading_enabled: boolean;
  paper_trading_only: boolean;
  safety_lock_engaged: boolean;
  safety_signature: string;
  private_key_detected: boolean;
  message: string;
}

export interface ExchangeFreshness {
  last_update_ms: number;
  age_ms: number;
  is_fresh: boolean;
  status: string;
}

export interface FreshnessReport {
  is_globally_fresh: boolean;
  threshold_ms: number;
  exchanges: Record<string, ExchangeFreshness>;
}

export interface HealthResponse {
  status: string;
  service: string;
  version: string;
  mode: string;
  uptime_secs: number;
  timestamp_ms: number;
  real_trading_enabled: boolean;
  safety_status: SafetyStatus;
  exchange_freshness: FreshnessReport;
}

export interface PriceSummary {
  exchange: string;
  asset: string;
  price: number;
  latency_ms: number;
  is_fresh: boolean;
  age_ms: number;
}

export interface CompositePriceSnapshot {
  asset: Asset;
  timestamp_ms: number;
  composite_price: number;
  return_1s: number;
  return_3s: number;
  return_5s: number;
  return_10s: number;
  return_30s: number;
  return_60s: number;
  realized_vol_5s: number;
  realized_vol_10s: number;
  realized_vol_30s: number;
  realized_vol_60s: number;
  distance_from_open?: number;
  distance_percent?: number;
  distance_to_vol_ratio?: number;
  spread_binance_okx?: number;
  spread_binance_bybit?: number;
  spread_binance_coinbase?: number;
}

export interface Polymarket5mMarket {
  id: string;
  condition_id: string;
  asset: Asset;
  question: string;
  start_time_ms: number;
  end_time_ms: number;
  resolution?: string;
  status: string;
  up_token_id: string;
  down_token_id: string;
  up_price: number;
  down_price: number;
}

export interface MarketDisplayInfo {
  id: string;
  condition_id: string;
  asset: Asset;
  question: string;
  start_time_ms: number;
  end_time_ms: number;
  status: string;
  up_token_id: string;
  down_token_id: string;
  up_price: number;
  down_price: number;
  remaining_seconds: number;
}

export interface BookSideDepth {
  best_bid?: number;
  best_ask?: number;
  spread?: number;
  total_bid_depth_usdc: number;
  total_ask_depth_usdc: number;
  obi_top5: number;
  obi_top10: number;
  obi_top20: number;
}

export interface MarketBookSummary {
  market_id: string;
  asset: Asset;
  timestamp_ms: number;
  up_book: BookSideDepth;
  down_book: BookSideDepth;
  implied_prob_up: number;
  implied_prob_down: number;
}

export interface FeatureSnapshot {
  asset: Asset;
  market_id: string;
  timestamp_ms: number;
  composite_price: number;
  return_1s: number;
  return_3s: number;
  return_5s: number;
  return_10s: number;
  return_30s: number;
  return_60s: number;
  realized_vol_5s: number;
  realized_vol_10s: number;
  realized_vol_30s: number;
  realized_vol_60s: number;
  velocity_5s: number;
  velocity_15s: number;
  acceleration_5s_15s: number;
  distance_from_open: number;
  distance_percent: number;
  distance_to_vol_ratio: number;
  remaining_seconds: number;
  elapsed_seconds: number;
  time_decay_factor: number;
  spread_binance_okx: number;
  spread_binance_bybit: number;
  spread_binance_coinbase: number;
  cvd_5s: number;
  cvd_15s: number;
  cvd_30s: number;
  cvd_60s: number;
  trade_imbalance_5s: number;
  trade_imbalance_15s: number;
  trade_imbalance_30s: number;
  trade_imbalance_60s: number;
  poly_obi_top5: number;
  poly_obi_top10: number;
  poly_obi_top20: number;
  poly_spread: number;
  poly_total_liquidity: number;
  poly_implied_prob: number;
}

export interface ModelPrediction {
  market_id: string;
  asset: Asset;
  timestamp_ms: number;
  model_version: string;
  raw_p_up: number;
  raw_p_down: number;
  calibrated_p_up: number;
  calibrated_p_down: number;
  confidence: 'LOW' | 'MEDIUM' | 'HIGH' | 'VERY_HIGH' | 'SKIP';
  top_contributions: Array<{ feature: string; weight: number; contribution: number }>;
}

export interface FilterResult {
  gate_name: string;
  passed: boolean;
  value: string;
  threshold: string;
  reason?: string;
}

export interface ScoreBreakdown {
  probability_points: number;
  edge_points: number;
  timing_points: number;
  obi_points: number;
  momentum_points: number;
  total_score: number;
  score_tier: string;
}

export interface PredictionSignal {
  prediction_id: string;
  market_id: string;
  asset: Asset;
  timestamp_ms: number;
  action: 'BUY_UP' | 'BUY_DOWN' | 'SKIP';
  target_side?: 'UP' | 'DOWN';
  model_version: string;
  p_up: number;
  p_down: number;
  fair_value_up: number;
  fair_value_down: number;
  market_implied_up: number;
  gross_edge: number;
  estimated_fee: number;
  estimated_slippage: number;
  net_edge: number;
  confidence: string;
  signal_score: number;
  decision_reason: string;
  gate_results: FilterResult[];
  score_breakdown: ScoreBreakdown;
}

export interface BankrollState {
  initial_bankroll: number;
  active_bankroll: number;
  bankroll_cap: number;
  locked_profit: number;
  total_equity: number;
  minimum_bankroll: number;
  mode: 'capital_recovery' | 'profit_isolation';
  daily_loss_current: number;
  consecutive_losses: number;
  peak_equity: number;
  current_drawdown: number;
  is_trading_halted: boolean;
  halt_reason?: string;
}

export interface BankrollHistoryEntry {
  id: number;
  timestamp_ms: number;
  active_bankroll: number;
  locked_profit: number;
  total_equity: number;
  change_amount: number;
  reason: string;
  trade_id?: string;
}

export interface RiskStatus {
  mode: string;
  active_bankroll: number;
  bankroll_cap: number;
  locked_profit: number;
  total_equity: number;
  daily_loss_current: number;
  daily_loss_limit: number;
  current_drawdown_pct: number;
  max_drawdown_limit_pct: number;
  consecutive_losses: number;
  max_consecutive_losses: number;
  cooldown_until_ms?: number;
  is_in_cooldown: boolean;
  is_halted: boolean;
  halt_reason?: string;
}

export interface PaperOrder {
  order_id: string;
  market_id: string;
  asset: Asset;
  side: 'UP' | 'DOWN';
  stake: number;
  shares: number;
  quote_price: number;
  fill_price: number;
  slippage: number;
  fee: number;
  status: string;
  signal_id: string;
  timestamp_ms: number;
}

export interface PaperPosition {
  position_id: string;
  order_id: string;
  market_id: string;
  asset: Asset;
  side: 'UP' | 'DOWN';
  entry_time_ms: number;
  entry_price: number;
  stake: number;
  shares: number;
  status: string;
  settled_at_ms?: number;
  created_at_ms: number;
}

export interface PaperResult {
  result_id: string;
  order_id: string;
  market_id: string;
  asset: Asset;
  side: 'UP' | 'DOWN';
  entry_time_ms: number;
  entry_price: number;
  fill_price: number;
  shares: number;
  stake: number;
  predicted_probability: number;
  model_confidence: string;
  gross_edge: number;
  net_edge: number;
  fee: number;
  slippage: number;
  outcome: 'WIN' | 'LOSS' | 'VOID' | 'PENDING';
  payout: number;
  pnl: number;
  bankroll_before: number;
  bankroll_after: number;
  locked_profit_before: number;
  locked_profit_after: number;
  strategy_version: string;
  model_version: string;
  created_at_ms: number;
}

export interface TradeStatistics {
  total_trades: number;
  winning_trades: number;
  losing_trades: number;
  void_trades: number;
  win_rate: number;
  gross_profit: number;
  gross_loss: number;
  net_pnl: number;
  profit_factor: number;
  avg_win: number;
  avg_loss: number;
  active_bankroll: number;
  locked_profit: number;
  total_equity: number;
}

export interface BacktestRequest {
  assets?: Asset[];
  start_time_ms?: number;
  end_time_ms?: number;
  initial_bankroll?: number;
  bankroll_cap?: number;
  mode?: 'capital_recovery' | 'profit_isolation';
  stake?: number;
  min_prob?: number;
  min_net_edge?: number;
  fee_rate?: number;
  slippage_rate?: number;
}

export interface EquityPoint {
  timestamp_ms: number;
  active_bankroll: number;
  locked_profit: number;
  total_equity: number;
  drawdown_pct: number;
  trade_pnl?: number;
}

export interface BacktestMetrics {
  total_trades: number;
  winning_trades: number;
  losing_trades: number;
  void_trades: number;
  win_rate: number;
  gross_profit: number;
  gross_loss: number;
  net_pnl: number;
  total_return_pct: number;
  profit_factor: number;
  sharpe_ratio: number;
  sortino_ratio: number;
  calmar_ratio: number;
  max_drawdown_pct: number;
  max_drawdown_usdc: number;
  avg_trade_pnl: number;
  avg_win_pnl: number;
  avg_loss_pnl: number;
  win_loss_payoff_ratio: number;
  max_consecutive_wins: number;
  max_consecutive_losses: number;
  final_active_bankroll: number;
  final_locked_profit: number;
  final_total_equity: number;
}

export interface BacktestResult {
  backtest_id: string;
  config: BacktestRequest;
  metrics: BacktestMetrics;
  equity_curve: EquityPoint[];
  trades: PaperResult[];
  executed_at_ms: number;
  execution_duration_ms: number;
}

export type ReplayStatus = 'idle' | 'playing' | 'paused' | 'completed';

export interface ReplayConfig {
  asset: Asset;
  start_time_ms?: number;
  end_time_ms?: number;
  speed_multiplier: number;
}

export interface ReplayFrame {
  frame_index: usize;
  timestamp_ms: number;
  asset: Asset;
  market_id: string;
  composite_price: number;
  poly_up_bid: number;
  poly_up_ask: number;
  poly_down_bid: number;
  poly_down_ask: number;
  implied_prob_up: number;
  features?: FeatureSnapshot;
  prediction?: ModelPrediction;
  signal?: PredictionSignal;
  executed_trade?: PaperResult;
}

type usize = number;

export interface ReplayStateResponse {
  status: ReplayStatus;
  config?: ReplayConfig;
  total_frames: number;
  current_frame_index: number;
  current_timestamp_ms?: number;
  speed_multiplier: number;
  current_frame?: ReplayFrame;
}

export interface SystemEvent {
  event_type: string;
  severity: string;
  component: string;
  message: string;
  payload_json?: string;
  timestamp_ms: number;
}
