import React from 'react';
import { Target, CheckCircle2, XCircle } from 'lucide-react';
import { PredictionSignal, ModelPrediction } from '../types';

interface OpportunityCenterProps {
  signal: PredictionSignal | null;
  prediction: ModelPrediction | null;
}

export const OpportunityCenter: React.FC<OpportunityCenterProps> = ({
  signal,
  prediction,
}) => {
  const action = signal?.action ?? 'SKIP';
  const score = signal?.signal_score ?? 0;
  const reason = signal?.decision_reason ?? 'Waiting for next evaluation cycle...';
  const breakdown = signal?.score_breakdown;
  const gates = signal?.gate_results ?? [];

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

  return (
    <div className="asmr-card p-6 h-full flex flex-col justify-between">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Target className="h-4 w-4 text-[#6c9bcf]" />
          <h2 className="text-xs font-bold text-[#7d8da1] dark:text-slate-400 uppercase tracking-wider">
            策略决策引擎与多因子评分
          </h2>
        </div>
        <span className="text-[10px] text-[#7d8da1] font-mono">
          7道严苛安全门槛 &bull; 综合得分 [0, 100]
        </span>
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
            <p className="text-xs text-[#7d8da1] dark:text-slate-400 mt-0.5">{reason}</p>
          </div>
        </div>

        {/* Opportunity Score Indicator */}
        <div className="flex items-center gap-3 bg-white dark:bg-[#202528] px-3.5 py-2 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
          <div>
            <span className="text-[10px] text-[#7d8da1] uppercase font-semibold block">综合评分</span>
            <span className="text-xl font-black text-[#363949] dark:text-white font-mono-num">{(typeof score === 'number' && !isNaN(score) ? score : 0).toFixed(1)}</span>
            <span className="text-[10px] text-[#7d8da1]">/100</span>
          </div>
          <div className="text-right">
            <span className="text-[10px] text-[#7d8da1] uppercase font-semibold block">等级</span>
            <span
              className={`text-[11px] font-bold font-mono px-2 py-0.5 rounded-md ${
                score >= 80
                  ? 'bg-[#1b9c85]/20 text-[#1b9c85]'
                  : score >= 60
                  ? 'bg-[#6c9bcf]/20 text-[#6c9bcf]'
                  : 'bg-slate-100 dark:bg-slate-800 text-[#7d8da1]'
              }`}
            >
              {breakdown?.score_tier ?? (score >= 60 ? 'PASS' : 'HOLD')}
            </span>
          </div>
        </div>
      </div>

      {/* Grid: 7 Hard Filter Gates + Scoring Breakdown */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* 1. The 7 Hard Filter Gates */}
        <div className="bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800 p-3.5 rounded-2xl">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-bold text-[#363949] dark:text-white uppercase tracking-wide">
              7道严苛安全硬门槛
            </span>
            <span className="text-[10px] text-[#7d8da1] font-mono">违背即关闸</span>
          </div>

          <div className="space-y-1.5">
            {gates.length > 0 ? (
              gates.map((g) => (
                <div
                  key={g.gate_name}
                  className="flex items-center justify-between text-xs py-1.5 px-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-100 dark:border-slate-800 shadow-sm"
                >
                  <div className="flex items-center gap-2">
                    {g.passed ? (
                      <CheckCircle2 className="h-3.5 w-3.5 text-[#1b9c85] shrink-0" />
                    ) : (
                      <XCircle className="h-3.5 w-3.5 text-[#ff0060] shrink-0" />
                    )}
                    <span className="font-semibold text-[#363949] dark:text-white">{g.gate_name}</span>
                  </div>

                  <div className="flex items-center gap-2 font-mono text-[11px]">
                    <span className="text-[#7d8da1]">{g.value}</span>
                  </div>
                </div>
              ))
            ) : (
              <div className="text-xs text-[#7d8da1] py-3 text-center font-mono">
                暂无活跃门槛评估数据。
              </div>
            )}
          </div>
        </div>

        {/* 2. Score Breakdown Points */}
        <div className="bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800 p-3.5 rounded-2xl">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-bold text-[#363949] dark:text-white uppercase tracking-wide">
              多因子评分细则
            </span>
            <span className="text-[10px] text-[#7d8da1] font-mono">满分 100 分</span>
          </div>

          <div className="space-y-2.5 text-xs font-medium">
            <div>
              <div className="flex justify-between text-[#7d8da1] dark:text-slate-400 mb-1">
                <span>模型胜率置信度得分</span>
                <span className="font-mono-num font-bold text-[#363949] dark:text-white">
                  {(breakdown?.probability_points ?? 0).toFixed(1)} 分
                </span>
              </div>
              <div className="h-1.5 w-full bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
                <div
                  className="h-full bg-[#6c9bcf] rounded-full"
                  style={{ width: `${((breakdown?.probability_points ?? 0) / 30) * 100}%` }}
                />
              </div>
            </div>

            <div>
              <div className="flex justify-between text-[#7d8da1] dark:text-slate-400 mb-1">
                <span>扣除滑点手续费净期望</span>
                <span className="font-mono-num font-bold text-[#363949] dark:text-white">
                  {(breakdown?.edge_points ?? 0).toFixed(1)} 分
                </span>
              </div>
              <div className="h-1.5 w-full bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
                <div
                  className="h-full bg-[#1b9c85] rounded-full"
                  style={{ width: `${((breakdown?.edge_points ?? 0) / 25) * 100}%` }}
                />
              </div>
            </div>

            <div>
              <div className="flex justify-between text-[#7d8da1] dark:text-slate-400 mb-1">
                <span>订单簿微观买卖盘失衡</span>
                <span className="font-mono-num font-bold text-[#363949] dark:text-white">
                  {(breakdown?.obi_points ?? 0).toFixed(1)} 分
                </span>
              </div>
              <div className="h-1.5 w-full bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
                <div
                  className="h-full bg-[#f7d154] rounded-full"
                  style={{ width: `${((breakdown?.obi_points ?? 0) / 15) * 100}%` }}
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
