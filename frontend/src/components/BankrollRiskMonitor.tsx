import React, { useState, useEffect } from 'react';
import {
  DollarSign,
  Lock,
  Unlock,
  AlertOctagon,
  TrendingUp,
  ShieldAlert,
  CheckCircle2,
  RotateCcw,
  Wallet,
  Zap,
  Gamepad2,
  RefreshCw,
  ExternalLink,
} from 'lucide-react';
import { BankrollState, RiskStatus, TradingMode, PolymarketAccountPublic } from '../types';
import { api } from '../services/api';
import { UnlockProfitModal } from './UnlockProfitModal';

interface BankrollRiskMonitorProps {
  bankroll: BankrollState | null;
  risk: RiskStatus | null;
  liveBankroll?: BankrollState | null;
  liveRisk?: RiskStatus | null;
  tradingMode?: TradingMode;
  activeAccount?: PolymarketAccountPublic | null;
  onOpenSetBankroll?: (mode?: 'paper' | 'live') => void;
  onBankrollUpdated?: (newBankroll: BankrollState, mode: 'paper' | 'live') => void;
  onRefreshLiveBalance?: () => Promise<void>;
}

export const BankrollRiskMonitor: React.FC<BankrollRiskMonitorProps> = ({
  bankroll,
  risk,
  liveBankroll,
  liveRisk,
  tradingMode = 'paper',
  activeAccount,
  onOpenSetBankroll,
  onBankrollUpdated,
  onRefreshLiveBalance,
}) => {
  const [selectedPool, setSelectedPool] = useState<'paper' | 'live'>(tradingMode);
  const [isUnhalting, setIsUnhalting] = useState(false);
  const [isRefreshingBalance, setIsRefreshingBalance] = useState(false);
  const [isUnlockModalOpen, setIsUnlockModalOpen] = useState(false);

  // Sync tab selection with global trading mode when mode changes
  useEffect(() => {
    setSelectedPool(tradingMode);
  }, [tradingMode]);

  const isLive = selectedPool === 'live';

  // Active dataset depending on selected pool tab
  const curBankroll = isLive ? liveBankroll : bankroll;
  const curRisk = isLive ? liveRisk : risk;

  // Real Polymarket balance takes precedence if in live pool and available
  const activeBankroll = isLive
    ? (curBankroll?.active_bankroll ?? activeAccount?.balance_usdc ?? 0.0)
    : (curBankroll?.active_bankroll ?? 10.0);

  const bankrollCap = curBankroll?.bankroll_cap ?? (isLive ? 10.0 : 10.0);
  const lockedProfit = curBankroll?.locked_profit ?? 0.0;
  const totalEquity = curBankroll?.total_equity ?? (activeBankroll + lockedProfit);
  const mode = curBankroll?.mode ?? 'capital_recovery';

  // Base bankroll for return calculation
  const baseBankroll =
    typeof curBankroll?.initial_bankroll === 'number' && curBankroll.initial_bankroll > 0
      ? curBankroll.initial_bankroll
      : typeof bankrollCap === 'number' && bankrollCap > 0
      ? bankrollCap
      : (isLive ? 10.0 : 10.0);

  const netProfit = totalEquity - baseBankroll;
  const totalReturnPct = baseBankroll > 0 ? (netProfit / baseBankroll) * 100 : 0.0;

  const dailyLoss = curRisk?.daily_loss_current ?? 0.0;
  const dailyLossLimit = curRisk?.daily_loss_limit ?? (isLive ? 5.0 : 2.0);
  const dailyLossPct = dailyLossLimit > 0 ? Math.min(100, (dailyLoss / dailyLossLimit) * 100) : 0;

  const maxDrawdown = (curRisk as any)?.current_drawdown ?? curRisk?.current_drawdown_pct ?? 0;
  const maxDrawdownPct = maxDrawdown * 100;
  const maxDrawdownLimit = (curRisk as any)?.max_drawdown_limit ?? curRisk?.max_drawdown_limit_pct ?? (isLive ? 0.15 : 0.20);
  const maxDrawdownLimitPct = maxDrawdownLimit * 100;

  const consecutiveLosses = curRisk?.consecutive_losses ?? 0;
  const maxConsecutiveLosses = curRisk?.max_consecutive_losses ?? (isLive ? 3 : 5);

  const isHalted =
    curBankroll?.is_trading_halted ||
    (curRisk?.is_halted ?? false) ||
    (curRisk?.is_trading_halted ?? false);
  const haltReason = curBankroll?.halt_reason || curRisk?.halt_reason || (isLive ? '实盘风控熔断停机' : '模拟盘风控熔断停机');
  const isInCooldown = curRisk?.is_in_cooldown ?? false;

  // Cross-pool halt status for badges on the tabs
  const isPaperHalted = bankroll?.is_trading_halted || risk?.is_halted || risk?.is_trading_halted;
  const isLiveHalted = liveBankroll?.is_trading_halted || liveRisk?.is_halted || liveRisk?.is_trading_halted;

  const handleUnhalt = async () => {
    try {
      setIsUnhalting(true);
      await api.unhaltTrading(selectedPool);
      if (curBankroll && onBankrollUpdated) {
        const refreshed = await api.getBankroll(selectedPool);
        onBankrollUpdated(refreshed, selectedPool);
      }
    } catch (e: any) {
      alert(`解除${isLive ? '实盘' : '模拟盘'}熔断失败: ${e.message || '网络异常'}`);
    } finally {
      setIsUnhalting(false);
    }
  };

  const handleManualBalanceRefresh = async () => {
    if (!onRefreshLiveBalance) return;
    try {
      setIsRefreshingBalance(true);
      await onRefreshLiveBalance();
    } catch (err: any) {
      alert(`刷新链上余额失败: ${err.message || '网络超时'}`);
    } finally {
      setIsRefreshingBalance(false);
    }
  };

  return (
    <div className="asmr-card p-6">
      {/* Header and Pool Switcher */}
      <div className="flex flex-wrap items-center justify-between gap-3 mb-4 pb-3 border-b border-[var(--color-light)]">
        <div className="flex items-center gap-3">
          <div className={`p-2.5 rounded-xl border ${
            isLive
              ? 'bg-rose-50 dark:bg-rose-950/40 border-rose-200 dark:border-rose-800 text-[#ff0060]'
              : 'bg-emerald-50 dark:bg-emerald-950/40 border-emerald-200 dark:border-emerald-800/80 text-[#1b9c85]'
          }`}>
            {isLive ? <Zap className="h-5 w-5" /> : <DollarSign className="h-5 w-5" />}
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-sm font-bold text-[var(--color-dark)]">
                资金管理与风控熔断机制
              </h2>
              <span className={`text-[10px] font-extrabold px-2 py-0.5 rounded-full border ${
                isLive
                  ? 'bg-rose-100 dark:bg-rose-950 text-[#ff0060] border-rose-300 dark:border-rose-800'
                  : 'bg-emerald-100 dark:bg-emerald-950 text-[#1b9c85] border-emerald-300 dark:border-emerald-800'
              }`}>
                {isLive ? '⚡ 实盘独立资金池' : '🎮 模拟盘独立资金池'}
              </span>
            </div>
            <p className="text-xs text-[var(--color-info-dark)]">
              {isLive
                ? '实盘真金资金池与模拟盘严格隔离，支持独立单日限额、链上余额核验与连亏熔断'
                : '自定义模拟本金、虚拟硬顶限额、实时回撤追踪与多级资金保护'}
            </p>
          </div>
        </div>

        {/* Segmented Pool Switcher Tabs */}
        <div className="flex items-center bg-slate-100 dark:bg-[#181a1e] p-1 rounded-2xl border border-slate-200 dark:border-slate-800">
          <button
            onClick={() => setSelectedPool('paper')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-bold transition-all cursor-pointer ${
              selectedPool === 'paper'
                ? 'bg-white dark:bg-[#202528] text-emerald-600 dark:text-emerald-400 shadow-sm border border-emerald-200 dark:border-emerald-800/60'
                : 'text-slate-500 hover:text-slate-900 dark:hover:text-slate-200'
            }`}
          >
            <Gamepad2 className="w-3.5 h-3.5" />
            <span>模拟盘资金池</span>
            {isPaperHalted && (
              <span className="w-2 h-2 rounded-full bg-rose-500 animate-pulse" title="模拟盘处于熔断状态" />
            )}
          </button>

          <button
            onClick={() => setSelectedPool('live')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-bold transition-all cursor-pointer ${
              selectedPool === 'live'
                ? 'bg-white dark:bg-[#202528] text-rose-600 dark:text-rose-400 shadow-sm border border-rose-200 dark:border-rose-800/60'
                : 'text-slate-500 hover:text-slate-900 dark:hover:text-slate-200'
            }`}
          >
            <Zap className="w-3.5 h-3.5" />
            <span>实盘资金池 (Polymarket)</span>
            {isLiveHalted && (
              <span className="w-2 h-2 rounded-full bg-rose-500 animate-pulse" title="实盘处于熔断状态" />
            )}
          </button>
        </div>

        {/* Actions & Circuit Breaker Status Badge */}
        <div className="flex flex-wrap items-center gap-2">
          {isLive && activeAccount && (
            <button
              onClick={handleManualBalanceRefresh}
              disabled={isRefreshingBalance}
              className="flex items-center gap-1 text-xs font-medium px-2.5 py-1 rounded-full bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-200 border border-slate-300 dark:border-slate-700 transition cursor-pointer"
              title="重新读取 Polygon 链上代理钱包真实 USDC 余额"
            >
              <RefreshCw className={`w-3 h-3 ${isRefreshingBalance ? 'animate-spin text-rose-500' : ''}`} />
              <span>{isRefreshingBalance ? '刷新中...' : '同步链上余额'}</span>
            </button>
          )}

          {onOpenSetBankroll && (
            <button
              onClick={() => onOpenSetBankroll(selectedPool)}
              className="flex items-center gap-1.5 text-xs font-semibold px-3 py-1 rounded-full bg-indigo-50 dark:bg-indigo-950/60 text-[var(--color-primary)] border border-indigo-200 dark:border-indigo-800/80 hover:bg-indigo-100 dark:hover:bg-indigo-900/80 transition-colors shadow-sm cursor-pointer"
              title={`配置${isLive ? '实盘' : '模拟盘'}资金硬顶、单日亏损限额与保护机制`}
            >
              ⚙️ {isLive ? '实盘风控设置' : '模拟资金设置'}
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
                title={`清除${isLive ? '实盘' : '模拟盘'}熔断限制并立即恢复交易`}
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

      {/* Live Account Banner if in Live mode */}
      {isLive && (
        <div className="mb-4 p-3 bg-slate-50 dark:bg-[#181a1e] border border-slate-200 dark:border-slate-800 rounded-xl flex flex-wrap items-center justify-between gap-3 text-xs">
          {activeAccount ? (
            <div className="flex items-center gap-2">
              <div className="w-2.5 h-2.5 rounded-full bg-emerald-500 animate-pulse" />
              <span className="text-slate-600 dark:text-slate-300">
                当前绑定实盘账户：<strong>{activeAccount.label}</strong>
              </span>
              <span className="font-mono text-slate-400 text-[11px] bg-slate-200 dark:bg-slate-800 px-2 py-0.5 rounded-md">
                {activeAccount.wallet_address.slice(0, 6)}...{activeAccount.wallet_address.slice(-4)}
              </span>
              <span className="font-mono font-bold text-emerald-600 dark:text-emerald-400 ml-1">
                链上真实 USDC: ${(typeof activeAccount.balance_usdc === 'number' ? activeAccount.balance_usdc : 0).toFixed(2)}
              </span>
            </div>
          ) : (
            <div className="flex items-center gap-2 text-rose-600 dark:text-rose-400 font-medium">
              <ShieldAlert className="w-4 h-4 shrink-0" />
              <span>暂未绑定激活 Polymarket 实盘账户。实盘交易前请点击右上角账户图标进行绑定。</span>
            </div>
          )}
          <span className="text-[11px] text-slate-400 font-mono">
            实盘资金池与模拟资金池 100% 独立计算
          </span>
        </div>
      )}

      {/* Prominent Circuit Breaker Alert Banner when halted */}
      {isHalted && (
        <div className="mb-4 p-3 bg-rose-50/90 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900/60 rounded-xl flex flex-wrap items-center justify-between gap-3 text-xs text-rose-800 dark:text-rose-200 animate-fade-in">
          <div className="flex items-center gap-2">
            <ShieldAlert className="h-4 w-4 shrink-0 text-[#ff0060]" />
            <span>
              <strong>安全熔断提示（{isLive ? '实盘' : '模拟盘'}）：</strong>
              {haltReason}（如需继续交易，可调高单日亏损限额或点击一键解除）
            </span>
          </div>
          <button
            onClick={handleUnhalt}
            disabled={isUnhalting}
            className="px-3.5 py-1.5 bg-[#1b9c85] hover:bg-[#178572] text-white font-extrabold rounded-lg shadow-sm transition shrink-0 cursor-pointer"
          >
            {isUnhalting ? '正在恢复...' : `🚀 一键解除${isLive ? '实盘' : '模拟盘'}熔断并恢复交易`}
          </button>
        </div>
      )}

      {/* 4 Financial Stat Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-4">
        {/* 1. Active Bankroll */}
        <div className="asmr-subcard p-3.5">
          <div className="flex items-center justify-between text-[11px] text-[var(--color-info-dark)] mb-1">
            <span>{isLive ? '实盘活跃资金 (链上)' : '模拟活跃交易资金'}</span>
            <span className="font-mono text-[var(--color-primary)] font-semibold">
              硬顶: ${(typeof bankrollCap === 'number' && !isNaN(bankrollCap) ? bankrollCap : 10).toFixed(2)}
            </span>
          </div>
          <div className="text-xl font-black text-[var(--color-dark)] font-mono-num">
            ${(typeof activeBankroll === 'number' && !isNaN(activeBankroll) ? activeBankroll : (isLive ? 0 : 10)).toFixed(2)}{' '}
            <span className="text-xs text-[var(--color-info-dark)] font-normal">USDC</span>
          </div>
          <div className="h-1.5 w-full bg-[var(--color-light)] rounded-full mt-2 overflow-hidden">
            <div
              className={`h-full rounded-full transition-all ${isLive ? 'bg-rose-500' : 'bg-[var(--color-primary)]'}`}
              style={{ width: `${Math.min(100, (activeBankroll / (bankrollCap || 10)) * 100)}%` }}
            />
          </div>
        </div>

        {/* 2. Locked Profits Vault */}
        <div className="asmr-subcard p-3.5 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between text-[11px] text-[var(--color-info-dark)] mb-1">
              <span className="flex items-center gap-1">
                <Lock className="h-3 w-3 text-[#1b9c85]" /> {isLive ? '实盘锁定利润' : '锁定利润金库'}
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
              title={`将${isLive ? '实盘' : '模拟盘'}已锁定的利润提取转入活跃交易资金`}
            >
              <Unlock className="w-3 h-3" />
              <span>提取利润</span>
            </button>
          </div>
        </div>

        {/* 3. Total Portfolio Equity */}
        <div className="asmr-subcard p-3.5">
          <div className="flex items-center justify-between text-[11px] text-[var(--color-info-dark)] mb-1">
            <span>{isLive ? '实盘净值总计' : '总资产净值'}</span>
            <span className="font-mono text-[var(--color-info-dark)]">活跃资金 + 锁定利润</span>
          </div>
          <div className="text-xl font-black text-[var(--color-dark)] font-mono-num">
            ${(typeof totalEquity === 'number' && !isNaN(totalEquity) ? totalEquity : (isLive ? 0 : 10)).toFixed(2)}{' '}
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
            <span>{isLive ? '实盘单日亏损限额' : '模拟单日最大亏损'}</span>
            <span className="font-mono text-[#ff0060] font-semibold">
              上限 ${(typeof dailyLossLimit === 'number' && !isNaN(dailyLossLimit) ? dailyLossLimit : (isLive ? 5 : 2)).toFixed(2)}
            </span>
          </div>
          <div className="text-xl font-black text-[var(--color-dark)] font-mono-num">
            ${(typeof dailyLoss === 'number' && !isNaN(dailyLoss) ? dailyLoss : 0).toFixed(2)}{' '}
            <span className="text-xs text-[var(--color-info-dark)] font-normal">
              / ${(typeof dailyLossLimit === 'number' && !isNaN(dailyLossLimit) ? dailyLossLimit : (isLive ? 5 : 2)).toFixed(2)}
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
          <span className="text-[var(--color-info-dark)]">高点最大回撤:</span>
          <span
            className={`font-mono-num font-bold ${maxDrawdownPct > 15 ? 'text-[#ff0060]' : 'text-[var(--color-dark)]'}`}
          >
            {(typeof maxDrawdownPct === 'number' && !isNaN(maxDrawdownPct) ? maxDrawdownPct : 0).toFixed(1)}% / 上限{' '}
            {(typeof maxDrawdownLimitPct === 'number' && !isNaN(maxDrawdownLimitPct) ? maxDrawdownLimitPct : (isLive ? 15 : 20)).toFixed(0)}%
          </span>
        </div>

        <div className="asmr-subcard p-2.5 flex items-center justify-between">
          <span className="text-[var(--color-info-dark)]">资金保护底线 (Floor):</span>
          <span className="font-mono-num font-bold text-[var(--color-dark)]">
            ${(typeof curBankroll?.minimum_bankroll === 'number' ? curBankroll.minimum_bankroll : 0).toFixed(2)} USDC
          </span>
        </div>
      </div>

      {/* Unlock / Withdraw Profit Modal */}
      <UnlockProfitModal
        isOpen={isUnlockModalOpen}
        onClose={() => setIsUnlockModalOpen(false)}
        mode={selectedPool}
        currentBankroll={curBankroll || null}
        onSuccess={(newB) => {
          if (onBankrollUpdated) {
            onBankrollUpdated(newB, selectedPool);
          }
        }}
      />
    </div>
  );
};
