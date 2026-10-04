import React from 'react';
import { DollarSign, Lock, AlertOctagon, TrendingUp, ShieldAlert, CheckCircle2 } from 'lucide-react';
import { BankrollState, RiskStatus } from '../types';

interface BankrollRiskMonitorProps {
  bankroll: BankrollState | null;
  risk: RiskStatus | null;
  onOpenSetBankroll?: () => void;
}

export const BankrollRiskMonitor: React.FC<BankrollRiskMonitorProps> = ({
  bankroll,
  risk,
  onOpenSetBankroll,
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
      <div className="flex flex-wrap items-center justify-between gap-3 mb-4 pb-3 border-b border-[var(--color-light)]">
        <div className="flex items-center gap-2">
          <div className="p-2 bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-800/80 rounded-xl">
            <DollarSign className="h-4 w-4 text-[#1b9c85]" />
          </div>
          <div>
            <h2 className="text-sm font-bold text-[var(--color-dark)]">
              资金管理与风控熔断机制
            </h2>
            <p className="text-xs text-[var(--color-info-dark)]">
              实时回撤追踪、连亏硬限制与资金利润锁定金库
            </p>
          </div>
        </div>

        {/* Actions & Circuit Breaker Status Badge */}
        <div className="flex flex-wrap items-center gap-2">
          {onOpenSetBankroll && (
            <button
              onClick={onOpenSetBankroll}
              className="flex items-center gap-1.5 text-xs font-semibold px-3 py-1 rounded-full bg-indigo-50 dark:bg-indigo-950/60 text-[var(--color-primary)] border border-indigo-200 dark:border-indigo-800/80 hover:bg-indigo-100 dark:hover:bg-indigo-900/80 transition-colors shadow-sm cursor-pointer"
              title="自定义配置模拟盘本金、硬顶并解除熔断"
            >
              ⚙️ 调整模拟本金
            </button>
          )}

          {isHalted ? (
            <span className="flex items-center gap-1.5 text-xs font-bold font-mono px-3 py-1 rounded-full bg-rose-100 dark:bg-rose-950 text-[#ff0060] border border-rose-300 dark:border-rose-800">
              <ShieldAlert className="h-3.5 w-3.5" /> 触发熔断: 交易暂停
            </span>
          ) : isInCooldown ? (
            <span className="flex items-center gap-1.5 text-xs font-bold font-mono px-3 py-1 rounded-full bg-amber-100 dark:bg-amber-950 text-amber-600 dark:text-amber-400 border border-amber-300 dark:border-amber-800">
              <AlertOctagon className="h-3.5 w-3.5" /> 连亏冷却中: 暂时休眠
            </span>
          ) : (
            <span className="flex items-center gap-1.5 text-xs font-bold font-mono px-3 py-1 rounded-full bg-emerald-100 dark:bg-emerald-950/80 text-[#1b9c85] border border-emerald-300 dark:border-emerald-800/80">
              <CheckCircle2 className="h-3.5 w-3.5" /> 风控状态: 正常运行
            </span>
          )}
          <span className="text-[10px] text-[var(--color-info-dark)] bg-[var(--color-background)] px-2.5 py-1 rounded-full font-mono uppercase border border-[var(--color-light)]">
            模式: {mode === 'capital_recovery' ? '模式 B (本金回收保护)' : '模式 A (利润隔离)'}
          </span>
        </div>
      </div>

      {/* 4 Financial Stat Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-4">
        {/* 1. Active Bankroll (Mode B Cap $10) */}
        <div className="asmr-subcard p-3.5">
          <div className="flex items-center justify-between text-[11px] text-[var(--color-info-dark)] mb-1">
            <span>活跃交易资金</span>
            <span className="font-mono text-[var(--color-primary)] font-semibold">硬顶: ${(typeof bankrollCap === 'number' && !isNaN(bankrollCap) ? bankrollCap : 10).toFixed(2)}</span>
          </div>
          <div className="text-xl font-black text-[var(--color-dark)] font-mono-num">
            ${(typeof activeBankroll === 'number' && !isNaN(activeBankroll) ? activeBankroll : 10).toFixed(2)} <span className="text-xs text-[var(--color-info-dark)] font-normal">USDC</span>
          </div>
          <div className="h-1.5 w-full bg-[var(--color-light)] rounded-full mt-2 overflow-hidden">
            <div
              className="h-full bg-[var(--color-primary)] rounded-full"
              style={{ width: `${Math.min(100, (activeBankroll / (bankrollCap || 10)) * 100)}%` }}
            />
          </div>
        </div>

        {/* 2. Locked Profits Vault */}
        <div className="asmr-subcard p-3.5">
          <div className="flex items-center justify-between text-[11px] text-[var(--color-info-dark)] mb-1">
            <span className="flex items-center gap-1">
              <Lock className="h-3 w-3 text-[#1b9c85]" /> 锁定利润金库
            </span>
            <span className="font-mono text-[#1b9c85] font-semibold">100% 绝对无风险</span>
          </div>
          <div className="text-xl font-black text-[#1b9c85] font-mono-num">
            +${(typeof lockedProfit === 'number' && !isNaN(lockedProfit) ? lockedProfit : 0).toFixed(2)} <span className="text-xs text-[#1b9c85]/70 font-normal">USDC</span>
          </div>
          <p className="text-[10px] text-[var(--color-info-dark)] mt-2 font-mono">超出 $10 硬顶部分自动隔离</p>
        </div>

        {/* 3. Total Portfolio Equity */}
        <div className="asmr-subcard p-3.5">
          <div className="flex items-center justify-between text-[11px] text-[var(--color-info-dark)] mb-1">
            <span>总资产净值</span>
            <span className="font-mono text-[var(--color-info-dark)]">活跃资金 + 锁定利润</span>
          </div>
          <div className="text-xl font-black text-[var(--color-dark)] font-mono-num">
            ${(typeof totalEquity === 'number' && !isNaN(totalEquity) ? totalEquity : 10).toFixed(2)} <span className="text-xs text-[var(--color-info-dark)] font-normal">USDC</span>
          </div>
          <div className="text-[10px] text-[var(--color-info-dark)] mt-2 font-mono">
            总收益率:{' '}
            <strong className={totalEquity >= 10.0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}>
              {(((totalEquity - 10.0) / 10.0) * 100).toFixed(1)}%
            </strong>
          </div>
        </div>

        {/* 4. Daily Loss & Circuit Breaker */}
        <div className="asmr-subcard p-3.5">
          <div className="flex items-center justify-between text-[11px] text-[var(--color-info-dark)] mb-1">
            <span>单日最大亏损限额</span>
            <span className="font-mono text-[#ff0060] font-semibold">上限 ${(typeof dailyLossLimit === 'number' && !isNaN(dailyLossLimit) ? dailyLossLimit : 2).toFixed(2)}</span>
          </div>
          <div className="text-xl font-black text-[var(--color-dark)] font-mono-num">
            ${(typeof dailyLoss === 'number' && !isNaN(dailyLoss) ? dailyLoss : 0).toFixed(2)}{' '}
            <span className="text-xs text-[var(--color-info-dark)] font-normal">/ ${(typeof dailyLossLimit === 'number' && !isNaN(dailyLossLimit) ? dailyLossLimit : 2).toFixed(2)}</span>
          </div>
          <div className="h-1.5 w-full bg-[var(--color-light)] rounded-full mt-2 overflow-hidden">
            <div
              className={`h-full rounded-full ${dailyLossPct >= 80 ? 'bg-[#ff0060]' : 'bg-[#f7d154]'}`}
              style={{ width: `${dailyLossPct}%` }}
            />
          </div>
        </div>
      </div>

      {/* Circuit Breaker Detailed Gauges */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 pt-3 border-t border-[var(--color-light)] text-xs">
        <div className="asmr-subcard p-2.5 flex items-center justify-between">
          <span className="text-[var(--color-info-dark)]">当前连续亏损:</span>
          <span className="font-mono-num font-bold text-[var(--color-dark)]">
            {consecutiveLosses} / {maxConsecutiveLosses} 笔
          </span>
        </div>

        <div className="asmr-subcard p-2.5 flex items-center justify-between">
          <span className="text-[var(--color-info-dark)]">历史高点最大回撤:</span>
          <span className={`font-mono-num font-bold ${maxDrawdownPct > 15 ? 'text-[#ff0060]' : 'text-[var(--color-dark)]'}`}>
            {(typeof maxDrawdownPct === 'number' && !isNaN(maxDrawdownPct) ? maxDrawdownPct : 0).toFixed(1)}% / 上限 {(typeof maxDrawdownLimit === 'number' && !isNaN(maxDrawdownLimit) ? maxDrawdownLimit : 20).toFixed(0)}%
          </span>
        </div>

        <div className="asmr-subcard p-2.5 flex items-center justify-between">
          <span className="text-[var(--color-info-dark)]">资金保护底线 (Floor):</span>
          <span className="font-mono-num font-bold text-[var(--color-dark)]">$2.00 USDC</span>
        </div>
      </div>
    </div>
  );
};
