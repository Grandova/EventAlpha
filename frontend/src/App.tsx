import React, { useState, useEffect } from 'react';
import { Header } from './components/Header';
import { MarketTicker } from './components/MarketTicker';
import { PolymarketRoundCard } from './components/PolymarketRoundCard';
import { OpportunityCenter } from './components/OpportunityCenter';
import { BankrollRiskMonitor } from './components/BankrollRiskMonitor';
import { TradingBlotter } from './components/TradingBlotter';
import { FeatureMonitor } from './components/FeatureMonitor';
import { BacktestConsole } from './components/BacktestConsole';
import { ReplayConsole } from './components/ReplayConsole';
import { EventLogViewer } from './components/EventLogViewer';
import { OrderbookVisualizer } from './components/OrderbookVisualizer';
import { StrategyTuner } from './components/StrategyTuner';
import { ErrorBoundary } from './components/ErrorBoundary';
import { api } from './services/api';
import { wsClient } from './services/ws';
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
  RiskStatus,
  PaperPosition,
  PaperOrder,
  PaperResult,
  TradeStatistics,
} from './types';

export const App: React.FC = () => {
  const [activeAsset, setActiveAsset] = useState<Asset>('BTC');
  const [activeTab, setActiveTab] = useState<string>('dashboard');

  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [spotPrices, setSpotPrices] = useState<PriceSummary[]>([]);
  const [composite, setComposite] = useState<CompositePriceSnapshot | null>(null);
  const [market, setMarket] = useState<MarketDisplayInfo | null>(null);
  const [book, setBook] = useState<MarketBookSummary | null>(null);
  const [features, setFeatures] = useState<FeatureSnapshot | null>(null);
  const [prediction, setPrediction] = useState<ModelPrediction | null>(null);
  const [signal, setSignal] = useState<PredictionSignal | null>(null);
  const [bankroll, setBankroll] = useState<BankrollState | null>(null);
  const [risk, setRisk] = useState<RiskStatus | null>(null);
  const [activePositions, setActivePositions] = useState<PaperPosition[]>([]);
  const [recentOrders, setRecentOrders] = useState<PaperOrder[]>([]);
  const [settledResults, setSettledResults] = useState<PaperResult[]>([]);
  const [statistics, setStatistics] = useState<TradeStatistics | null>(null);

  // 1. High frequency polling for live prices & round state (1000ms)
  useEffect(() => {
    let isMounted = true;
    const pollFast = async () => {
      try {
        const [prices, comp, markets, bk, feat, pred, sig] = await Promise.allSettled([
          api.getCollectorPrices(),
          api.getCompositePrice(activeAsset),
          api.getPolymarketMarkets(),
          api.getPolymarketBook(activeAsset),
          api.getFeaturesLatest(activeAsset),
          api.getPrediction(activeAsset),
          api.getLatestSignal(activeAsset),
        ]);

        if (!isMounted) return;

        if (prices.status === 'fulfilled') setSpotPrices(prices.value);
        if (comp.status === 'fulfilled') setComposite(comp.value);
        if (markets.status === 'fulfilled') {
          const match = markets.value.find((m) => m.asset === activeAsset) || markets.value[0];
          setMarket(match || null);
        }
        if (bk.status === 'fulfilled') setBook(bk.value);
        if (feat.status === 'fulfilled') setFeatures(feat.value);
        if (pred.status === 'fulfilled') setPrediction(pred.value);
        if (sig.status === 'fulfilled') setSignal(sig.value);
      } catch (err) {
        // network error / backend restarting
      }
    };

    pollFast();
    const timer = setInterval(pollFast, 1000);
    return () => {
      isMounted = false;
      clearInterval(timer);
    };
  }, [activeAsset]);

  // 2. Medium frequency polling for bankroll, risk, positions & blotter (2000ms)
  useEffect(() => {
    let isMounted = true;
    const pollMedium = async () => {
      try {
        const [hlth, br, rsk, pos, ord, res, stat] = await Promise.allSettled([
          api.getHealth(),
          api.getBankroll(),
          api.getRiskStatus(),
          api.getActivePositions(),
          api.getPaperOrders(20),
          api.getPaperResults(50),
          api.getPaperStatistics(),
        ]);

        if (!isMounted) return;

        if (hlth.status === 'fulfilled') setHealth(hlth.value);
        if (br.status === 'fulfilled') setBankroll(br.value);
        if (rsk.status === 'fulfilled') setRisk(rsk.value);
        if (pos.status === 'fulfilled') setActivePositions(pos.value);
        if (ord.status === 'fulfilled') setRecentOrders(ord.value);
        if (res.status === 'fulfilled') setSettledResults(res.value);
        if (stat.status === 'fulfilled') setStatistics(stat.value);
      } catch (err) {
        // network error
      }
    };

    pollMedium();
    const timer = setInterval(pollMedium, 2000);
    return () => {
      isMounted = false;
      clearInterval(timer);
    };
  }, []);

  // 3. Real-time sub-100ms WebSocket Multiplexed Stream
  useEffect(() => {
    const unsubscribe = wsClient.subscribe((msg) => {
      switch (msg.type) {
        case 'ticker':
          if (msg.data.asset === activeAsset) {
            setComposite((prev) =>
              prev
                ? { ...prev, composite_price: msg.data.composite_price }
                : null
            );
            if (msg.data.spot_prices && msg.data.spot_prices.length > 0) {
              setSpotPrices(msg.data.spot_prices);
            }
          }
          break;

        case 'signal':
          if (msg.data.signal.asset === activeAsset) {
            setSignal(msg.data.signal);
          }
          break;

        case 'bankroll':
          setBankroll((prev) =>
            prev
              ? {
                  ...prev,
                  active_bankroll: msg.data.active,
                  locked_profit: msg.data.locked,
                  total_equity: msg.data.total,
                }
              : null
          );
          break;

        case 'resolution':
          api.getActivePositions().then(setActivePositions).catch(() => {});
          api.getPaperResults(50).then(setSettledResults).catch(() => {});
          api.getPaperStatistics().then(setStatistics).catch(() => {});
          api.getBankroll().then(setBankroll).catch(() => {});
          break;

        case 'heartbeat':
          setHealth((prev) =>
            prev
              ? {
                  ...prev,
                  uptime_secs: msg.data.uptime_secs,
                  timestamp_ms: msg.data.timestamp_ms,
                }
              : null
          );
          break;
      }
    });

    return () => {
      unsubscribe();
    };
  }, [activeAsset]);

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100 p-3 sm:p-4 lg:p-6 max-w-[1720px] mx-auto">
      {/* Top Header & Navigation */}
      <Header
        health={health}
        activeAsset={activeAsset}
        onSelectAsset={(a) => setActiveAsset(a)}
        activeTab={activeTab}
        onSelectTab={(t) => setActiveTab(t)}
      />

      {/* Global Market Ticker */}
      <MarketTicker asset={activeAsset} spotPrices={spotPrices} composite={composite} />

      {/* Main Tab Views */}
      <main>
        <ErrorBoundary fallbackTitle="Module Render Error">
          {activeTab === 'dashboard' && (
            <div className="space-y-4">
              {/* Top row: Polymarket 5M Round & AI Opportunity Center */}
              <div className="grid grid-cols-1 lg:grid-cols-12 gap-4">
                <div className="lg:col-span-6">
                  <PolymarketRoundCard market={market} book={book} prediction={prediction} />
                </div>
                <div className="lg:col-span-6">
                  <OpportunityCenter signal={signal} prediction={prediction} />
                </div>
              </div>

              {/* Orderbook Depth Ladder */}
              <OrderbookVisualizer
                asset={activeAsset}
                book={book}
                onRefresh={() => api.getPolymarketBook(activeAsset).then(setBook)}
              />

              {/* Middle row: Bankroll & Risk Management */}
              <BankrollRiskMonitor bankroll={bankroll} risk={risk} />

              {/* Bottom row: Paper Trading Blotter */}
              <TradingBlotter
                activePositions={activePositions}
                recentOrders={recentOrders}
                settledResults={settledResults}
                statistics={statistics}
              />
            </div>
          )}

          {activeTab === 'microstructure' && (
            <div className="space-y-4">
              <OrderbookVisualizer
                asset={activeAsset}
                book={book}
                onRefresh={() => api.getPolymarketBook(activeAsset).then(setBook)}
              />
              <FeatureMonitor asset={activeAsset} features={features} />
            </div>
          )}

          {activeTab === 'features' && (
            <FeatureMonitor asset={activeAsset} features={features} />
          )}

          {activeTab === 'tuning' && (
            <StrategyTuner />
          )}

          {activeTab === 'backtest' && (
            <BacktestConsole defaultAsset={activeAsset} />
          )}

          {activeTab === 'replay' && (
            <ReplayConsole defaultAsset={activeAsset} />
          )}

          {activeTab === 'events' && (
            <EventLogViewer />
          )}
        </ErrorBoundary>
      </main>

      {/* Footer / Safety Lock Legal Invariant */}
      <footer className="mt-8 pt-4 border-t border-slate-900 text-center text-xs text-slate-500 font-mono">
        <p>
          POLYQUANT-5M QUANTITATIVE TERMINAL &bull; SAFETY LOCK ACTIVE &bull; STRICTLY PAPER TRADING SIMULATION
        </p>
        <p className="text-[10px] text-slate-600 mt-1">
          Zero real orders permitted &bull; Private key storage disabled &bull; Mode B Capital Recovery $10.00 Cap Enforced
        </p>
      </footer>
    </div>
  );
};

export default App;
