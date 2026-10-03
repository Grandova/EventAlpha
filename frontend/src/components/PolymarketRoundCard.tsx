import React from 'react';
import { Timer, ArrowUpRight, ArrowDownRight } from 'lucide-react';
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
    <div className="asmr-card p-6 h-full flex flex-col justify-between">
      {/* Header: Market Question & 5M Countdown Timer */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-4">
        <div>
          <div className="flex items-center gap-2">
            <span className="text-[10px] font-extrabold bg-[#6c9bcf]/15 text-[#6c9bcf] px-2.5 py-0.5 rounded-full uppercase tracking-wider font-mono">
              Polymarket 5M
            </span>
            <span className="text-xs text-[#7d8da1] dark:text-slate-400 font-mono">
              ID: {market?.id ? market.id.slice(0, 16) : 'Awaiting...'}
            </span>
          </div>
          <h3 className="text-base font-extrabold text-[#363949] dark:text-white mt-1 tracking-tight">
            {market?.question ?? 'Active 5-Minute Crypto Up/Down Contract'}
          </h3>
        </div>

        {/* 5-Minute Round Countdown */}
        <div className="bg-[#f6f6f9] dark:bg-[#181a1e] p-3 rounded-2xl min-w-[190px] border border-slate-100 dark:border-slate-800">
          <div className="flex items-center justify-between text-xs mb-1.5 font-semibold">
            <span className="text-[#7d8da1] dark:text-slate-400 flex items-center gap-1.5">
              <Timer className="h-3.5 w-3.5 text-[#6c9bcf]" />
              Window:
            </span>
            <span
              className={`font-mono-num font-black text-sm ${
                remainingSecs <= 30
                  ? 'text-[#ff0060] animate-pulse'
                  : remainingSecs <= 60
                  ? 'text-[#f7d154]'
                  : 'text-[#1b9c85]'
              }`}
            >
              {timerStr}
            </span>
          </div>
          <div className="h-2 w-full bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
            <div
              className={`h-full transition-all duration-1000 ${
                remainingSecs <= 30
                  ? 'bg-[#ff0060]'
                  : remainingSecs <= 60
                  ? 'bg-[#f7d154]'
                  : 'bg-[#1b9c85]'
              }`}
              style={{ width: `${progressPct}%` }}
            />
          </div>
        </div>
      </div>

      {/* Main Orderbook Prices & Microstructure Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* UP Side Book */}
        <div className="p-4 rounded-2xl border border-[#1b9c85]/20 bg-[#1b9c85]/5">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-extrabold text-[#1b9c85] flex items-center gap-1 uppercase tracking-wider">
              <ArrowUpRight className="h-4 w-4" /> UP Outcome
            </span>
            <span className="text-[11px] text-[#7d8da1] font-mono">
              Implied: <strong className="text-[#1b9c85]">{impliedUp.toFixed(1)}%</strong>
            </span>
          </div>

          <div className="grid grid-cols-2 gap-2.5 mb-3">
            <div className="bg-white dark:bg-[#202528] p-2.5 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">Best Bid</span>
              <span className="text-base font-mono-num font-extrabold text-[#1b9c85]">
                ${upBid.toFixed(3)}
              </span>
            </div>
            <div className="bg-white dark:bg-[#202528] p-2.5 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">Best Ask</span>
              <span className="text-base font-mono-num font-extrabold text-[#363949] dark:text-white">
                ${upAsk.toFixed(3)}
              </span>
            </div>
          </div>

          <div className="text-xs space-y-1 font-medium">
            <div className="flex justify-between text-[#7d8da1] dark:text-slate-400">
              <span>Model P(Up):</span>
              <span className="font-mono-num font-bold text-[#363949] dark:text-white">{modelUp.toFixed(1)}%</span>
            </div>
            <div className="flex justify-between text-[#7d8da1] dark:text-slate-400">
              <span>Net Edge:</span>
              <span
                className={`font-mono-num font-bold ${
                  modelUp / 100 - upAsk >= 0.05 ? 'text-[#1b9c85]' : 'text-[#7d8da1]'
                }`}
              >
                {((modelUp / 100 - upAsk) * 100).toFixed(1)}%
              </span>
            </div>
          </div>
        </div>

        {/* DOWN Side Book */}
        <div className="p-4 rounded-2xl border border-[#ff0060]/20 bg-[#ff0060]/5">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-extrabold text-[#ff0060] flex items-center gap-1 uppercase tracking-wider">
              <ArrowDownRight className="h-4 w-4" /> DOWN Outcome
            </span>
            <span className="text-[11px] text-[#7d8da1] font-mono">
              Implied: <strong className="text-[#ff0060]">{impliedDown.toFixed(1)}%</strong>
            </span>
          </div>

          <div className="grid grid-cols-2 gap-2.5 mb-3">
            <div className="bg-white dark:bg-[#202528] p-2.5 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">Best Bid</span>
              <span className="text-base font-mono-num font-extrabold text-[#ff0060]">
                ${downBid.toFixed(3)}
              </span>
            </div>
            <div className="bg-white dark:bg-[#202528] p-2.5 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">Best Ask</span>
              <span className="text-base font-mono-num font-extrabold text-[#363949] dark:text-white">
                ${downAsk.toFixed(3)}
              </span>
            </div>
          </div>

          <div className="text-xs space-y-1 font-medium">
            <div className="flex justify-between text-[#7d8da1] dark:text-slate-400">
              <span>Model P(Down):</span>
              <span className="font-mono-num font-bold text-[#363949] dark:text-white">{modelDown.toFixed(1)}%</span>
            </div>
            <div className="flex justify-between text-[#7d8da1] dark:text-slate-400">
              <span>Net Edge:</span>
              <span
                className={`font-mono-num font-bold ${
                  modelDown / 100 - downAsk >= 0.05 ? 'text-[#1b9c85]' : 'text-[#7d8da1]'
                }`}
              >
                {((modelDown / 100 - downAsk) * 100).toFixed(1)}%
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="mt-4 pt-3 border-t border-slate-100 dark:border-slate-800 flex items-center justify-between text-xs text-[#7d8da1] font-mono">
        <span>Spread: ${spreadUp.toFixed(3)}</span>
        <span>Liquidity: ${totalLiquidity.toFixed(0)} USDC</span>
      </div>
    </div>
  );
};
