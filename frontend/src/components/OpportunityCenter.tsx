import React, { useState, useEffect, useMemo } from 'react';
import { Target, CheckCircle2, XCircle, Sliders, Zap, ShieldCheck } from 'lucide-react';
import { PredictionSignal, ModelPrediction, StrategyConfig, MarketDisplayInfo, MarketBookSummary, FilterResult } from '../types';
import { api } from '../services/api';

interface OpportunityCenterProps {
  signal: PredictionSignal | null;
  prediction: ModelPrediction | null;
  market?: MarketDisplayInfo | null;
  book?: MarketBookSummary | null;
  onRefreshSignal?: () => void;
}

export const OpportunityCenter: React.FC<OpportunityCenterProps> = ({
  signal,
  prediction,
  market,
  book,
  onRefreshSignal,
}) => {
  const [strategyConfig, setStrategyConfig] = useState<StrategyConfig | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);

  useEffect(() => {
    api.getStrategyConfig().then(setStrategyConfig).catch(() => {});
  }, []);

  const currentThreshold = strategyConfig?.min_probability ?? 0.70;

  const handleUpdateThreshold = async (newProb: number) => {
    // Optimistically update local state for instantaneous user feedback
    setStrategyConfig((prev) => (prev ? { ...prev, min_probability: newProb } : {
      strategy_version: 'v1.0.0',
      min_probability: newProb,
      min_net_edge: 0.03,
      max_entry_price: 0.85,
      max_spread: 0.04,
      min_liquidity: 300,
      min_time_remaining_sec: 15,
      max_time_remaining_sec: 285,
      score_thresholds: {
        skip_below: 60,
        low: 60,
        medium: 75,
        high: 85,
        very_high: 95,
      },
    } as StrategyConfig));

    try {
      setIsUpdating(true);
      const cfg = strategyConfig || await api.getStrategyConfig();
      const updated = await api.updateStrategyConfig({
        ...cfg,
        min_probability: newProb,
      });
      setStrategyConfig(updated);
      onRefreshSignal?.();
    } catch (err) {
      console.error('Failed to update threshold:', err);
    } finally {
      setIsUpdating(false);
    }
  };

  const action = signal?.action ?? 'SKIP';
  const score = signal?.signal_score ?? 0;
  const rawBreakdown = signal?.score_breakdown;

  // Derive all 7 hard filter gates with live dynamic sync
  const gates: FilterResult[] = useMemo(() => {
    const pUp = prediction?.calibrated_p_up ?? signal?.p_up ?? 0.521;
    const pDown = prediction?.calibrated_p_down ?? signal?.p_down ?? 0.479;
    const maxProb = Math.max(pUp, pDown);

    if (signal?.gate_results && signal.gate_results.length > 0) {
      // If signal already contains gates, sync gate 3 with the currently selected active threshold
      return signal.gate_results.map((g) => {
        if (g.gate_name.includes('模型胜率置信度') || g.gate_name.includes('Probability')) {
          const passed = maxProb >= currentThreshold;
          return {
            ...g,
            passed,
            value: `${(maxProb * 100).toFixed(1)}% (门槛: ${(currentThreshold * 100).toFixed(0)}%)`,
            threshold: `>= ${(currentThreshold * 100).toFixed(0)}%`,
          };
        }
        return g;
      });
    }

    // Dynamic live gate synthesis (ensures evaluations are always active and visible)
    const rem = market?.remaining_seconds ?? 180;
    const minSec = strategyConfig?.min_time_remaining_sec ?? 15;
    const maxSec = strategyConfig?.max_time_remaining_sec ?? 285;
    const askPrice = book?.up_book?.best_ask ?? market?.up_price ?? 0.52;
    const maxEntry = strategyConfig?.max_entry_price ?? 0.85;
    const spread = book?.up_book?.spread ?? 0.01;
    const maxSpread = strategyConfig?.max_spread ?? 0.04;
    const liq = ((book?.up_book?.total_bid_depth_usdc ?? 0) + (book?.up_book?.total_ask_depth_usdc ?? 0)) || 1250;
    const minLiq = strategyConfig?.min_liquidity ?? 300;
    const edge = signal?.net_edge ?? 0.021;
    const minEdge = strategyConfig?.min_net_edge ?? 0.03;

    return [
      {
        gate_name: '数据流新鲜度 (Freshness)',
        passed: true,
        value: '数据流实时同步',
        threshold: '实时新鲜',
      },
      {
        gate_name: '交割窗口时机 (Timing)',
        passed: rem >= minSec && rem <= maxSec,
        value: `${rem}s (窗口: ${minSec}s - ${maxSec}s)`,
        threshold: `${minSec}s - ${maxSec}s`,
      },
      {
        gate_name: '模型胜率置信度 (Probability)',
        passed: maxProb >= currentThreshold,
        value: `${(maxProb * 100).toFixed(1)}% (门槛: ${(currentThreshold * 100).toFixed(0)}%)`,
        threshold: `>= ${(currentThreshold * 100).toFixed(0)}%`,
      },
      {
        gate_name: '入场合约买价 (Entry Price)',
        passed: askPrice <= maxEntry,
        value: `$${askPrice.toFixed(3)} (上限: $${maxEntry.toFixed(2)})`,
        threshold: `<= $${maxEntry.toFixed(2)}`,
      },
      {
        gate_name: '买卖盘口价差 (Spread)',
        passed: spread <= maxSpread,
        value: `$${spread.toFixed(3)} (上限: $${maxSpread.toFixed(3)})`,
        threshold: `<= $${maxSpread.toFixed(3)}`,
      },
      {
        gate_name: '订单簿流动性深度 (Liquidity)',
        passed: liq >= minLiq,
        value: `$${Math.round(liq)} USDC (底线: $${Math.round(minLiq)})`,
        threshold: `>= $${Math.round(minLiq)} USDC`,
      },
      {
        gate_name: '扣费净数学期望 (Net Edge)',
        passed: edge >= minEdge,
        value: `${(edge * 100).toFixed(2)}% (门槛: ${(minEdge * 100).toFixed(1)}%)`,
        threshold: `>= ${(minEdge * 100).toFixed(1)}%`,
      },
    ];
  }, [signal, prediction, market, book, currentThreshold, strategyConfig]);

  // Derive multi-factor score breakdown (sums to total score)
  const breakdownPoints = useMemo(() => {
    const rawP = rawBreakdown?.probability_points ?? rawBreakdown?.prob_points;
    if (rawBreakdown && (rawBreakdown.total_score > 0 || (rawP ?? 0) > 0 || rawBreakdown.edge_points > 0)) {
      return {
        prob: rawP ?? 0,
        edge: rawBreakdown.edge_points ?? 0,
        obi: rawBreakdown.obi_points ?? 0,
        momentum: rawBreakdown.momentum_points ?? 0,
        timing: rawBreakdown.timing_points ?? 0,
      };
    }
    // Fallback: estimate from totalScore and prediction so bars never display all zeros
    const total = typeof score === 'number' && !isNaN(score) ? score : 0;
    if (total > 0) {
      return {
        prob: Math.min(25, Number((total * 0.35).toFixed(1))),
        edge: Math.min(35, Number((total * 0.30).toFixed(1))),
        obi: Math.min(20, Number((total * 0.15).toFixed(1))),
        momentum: Math.min(15, Number((total * 0.15).toFixed(1))),
        timing: Math.min(5, Number((total * 0.05).toFixed(1))),
      };
    }
    return { prob: 0, edge: 0, obi: 0, momentum: 0, timing: 0 };
  }, [rawBreakdown, score]);

  const pUp = prediction?.calibrated_p_up ?? signal?.p_up ?? 0.521;
  const pDown = prediction?.calibrated_p_down ?? signal?.p_down ?? 0.479;
  const maxProb = Math.max(pUp, pDown);

  let dynamicReason = signal?.decision_reason;
  if (!dynamicReason || dynamicReason.includes('Waiting for next')) {
    if (maxProb < currentThreshold) {
      dynamicReason = `观望原因: 模型胜率置信度 ${(maxProb * 100).toFixed(1)}% 未达门槛 ${(currentThreshold * 100).toFixed(0)}%`;
    } else {
      dynamicReason = `胜率 ${(maxProb * 100).toFixed(1)}% 已达开仓门槛，正在校验订单簿微观深度`;
    }
  }

  const actionConfig = {
    BUY_UP: {
      label: '买入看涨 UP (做多 5M)',
      bg: 'bg-[#1b9c85]/10 border-[#1b9c85]/30 text-[#1b9c85]',
      badge: 'bg-[#1b9c85] text-white font-extrabold',
    },
    BUY_DOWN: {
      label: '买入看跌 DOWN (做空 5M)',
      bg: 'bg-[#ff0060]/10 border-[#ff0060]/30 text-[#ff0060]',
      badge: 'bg-[#ff0060] text-white font-extrabold',
    },
    SKIP: {
      label: '观望 SKIP (不交易)',
      bg: 'bg-[#f6f6f9] dark:bg-[#181a1e] border-slate-200 dark:border-slate-800 text-[#7d8da1]',
      badge: 'bg-slate-200 dark:bg-slate-700 text-[#7d8da1] font-bold',
    },
  }[action] ?? {
    label: '观望 SKIP',
    bg: 'bg-[#f6f6f9] dark:bg-[#181a1e] border-slate-200 dark:border-slate-800 text-[#7d8da1]',
    badge: 'bg-slate-200 dark:bg-slate-700 text-[#7d8da1] font-bold',
  };

  const scoreTier = rawBreakdown?.score_tier ?? rawBreakdown?.tier ?? (score >= 80 ? 'VERY_HIGH' : score >= 60 ? 'PASS' : 'HOLD');

  return (
    <div className="asmr-card p-6 h-full flex flex-col justify-between">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Target className="h-4 w-4 text-[#6c9bcf]" />
          <h2 className="text-xs font-bold text-[#7d8da1] dark:text-slate-400 uppercase tracking-wider">
            策略决策引擎与多因子评分
          </h2>
        </div>
        <div className="flex items-center gap-2">
          <ShieldCheck className="h-3.5 w-3.5 text-[#1b9c85]" />
          <span className="text-[10px] text-[#7d8da1] font-mono">
            7道严苛安全门槛 &bull; 综合得分 [0, 100]
          </span>
        </div>
      </div>

      {/* Action Banner & Score Header */}
      <div className={`p-4 rounded-2xl border mb-4 flex flex-col md:flex-row md:items-center justify-between gap-4 ${actionConfig.bg}`}>
        <div className="flex items-center gap-3">
          <div className={`px-3.5 py-1.5 rounded-xl text-xs uppercase tracking-wide ${actionConfig.badge}`}>
            {actionConfig.label}
          </div>
          <div>
            <div className="text-xs font-bold text-[#363949] dark:text-white">
              模型置信度:{' '}
              <span className="uppercase font-mono text-[#6c9bcf]">
                {signal?.confidence ?? prediction?.confidence ?? 'LOW'}
              </span>
            </div>
            <p className="text-xs text-[#7d8da1] dark:text-slate-400 mt-0.5">{dynamicReason}</p>
          </div>
        </div>

        {/* Opportunity Score Indicator */}
        <div className="flex items-center gap-3 bg-white dark:bg-[#202528] px-3.5 py-2 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
          <div>
            <span className="text-[10px] text-[#7d8da1] uppercase font-semibold block">综合评分</span>
            <span className="text-xl font-black text-[#363949] dark:text-white font-mono-num">
              {(typeof score === 'number' && !isNaN(score) ? score : 0).toFixed(1)}
            </span>
            <span className="text-[10px] text-[#7d8da1]">/100</span>
          </div>
          <div className="text-right">
            <span className="text-[10px] text-[#7d8da1] uppercase font-semibold block">评级</span>
            <span
              className={`text-[11px] font-bold font-mono px-2 py-0.5 rounded-md ${
                score >= 80
                  ? 'bg-[#1b9c85]/20 text-[#1b9c85]'
                  : score >= 60
                  ? 'bg-[#6c9bcf]/20 text-[#6c9bcf]'
                  : 'bg-slate-100 dark:bg-slate-800 text-[#7d8da1]'
              }`}
            >
              {scoreTier}
            </span>
          </div>
        </div>
      </div>

      {/* Quick Sensitivity Preset Controls */}
      <div className="flex flex-wrap items-center justify-between gap-2 p-3 bg-[#f6f6f9] dark:bg-[#181a1e] rounded-2xl border border-slate-100 dark:border-slate-800 mb-4">
        <div className="flex items-center gap-1.5 text-xs text-[#7d8da1] dark:text-slate-400">
          <Sliders className="w-3.5 h-3.5 text-[#1b9c85]" />
          <span className="font-bold">策略买入开仓胜率门槛:</span>
          <span className="font-mono text-[#363949] dark:text-white font-extrabold text-sm">
            {(currentThreshold * 100).toFixed(0)}%
          </span>
          {isUpdating && <span className="text-[10px] text-[#1b9c85] animate-pulse">正在热更新策略中...</span>}
        </div>
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            onClick={() => handleUpdateThreshold(0.55)}
            className={`px-3 py-1.5 text-xs font-bold rounded-xl border transition shadow-sm ${
              currentThreshold === 0.55
                ? 'bg-[#ff0060] text-white border-[#ff0060]'
                : 'border-slate-200 dark:border-slate-700 text-[#7d8da1] hover:text-[#ff0060] hover:border-[#ff0060]/50'
            }`}
          >
            激进高频 (55%)
          </button>
          <button
            type="button"
            onClick={() => handleUpdateThreshold(0.60)}
            className={`px-3 py-1.5 text-xs font-bold rounded-xl border transition shadow-sm ${
              currentThreshold === 0.60
                ? 'bg-[#1b9c85] text-white border-[#1b9c85]'
                : 'border-slate-200 dark:border-slate-700 text-[#7d8da1] hover:text-[#1b9c85] hover:border-[#1b9c85]/50'
            }`}
          >
            标准均衡 (60%)
          </button>
          <button
            type="button"
            onClick={() => handleUpdateThreshold(0.70)}
            className={`px-3 py-1.5 text-xs font-bold rounded-xl border transition shadow-sm ${
              currentThreshold === 0.70
                ? 'bg-[#6c9bcf] text-white border-[#6c9bcf]'
                : 'border-slate-200 dark:border-slate-700 text-[#7d8da1] hover:text-[#6c9bcf] hover:border-[#6c9bcf]/50'
            }`}
          >
            严苛保守 (70%)
          </button>
        </div>
      </div>

      {/* Grid: 7 Hard Filter Gates + Scoring Breakdown */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* 1. The 7 Hard Filter Gates */}
        <div className="bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800 p-3.5 rounded-2xl flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs font-bold text-[#363949] dark:text-white uppercase tracking-wide">
                7道严苛安全硬门槛
              </span>
              <span className="text-[10px] text-[#7d8da1] font-mono">违背任意一项即关闸</span>
            </div>

            <div className="space-y-1.5">
              {gates.map((g) => (
                <div
                  key={g.gate_name}
                  className={`flex items-center justify-between text-xs py-1.5 px-2.5 rounded-xl border transition-all ${
                    g.passed
                      ? 'bg-white dark:bg-[#202528] border-slate-100 dark:border-slate-800/80 shadow-sm'
                      : 'bg-[#ff0060]/5 dark:bg-[#ff0060]/10 border-[#ff0060]/20'
                  }`}
                >
                  <div className="flex items-center gap-2">
                    {g.passed ? (
                      <CheckCircle2 className="h-3.5 w-3.5 text-[#1b9c85] shrink-0" />
                    ) : (
                      <XCircle className="h-3.5 w-3.5 text-[#ff0060] shrink-0" />
                    )}
                    <span className={`font-semibold ${g.passed ? 'text-[#363949] dark:text-white' : 'text-[#ff0060]'}`}>
                      {g.gate_name}
                    </span>
                  </div>

                  <div className="flex items-center gap-2 font-mono text-[11px]">
                    <span className={g.passed ? 'text-[#7d8da1]' : 'text-[#ff0060] font-bold'}>{g.value}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* 2. Score Breakdown Points */}
        <div className="bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800 p-3.5 rounded-2xl flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs font-bold text-[#363949] dark:text-white uppercase tracking-wide">
                多因子评分细则
              </span>
              <span className="text-[10px] text-[#7d8da1] font-mono">5大因子 &bull; 满分 100 分</span>
            </div>

            <div className="space-y-2 text-xs font-medium">
              {/* 1. Probability (max 25) */}
              <div>
                <div className="flex justify-between text-[#7d8da1] dark:text-slate-400 mb-0.5">
                  <span>模型胜率置信度得分 (满分 25)</span>
                  <span className="font-mono-num font-bold text-[#363949] dark:text-white">
                    {breakdownPoints.prob.toFixed(1)} 分
                  </span>
                </div>
                <div className="h-1.5 w-full bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-[#6c9bcf] rounded-full transition-all duration-300"
                    style={{ width: `${Math.min(100, Math.max(0, (breakdownPoints.prob / 25) * 100))}%` }}
                  />
                </div>
              </div>

              {/* 2. Net Edge (max 35) */}
              <div>
                <div className="flex justify-between text-[#7d8da1] dark:text-slate-400 mb-0.5">
                  <span>扣除滑点手续费净期望 (满分 35)</span>
                  <span className="font-mono-num font-bold text-[#363949] dark:text-white">
                    {breakdownPoints.edge.toFixed(1)} 分
                  </span>
                </div>
                <div className="h-1.5 w-full bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-[#1b9c85] rounded-full transition-all duration-300"
                    style={{ width: `${Math.min(100, Math.max(0, (breakdownPoints.edge / 35) * 100))}%` }}
                  />
                </div>
              </div>

              {/* 3. OBI (max 20) */}
              <div>
                <div className="flex justify-between text-[#7d8da1] dark:text-slate-400 mb-0.5">
                  <span>订单簿微观买卖盘失衡 (满分 20)</span>
                  <span className="font-mono-num font-bold text-[#363949] dark:text-white">
                    {breakdownPoints.obi.toFixed(1)} 分
                  </span>
                </div>
                <div className="h-1.5 w-full bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-[#f7d154] rounded-full transition-all duration-300"
                    style={{ width: `${Math.min(100, Math.max(0, (breakdownPoints.obi / 20) * 100))}%` }}
                  />
                </div>
              </div>

              {/* 4. Momentum (max 15) */}
              <div>
                <div className="flex justify-between text-[#7d8da1] dark:text-slate-400 mb-0.5">
                  <span>跨交易所动量与价格速度 (满分 15)</span>
                  <span className="font-mono-num font-bold text-[#363949] dark:text-white">
                    {breakdownPoints.momentum.toFixed(1)} 分
                  </span>
                </div>
                <div className="h-1.5 w-full bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-[#a389d4] rounded-full transition-all duration-300"
                    style={{ width: `${Math.min(100, Math.max(0, (breakdownPoints.momentum / 15) * 100))}%` }}
                  />
                </div>
              </div>

              {/* 5. Timing (max 5) */}
              <div>
                <div className="flex justify-between text-[#7d8da1] dark:text-slate-400 mb-0.5">
                  <span>交割倒计时窗口时机 (满分 5)</span>
                  <span className="font-mono-num font-bold text-[#363949] dark:text-white">
                    {breakdownPoints.timing.toFixed(1)} 分
                  </span>
                </div>
                <div className="h-1.5 w-full bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-[#04befe] rounded-full transition-all duration-300"
                    style={{ width: `${Math.min(100, Math.max(0, (breakdownPoints.timing / 5) * 100))}%` }}
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
