import React, { useState, useEffect } from 'react';
import {
  X,
  DollarSign,
  Wallet,
  ShieldCheck,
  RefreshCw,
  AlertCircle,
  ShieldAlert,
  RotateCcw,
  Zap,
  Gamepad2,
  Lock,
} from 'lucide-react';
import { api } from '../services/api';
import { BankrollState, RiskStatus, PolymarketAccountPublic } from '../types';

interface SetBankrollModalProps {
  isOpen: boolean;
  onClose: () => void;
  mode?: 'paper' | 'live';
  currentBankroll: BankrollState | null;
  currentRisk?: RiskStatus | null;
  liveBankroll?: BankrollState | null;
  liveRisk?: RiskStatus | null;
  activeAccount?: PolymarketAccountPublic | null;
  onSuccess: (newBankroll: BankrollState, mode: 'paper' | 'live') => void;
}

export const SetBankrollModal: React.FC<SetBankrollModalProps> = ({
  isOpen,
  onClose,
  mode = 'paper',
  currentBankroll,
  currentRisk,
  liveBankroll,
  liveRisk,
  activeAccount,
  onSuccess,
}) => {
  const [selectedMode, setSelectedMode] = useState<'paper' | 'live'>(mode);

  // Paper form fields
  const [paperAmount, setPaperAmount] = useState<string>('100');
  const [paperCap, setPaperCap] = useState<string>('100');
  const [paperMinFloor, setPaperMinFloor] = useState<string>('0');
  const [paperDailyLossLimit, setPaperDailyLossLimit] = useState<string>('10');
  const [paperMaxConsecutive, setPaperMaxConsecutive] = useState<string>('5');

  // Live form fields
  const [liveCap, setLiveCap] = useState<string>('10');
  const [liveMinFloor, setLiveMinFloor] = useState<string>('0');
  const [liveDailyLossLimit, setLiveDailyLossLimit] = useState<string>('5');
  const [liveMaxConsecutive, setLiveMaxConsecutive] = useState<string>('3');

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isUnhalting, setIsUnhalting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Sync mode prop when modal is opened
  useEffect(() => {
    if (isOpen) {
      setSelectedMode(mode);
    }
  }, [isOpen, mode]);

  // Load paper initial data
  useEffect(() => {
    if (!isOpen) return;
    if (currentBankroll) {
      setPaperAmount(currentBankroll.active_bankroll.toString());
      setPaperCap(currentBankroll.bankroll_cap.toString());
      setPaperMinFloor(currentBankroll.minimum_bankroll.toString());
    } else {
      api.getBankroll('paper').then((b) => {
        if (b) {
          setPaperAmount(b.active_bankroll.toString());
          setPaperCap(b.bankroll_cap.toString());
          setPaperMinFloor(b.minimum_bankroll.toString());
        }
      }).catch(() => {});
    }

    if (currentRisk) {
      setPaperDailyLossLimit(currentRisk.daily_loss_limit.toString());
      setPaperMaxConsecutive(currentRisk.max_consecutive_losses.toString());
    } else {
      api.getRiskStatus('paper').then((r) => {
        if (r) {
          setPaperDailyLossLimit(r.daily_loss_limit.toString());
          setPaperMaxConsecutive(r.max_consecutive_losses.toString());
        }
      }).catch(() => {});
    }
  }, [isOpen, currentBankroll, currentRisk]);

  // Load live initial data
  useEffect(() => {
    if (!isOpen) return;
    if (liveBankroll) {
      setLiveCap(liveBankroll.bankroll_cap.toString());
      setLiveMinFloor(liveBankroll.minimum_bankroll.toString());
    } else {
      api.getBankroll('live').then((b) => {
        if (b) {
          setLiveCap(b.bankroll_cap.toString());
          setLiveMinFloor(b.minimum_bankroll.toString());
        }
      }).catch(() => {});
    }

    if (liveRisk) {
      setLiveDailyLossLimit(liveRisk.daily_loss_limit.toString());
      setLiveMaxConsecutive(liveRisk.max_consecutive_losses.toString());
    } else {
      api.getRiskStatus('live').then((r) => {
        if (r) {
          setLiveDailyLossLimit(r.daily_loss_limit.toString());
          setLiveMaxConsecutive(r.max_consecutive_losses.toString());
        }
      }).catch(() => {});
    }
  }, [isOpen, liveBankroll, liveRisk]);

  if (!isOpen) return null;

  const isLive = selectedMode === 'live';

  const paperQuickPresets = [10, 50, 100, 500, 1000];
  const paperLossPresets = [5, 10, 20, 50, 100];
  const liveLossPresets = [1, 2, 5, 10, 20];
  const liveCapPresets = [5, 10, 20, 50, 100];

  const handleSelectPaperPreset = (val: number) => {
    setPaperAmount(val.toString());
    setPaperCap(val.toString());
    setPaperMinFloor('0');
    const recLoss = Math.max(5, Math.round(val * 0.15));
    setPaperDailyLossLimit(recLoss.toString());
    setError(null);
  };

  const handleUnhalt = async () => {
    try {
      setIsUnhalting(true);
      setError(null);
      await api.unhaltTrading(selectedMode);
      const refreshed = await api.getBankroll(selectedMode);
      onSuccess(refreshed, selectedMode);
      onClose();
    } catch (err: any) {
      setError(err.message || '解除熔断失败');
    } finally {
      setIsUnhalting(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (isLive) {
      const numCap = parseFloat(liveCap);
      const numMin = parseFloat(liveMinFloor);
      const numLossLimit = parseFloat(liveDailyLossLimit);
      const numMaxConsecutive = parseInt(liveMaxConsecutive, 10);

      if (isNaN(numCap) || numCap <= 0) {
        setError('实盘单笔/最高资金硬顶必须大于 0');
        return;
      }
      if (isNaN(numLossLimit) || numLossLimit <= 0) {
        setError('实盘单日最大亏损限额必须大于 0');
        return;
      }

      try {
        setIsSubmitting(true);
        const liveBalance = activeAccount?.balance_usdc ?? (liveBankroll?.active_bankroll ?? 10.0);

        const res = await api.setBankrollFunds(
          {
            active_bankroll: liveBalance,
            bankroll_cap: numCap,
            minimum_bankroll: isNaN(numMin) ? 0 : numMin,
          },
          'live'
        );

        await api.updateRiskConfig(
          {
            daily_loss_limit: numLossLimit,
            max_consecutive_losses: isNaN(numMaxConsecutive) ? 3 : numMaxConsecutive,
          },
          'live'
        );

        if (res.success && res.bankroll) {
          onSuccess(res.bankroll, 'live');
          onClose();
        } else {
          setError(res.message || '实盘风控设置保存失败');
        }
      } catch (err: any) {
        setError(err.message || '网络或接口异常');
      } finally {
        setIsSubmitting(false);
      }
    } else {
      const numAmount = parseFloat(paperAmount);
      const numCap = parseFloat(paperCap);
      const numMin = parseFloat(paperMinFloor);
      const numLossLimit = parseFloat(paperDailyLossLimit);
      const numMaxConsecutive = parseInt(paperMaxConsecutive, 10);

      if (isNaN(numAmount) || numAmount <= 0) {
        setError('请输入有效的模拟本金金额 (大于 0)');
        return;
      }
      if (isNaN(numLossLimit) || numLossLimit <= 0) {
        setError('模拟盘单日最大亏损限额必须大于 0');
        return;
      }

      try {
        setIsSubmitting(true);
        const res = await api.setBankrollFunds(
          {
            active_bankroll: numAmount,
            bankroll_cap: isNaN(numCap) ? numAmount : numCap,
            minimum_bankroll: isNaN(numMin) ? 0 : numMin,
          },
          'paper'
        );

        await api.updateRiskConfig(
          {
            daily_loss_limit: numLossLimit,
            max_consecutive_losses: isNaN(numMaxConsecutive) ? 5 : numMaxConsecutive,
          },
          'paper'
        );

        if (res.success && res.bankroll) {
          onSuccess(res.bankroll, 'paper');
          onClose();
        } else {
          setError(res.message || '模拟资金设置失败');
        }
      } catch (err: any) {
        setError(err.message || '网络或接口异常');
      } finally {
        setIsSubmitting(false);
      }
    }
  };

  const curBankroll = isLive ? liveBankroll : currentBankroll;
  const curRisk = isLive ? liveRisk : currentRisk;
  const isHalted =
    curBankroll?.is_trading_halted ||
    (curRisk?.is_halted ?? false) ||
    (curRisk?.is_trading_halted ?? false);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-sm animate-fade-in">
      <div className="bg-white dark:bg-[#202528] rounded-2xl w-full max-w-lg shadow-2xl border border-slate-100 dark:border-slate-800 overflow-hidden transform transition-all max-h-[92vh] flex flex-col">
        {/* Header */}
        <div className="p-5 border-b border-slate-100 dark:border-slate-800/80 flex items-center justify-between shrink-0">
          <div className="flex items-center gap-3">
            <div className={`w-10 h-10 rounded-xl flex items-center justify-center shadow-sm ${
              isLive
                ? 'bg-rose-50 dark:bg-rose-950/40 text-[#ff0060]'
                : 'bg-emerald-50 dark:bg-emerald-950/40 text-emerald-500'
            }`}>
              {isLive ? <Zap className="w-5 h-5" /> : <Wallet className="w-5 h-5" />}
            </div>
            <div>
              <h3 className="text-base font-extrabold text-[#363949] dark:text-white">
                {isLive ? 'Polymarket 实盘资金风控与熔断设置' : '模拟资金与风控熔断设置'}
              </h3>
              <p className="text-xs text-[#7d8da1]">
                {isLive
                  ? '实盘专用：设置真实 USDC 交易硬顶、单日亏损上限及防爆仓熔断'
                  : '模拟盘专用：自定义初始虚拟本金、单日亏损上限及熔断机制'}
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 text-[#7d8da1] hover:text-[#363949] dark:hover:text-white rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 transition cursor-pointer"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Mode Segmented Switcher inside modal */}
        <div className="px-6 pt-4 pb-2">
          <div className="grid grid-cols-2 gap-2 bg-slate-100 dark:bg-[#181a1e] p-1 rounded-xl border border-slate-200 dark:border-slate-800 text-xs font-bold">
            <button
              type="button"
              onClick={() => { setSelectedMode('paper'); setError(null); }}
              className={`py-2 rounded-lg flex items-center justify-center gap-2 transition cursor-pointer ${
                !isLive
                  ? 'bg-white dark:bg-[#202528] text-emerald-600 dark:text-emerald-400 shadow-sm border border-emerald-200 dark:border-emerald-800/60'
                  : 'text-slate-500 hover:text-slate-800 dark:hover:text-slate-300'
              }`}
            >
              <Gamepad2 className="w-4 h-4" />
              <span>🎮 模拟盘资金预设</span>
            </button>
            <button
              type="button"
              onClick={() => { setSelectedMode('live'); setError(null); }}
              className={`py-2 rounded-lg flex items-center justify-center gap-2 transition cursor-pointer ${
                isLive
                  ? 'bg-white dark:bg-[#202528] text-rose-600 dark:text-rose-400 shadow-sm border border-rose-200 dark:border-rose-800/60'
                  : 'text-slate-500 hover:text-slate-800 dark:hover:text-slate-300'
              }`}
            >
              <Zap className="w-4 h-4" />
              <span>⚡ 实盘风控熔断设置</span>
            </button>
          </div>
        </div>

        {/* Form Body with scroll */}
        <form onSubmit={handleSubmit} className="p-6 pt-2 space-y-4 overflow-y-auto">
          {error && (
            <div className="p-3 bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/50 rounded-xl flex items-center gap-2.5 text-xs text-rose-600 dark:text-rose-400">
              <AlertCircle className="w-4 h-4 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {isLive ? (
            /* --- LIVE MODE SETTINGS --- */
            <>
              {/* Account Balance Real Status */}
              <div className="p-3.5 bg-slate-50 dark:bg-[#181a1e] rounded-xl border border-slate-200 dark:border-slate-800 space-y-2">
                <div className="flex items-center justify-between text-xs">
                  <span className="font-bold text-[#363949] dark:text-slate-200">
                    当前实盘账户余额
                  </span>
                  {activeAccount ? (
                    <span className="font-mono text-emerald-600 dark:text-emerald-400 font-extrabold text-sm">
                      ${(typeof activeAccount.balance_usdc === 'number' ? activeAccount.balance_usdc : 0).toFixed(2)} USDC
                    </span>
                  ) : (
                    <span className="text-rose-500 font-semibold">未绑定账户</span>
                  )}
                </div>
                {activeAccount && (
                  <div className="text-[11px] text-[#7d8da1] font-mono flex items-center justify-between">
                    <span>{activeAccount.label}</span>
                    <span>{activeAccount.wallet_address.slice(0, 8)}...{activeAccount.wallet_address.slice(-6)}</span>
                  </div>
                )}
                <p className="text-[11px] text-slate-500 dark:text-slate-400 leading-relaxed border-t border-slate-200/60 dark:border-slate-800/80 pt-2">
                  💡 实盘交易资金完全由您的 Polygon 链上真实账户余额决定。为防止极端行情大额下单，您可以在下方配置<strong>交易硬顶 (Cap)</strong> 与<strong>单日止损限额</strong>。
                </p>
              </div>

              {/* Live Bankroll Cap Presets */}
              <div>
                <label className="block text-xs font-bold text-[#7d8da1] mb-2 uppercase tracking-wider">
                  实盘资金硬顶预设 (Cap USDC)
                </label>
                <div className="grid grid-cols-5 gap-2">
                  {liveCapPresets.map((val) => (
                    <button
                      type="button"
                      key={val}
                      onClick={() => setLiveCap(val.toString())}
                      className={`py-2 px-1 text-xs font-mono font-bold rounded-xl border transition cursor-pointer ${
                        parseFloat(liveCap) === val
                          ? 'bg-[#1b9c85] text-white border-[#1b9c85] shadow-sm'
                          : 'border-slate-200 dark:border-slate-700 hover:border-[#1b9c85] text-[#363949] dark:text-slate-300'
                      }`}
                    >
                      ${val}
                    </button>
                  ))}
                </div>
              </div>

              {/* Live Cap & Floor Custom */}
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-bold text-[#363949] dark:text-slate-200 mb-1">
                    实盘交易硬顶 (Cap)
                  </label>
                  <div className="relative">
                    <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[#7d8da1] font-mono font-bold text-xs">
                      $
                    </span>
                    <input
                      type="number"
                      step="any"
                      min="0.5"
                      value={liveCap}
                      onChange={(e) => setLiveCap(e.target.value)}
                      required
                      placeholder="默认: 10.00"
                      className="w-full pl-7 pr-3 py-2 bg-slate-50 dark:bg-[#181a1e] border border-slate-200 dark:border-slate-700 rounded-xl font-mono text-sm font-bold text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#1b9c85]"
                    />
                  </div>
                </div>

                <div>
                  <label className="block text-xs font-bold text-[#363949] dark:text-slate-200 mb-1">
                    资金安全底线 (Floor)
                  </label>
                  <div className="relative">
                    <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[#7d8da1] font-mono font-bold text-xs">
                      $
                    </span>
                    <input
                      type="number"
                      step="any"
                      min="0"
                      value={liveMinFloor}
                      onChange={(e) => setLiveMinFloor(e.target.value)}
                      placeholder="默认: 0.00"
                      className="w-full pl-7 pr-3 py-2 bg-slate-50 dark:bg-[#181a1e] border border-slate-200 dark:border-slate-700 rounded-xl font-mono text-sm text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#1b9c85]"
                    />
                  </div>
                </div>
              </div>

              {/* Live Risk Limits: Daily Loss & Consecutive Losses */}
              <div className="p-3.5 bg-slate-50 dark:bg-[#181a1e] rounded-xl border border-slate-200 dark:border-slate-800 space-y-3">
                <div className="flex items-center justify-between">
                  <label className="block text-xs font-extrabold text-[#363949] dark:text-slate-200">
                    🛡️ 实盘单日最大亏损限额 (USDC)
                  </label>
                  <span className="text-[10px] text-[#ff0060] font-mono font-bold">
                    触发后立即熔断停机
                  </span>
                </div>

                {/* Live Daily loss presets */}
                <div className="grid grid-cols-5 gap-1.5">
                  {liveLossPresets.map((val) => (
                    <button
                      type="button"
                      key={val}
                      onClick={() => setLiveDailyLossLimit(val.toString())}
                      className={`py-1.5 px-1 text-xs font-mono font-bold rounded-lg border transition cursor-pointer ${
                        parseFloat(liveDailyLossLimit) === val
                          ? 'bg-[#ff0060] text-white border-[#ff0060] shadow-sm'
                          : 'border-slate-200 dark:border-slate-700 hover:border-[#ff0060] text-[#363949] dark:text-slate-300'
                      }`}
                    >
                      ${val}
                    </button>
                  ))}
                </div>

                <div className="grid grid-cols-2 gap-3 pt-1">
                  <div>
                    <label className="block text-[11px] font-semibold text-[#7d8da1] mb-1">
                      自定义限额 ($)
                    </label>
                    <div className="relative">
                      <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[#7d8da1] font-mono text-xs">
                        $
                      </span>
                      <input
                        type="number"
                        step="any"
                        min="0.5"
                        value={liveDailyLossLimit}
                        onChange={(e) => setLiveDailyLossLimit(e.target.value)}
                        required
                        placeholder="例如: 5.00"
                        className="w-full pl-7 pr-3 py-1.5 bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 rounded-lg font-mono text-sm text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#ff0060]"
                      />
                    </div>
                  </div>

                  <div>
                    <label className="block text-[11px] font-semibold text-[#7d8da1] mb-1">
                      连续亏损休眠 (笔)
                    </label>
                    <input
                      type="number"
                      min="1"
                      max="10"
                      value={liveMaxConsecutive}
                      onChange={(e) => setLiveMaxConsecutive(e.target.value)}
                      placeholder="默认: 3"
                      className="w-full px-3 py-1.5 bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 rounded-lg font-mono text-sm text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#6c9bcf]"
                    />
                  </div>
                </div>
              </div>
            </>
          ) : (
            /* --- PAPER MODE SETTINGS --- */
            <>
              {/* Quick Presets */}
              <div>
                <label className="block text-xs font-bold text-[#7d8da1] mb-2 uppercase tracking-wider">
                  快捷模拟本金预设 (USDC)
                </label>
                <div className="grid grid-cols-5 gap-2">
                  {paperQuickPresets.map((val) => (
                    <button
                      type="button"
                      key={val}
                      onClick={() => handleSelectPaperPreset(val)}
                      className={`py-2 px-1 text-xs font-mono font-bold rounded-xl border transition cursor-pointer ${
                        parseFloat(paperAmount) === val
                          ? 'bg-[#1b9c85] text-white border-[#1b9c85] shadow-sm'
                          : 'border-slate-200 dark:border-slate-700 hover:border-[#1b9c85] text-[#363949] dark:text-slate-300'
                      }`}
                    >
                      ${val}
                    </button>
                  ))}
                </div>
              </div>

              {/* Custom Amount */}
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-bold text-[#363949] dark:text-slate-200 mb-1">
                    模拟活跃本金 (USDC)
                  </label>
                  <div className="relative">
                    <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[#7d8da1] font-mono font-bold text-xs">
                      $
                    </span>
                    <input
                      type="number"
                      step="any"
                      min="0.1"
                      value={paperAmount}
                      onChange={(e) => setPaperAmount(e.target.value)}
                      required
                      placeholder="例如: 50.00"
                      className="w-full pl-7 pr-3 py-2 bg-slate-50 dark:bg-[#181a1e] border border-slate-200 dark:border-slate-700 rounded-xl font-mono text-sm font-bold text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#1b9c85]"
                    />
                  </div>
                </div>

                <div>
                  <label className="block text-xs font-bold text-[#363949] dark:text-slate-200 mb-1">
                    动态硬顶 (Cap USDC)
                  </label>
                  <div className="relative">
                    <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[#7d8da1] font-mono font-bold text-xs">
                      $
                    </span>
                    <input
                      type="number"
                      step="any"
                      min="0.1"
                      value={paperCap}
                      onChange={(e) => setPaperCap(e.target.value)}
                      placeholder="默认等于初始本金"
                      className="w-full pl-7 pr-3 py-2 bg-slate-50 dark:bg-[#181a1e] border border-slate-200 dark:border-slate-700 rounded-xl font-mono text-sm text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#1b9c85]"
                    />
                  </div>
                </div>
              </div>

              {/* Risk Limit Section: Daily Loss Limit */}
              <div className="p-3.5 bg-slate-50 dark:bg-[#181a1e] rounded-xl border border-slate-200 dark:border-slate-800 space-y-3">
                <div className="flex items-center justify-between">
                  <label className="block text-xs font-extrabold text-[#363949] dark:text-slate-200">
                    🛡️ 模拟单日最大亏损限额 (USDC)
                  </label>
                  <span className="text-[10px] text-[#7d8da1] font-mono">
                    达到此亏损后自动触发熔断暂停买入
                  </span>
                </div>

                {/* Daily loss presets */}
                <div className="grid grid-cols-5 gap-1.5">
                  {paperLossPresets.map((val) => (
                    <button
                      type="button"
                      key={val}
                      onClick={() => setPaperDailyLossLimit(val.toString())}
                      className={`py-1.5 px-1 text-xs font-mono font-bold rounded-lg border transition cursor-pointer ${
                        parseFloat(paperDailyLossLimit) === val
                          ? 'bg-[#ff0060] text-white border-[#ff0060] shadow-sm'
                          : 'border-slate-200 dark:border-slate-700 hover:border-[#ff0060] text-[#363949] dark:text-slate-300'
                      }`}
                    >
                      ${val}
                    </button>
                  ))}
                </div>

                <div className="grid grid-cols-2 gap-3 pt-1">
                  <div>
                    <label className="block text-[11px] font-semibold text-[#7d8da1] mb-1">
                      自定义限额 ($)
                    </label>
                    <div className="relative">
                      <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[#7d8da1] font-mono text-xs">
                        $
                      </span>
                      <input
                        type="number"
                        step="any"
                        min="1"
                        value={paperDailyLossLimit}
                        onChange={(e) => setPaperDailyLossLimit(e.target.value)}
                        required
                        placeholder="例如: 20"
                        className="w-full pl-7 pr-3 py-1.5 bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 rounded-lg font-mono text-sm text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#ff0060]"
                      />
                    </div>
                  </div>

                  <div>
                    <label className="block text-[11px] font-semibold text-[#7d8da1] mb-1">
                      连续亏损休眠 (笔)
                    </label>
                    <input
                      type="number"
                      min="1"
                      max="20"
                      value={paperMaxConsecutive}
                      onChange={(e) => setPaperMaxConsecutive(e.target.value)}
                      placeholder="默认: 5"
                      className="w-full px-3 py-1.5 bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 rounded-lg font-mono text-sm text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#6c9bcf]"
                    />
                  </div>
                </div>
              </div>
            </>
          )}

          {/* If halted, show unhalt button */}
          {isHalted && (
            <div className="p-3 bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900/60 rounded-xl flex items-center justify-between text-xs text-rose-700 dark:text-rose-300">
              <div className="flex items-center gap-2">
                <ShieldAlert className="w-4 h-4 shrink-0 text-[#ff0060]" />
                <span>当前处于<strong>{isLive ? '实盘' : '模拟盘'}熔断停机</strong>状态</span>
              </div>
              <button
                type="button"
                onClick={handleUnhalt}
                disabled={isUnhalting}
                className="px-3 py-1.5 bg-[#1b9c85] hover:bg-[#178572] text-white font-bold rounded-lg shadow transition flex items-center gap-1.5 cursor-pointer"
              >
                <RotateCcw className={`w-3.5 h-3.5 ${isUnhalting ? 'animate-spin' : ''}`} />
                {isUnhalting ? '恢复中...' : `立即解除${isLive ? '实盘' : '模拟'}熔断`}
              </button>
            </div>
          )}

          {/* Protection Notice */}
          <div className="p-3 bg-emerald-50/60 dark:bg-emerald-950/20 border border-emerald-200/60 dark:border-emerald-900/40 rounded-xl text-xs text-emerald-700 dark:text-emerald-400 flex items-start gap-2">
            <ShieldCheck className="w-4 h-4 shrink-0 mt-0.5" />
            <span>
              保存后，系统将自动应用新限额。若调高了单日亏损限额，系统将自动清除超限熔断，交易将自动恢复。实盘与模拟盘风控规则完全隔离，互不干扰。
            </span>
          </div>

          {/* Submit Button */}
          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 py-2.5 border border-slate-200 dark:border-slate-700 text-[#7d8da1] font-bold rounded-xl hover:bg-slate-50 dark:hover:bg-slate-800 transition cursor-pointer"
            >
              取消
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className={`flex-1 py-2.5 text-white font-extrabold rounded-xl shadow-lg flex items-center justify-center gap-2 transition disabled:opacity-50 cursor-pointer ${
                isLive
                  ? 'bg-rose-600 hover:bg-rose-700 shadow-rose-500/20'
                  : 'bg-[#1b9c85] hover:bg-[#178572] shadow-[#1b9c85]/20'
              }`}
            >
              {isSubmitting ? (
                <>
                  <RefreshCw className="w-4 h-4 animate-spin" />
                  <span>正在保存...</span>
                </>
              ) : (
                <>
                  <DollarSign className="w-4 h-4" />
                  <span>确认并应用设置 ({isLive ? '实盘' : '模拟盘'})</span>
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
