import React, { useState } from 'react';
import { DollarSign, Lock, Unlock, AlertOctagon, TrendingUp, ShieldAlert, CheckCircle2, RotateCcw } from 'lucide-react';
import { BankrollState, RiskStatus } from '../types';
import { api } from '../services/api';
import { UnlockProfitModal } from './UnlockProfitModal';

interface BankrollRiskMonitorProps {
  bankroll: BankrollState | null;
  risk: RiskStatus | null;
  onOpenSetBankroll?: () => void;
  onBankrollUpdated?: (newBankroll: BankrollState) => void;
}

export const BankrollRiskMonitor: React.FC<BankrollRiskMonitorProps> = ({
  bankroll,
  risk,
  onOpenSetBankroll,
  onBankrollUpdated,
}) => {
  const [isUnhalting, setIsUnhalting] = useState(false);
  const [isUnlockModalOpen, setIsUnlockModalOpen] = useState(false);

  const activeBankroll = bankroll?.active_bankroll ?? 10.0;
  const bankrollCap = bankroll?.bankroll_cap ?? 10.0;
  const lockedProfit = bankroll?.locked_profit ?? 0.0;
  const totalEquity = bankroll?.total_equity ?? (activeBankroll + lockedProfit);
  const mode = bankroll?.mode ?? 'capital_recovery';

  // Base bankroll for return calculation (initial or cap, never hardcoded)
  const baseBankroll =
    typeof bankroll?.initial_bankroll === 'number' && bankroll.initial_bankroll > 0
      ? bankroll.initial_bankroll
      : typeof bankrollCap === 'number' && bankrollCap > 0
      ? bankrollCap
      : 10.0;

  const netProfit = totalEquity - baseBankroll;
  const totalReturnPct = baseBankroll > 0 ? (netProfit / baseBankroll) * 100 : 0.0;

  const dailyLoss = risk?.daily_loss_current ?? 0.0;
  const dailyLossLimit = risk?.daily_loss_limit ?? 2.0;
  const dailyLossPct = dailyLossLimit > 0 ? Math.min(100, (dailyLoss / dailyLossLimit) * 100) : 0;

  const maxDrawdown = (risk as any)?.current_drawdown ?? risk?.current_drawdown_pct ?? 0;
  const maxDrawdownPct = maxDrawdown * 100;
  const maxDrawdownLimit = (risk as any)?.max_drawdown_limit ?? risk?.max_drawdown_limit_pct ?? 0.20;
  const maxDrawdownLimitPct = maxDrawdownLimit * 100;

  const consecutiveLosses = risk?.consecutive_losses ?? 0;
  const maxConsecutiveLosses = risk?.max_consecutive_losses ?? 5;

  const isHalted =
    bankroll?.is_trading_halted ||
    (risk?.is_halted ?? false) ||
    (risk?.is_trading_halted ?? false);
  const haltReason = bankroll?.halt_reason || risk?.halt_reason || '系统风控熔断停机';
  const isInCooldown = risk?.is_in_cooldown ?? false;

  const handleUnhalt = async () => {
    try {
      setIsUnhalting(true);
      await api.unhaltTrading();
    } catch (e: any) {
      alert(`解除熔断失败: ${e.message || '网络异常'}`);
    } finally {
      setIsUnhalting(false);
    }
  };

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
              自定义单日亏损限额、实时回撤追踪与多级资金保护
            </p>
          </div>
        </div>

        {/* Actions & Circuit Breaker Status Badge */}
        <div className="flex flex-wrap items-center gap-2">
          {onOpenSetBankroll && (
            <button
              onClick={onOpenSetBankroll}
              className="flex items-center gap-1.5 text-xs font-semibold px-3 py-1 rounded-full bg-indigo-50 dark:bg-indigo-950/60 text-[var(--color-primary)] border border-indigo-200 dark:border-indigo-800/80 hover:bg-indigo-100 dark:hover:bg-indigo-900/80 transition-colors shadow-sm cursor-pointer"
              title="自定义配置模拟盘本金、单日亏损限额并解除熔断"
            >
              ⚙️ 资金与风控设置
            </button>
          )}

          {isHalted ? (
            <div className="flex items-center gap-2">
              <span className="flex items-center gap-1.5 text-xs font-bold font-mono px-3 py-1 rounded-full bg-rose-100 dark:bg-rose-950 text-[#ff0060] border border-rose-300 dark:border-rose-800">
                <ShieldAlert className="h-3.5 w-3.5" /> 触发熔断: 交易暂停
              </span>
              <button
                onClick={handleUnhalt}
                disabled={isUnhalting}
                className="flex items-center gap-1 text-xs font-bold px-3 py-1 rounded-full bg-emerald-600 hover:bg-emerald-700 text-white shadow-sm transition cursor-pointer"
                title="清除熔断限制并立即恢复自动交易"
              >
                <RotateCcw className={`h-3 w-3 ${isUnhalting ? 'animate-spin' : ''}`} />
                {isUnhalting ? '恢复中...' : '解除熔断'}
              </button>
            </div>
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

      {/* Prominent Circuit Breaker Alert Banner when halted */}
      {isHalted && (
        <div className="mb-4 p-3 bg-rose-50/90 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900/60 rounded-xl flex flex-wrap items-center justify-between gap-3 text-xs text-rose-800 dark:text-rose-200 animate-fade-in">
          <div className="flex items-center gap-2">
            <ShieldAlert className="h-4 w-4 shrink-0 text-[#ff0060]" />
            <span>
              <strong>安全熔断提示：</strong>
              {haltReason}（如需继续交易，可调高单日亏损限额或点击一键解除）
            </span>
          </div>
          <button
            onClick={handleUnhalt}
            disabled={isUnhalting}
            className="px-3.5 py-1.5 bg-[#1b9c85] hover:bg-[#178572] text-white font-extrabold rounded-lg shadow-sm transition shrink-0 cursor-pointer"
          >
            {isUnhalting ? '正在恢复...' : '🚀 一键解除熔断并恢复交易'}
          </button>
        </div>
      )}

      {/* 4 Financial Stat Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-4">
        {/* 1. Active Bankroll */}
        <div className="asmr-subcard p-3.5">
          <div className="flex items-center justify-between text-[11px] text-[var(--color-info-dark)] mb-1">
            <span>活跃交易资金</span>
            <span className="font-mono text-[var(--color-primary)] font-semibold">
              硬顶: ${(typeof bankrollCap === 'number' && !isNaN(bankrollCap) ? bankrollCap : 10).toFixed(2)}
            </span>
          </div>
          <div className="text-xl font-black text-[var(--color-dark)] font-mono-num">
            ${(typeof activeBankroll === 'number' && !isNaN(activeBankroll) ? activeBankroll : 10).toFixed(2)}{' '}
            <span className="text-xs text-[var(--color-info-dark)] font-normal">USDC</span>
          </div>
          <div className="h-1.5 w-full bg-[var(--color-light)] rounded-full mt-2 overflow-hidden">
            <div
              className="h-full bg-[var(--color-primary)] rounded-full transition-all"
              style={{ width: `${Math.min(100, (activeBankroll / (bankrollCap || 10)) * 100)}%` }}
            />
          </div>
        </div>

        {/* 2. Locked Profits Vault */}
        <div className="asmr-subcard p-3.5 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between text-[11px] text-[var(--color-info-dark)] mb-1">
              <span className="flex items-center gap-1">
                <Lock className="h-3 w-3 text-[#1b9c85]" /> 锁定利润金库
              </span>
              <span className="font-mono text-[#1b9c85] font-semibold">100% 绝对隔离</span>
            </div>
            <div className="text-xl font-black text-[#1b9c85] font-mono-num">
              +${(typeof lockedProfit === 'number' && !isNaN(lockedProfit) ? lockedProfit : 0).toFixed(2)}{' '}
              <span className="text-xs text-[#1b9c85]/70 font-normal">USDC</span>
            </div>
          </div>
          <div className="mt-2 pt-2 border-t border-[var(--color-light)] flex items-center justify-between">
            <span className="text-[10px] text-[var(--color-info-dark)] font-mono">
              硬顶: ${(typeof bankrollCap === 'number' ? bankrollCap : 10).toFixed(0)}
            </span>
            <button
              onClick={() => setIsUnlockModalOpen(true)}
              disabled={lockedProfit <= 0}
              className={`flex items-center gap-1 text-[11px] font-bold px-2.5 py-1 rounded-lg transition shadow-sm cursor-pointer ${
                lockedProfit > 0
                  ? 'bg-emerald-600 hover:bg-emerald-700 text-white'
                  : 'bg-slate-100 dark:bg-slate-800 text-slate-400 cursor-not-allowed opacity-50'
              }`}
              title="将已锁定的利润提取转入活跃交易资金"
            >
              <Unlock className="w-3 h-3" />
              <span>提取利润</span>
            </button>
          </div>
        </div>

        {/* 3. Total Portfolio Equity */}
        <div className="asmr-subcard p-3.5">
          <div className="flex items-center justify-between text-[11px] text-[var(--color-info-dark)] mb-1">
            <span>总资产净值</span>
            <span className="font-mono text-[var(--color-info-dark)]">活跃资金 + 锁定利润</span>
          </div>
          <div className="text-xl font-black text-[var(--color-dark)] font-mono-num">
            ${(typeof totalEquity === 'number' && !isNaN(totalEquity) ? totalEquity : 10).toFixed(2)}{' '}
            <span className="text-xs text-[var(--color-info-dark)] font-normal">USDC</span>
          </div>
          <div className="text-[10px] text-[var(--color-info-dark)] mt-2 font-mono flex items-center justify-between">
            <span>总收益率:</span>
            <strong className={netProfit >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}>
              {netProfit >= 0 ? '+' : ''}
              {totalReturnPct.toFixed(1)}% ({netProfit >= 0 ? '+$' : '-$'}
              {Math.abs(netProfit).toFixed(2)})
            </strong>
          </div>
        </div>

        {/* 4. Daily Loss & Circuit Breaker */}
        <div className="asmr-subcard p-3.5">
          <div className="flex items-center justify-between text-[11px] text-[var(--color-info-dark)] mb-1">
            <span>单日最大亏损限额</span>
            <span className="font-mono text-[#ff0060] font-semibold">
              上限 ${(typeof dailyLossLimit === 'number' && !isNaN(dailyLossLimit) ? dailyLossLimit : 2).toFixed(2)}
            </span>
          </div>
          <div className="text-xl font-black text-[var(--color-dark)] font-mono-num">
            ${(typeof dailyLoss === 'number' && !isNaN(dailyLoss) ? dailyLoss : 0).toFixed(2)}{' '}
            <span className="text-xs text-[var(--color-info-dark)] font-normal">
              / ${(typeof dailyLossLimit === 'number' && !isNaN(dailyLossLimit) ? dailyLossLimit : 2).toFixed(2)}
            </span>
          </div>
          <div className="h-1.5 w-full bg-[var(--color-light)] rounded-full mt-2 overflow-hidden">
            <div
              className={`h-full rounded-full transition-all ${dailyLossPct >= 80 ? 'bg-[#ff0060]' : 'bg-[#f7d154]'}`}
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
          <span
            className={`font-mono-num font-bold ${maxDrawdownPct > 15 ? 'text-[#ff0060]' : 'text-[var(--color-dark)]'}`}
          >
            {(typeof maxDrawdownPct === 'number' && !isNaN(maxDrawdownPct) ? maxDrawdownPct : 0).toFixed(1)}% / 上限{' '}
            {(typeof maxDrawdownLimitPct === 'number' && !isNaN(maxDrawdownLimitPct) ? maxDrawdownLimitPct : 20).toFixed(0)}%
          </span>
        </div>

        <div className="asmr-subcard p-2.5 flex items-center justify-between">
          <span className="text-[var(--color-info-dark)]">资金保护底线 (Floor):</span>
          <span className="font-mono-num font-bold text-[var(--color-dark)]">
            ${(typeof bankroll?.minimum_bankroll === 'number' ? bankroll.minimum_bankroll : 0).toFixed(2)} USDC
          </span>
        </div>
      </div>

      {/* Unlock / Withdraw Profit Modal */}
      <UnlockProfitModal
        isOpen={isUnlockModalOpen}
        onClose={() => setIsUnlockModalOpen(false)}
        currentBankroll={bankroll}
        onSuccess={(newB) => {
          if (onBankrollUpdated) {
            onBankrollUpdated(newB);
          }
        }}
      />
    </div>
  );
};
