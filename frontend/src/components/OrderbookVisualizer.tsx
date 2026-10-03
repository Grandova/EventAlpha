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
      <div className="flex flex-wrap items-center justify-between gap-3 mb-4 pb-3 border-b border-[var(--color-light)]">
        <div className="flex items-center gap-2">
          <div className="p-2 bg-blue-50 dark:bg-cyan-950/60 border border-blue-200 dark:border-cyan-800/80 rounded-xl">
            <Layers className="h-4 w-4 text-[var(--color-primary)]" />
          </div>
          <div>
            <h3 className="text-sm font-bold text-[var(--color-dark)] flex items-center gap-2">
              CLOB Orderbook Depth Ladder
              <span className="text-[10px] text-[var(--color-primary)] font-mono bg-blue-50 dark:bg-cyan-950/60 px-2 py-0.5 rounded-full border border-blue-200 dark:border-cyan-800/60">
                {asset} 5M
              </span>
            </h3>
            <p className="text-xs text-[var(--color-info-dark)]">
              Live Polymarket L2 Depth, Spreads & Orderbook Imbalance (OBI)
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* UP / DOWN Token Selector */}
          <div className="flex bg-[var(--color-background)] border border-[var(--color-light)] rounded-xl p-0.5">
            <button
              onClick={() => setActiveSide('UP')}
              className={`px-3 py-1 text-xs font-semibold rounded-lg transition-all flex items-center gap-1.5 ${
                activeSide === 'UP'
                  ? 'bg-[#1b9c85] text-white shadow-sm'
                  : 'text-[var(--color-info-dark)] hover:text-[var(--color-dark)]'
              }`}
            >
              <ArrowUp className="h-3 w-3" />
              UP Token
            </button>
            <button
              onClick={() => setActiveSide('DOWN')}
              className={`px-3 py-1 text-xs font-semibold rounded-lg transition-all flex items-center gap-1.5 ${
                activeSide === 'DOWN'
                  ? 'bg-[#ff0060] text-white shadow-sm'
                  : 'text-[var(--color-info-dark)] hover:text-[var(--color-dark)]'
              }`}
            >
              <ArrowDown className="h-3 w-3" />
              DOWN Token
            </button>
          </div>

          {onRefresh && (
            <button
              onClick={onRefresh}
              className="p-1.5 bg-[var(--color-background)] hover:bg-[var(--color-light)] border border-[var(--color-light)] rounded-xl text-[var(--color-info-dark)] hover:text-[var(--color-dark)] transition-colors"
              title="Refresh orderbook"
            >
              <RefreshCw className="h-3.5 w-3.5" />
            </button>
          )}
        </div>
      </div>

      {/* Summary Metrics Bar */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2.5 mb-4">
        <div className="asmr-subcard p-3">
          <div className="text-[10px] text-[var(--color-info-dark)] font-semibold mb-0.5">BEST BID / ASK</div>
          <div className="text-xs font-mono font-bold flex items-center gap-1">
            <span className="text-[#1b9c85]">${bestBid.toFixed(3)}</span>
            <span className="text-[var(--color-info-dark)]">/</span>
            <span className="text-[#ff0060]">${bestAsk.toFixed(3)}</span>
          </div>
        </div>

        <div className="asmr-subcard p-3">
          <div className="text-[10px] text-[var(--color-info-dark)] font-semibold mb-0.5">MID & SPREAD</div>
          <div className="text-xs font-mono font-bold text-[var(--color-dark)] flex items-center justify-between">
            <span>${mid.toFixed(3)}</span>
            <span className="text-amber-500 text-[11px] font-semibold">
              +{(spread * 100).toFixed(1)}¢
            </span>
          </div>
        </div>

        <div className="asmr-subcard p-3">
          <div className="text-[10px] text-[var(--color-info-dark)] font-semibold mb-0.5">OBI (TOP 5 / 10 / 20)</div>
          <div className="text-xs font-mono font-bold flex items-center gap-1.5">
            <span className={obi5 >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}>
              {obi5 >= 0 ? '+' : ''}{(obi5 * 100).toFixed(0)}%
            </span>
            <span className="text-[var(--color-info-dark)]">|</span>
            <span className={obi10 >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}>
              {obi10 >= 0 ? '+' : ''}{(obi10 * 100).toFixed(0)}%
            </span>
            <span className="text-[var(--color-info-dark)]">|</span>
            <span className={obi20 >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}>
              {obi20 >= 0 ? '+' : ''}{(obi20 * 100).toFixed(0)}%
            </span>
          </div>
        </div>

        <div className="asmr-subcard p-3">
          <div className="text-[10px] text-[var(--color-info-dark)] font-semibold mb-0.5">TOTAL DEPTH (BID/ASK)</div>
          <div className="text-xs font-mono font-bold text-[var(--color-dark)]">
            ${totalBidDepth.toFixed(0)} / ${totalAskDepth.toFixed(0)} USDC
          </div>
        </div>
      </div>

      {/* Two-Column Depth Ladder */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3.5 font-mono text-xs">
        {/* Bids Ladder (Buy orders, green depth bars) */}
        <div className="asmr-subcard p-3.5">
          <div className="flex items-center justify-between text-[11px] font-bold text-[#1b9c85] pb-2 mb-2 border-b border-[var(--color-light)]">
            <span>BIDS (BUY)</span>
            <span>SIZE (SHARES)</span>
            <span>TOTAL ($)</span>
          </div>

          <div className="space-y-1">
            {bids.length === 0 ? (
              <div className="text-center py-6 text-[var(--color-info-dark)] italic text-[11px]">
                No active bids in orderbook
              </div>
            ) : (
              bids.slice(0, 8).map((level: OrderBookLevel, idx: number) => {
                const fillPct = Math.min(100, (level.size / maxSize) * 100);
                const totalUsd = level.price * level.size;
                return (
                  <div key={`bid-${idx}`} className="relative flex items-center justify-between py-1 px-1.5 rounded-lg overflow-hidden">
                    <div
                      className="absolute right-0 top-0 bottom-0 bg-[#1b9c85]/15 rounded pointer-events-none"
                      style={{ width: `${fillPct}%` }}
                    />
                    <span className="relative font-bold text-[#1b9c85] z-10">
                      ${level.price.toFixed(3)}
                    </span>
                    <span className="relative text-[var(--color-dark)] font-medium z-10">
                      {level.size.toLocaleString(undefined, { maximumFractionDigits: 0 })}
                    </span>
                    <span className="relative text-[var(--color-info-dark)] z-10 text-[10px]">
                      ${totalUsd.toFixed(1)}
                    </span>
                  </div>
                );
              })
            )}
          </div>
        </div>

        {/* Asks Ladder (Sell orders, red depth bars) */}
        <div className="asmr-subcard p-3.5">
          <div className="flex items-center justify-between text-[11px] font-bold text-[#ff0060] pb-2 mb-2 border-b border-[var(--color-light)]">
            <span>ASKS (SELL)</span>
            <span>SIZE (SHARES)</span>
            <span>TOTAL ($)</span>
          </div>

          <div className="space-y-1">
            {asks.length === 0 ? (
              <div className="text-center py-6 text-[var(--color-info-dark)] italic text-[11px]">
                No active asks in orderbook
              </div>
            ) : (
              asks.slice(0, 8).map((level: OrderBookLevel, idx: number) => {
                const fillPct = Math.min(100, (level.size / maxSize) * 100);
                const totalUsd = level.price * level.size;
                return (
                  <div key={`ask-${idx}`} className="relative flex items-center justify-between py-1 px-1.5 rounded-lg overflow-hidden">
                    <div
                      className="absolute right-0 top-0 bottom-0 bg-[#ff0060]/15 rounded pointer-events-none"
                      style={{ width: `${fillPct}%` }}
                    />
                    <span className="relative font-bold text-[#ff0060] z-10">
                      ${level.price.toFixed(3)}
                    </span>
                    <span className="relative text-[var(--color-dark)] font-medium z-10">
                      {level.size.toLocaleString(undefined, { maximumFractionDigits: 0 })}
                    </span>
                    <span className="relative text-[var(--color-info-dark)] z-10 text-[10px]">
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
