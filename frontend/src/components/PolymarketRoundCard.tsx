import React from 'react';
import { Timer, ArrowUpRight, ArrowDownRight, BarChart2, Shield } from 'lucide-react';
import { MarketDisplayInfo, MarketBookSummary, ModelPrediction } from '../types';

interface PolymarketRoundCardProps {
  market: MarketDisplayInfo | null;
  book: MarketBookSummary | null;
  prediction: ModelPrediction | null;
}

export const PolymarketRoundCard: React.FC<PolymarketRoundCardProps> = ({
  market,
  book,
  prediction,
}) => {
  const remainingSecs = market?.remaining_seconds ?? 0;
  const progressPct = Math.max(0, Math.min(100, ((300 - remainingSecs) / 300) * 100));

  const mins = Math.floor(remainingSecs / 60);
  const secs = remainingSecs % 60;
  const timerStr = `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;

  const impliedUp = (book?.implied_prob_up ?? market?.up_price ?? 0.50) * 100;
  const impliedDown = (book?.implied_prob_down ?? market?.down_price ?? 0.50) * 100;

  const modelUp = (prediction?.calibrated_p_up ?? 0.50) * 100;
  const modelDown = (prediction?.calibrated_p_down ?? 0.50) * 100;

  const upBid = book?.up_book?.best_bid ?? market?.up_price ?? 0.50;
  const upAsk = book?.up_book?.best_ask ?? (upBid + 0.01);
  const downBid = book?.down_book?.best_bid ?? market?.down_price ?? 0.50;
  const downAsk = book?.down_book?.best_ask ?? (downBid + 0.01);

  const spreadUp = book?.up_book?.spread ?? (upAsk - upBid);
  const totalLiquidity = (book?.up_book?.total_bid_depth_usdc ?? 500) + (book?.down_book?.total_bid_depth_usdc ?? 500);

  return (
    <div className="quant-card p-4 mb-4">
      {/* Header: Market Question & 5M Countdown Timer */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-4">
        <div>
          <div className="flex items-center gap-2">
            <span className="text-[10px] font-bold bg-cyan-950 text-cyan-400 border border-cyan-800/80 px-2 py-0.5 rounded uppercase">
              Polymarket 5M Round
            </span>
            <span className="text-xs text-slate-500 font-mono">ID: {market?.id ?? 'Awaiting Discovery...'}</span>
          </div>
          <h3 className="text-base font-bold text-white mt-1">
            {market?.question ?? 'Active 5-Minute Crypto Up/Down Contract'}
          </h3>
        </div>

        {/* 5-Minute Round Countdown */}
        <div className="bg-slate-900 border border-slate-800 p-2.5 rounded-lg min-w-[200px]">
          <div className="flex items-center justify-between text-xs mb-1">
            <span className="text-slate-400 font-semibold flex items-center gap-1.5">
              <Timer className="h-3.5 w-3.5 text-cyan-400" />
              Round Closes In:
            </span>
            <span
              className={`font-mono-num font-black text-sm ${
                remainingSecs <= 30
                  ? 'text-rose-400 animate-pulse'
                  : remainingSecs <= 60
                  ? 'text-amber-400'
                  : 'text-cyan-400'
              }`}
            >
              {timerStr}
            </span>
          </div>
          <div className="h-2 w-full bg-slate-800 rounded-full overflow-hidden">
            <div
              className={`h-full transition-all duration-1000 ${
                remainingSecs <= 30
                  ? 'bg-rose-500'
                  : remainingSecs <= 60
                  ? 'bg-amber-500'
                  : 'bg-gradient-to-r from-cyan-500 to-emerald-400'
              }`}
              style={{ width: `${progressPct}%` }}
            />
          </div>
          <div className="flex justify-between text-[10px] text-slate-500 font-mono mt-1">
            <span>Elapsed: {300 - remainingSecs}s</span>
            <span>Window: 300s</span>
          </div>
        </div>
      </div>

      {/* Main Orderbook Prices & Microstructure Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* UP Side Book */}
        <div className="p-3.5 rounded-lg border border-emerald-500/20 bg-emerald-950/10">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-bold text-emerald-400 flex items-center gap-1 uppercase tracking-wider">
              <ArrowUpRight className="h-4 w-4" /> UP Outcome
            </span>
            <span className="text-[10px] text-slate-400 font-mono">
              Implied: <strong className="text-emerald-400">{impliedUp.toFixed(1)}%</strong>
            </span>
          </div>

          <div className="grid grid-cols-2 gap-3 mb-3">
            <div className="bg-slate-900/80 p-2 rounded border border-emerald-500/30">
              <span className="text-[10px] text-slate-500 block">Best Bid (Buy)</span>
              <span className="text-lg font-mono-num font-bold text-emerald-400">
                ${upBid.toFixed(3)}
              </span>
            </div>
            <div className="bg-slate-900/80 p-2 rounded border border-emerald-500/30">
              <span className="text-[10px] text-slate-500 block">Best Ask (Sell)</span>
              <span className="text-lg font-mono-num font-bold text-emerald-300">
                ${upAsk.toFixed(3)}
              </span>
            </div>
          </div>

          {/* Model Calibrated vs Market Implied Comparison */}
          <div className="text-xs space-y-1">
            <div className="flex justify-between text-slate-400">
              <span>Model Predicted P(Up):</span>
              <span className="font-mono-num font-bold text-white">{modelUp.toFixed(1)}%</span>
            </div>
            <div className="flex justify-between text-slate-400">
              <span>Gross Edge (Up):</span>
              <span
                className={`font-mono-num font-bold ${
                  modelUp / 100 - upAsk >= 0.05 ? 'text-emerald-400' : 'text-slate-400'
                }`}
              >
                {((modelUp / 100 - upAsk) * 100).toFixed(1)}%
              </span>
            </div>
            <div className="flex justify-between text-slate-400">
              <span>OBI Top 10 (Up):</span>
              <span className="font-mono-num font-semibold text-slate-300">
                {(book?.up_book?.obi_top10 ?? 0).toFixed(3)}
              </span>
            </div>
          </div>
        </div>

        {/* DOWN Side Book */}
        <div className="p-3.5 rounded-lg border border-rose-500/20 bg-rose-950/10">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-bold text-rose-400 flex items-center gap-1 uppercase tracking-wider">
              <ArrowDownRight className="h-4 w-4" /> DOWN Outcome
            </span>
            <span className="text-[10px] text-slate-400 font-mono">
              Implied: <strong className="text-rose-400">{impliedDown.toFixed(1)}%</strong>
            </span>
          </div>

          <div className="grid grid-cols-2 gap-3 mb-3">
            <div className="bg-slate-900/80 p-2 rounded border border-rose-500/30">
              <span className="text-[10px] text-slate-500 block">Best Bid (Buy)</span>
              <span className="text-lg font-mono-num font-bold text-rose-400">
                ${downBid.toFixed(3)}
              </span>
            </div>
            <div className="bg-slate-900/80 p-2 rounded border border-rose-500/30">
              <span className="text-[10px] text-slate-500 block">Best Ask (Sell)</span>
              <span className="text-lg font-mono-num font-bold text-rose-300">
                ${downAsk.toFixed(3)}
              </span>
            </div>
          </div>

          {/* Model Calibrated vs Market Implied Comparison */}
          <div className="text-xs space-y-1">
            <div className="flex justify-between text-slate-400">
              <span>Model Predicted P(Down):</span>
              <span className="font-mono-num font-bold text-white">{modelDown.toFixed(1)}%</span>
            </div>
            <div className="flex justify-between text-slate-400">
              <span>Gross Edge (Down):</span>
              <span
                className={`font-mono-num font-bold ${
                  modelDown / 100 - downAsk >= 0.05 ? 'text-emerald-400' : 'text-slate-400'
                }`}
              >
                {((modelDown / 100 - downAsk) * 100).toFixed(1)}%
              </span>
            </div>
            <div className="flex justify-between text-slate-400">
              <span>OBI Top 10 (Down):</span>
              <span className="font-mono-num font-semibold text-slate-300">
                {(book?.down_book?.obi_top10 ?? 0).toFixed(3)}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Book Microstructure Metadata Footer */}
      <div className="mt-3 pt-3 border-t border-slate-800 flex flex-wrap items-center justify-between text-xs text-slate-400 gap-2">
        <div className="flex items-center gap-4">
          <span>
            Spread: <strong className="font-mono-num text-slate-200">${spreadUp.toFixed(3)}</strong>
          </span>
          <span>
            Book Liquidity: <strong className="font-mono-num text-slate-200">${totalLiquidity.toFixed(0)} USDC</strong>
          </span>
        </div>
        <div className="text-[11px] text-slate-500 font-mono">
          Settlement Rule: Binary 1.00 USDC on WIN / 0.00 on LOSS (Orderbook Depth Fill Enabled)
        </div>
      </div>
    </div>
  );
};
