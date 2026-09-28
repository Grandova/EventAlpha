import React from 'react';
import { TrendingUp, TrendingDown, Layers, Zap, Gauge } from 'lucide-react';
import { PriceSummary, CompositePriceSnapshot, Asset } from '../types';

interface MarketTickerProps {
  asset: Asset;
  spotPrices: PriceSummary[];
  composite: CompositePriceSnapshot | null;
}

export const MarketTicker: React.FC<MarketTickerProps> = ({
  asset,
  spotPrices,
  composite,
}) => {
  const currentCompPrice = composite?.composite_price ?? 0;
  const ret1s = (composite?.return_1s ?? 0) * 100;
  const ret5s = (composite?.return_5s ?? 0) * 100;
  const ret60s = (composite?.return_60s ?? 0) * 100;
  const vol5s = (composite?.realized_vol_5s ?? 0) * 100;
  const vol60s = (composite?.realized_vol_60s ?? 0) * 100;
  const distOpen = composite?.distance_from_open ?? 0;
  const distPct = (composite?.distance_percent ?? 0) * 100;

  // Filter spot prices for the selected asset
  const assetPrices = spotPrices.filter((p) => p.asset.toUpperCase() === asset.toUpperCase());

  return (
    <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-4">
      {/* 1. Main Composite Price Card */}
      <div className="quant-card p-4 md:col-span-2 border-cyan-500/20 bg-gradient-to-br from-slate-900 via-slate-900 to-cyan-950/20">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-cyan-400 animate-ping" />
            <h2 className="text-xs font-bold text-slate-400 uppercase tracking-wider">
              {asset} / USD Composite Robust Index
            </h2>
          </div>
          <span className="text-[10px] bg-slate-800 text-slate-300 font-mono px-2 py-0.5 rounded">
            L2 Weighted Aggregation
          </span>
        </div>

        <div className="flex items-baseline justify-between">
          <div className="flex items-baseline gap-3">
            <span className="text-3xl font-black text-white font-mono-num tracking-tight">
              ${currentCompPrice.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </span>
            <div
              className={`flex items-center text-xs font-bold font-mono-num px-2 py-0.5 rounded ${
                ret5s >= 0 ? 'bg-emerald-950/60 text-emerald-400' : 'bg-rose-950/60 text-rose-400'
              }`}
            >
              {ret5s >= 0 ? <TrendingUp className="h-3 w-3 mr-1" /> : <TrendingDown className="h-3 w-3 mr-1" />}
              {ret5s >= 0 ? '+' : ''}
              {ret5s.toFixed(3)}% (5s)
            </div>
          </div>

          <div className="text-right">
            <div className="text-xs font-bold text-slate-300 font-mono-num">
              Dist to Open: {distOpen >= 0 ? '+' : ''}${distOpen.toFixed(2)} ({distPct >= 0 ? '+' : ''}{distPct.toFixed(2)}%)
            </div>
            <div className="text-[11px] text-slate-500 font-mono">
              60s Ret: <span className={ret60s >= 0 ? 'text-emerald-400' : 'text-rose-400'}>{ret60s >= 0 ? '+' : ''}{ret60s.toFixed(3)}%</span>
            </div>
          </div>
        </div>

        {/* Multi-scale Realized Volatility & Dynamic Metrics */}
        <div className="mt-3 pt-3 border-t border-slate-800/80 grid grid-cols-4 gap-2 text-center text-xs">
          <div className="bg-slate-950/40 p-1.5 rounded border border-slate-800/60">
            <span className="text-[10px] text-slate-500 block">Realized Vol (5s)</span>
            <span className="font-mono-num font-bold text-slate-200">{vol5s.toFixed(4)}%</span>
          </div>
          <div className="bg-slate-950/40 p-1.5 rounded border border-slate-800/60">
            <span className="text-[10px] text-slate-500 block">Realized Vol (60s)</span>
            <span className="font-mono-num font-bold text-slate-200">{vol60s.toFixed(4)}%</span>
          </div>
          <div className="bg-slate-950/40 p-1.5 rounded border border-slate-800/60">
            <span className="text-[10px] text-slate-500 block">Binance-OKX Spread</span>
            <span className="font-mono-num font-bold text-slate-300">
              ${(composite?.spread_binance_okx ?? 0).toFixed(2)}
            </span>
          </div>
          <div className="bg-slate-950/40 p-1.5 rounded border border-slate-800/60">
            <span className="text-[10px] text-slate-500 block">Binance-Coinbase</span>
            <span className="font-mono-num font-bold text-slate-300">
              ${(composite?.spread_binance_coinbase ?? 0).toFixed(2)}
            </span>
          </div>
        </div>
      </div>

      {/* 2. Spot Exchanges Feed Cards */}
      <div className="quant-card p-4 md:col-span-2">
        <div className="flex items-center justify-between mb-2">
          <h2 className="text-xs font-bold text-slate-400 uppercase tracking-wider flex items-center gap-1.5">
            <Layers className="h-3.5 w-3.5 text-cyan-400" />
            Live Exchange Spot Feeds
          </h2>
          <span className="text-[10px] text-slate-500 font-mono">Max allowable skew: &lt;2000ms</span>
        </div>

        <div className="grid grid-cols-2 gap-2 mt-2">
          {['Binance', 'OKX', 'Bybit', 'Coinbase'].map((exch) => {
            const feed = assetPrices.find((p) => p.exchange.toLowerCase() === exch.toLowerCase());
            const price = feed?.price ?? 0;
            const isFresh = feed?.is_fresh ?? false;
            const diffFromComp = currentCompPrice > 0 && price > 0 ? price - currentCompPrice : 0;

            return (
              <div
                key={exch}
                className="bg-slate-900/60 border border-slate-800/80 p-2.5 rounded-lg flex items-center justify-between"
              >
                <div>
                  <div className="flex items-center gap-1.5">
                    <span className={`h-1.5 w-1.5 rounded-full ${isFresh ? 'bg-emerald-400' : 'bg-rose-500'}`} />
                    <span className="text-xs font-bold text-slate-300 uppercase">{exch}</span>
                  </div>
                  <span className="text-sm font-black text-white font-mono-num">
                    ${price.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                  </span>
                </div>

                <div className="text-right">
                  <span
                    className={`text-[11px] font-mono-num font-semibold ${
                      diffFromComp >= 0 ? 'text-emerald-400' : 'text-rose-400'
                    }`}
                  >
                    {diffFromComp >= 0 ? '+' : ''}${diffFromComp.toFixed(2)}
                  </span>
                  <span className="block text-[10px] text-slate-500 font-mono">{feed?.latency_ms ?? 0}ms</span>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
