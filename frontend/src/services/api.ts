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
} from '../types';

const BASE_URL = '';

async function fetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE_URL}${url}`, init);
  if (!res.ok) {
    const errorText = await res.text().catch(() => 'Unknown error');
    throw new Error(`API error ${res.status}: ${errorText}`);
  }
  return res.json();
}

export const api = {
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
  getBankrollHistory: (limit = 50) =>
    fetchJson<BankrollHistoryEntry[]>(`/api/v1/paper/bankroll/history?limit=${limit}`),
  getRiskStatus: () => fetchJson<RiskStatus>('/api/v1/risk/status'),
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
};
