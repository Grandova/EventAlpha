import {
  Asset,
  HealthResponse,
  PriceSummary,
  CompositePriceSnapshot,
  MarketDisplayInfo,
  MarketBookSummary,
  FeatureSnapshot,
  ModelPrediction,
  PredictionSignal,
  BankrollState,
  BankrollHistoryEntry,
  RiskStatus,
  PaperOrder,
  PaperPosition,
  PaperResult,
  TradeStatistics,
  BacktestRequest,
  BacktestResult,
  ReplayConfig,
  ReplayStateResponse,
  ReplayFrame,
  SystemEvent,
  StrategyConfig,
  ModelTrainRequest,
  ModelTrainResult,
  GenerateSyntheticResponse,
  TradingMode,
  TradingModeStatus,
  PolymarketAccountPublic,
  CreateAccountRequest,
  RealOrder,
  LearningState,
  LearningHistoryEntry,
  LoginRequest,
  LoginResponse,
  AuthStatusResponse,
} from '../types';

const BASE_URL = '';

export function getStoredToken(): string | null {
  return localStorage.getItem('polyquant_auth_token');
}

export function setStoredToken(token: string) {
  localStorage.setItem('polyquant_auth_token', token);
}

export function clearStoredToken() {
  localStorage.removeItem('polyquant_auth_token');
}

async function fetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  const token = getStoredToken();
  const headers = new Headers(init?.headers || {});
  if (token) {
    headers.set('Authorization', `Bearer ${token}`);
  }
  if (!headers.has('Content-Type') && init?.body) {
    headers.set('Content-Type', 'application/json');
  }

  const res = await fetch(`${BASE_URL}${url}`, {
    ...init,
    headers,
  });

  if (res.status === 401 && !url.includes('/api/v1/auth/')) {
    clearStoredToken();
    window.dispatchEvent(new Event('polyquant_auth_required'));
  }

  if (!res.ok) {
    const errorText = await res.text().catch(() => 'Unknown error');
    throw new Error(`API error ${res.status}: ${errorText}`);
  }
  return res.json();
}

export const api = {
  // Authentication & Security
  login: (req: LoginRequest) =>
    fetchJson<LoginResponse>('/api/v1/auth/login', {
      method: 'POST',
      body: JSON.stringify(req),
    }),
  getAuthStatus: () => fetchJson<AuthStatusResponse>('/api/v1/auth/me'),
  logout: () =>
    fetchJson<any>('/api/v1/auth/logout', {
      method: 'POST',
    }),

  // System Health & Safety
  getHealth: () => fetchJson<HealthResponse>('/api/v1/health'),
  getSafety: () => fetchJson<any>('/api/v1/safety'),
  getEvents: (limit = 50) => fetchJson<SystemEvent[]>(`/api/v1/events?limit=${limit}`),

  // Collectors & Market Prices
  getCollectorStatus: () => fetchJson<any>('/api/v1/collector/status'),
  getCollectorPrices: () => fetchJson<PriceSummary[]>('/api/v1/collector/prices'),
  getCompositePrice: (asset: Asset) =>
    fetchJson<CompositePriceSnapshot>(`/api/v1/composite/price/${asset}`),
  getAllCompositePrices: () =>
    fetchJson<Record<string, CompositePriceSnapshot>>('/api/v1/composite/all'),

  // Polymarket Active Markets & Books
  getPolymarketMarkets: () => fetchJson<MarketDisplayInfo[]>('/api/v1/polymarket/markets'),
  getPolymarketBook: (asset: Asset) =>
    fetchJson<MarketBookSummary>(`/api/v1/polymarket/book/${asset}`),

  // Features & Models
  getFeaturesLatest: (asset: Asset) =>
    fetchJson<FeatureSnapshot>(`/api/v1/features/latest/${asset}`),
  getAllFeatures: () => fetchJson<Record<string, FeatureSnapshot>>('/api/v1/features/all'),
  getPrediction: (asset: Asset) =>
    fetchJson<ModelPrediction>(`/api/v1/models/prediction/${asset}`),
  getAllPredictions: () => fetchJson<Record<string, ModelPrediction>>('/api/v1/models/all'),

  // Strategy Signals & Decisions
  getLatestSignal: (asset: Asset) =>
    fetchJson<PredictionSignal>(`/api/v1/strategy/signals/latest/${asset}`),
  getAllLatestSignals: () =>
    fetchJson<Record<string, PredictionSignal>>('/api/v1/strategy/signals/latest'),
  getRecentDecisions: (limit = 20) =>
    fetchJson<any[]>(`/api/v1/strategy/decisions/recent?limit=${limit}`),

  // Paper Trading & Risk Management
  getBankroll: () => fetchJson<BankrollState>('/api/v1/paper/bankroll'),
  setBankrollFunds: (req: { active_bankroll: number; bankroll_cap?: number; minimum_bankroll?: number }) =>
    fetchJson<{ success: boolean; message: string; bankroll: BankrollState }>('/api/v1/paper/bankroll/set', {
      method: 'POST',
      body: JSON.stringify(req),
    }),
  unlockProfit: (amount?: number) =>
    fetchJson<{ success: boolean; message: string; bankroll: BankrollState }>('/api/v1/paper/bankroll/unlock', {
      method: 'POST',
      body: JSON.stringify({ amount }),
    }),
  getBankrollHistory: (limit = 50) =>
    fetchJson<BankrollHistoryEntry[]>(`/api/v1/paper/bankroll/history?limit=${limit}`),
  getRiskStatus: () => fetchJson<RiskStatus>('/api/v1/risk/status'),
  updateRiskConfig: (req: {
    daily_loss_limit?: number;
    max_consecutive_losses?: number;
    max_drawdown?: number;
    cooldown_minutes?: number;
  }) =>
    fetchJson<{ success: boolean; message: string; risk: RiskStatus }>('/api/v1/risk/config', {
      method: 'POST',
      body: JSON.stringify(req),
    }),
  unhaltTrading: () =>
    fetchJson<{ success: boolean; message: string; risk: RiskStatus }>('/api/v1/risk/unhalt', {
      method: 'POST',
    }),
  getEnabledAssets: () =>
    fetchJson<{ success: boolean; assets: Asset[] }>('/api/v1/strategy/assets'),
  setEnabledAssets: (assets: Asset[]) =>
    fetchJson<{ success: boolean; message: string; assets: Asset[] }>('/api/v1/strategy/assets', {
      method: 'POST',
      body: JSON.stringify({ assets }),
    }),
  getAutoTrading: () =>
    fetchJson<{
      enabled: boolean;
      paper_enabled: boolean;
      live_enabled: boolean;
      mode: string;
    }>('/api/v1/strategy/autotrade'),
  setAutoTrading: (req: {
    enabled?: boolean;
    mode?: 'paper' | 'live';
    paper_enabled?: boolean;
    live_enabled?: boolean;
  } | boolean) => {
    const payload = typeof req === 'boolean' ? { enabled: req } : req;
    return fetchJson<{
      success: boolean;
      enabled: boolean;
      paper_enabled: boolean;
      live_enabled: boolean;
      message: string;
    }>('/api/v1/strategy/autotrade', {
      method: 'POST',
      body: JSON.stringify(payload),
    });
  },
  executeManualTrade: (req: {
    market_id: string;
    asset: Asset;
    side: 'UP' | 'DOWN' | 'Up' | 'Down';
    stake?: number;
    mode?: 'paper' | 'live';
  }) =>
    fetchJson<{
      success: boolean;
      mode: string;
      order?: any;
      position?: any;
      message: string;
    }>('/api/v1/trade/manual', {
      method: 'POST',
      body: JSON.stringify(req),
    }),
  getPaperOrders: (limit = 20) =>
    fetchJson<PaperOrder[]>(`/api/v1/paper/orders?limit=${limit}`),
  getActivePositions: () =>
    fetchJson<PaperPosition[]>('/api/v1/paper/positions/active'),
  getPositionsHistory: (limit = 20) =>
    fetchJson<PaperPosition[]>(`/api/v1/paper/positions/history?limit=${limit}`),
  getPaperResults: (limit = 50) =>
    fetchJson<PaperResult[]>(`/api/v1/paper/results?limit=${limit}`),
  getPaperStatistics: () => fetchJson<TradeStatistics>('/api/v1/paper/statistics'),

  // Backtest
  runBacktest: (req: BacktestRequest) =>
    fetchJson<BacktestResult>('/api/v1/backtest/run', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    }),
  getLatestBacktest: () => fetchJson<BacktestResult>('/api/v1/backtest/latest'),
  getBacktestHistory: (limit = 10) =>
    fetchJson<BacktestResult[]>(`/api/v1/backtest/history?limit=${limit}`),

  // Replay
  startReplay: (config: ReplayConfig) =>
    fetchJson<ReplayStateResponse>('/api/v1/replay/start', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }),
  pauseReplay: () =>
    fetchJson<ReplayStateResponse>('/api/v1/replay/pause', { method: 'POST' }),
  resumeReplay: () =>
    fetchJson<ReplayStateResponse>('/api/v1/replay/resume', { method: 'POST' }),
  stepReplay: () =>
    fetchJson<ReplayFrame | null>('/api/v1/replay/step', { method: 'POST' }),
  seekReplay: (req: { target_frame_index?: number; target_timestamp_ms?: number }) =>
    fetchJson<ReplayFrame | null>('/api/v1/replay/seek', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    }),
  setReplaySpeed: (speed_multiplier: number) =>
    fetchJson<{ speed_multiplier: number }>('/api/v1/replay/speed', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ speed_multiplier }),
    }),
  stopReplay: () =>
    fetchJson<{ status: string }>('/api/v1/replay/stop', { method: 'POST' }),
  getReplayStatus: () => fetchJson<ReplayStateResponse>('/api/v1/replay/status'),
  getReplayFrames: (offset = 0, limit = 100) =>
    fetchJson<ReplayFrame[]>(`/api/v1/replay/frames?offset=${offset}&limit=${limit}`),

  // Strategy Config Hot Reload
  getStrategyConfig: () => fetchJson<StrategyConfig>('/api/v1/strategy/config'),
  updateStrategyConfig: (config: StrategyConfig) =>
    fetchJson<StrategyConfig>('/api/v1/strategy/config', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }),

  // Model Training
  trainModel: (req: ModelTrainRequest) =>
    fetchJson<ModelTrainResult>('/api/v1/models/train', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    }),

  // Synthetic Dataset Generation
  generateSyntheticDataset: (rounds = 50) =>
    fetchJson<GenerateSyntheticResponse>('/api/v1/dataset/generate_synthetic', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ rounds }),
    }),

  // Trading Mode & Real Accounts
  getTradingMode: () => fetchJson<TradingModeStatus>('/api/v1/trading/mode'),
  setTradingMode: (mode: TradingMode) =>
    fetchJson<{ mode: string; status: string }>('/api/v1/trading/mode', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ mode }),
    }),
  getAccounts: () => fetchJson<PolymarketAccountPublic[]>('/api/v1/accounts'),
  createAccount: (req: CreateAccountRequest) =>
    fetchJson<PolymarketAccountPublic>('/api/v1/accounts', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    }),
  activateAccount: (id: string) =>
    fetchJson<{ status: string; account_id: string }>(`/api/v1/accounts/${id}/activate`, {
      method: 'POST',
    }),
  deleteAccount: (id: string) =>
    fetchJson<{ status: string; deleted: boolean }>(`/api/v1/accounts/${id}`, {
      method: 'DELETE',
    }),
  getAccountBalance: (id: string) =>
    fetchJson<{ account_id: string; balance_usdc: number; proxy_wallet_address?: string }>(`/api/v1/accounts/${id}/balance`),
  updateAccount: (id: string, req: Partial<CreateAccountRequest> & { balance_usdc?: number }) =>
    fetchJson<PolymarketAccountPublic>(`/api/v1/accounts/${id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    }),
  calibrateAccountBalance: (id: string, balance_usdc: number) =>
    fetchJson<{ account_id: string; balance_usdc: number; success: boolean }>(`/api/v1/accounts/${id}/calibrate-balance`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ balance_usdc }),
    }),

  // Real CLOB Trading & Emergency Halt
  getRealOrders: (limit = 20) => fetchJson<RealOrder[]>(`/api/v1/real/orders?limit=${limit}`),
  emergencyHalt: () =>
    fetchJson<{ status: string; mode: string; message: string }>('/api/v1/real/emergency_halt', {
      method: 'POST',
    }),

  // Continuous Self-Learning Engine
  getLearningStatus: (asset: Asset) =>
    fetchJson<LearningState>(`/api/v1/learning/status?asset=${asset}`),
  toggleLearning: (enabled: boolean) =>
    fetchJson<{ auto_learning_enabled: boolean }>('/api/v1/learning/toggle', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled }),
    }),
  retrainLearning: (asset: Asset) =>
    fetchJson<{ status: string; asset: string; samples: number; accuracy: number; brier_score: number }>(
      '/api/v1/learning/retrain',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ asset }),
      }
    ),
  getLearningHistory: (asset: Asset, limit = 50) =>
    fetchJson<LearningHistoryEntry[]>(`/api/v1/learning/history?asset=${asset}&limit=${limit}`),
};

