import React, { useState } from 'react';
import { Layers, ArrowUp, ArrowDown, Activity, DollarSign, RefreshCw } from 'lucide-react';
import { MarketBookSummary, Asset, OrderBookLevel } from '../types';

interface OrderbookVisualizerProps {
  asset: Asset;
  book: MarketBookSummary | null;
  onRefresh?: () => void;
}

export const OrderbookVisualizer: React.FC<OrderbookVisualizerProps> = ({
  asset,
  book,
  onRefresh,
}) => {
  const [activeSide, setActiveSide] = useState<'UP' | 'DOWN'>('UP');

  const selectedBook = activeSide === 'UP' ? book?.up_book : book?.down_book;

  const bids: OrderBookLevel[] = selectedBook?.bids || [];
  const asks: OrderBookLevel[] = selectedBook?.asks || [];

  // Compute maximum cumulative size for depth chart bars
  const maxBidSize = bids.reduce((acc: number, curr: OrderBookLevel) => Math.max(acc, curr.size), 1);
  const maxAskSize = asks.reduce((acc: number, curr: OrderBookLevel) => Math.max(acc, curr.size), 1);
  const maxSize = Math.max(maxBidSize, maxAskSize);

  const bestBid = selectedBook?.best_bid ?? 0.50;
  const bestAsk = selectedBook?.best_ask ?? 0.51;
  const spread = selectedBook?.spread ?? (bestAsk - bestBid);
  const mid = selectedBook?.mid ?? ((bestBid + bestAsk) / 2);

  const obi5 = selectedBook?.obi_top5 ?? 0;
  const obi10 = selectedBook?.obi_top10 ?? 0;
  const obi20 = selectedBook?.obi_top20 ?? 0;

  const totalBidDepth = selectedBook?.total_bid_depth_usdc ?? 0;
  const totalAskDepth = selectedBook?.total_ask_depth_usdc ?? 0;

  return (
    <div className="asmr-card p-6">
      {/* Title & Selector */}
      <div className="flex flex-wrap items-center justify-between gap-3 mb-4 pb-3 border-b border-slate-800">
        <div className="flex items-center gap-2">
          <div className="p-1.5 bg-cyan-950 border border-cyan-800/80 rounded">
            <Layers className="h-4 w-4 text-cyan-400" />
          </div>
          <div>
            <h3 className="text-sm font-bold text-white flex items-center gap-2">
              CLOB Orderbook Depth Ladder
              <span className="text-[10px] text-cyan-400 font-mono bg-cyan-950/60 px-2 py-0.5 rounded border border-cyan-800/60">
                {asset} 5M
              </span>
            </h3>
            <p className="text-xs text-slate-400">
              Live Polymarket L2 Depth, Spreads & Orderbook Imbalance (OBI)
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* UP / DOWN Token Selector */}
          <div className="flex bg-slate-900 border border-slate-800 rounded-lg p-0.5">
            <button
              onClick={() => setActiveSide('UP')}
              className={`px-3 py-1 text-xs font-semibold rounded-md transition-all flex items-center gap-1.5 ${
                activeSide === 'UP'
                  ? 'bg-emerald-600 text-white shadow-sm'
                  : 'text-slate-400 hover:text-white'
              }`}
            >
              <ArrowUp className="h-3 w-3" />
              UP Token
            </button>
            <button
              onClick={() => setActiveSide('DOWN')}
              className={`px-3 py-1 text-xs font-semibold rounded-md transition-all flex items-center gap-1.5 ${
                activeSide === 'DOWN'
                  ? 'bg-rose-600 text-white shadow-sm'
                  : 'text-slate-400 hover:text-white'
              }`}
            >
              <ArrowDown className="h-3 w-3" />
              DOWN Token
            </button>
          </div>

          {onRefresh && (
            <button
              onClick={onRefresh}
              className="p-1.5 bg-slate-900 hover:bg-slate-800 border border-slate-800 rounded-lg text-slate-400 hover:text-cyan-400 transition-colors"
              title="Refresh orderbook"
            >
              <RefreshCw className="h-3.5 w-3.5" />
            </button>
          )}
        </div>
      </div>

      {/* Summary Metrics Bar */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 mb-4">
        <div className="bg-slate-900/80 border border-slate-800/80 p-2.5 rounded-lg">
          <div className="text-[10px] text-slate-400 font-semibold mb-0.5">BEST BID / ASK</div>
          <div className="text-xs font-mono font-bold flex items-center gap-1">
            <span className="text-emerald-400">${bestBid.toFixed(3)}</span>
            <span className="text-slate-600">/</span>
            <span className="text-rose-400">${bestAsk.toFixed(3)}</span>
          </div>
        </div>

        <div className="bg-slate-900/80 border border-slate-800/80 p-2.5 rounded-lg">
          <div className="text-[10px] text-slate-400 font-semibold mb-0.5">MID & SPREAD</div>
          <div className="text-xs font-mono font-bold text-white flex items-center justify-between">
            <span>${mid.toFixed(3)}</span>
            <span className="text-amber-400 text-[11px]">
              +{(spread * 100).toFixed(1)}¢
            </span>
          </div>
        </div>

        <div className="bg-slate-900/80 border border-slate-800/80 p-2.5 rounded-lg">
          <div className="text-[10px] text-slate-400 font-semibold mb-0.5">OBI (TOP 5 / 10 / 20)</div>
          <div className="text-xs font-mono font-bold flex items-center gap-1.5">
            <span className={obi5 >= 0 ? 'text-emerald-400' : 'text-rose-400'}>
              {obi5 >= 0 ? '+' : ''}{(obi5 * 100).toFixed(0)}%
            </span>
            <span className="text-slate-600">|</span>
            <span className={obi10 >= 0 ? 'text-emerald-400' : 'text-rose-400'}>
              {obi10 >= 0 ? '+' : ''}{(obi10 * 100).toFixed(0)}%
            </span>
            <span className="text-slate-600">|</span>
            <span className={obi20 >= 0 ? 'text-emerald-400' : 'text-rose-400'}>
              {obi20 >= 0 ? '+' : ''}{(obi20 * 100).toFixed(0)}%
            </span>
          </div>
        </div>

        <div className="bg-slate-900/80 border border-slate-800/80 p-2.5 rounded-lg">
          <div className="text-[10px] text-slate-400 font-semibold mb-0.5">TOTAL DEPTH (BID/ASK)</div>
          <div className="text-xs font-mono font-bold text-slate-300">
            ${totalBidDepth.toFixed(0)} / ${totalAskDepth.toFixed(0)} USDC
          </div>
        </div>
      </div>

      {/* Two-Column Depth Ladder */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 font-mono text-xs">
        {/* Bids Ladder (Buy orders, green depth bars) */}
        <div className="bg-slate-950/70 border border-slate-800/80 rounded-lg p-2.5">
          <div className="flex items-center justify-between text-[11px] font-bold text-emerald-400 pb-1.5 mb-1.5 border-b border-slate-800">
            <span>BIDS (BUY)</span>
            <span>SIZE (SHARES)</span>
            <span>TOTAL ($)</span>
          </div>

          <div className="space-y-1">
            {bids.length === 0 ? (
              <div className="text-center py-6 text-slate-600 italic text-[11px]">
                No active bids in orderbook
              </div>
            ) : (
              bids.slice(0, 8).map((level: OrderBookLevel, idx: number) => {
                const fillPct = Math.min(100, (level.size / maxSize) * 100);
                const totalUsd = level.price * level.size;
                return (
                  <div key={`bid-${idx}`} className="relative flex items-center justify-between py-0.5 px-1 rounded overflow-hidden">
                    <div
                      className="absolute right-0 top-0 bottom-0 bg-emerald-950/40 rounded pointer-events-none"
                      style={{ width: `${fillPct}%` }}
                    />
                    <span className="relative font-bold text-emerald-400 z-10">
                      ${level.price.toFixed(3)}
                    </span>
                    <span className="relative text-slate-300 z-10">
                      {level.size.toLocaleString(undefined, { maximumFractionDigits: 0 })}
                    </span>
                    <span className="relative text-slate-500 z-10 text-[10px]">
                      ${totalUsd.toFixed(1)}
                    </span>
                  </div>
                );
              })
            )}
          </div>
        </div>

        {/* Asks Ladder (Sell orders, red depth bars) */}
        <div className="bg-slate-950/70 border border-slate-800/80 rounded-lg p-2.5">
          <div className="flex items-center justify-between text-[11px] font-bold text-rose-400 pb-1.5 mb-1.5 border-b border-slate-800">
            <span>ASKS (SELL)</span>
            <span>SIZE (SHARES)</span>
            <span>TOTAL ($)</span>
          </div>

          <div className="space-y-1">
            {asks.length === 0 ? (
              <div className="text-center py-6 text-slate-600 italic text-[11px]">
                No active asks in orderbook
              </div>
            ) : (
              asks.slice(0, 8).map((level: OrderBookLevel, idx: number) => {
                const fillPct = Math.min(100, (level.size / maxSize) * 100);
                const totalUsd = level.price * level.size;
                return (
                  <div key={`ask-${idx}`} className="relative flex items-center justify-between py-0.5 px-1 rounded overflow-hidden">
                    <div
                      className="absolute right-0 top-0 bottom-0 bg-rose-950/40 rounded pointer-events-none"
                      style={{ width: `${fillPct}%` }}
                    />
                    <span className="relative font-bold text-rose-400 z-10">
                      ${level.price.toFixed(3)}
                    </span>
                    <span className="relative text-slate-300 z-10">
                      {level.size.toLocaleString(undefined, { maximumFractionDigits: 0 })}
                    </span>
                    <span className="relative text-slate-500 z-10 text-[10px]">
                      ${totalUsd.toFixed(1)}
                    </span>
                  </div>
                );
              })
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
