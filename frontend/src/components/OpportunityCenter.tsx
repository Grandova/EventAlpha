import React from 'react';
import { Target, CheckCircle2, XCircle, AlertTriangle, Cpu, TrendingUp } from 'lucide-react';
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
      label: 'BUY UP (LONG 5M)',
      bg: 'bg-emerald-500/15 border-emerald-500/50 text-emerald-400',
      badge: 'bg-emerald-500 text-slate-950 font-black',
      glow: 'glow-emerald',
    },
    BUY_DOWN: {
      label: 'BUY DOWN (SHORT 5M)',
      bg: 'bg-rose-500/15 border-rose-500/50 text-rose-400',
      badge: 'bg-rose-500 text-white font-black',
      glow: 'glow-rose',
    },
    SKIP: {
      label: 'SKIP (NO TRADE / GATED)',
      bg: 'bg-slate-900 border-slate-800 text-slate-400',
      badge: 'bg-slate-800 text-slate-400 font-bold',
      glow: '',
    },
  }[action] ?? {
    label: 'SKIP',
    bg: 'bg-slate-900 border-slate-800 text-slate-400',
    badge: 'bg-slate-800 text-slate-400 font-bold',
    glow: '',
  };

  return (
    <div className="asmr-card p-6 h-full flex flex-col justify-between">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Target className="h-4 w-4 text-cyan-400" />
          <h2 className="text-xs font-bold text-slate-400 uppercase tracking-wider">
            Strategy Engine & Opportunity Scoring Rubric
          </h2>
        </div>
        <span className="text-[10px] text-slate-500 font-mono">
          Model: {signal?.model_version ?? 'logistic-v1'} &bull; 7 Hard Gates &bull; Score Range [0, 100]
        </span>
      </div>

      {/* Action Banner & Score Header */}
      <div className={`p-4 rounded-xl border mb-4 flex flex-col md:flex-row md:items-center justify-between gap-4 ${actionConfig.bg} ${actionConfig.glow}`}>
        <div className="flex items-center gap-4">
          <div className={`px-4 py-2 rounded-lg text-sm tracking-wider uppercase ${actionConfig.badge}`}>
            {actionConfig.label}
          </div>
          <div>
            <div className="text-xs font-semibold text-slate-300">
              Confidence:{' '}
              <strong className="text-white uppercase font-mono">
                {signal?.confidence ?? prediction?.confidence ?? 'LOW'}
              </strong>
              {signal?.target_side && ` &bull; Target Side: ${signal.target_side}`}
            </div>
            <p className="text-xs text-slate-400 mt-0.5">{reason}</p>
          </div>
        </div>

        {/* Opportunity Score Indicator */}
        <div className="flex items-center gap-4 bg-slate-950/60 px-4 py-2 rounded-lg border border-slate-800/80">
          <div>
            <span className="text-[10px] text-slate-400 uppercase font-semibold block">Opportunity Score</span>
            <span className="text-2xl font-black text-white font-mono-num">{score.toFixed(1)}</span>
            <span className="text-[10px] text-slate-500"> / 100</span>
          </div>
          <div className="text-right">
            <span className="text-[10px] text-slate-400 uppercase font-semibold block">Score Tier</span>
            <span
              className={`text-xs font-bold font-mono px-2 py-0.5 rounded ${
                score >= 80
                  ? 'bg-emerald-950 text-emerald-400 border border-emerald-800'
                  : score >= 60
                  ? 'bg-cyan-950 text-cyan-400 border border-cyan-800'
                  : 'bg-slate-900 text-slate-400 border border-slate-800'
              }`}
            >
              {breakdown?.score_tier ?? (score >= 60 ? 'ELIGIBLE' : 'BELOW_THRESHOLD')}
            </span>
          </div>
        </div>
      </div>

      {/* Grid: 7 Hard Filter Gates + Scoring Breakdown */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* 1. The 7 Hard Filter Gates */}
        <div className="bg-slate-900/60 border border-slate-800 p-3.5 rounded-lg">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-bold text-slate-300 uppercase tracking-wide">
              The 7 Hard Filter Gates
            </span>
            <span className="text-[10px] text-slate-500 font-mono">Fail-Closed Invariant</span>
          </div>

          <div className="space-y-1.5">
            {gates.length > 0 ? (
              gates.map((g) => (
                <div
                  key={g.gate_name}
                  className="flex items-center justify-between text-xs py-1 px-2 rounded bg-slate-950/40 border border-slate-800/50"
                >
                  <div className="flex items-center gap-2">
                    {g.passed ? (
                      <CheckCircle2 className="h-3.5 w-3.5 text-emerald-400 shrink-0" />
                    ) : (
                      <XCircle className="h-3.5 w-3.5 text-rose-500 shrink-0" />
                    )}
                    <span className="font-semibold text-slate-300">{g.gate_name}</span>
                  </div>

                  <div className="flex items-center gap-2 font-mono text-[11px]">
                    <span className="text-slate-400">{g.value}</span>
                    <span className="text-slate-600">req: {g.threshold}</span>
                  </div>
                </div>
              ))
            ) : (
              <div className="text-xs text-slate-500 py-2 text-center font-mono">
                No active gate evaluation yet.
              </div>
            )}
          </div>
        </div>

        {/* 2. Score Breakdown Points */}
        <div className="bg-slate-900/60 border border-slate-800 p-3.5 rounded-lg">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-bold text-slate-300 uppercase tracking-wide">
              Score Rubric Breakdown
            </span>
            <span className="text-[10px] text-slate-500 font-mono">Max: 100 Points</span>
          </div>

          <div className="space-y-2 text-xs">
            <div>
              <div className="flex justify-between text-slate-400 mb-1">
                <span>Probability Confidence (Max 30)</span>
                <span className="font-mono-num font-bold text-white">
                  {breakdown?.probability_points?.toFixed(1) ?? 0.0} pts
                </span>
              </div>
              <div className="h-1.5 w-full bg-slate-800 rounded-full overflow-hidden">
                <div
                  className="h-full bg-cyan-400"
                  style={{ width: `${((breakdown?.probability_points ?? 0) / 30) * 100}%` }}
                />
              </div>
            </div>

            <div>
              <div className="flex justify-between text-slate-400 mb-1">
                <span>Net Edge &gt; Cost (Max 25)</span>
                <span className="font-mono-num font-bold text-white">
                  {breakdown?.edge_points?.toFixed(1) ?? 0.0} pts
                </span>
              </div>
              <div className="h-1.5 w-full bg-slate-800 rounded-full overflow-hidden">
                <div
                  className="h-full bg-emerald-400"
                  style={{ width: `${((breakdown?.edge_points ?? 0) / 25) * 100}%` }}
                />
              </div>
            </div>

            <div>
              <div className="flex justify-between text-slate-400 mb-1">
                <span>Window Timing Optimal (Max 20)</span>
                <span className="font-mono-num font-bold text-white">
                  {breakdown?.timing_points?.toFixed(1) ?? 0.0} pts
                </span>
              </div>
              <div className="h-1.5 w-full bg-slate-800 rounded-full overflow-hidden">
                <div
                  className="h-full bg-amber-400"
                  style={{ width: `${((breakdown?.timing_points ?? 0) / 20) * 100}%` }}
                />
              </div>
            </div>

            <div>
              <div className="flex justify-between text-slate-400 mb-1">
                <span>Orderbook Imbalance OBI (Max 15)</span>
                <span className="font-mono-num font-bold text-white">
                  {breakdown?.obi_points?.toFixed(1) ?? 0.0} pts
                </span>
              </div>
              <div className="h-1.5 w-full bg-slate-800 rounded-full overflow-hidden">
                <div
                  className="h-full bg-indigo-400"
                  style={{ width: `${((breakdown?.obi_points ?? 0) / 15) * 100}%` }}
                />
              </div>
            </div>

            <div>
              <div className="flex justify-between text-slate-400 mb-1">
                <span>Volatility Alignment (Max 10)</span>
                <span className="font-mono-num font-bold text-white">
                  {breakdown?.momentum_points?.toFixed(1) ?? 0.0} pts
                </span>
              </div>
              <div className="h-1.5 w-full bg-slate-800 rounded-full overflow-hidden">
                <div
                  className="h-full bg-purple-400"
                  style={{ width: `${((breakdown?.momentum_points ?? 0) / 10) * 100}%` }}
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
