import React, { useState } from 'react';
import { Timer, ArrowUpRight, ArrowDownRight, Zap, CheckCircle2, AlertCircle, Loader2 } from 'lucide-react';
import { MarketDisplayInfo, MarketBookSummary, ModelPrediction, Asset, TradingMode } from '../types';
import { api } from '../services/api';

interface PolymarketRoundCardProps {
  market: MarketDisplayInfo | null;
  book: MarketBookSummary | null;
  prediction: ModelPrediction | null;
  tradingMode?: TradingMode;
  activeAsset?: Asset;
  onTradeExecuted?: () => void;
}

export const PolymarketRoundCard: React.FC<PolymarketRoundCardProps> = ({
  market,
  book,
  prediction,
  tradingMode = 'paper',
  activeAsset = 'BTC',
  onTradeExecuted,
}) => {
  const [stake, setStake] = useState<number>(1.0);
  const [isExecuting, setIsExecuting] = useState<boolean>(false);
  const [execStatus, setExecStatus] = useState<{ success: boolean; msg: string } | null>(null);

  const handleManualOrder = async (side: 'UP' | 'DOWN') => {
    if (!market) {
      setExecStatus({ success: false, msg: '暂无当前市场信息' });
      return;
    }
    if (stake <= 0) {
      setExecStatus({ success: false, msg: '下单金额必须大于 0' });
      return;
    }

    try {
      setIsExecuting(true);
      setExecStatus(null);
      const res = await api.executeManualTrade({
        market_id: market.id,
        asset: activeAsset,
        side,
        stake,
        mode: tradingMode,
      });

      if (res.success) {
        setExecStatus({
          success: true,
          msg: res.message || `手动下单成功！(${side} $${stake.toFixed(2)})`,
        });
        if (onTradeExecuted) {
          onTradeExecuted();
        }
      } else {
        setExecStatus({ success: false, msg: res.message || '下单失败' });
      }
    } catch (err: any) {
      setExecStatus({ success: false, msg: err.message || '网络异常，下单失败' });
    } finally {
      setIsExecuting(false);
      setTimeout(() => {
        setExecStatus((prev) => (prev?.success ? null : prev));
      }, 5000);
    }
  };
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
              ID: {market?.id ? market.id.slice(0, 16) : '等待市场中...'}
            </span>
          </div>
          <h3 className="text-base font-extrabold text-[#363949] dark:text-white mt-1 tracking-tight">
            {market?.question ?? '进行中的 5 分钟加密货币涨跌期权合约'}
          </h3>
        </div>

        {/* 5-Minute Round Countdown */}
        <div className="bg-[#f6f6f9] dark:bg-[#181a1e] p-3 rounded-2xl min-w-[190px] border border-slate-100 dark:border-slate-800">
          <div className="flex items-center justify-between text-xs mb-1.5 font-semibold">
            <span className="text-[#7d8da1] dark:text-slate-400 flex items-center gap-1.5">
              <Timer className="h-3.5 w-3.5 text-[#6c9bcf]" />
              交割窗口:
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
              <ArrowUpRight className="h-4 w-4" /> 看涨合约 (UP)
            </span>
            <span className="text-[11px] text-[#7d8da1] font-mono">
              盘口胜率: <strong className="text-[#1b9c85]">{(typeof impliedUp === 'number' && !isNaN(impliedUp) ? impliedUp : 50).toFixed(1)}%</strong>
            </span>
          </div>

          <div className="grid grid-cols-2 gap-2.5 mb-3">
            <div className="bg-white dark:bg-[#202528] p-2.5 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">买一价 (Bid)</span>
              <span className="text-base font-mono-num font-extrabold text-[#1b9c85]">
                ${(typeof upBid === 'number' && !isNaN(upBid) ? upBid : 0.5).toFixed(3)}
              </span>
            </div>
            <div className="bg-white dark:bg-[#202528] p-2.5 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">卖一价 (Ask)</span>
              <span className="text-base font-mono-num font-extrabold text-[#363949] dark:text-white">
                ${(typeof upAsk === 'number' && !isNaN(upAsk) ? upAsk : 0.51).toFixed(3)}
              </span>
            </div>
          </div>

          <div className="text-xs space-y-1 font-medium">
            <div className="flex justify-between text-[#7d8da1] dark:text-slate-400">
              <span>模型胜率:</span>
              <span className="font-mono-num font-bold text-[#363949] dark:text-white">{(typeof modelUp === 'number' && !isNaN(modelUp) ? modelUp : 50).toFixed(1)}%</span>
            </div>
            <div className="flex justify-between text-[#7d8da1] dark:text-slate-400">
              <span>净数学期望:</span>
              <span
                className={`font-mono-num font-bold ${
                  modelUp / 100 - upAsk >= 0.05 ? 'text-[#1b9c85]' : 'text-[#7d8da1]'
                }`}
              >
                {(((typeof modelUp === 'number' ? modelUp : 50) / 100 - (typeof upAsk === 'number' ? upAsk : 0.51)) * 100).toFixed(1)}%
              </span>
            </div>
          </div>
        </div>

        {/* DOWN Side Book */}
        <div className="p-4 rounded-2xl border border-[#ff0060]/20 bg-[#ff0060]/5">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-extrabold text-[#ff0060] flex items-center gap-1 uppercase tracking-wider">
              <ArrowDownRight className="h-4 w-4" /> 看跌合约 (DOWN)
            </span>
            <span className="text-[11px] text-[#7d8da1] font-mono">
              盘口胜率: <strong className="text-[#ff0060]">{(typeof impliedDown === 'number' && !isNaN(impliedDown) ? impliedDown : 50).toFixed(1)}%</strong>
            </span>
          </div>

          <div className="grid grid-cols-2 gap-2.5 mb-3">
            <div className="bg-white dark:bg-[#202528] p-2.5 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">买一价 (Bid)</span>
              <span className="text-base font-mono-num font-extrabold text-[#ff0060]">
                ${(typeof downBid === 'number' && !isNaN(downBid) ? downBid : 0.5).toFixed(3)}
              </span>
            </div>
            <div className="bg-white dark:bg-[#202528] p-2.5 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">卖一价 (Ask)</span>
              <span className="text-base font-mono-num font-extrabold text-[#363949] dark:text-white">
                ${(typeof downAsk === 'number' && !isNaN(downAsk) ? downAsk : 0.51).toFixed(3)}
              </span>
            </div>
          </div>

          <div className="text-xs space-y-1 font-medium">
            <div className="flex justify-between text-[#7d8da1] dark:text-slate-400">
              <span>模型胜率:</span>
              <span className="font-mono-num font-bold text-[#363949] dark:text-white">{(typeof modelDown === 'number' && !isNaN(modelDown) ? modelDown : 50).toFixed(1)}%</span>
            </div>
            <div className="flex justify-between text-[#7d8da1] dark:text-slate-400">
              <span>净数学期望:</span>
              <span
                className={`font-mono-num font-bold ${
                  modelDown / 100 - downAsk >= 0.05 ? 'text-[#1b9c85]' : 'text-[#7d8da1]'
                }`}
              >
                {(((typeof modelDown === 'number' ? modelDown : 50) / 100 - (typeof downAsk === 'number' ? downAsk : 0.51)) * 100).toFixed(1)}%
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Manual Quick Trade Panel */}
      <div className="mt-4 p-3.5 bg-slate-50 dark:bg-[#181a1e] rounded-2xl border border-slate-200/70 dark:border-slate-800">
        <div className="flex items-center justify-between mb-2.5">
          <div className="flex items-center gap-1.5">
            <Zap className="h-4 w-4 text-amber-500" />
            <span className="text-xs font-bold text-[#363949] dark:text-white">
              手动快速下单
            </span>
            <span className="text-[10px] text-[#7d8da1] font-mono">
              ({tradingMode === 'live' ? '⚡ Polymarket 实盘' : '🛡️ 模拟盘撮合'})
            </span>
          </div>

          {/* Quick Stake Selector */}
          <div className="flex items-center gap-1">
            {[1, 2, 5, 10].map((amt) => (
              <button
                key={amt}
                type="button"
                onClick={() => setStake(amt)}
                className={`px-2 py-0.5 text-[11px] font-mono font-bold rounded-lg border transition cursor-pointer ${
                  stake === amt
                    ? 'bg-[#6c9bcf] text-white border-[#6c9bcf] shadow-sm'
                    : 'bg-white dark:bg-[#202528] text-[#7d8da1] border-slate-200 dark:border-slate-700 hover:text-[#363949]'
                }`}
              >
                ${amt}
              </button>
            ))}
          </div>
        </div>

        {/* Action Buttons: Buy UP / Buy DOWN */}
        <div className="grid grid-cols-2 gap-2.5">
          <button
            type="button"
            disabled={isExecuting || !market}
            onClick={() => handleManualOrder('UP')}
            className="flex items-center justify-center gap-1.5 py-2 px-3 rounded-xl bg-emerald-600 hover:bg-emerald-700 text-white font-extrabold text-xs shadow-sm transition disabled:opacity-50 cursor-pointer"
          >
            {isExecuting ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <ArrowUpRight className="h-4 w-4" />
            )}
            <span>买入看涨 UP (${stake.toFixed(0)})</span>
          </button>

          <button
            type="button"
            disabled={isExecuting || !market}
            onClick={() => handleManualOrder('DOWN')}
            className="flex items-center justify-center gap-1.5 py-2 px-3 rounded-xl bg-[#ff0060] hover:bg-[#e00055] text-white font-extrabold text-xs shadow-sm transition disabled:opacity-50 cursor-pointer"
          >
            {isExecuting ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <ArrowDownRight className="h-4 w-4" />
            )}
            <span>买入看跌 DOWN (${stake.toFixed(0)})</span>
          </button>
        </div>

        {/* Feedback Alert */}
        {execStatus && (
          <div
            className={`mt-2 p-2 rounded-xl text-xs flex items-center gap-1.5 font-medium animate-fade-in ${
              execStatus.success
                ? 'bg-emerald-50 dark:bg-emerald-950/40 text-[#1b9c85] border border-emerald-200 dark:border-emerald-800'
                : 'bg-rose-50 dark:bg-rose-950/40 text-[#ff0060] border border-rose-200 dark:border-rose-900'
            }`}
          >
            {execStatus.success ? (
              <CheckCircle2 className="h-3.5 w-3.5 shrink-0" />
            ) : (
              <AlertCircle className="h-3.5 w-3.5 shrink-0" />
            )}
            <span>{execStatus.msg}</span>
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="mt-4 pt-3 border-t border-slate-100 dark:border-slate-800 flex items-center justify-between text-xs text-[#7d8da1] font-mono">
        <span>买卖价差: ${(typeof spreadUp === 'number' && !isNaN(spreadUp) ? spreadUp : 0.01).toFixed(3)}</span>
        <span>流动性深度: ${(typeof totalLiquidity === 'number' && !isNaN(totalLiquidity) ? totalLiquidity : 1000).toFixed(0)} USDC</span>
      </div>
    </div>
  );
};
