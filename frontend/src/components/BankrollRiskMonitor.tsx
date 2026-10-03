import React from 'react';
import { DollarSign, Lock, AlertOctagon, TrendingUp, ShieldAlert, CheckCircle2 } from 'lucide-react';
import { BankrollState, RiskStatus } from '../types';

interface BankrollRiskMonitorProps {
  bankroll: BankrollState | null;
  risk: RiskStatus | null;
}

export const BankrollRiskMonitor: React.FC<BankrollRiskMonitorProps> = ({
  bankroll,
  risk,
}) => {
  const activeBankroll = bankroll?.active_bankroll ?? 10.0;
  const bankrollCap = bankroll?.bankroll_cap ?? 10.0;
  const lockedProfit = bankroll?.locked_profit ?? 0.0;
  const totalEquity = bankroll?.total_equity ?? (activeBankroll + lockedProfit);
  const mode = bankroll?.mode ?? 'capital_recovery';

  const dailyLoss = risk?.daily_loss_current ?? 0.0;
  const dailyLossLimit = risk?.daily_loss_limit ?? 2.0;
  const dailyLossPct = Math.min(100, (dailyLoss / dailyLossLimit) * 100);

  const maxDrawdownPct = (risk?.current_drawdown_pct ?? 0) * 100;
  const maxDrawdownLimit = (risk?.max_drawdown_limit_pct ?? 0.20) * 100;

  const consecutiveLosses = risk?.consecutive_losses ?? 0;
  const maxConsecutiveLosses = risk?.max_consecutive_losses ?? 5;

  const isHalted = bankroll?.is_trading_halted || (risk?.is_halted ?? false);
  const isInCooldown = risk?.is_in_cooldown ?? false;

  return (
    <div className="asmr-card p-6">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <DollarSign className="h-4 w-4 text-emerald-400" />
          <h2 className="text-xs font-bold text-slate-400 uppercase tracking-wider">
            Capital Management & Risk Circuit Breakers
          </h2>
        </div>

        {/* Circuit Breaker Status Badge */}
        <div className="flex items-center gap-2">
          {isHalted ? (
            <span className="flex items-center gap-1 text-xs font-bold font-mono px-2.5 py-1 rounded bg-rose-950 text-rose-400 border border-rose-800">
              <ShieldAlert className="h-3.5 w-3.5" /> CIRCUIT BREAKER: HALTED
            </span>
          ) : isInCooldown ? (
            <span className="flex items-center gap-1 text-xs font-bold font-mono px-2.5 py-1 rounded bg-amber-950 text-amber-400 border border-amber-800">
              <AlertOctagon className="h-3.5 w-3.5" /> LOSS COOLDOWN ACTIVE
            </span>
          ) : (
            <span className="flex items-center gap-1 text-xs font-bold font-mono px-2.5 py-1 rounded bg-emerald-950/80 text-emerald-400 border border-emerald-800/80">
              <CheckCircle2 className="h-3.5 w-3.5" /> RISK STATUS: NORMAL
            </span>
          )}
          <span className="text-[10px] text-slate-400 bg-slate-800 px-2 py-1 rounded font-mono uppercase">
            Mode: {mode === 'capital_recovery' ? 'Mode B (Capital Recovery)' : 'Mode A (Profit Isolation)'}
          </span>
        </div>
      </div>

      {/* 4 Financial Stat Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-4">
        {/* 1. Active Bankroll (Mode B Cap $10) */}
        <div className="bg-slate-900/80 p-3 rounded-lg border border-slate-800">
          <div className="flex items-center justify-between text-[11px] text-slate-400 mb-1">
            <span>Active Bankroll</span>
            <span className="font-mono text-cyan-400">Cap: ${bankrollCap.toFixed(2)}</span>
          </div>
          <div className="text-xl font-black text-white font-mono-num">
            ${activeBankroll.toFixed(2)} <span className="text-xs text-slate-500 font-normal">USDC</span>
          </div>
          <div className="h-1.5 w-full bg-slate-800 rounded-full mt-2 overflow-hidden">
            <div
              className="h-full bg-cyan-400"
              style={{ width: `${Math.min(100, (activeBankroll / bankrollCap) * 100)}%` }}
            />
          </div>
        </div>

        {/* 2. Locked Profits Vault */}
        <div className="bg-slate-900/80 p-3 rounded-lg border border-slate-800">
          <div className="flex items-center justify-between text-[11px] text-slate-400 mb-1">
            <span className="flex items-center gap-1">
              <Lock className="h-3 w-3 text-emerald-400" /> Locked Profit Vault
            </span>
            <span className="font-mono text-emerald-400">100% Risk Free</span>
          </div>
          <div className="text-xl font-black text-emerald-400 font-mono-num">
            +${lockedProfit.toFixed(2)} <span className="text-xs text-emerald-500/70 font-normal">USDC</span>
          </div>
          <p className="text-[10px] text-slate-500 mt-2 font-mono">Excess gains above $10 cap</p>
        </div>

        {/* 3. Total Portfolio Equity */}
        <div className="bg-slate-900/80 p-3 rounded-lg border border-slate-800">
          <div className="flex items-center justify-between text-[11px] text-slate-400 mb-1">
            <span>Total Equity</span>
            <span className="font-mono text-slate-300">Active + Locked</span>
          </div>
          <div className="text-xl font-black text-white font-mono-num">
            ${totalEquity.toFixed(2)} <span className="text-xs text-slate-500 font-normal">USDC</span>
          </div>
          <div className="text-[10px] text-slate-500 mt-2 font-mono">
            Return:{' '}
            <strong className={totalEquity >= 10.0 ? 'text-emerald-400' : 'text-rose-400'}>
              {(((totalEquity - 10.0) / 10.0) * 100).toFixed(1)}%
            </strong>
          </div>
        </div>

        {/* 4. Daily Loss & Circuit Breaker */}
        <div className="bg-slate-900/80 p-3 rounded-lg border border-slate-800">
          <div className="flex items-center justify-between text-[11px] text-slate-400 mb-1">
            <span>Daily Loss Limit</span>
            <span className="font-mono text-rose-400">Max ${dailyLossLimit.toFixed(2)}</span>
          </div>
          <div className="text-xl font-black text-slate-200 font-mono-num">
            ${dailyLoss.toFixed(2)}{' '}
            <span className="text-xs text-slate-500 font-normal">/ ${dailyLossLimit.toFixed(2)}</span>
          </div>
          <div className="h-1.5 w-full bg-slate-800 rounded-full mt-2 overflow-hidden">
            <div
              className={`h-full ${dailyLossPct >= 80 ? 'bg-rose-500' : 'bg-amber-400'}`}
              style={{ width: `${dailyLossPct}%` }}
            />
          </div>
        </div>
      </div>

      {/* Circuit Breaker Detailed Gauges */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 pt-3 border-t border-slate-800/80 text-xs">
        <div className="bg-slate-950/40 p-2.5 rounded border border-slate-800/60 flex items-center justify-between">
          <span className="text-slate-400">Consecutive Losses:</span>
          <span className="font-mono-num font-bold text-white">
            {consecutiveLosses} / {maxConsecutiveLosses} trades
          </span>
        </div>

        <div className="bg-slate-950/40 p-2.5 rounded border border-slate-800/60 flex items-center justify-between">
          <span className="text-slate-400">Peak-to-Trough Drawdown:</span>
          <span className={`font-mono-num font-bold ${maxDrawdownPct > 15 ? 'text-rose-400' : 'text-slate-300'}`}>
            {maxDrawdownPct.toFixed(1)}% / max {maxDrawdownLimit.toFixed(0)}%
          </span>
        </div>

        <div className="bg-slate-950/40 p-2.5 rounded border border-slate-800/60 flex items-center justify-between">
          <span className="text-slate-400">Minimum Bankroll Floor:</span>
          <span className="font-mono-num font-bold text-slate-300">$2.00 USDC</span>
        </div>
      </div>
    </div>
  );
};
