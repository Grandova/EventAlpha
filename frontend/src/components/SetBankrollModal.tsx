import React, { useState, useEffect } from 'react';
import { X, DollarSign, Wallet, ShieldCheck, RefreshCw, AlertCircle, ShieldAlert, RotateCcw } from 'lucide-react';
import { api } from '../services/api';
import { BankrollState, RiskStatus } from '../types';

interface SetBankrollModalProps {
  isOpen: boolean;
  onClose: () => void;
  currentBankroll: BankrollState | null;
  currentRisk?: RiskStatus | null;
  onSuccess: (newBankroll: BankrollState) => void;
}

export const SetBankrollModal: React.FC<SetBankrollModalProps> = ({
  isOpen,
  onClose,
  currentBankroll,
  currentRisk,
  onSuccess,
}) => {
  const [amount, setAmount] = useState<string>('100');
  const [cap, setCap] = useState<string>('100');
  const [minFloor, setMinFloor] = useState<string>('0');
  const [dailyLossLimit, setDailyLossLimit] = useState<string>('10');
  const [maxConsecutive, setMaxConsecutive] = useState<string>('5');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isUnhalting, setIsUnhalting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      if (currentBankroll) {
        setAmount(currentBankroll.active_bankroll.toString());
        setCap(currentBankroll.bankroll_cap.toString());
        setMinFloor(currentBankroll.minimum_bankroll.toString());
      } else {
        api.getBankroll().then((b) => {
          if (b) {
            setAmount(b.active_bankroll.toString());
            setCap(b.bankroll_cap.toString());
            setMinFloor(b.minimum_bankroll.toString());
          }
        }).catch(() => {});
      }

      if (currentRisk) {
        setDailyLossLimit(currentRisk.daily_loss_limit.toString());
        setMaxConsecutive(currentRisk.max_consecutive_losses.toString());
      } else {
        api.getRiskStatus().then((r) => {
          if (r) {
            setDailyLossLimit(r.daily_loss_limit.toString());
            setMaxConsecutive(r.max_consecutive_losses.toString());
          }
        }).catch(() => {});
      }
      setError(null);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const quickPresets = [10, 50, 100, 500, 1000];
  const lossPresets = [5, 10, 20, 50, 100];

  const handleSelectPreset = (val: number) => {
    setAmount(val.toString());
    setCap(val.toString());
    setMinFloor('0');
    // Scale recommended daily loss limit proportionally
    const recLoss = Math.max(5, Math.round(val * 0.15));
    setDailyLossLimit(recLoss.toString());
    setError(null);
  };

  const handleUnhalt = async () => {
    try {
      setIsUnhalting(true);
      setError(null);
      await api.unhaltTrading();
      if (currentBankroll) {
        const refreshed = await api.getBankroll();
        onSuccess(refreshed);
      }
      onClose();
    } catch (err: any) {
      setError(err.message || '解除熔断失败');
    } finally {
      setIsUnhalting(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const numAmount = parseFloat(amount);
    const numCap = parseFloat(cap);
    const numMin = parseFloat(minFloor);
    const numLossLimit = parseFloat(dailyLossLimit);
    const numMaxConsecutive = parseInt(maxConsecutive, 10);

    if (isNaN(numAmount) || numAmount <= 0) {
      setError('请输入有效的本金金额 (大于 0)');
      return;
    }
    if (isNaN(numLossLimit) || numLossLimit <= 0) {
      setError('单日最大亏损限额必须大于 0');
      return;
    }

    try {
      setIsSubmitting(true);
      setError(null);

      // 1. Update bankroll funds
      const res = await api.setBankrollFunds({
        active_bankroll: numAmount,
        bankroll_cap: isNaN(numCap) ? numAmount : numCap,
        minimum_bankroll: isNaN(numMin) ? 0 : numMin,
      });

      // 2. Update risk thresholds
      await api.updateRiskConfig({
        daily_loss_limit: numLossLimit,
        max_consecutive_losses: isNaN(numMaxConsecutive) ? 5 : numMaxConsecutive,
      });

      if (res.success && res.bankroll) {
        onSuccess(res.bankroll);
        onClose();
      } else {
        setError(res.message || '资金设置失败');
      }
    } catch (err: any) {
      setError(err.message || '网络或接口异常');
    } finally {
      setIsSubmitting(false);
    }
  };

  const isHalted =
    currentBankroll?.is_trading_halted ||
    (currentRisk?.is_halted ?? false) ||
    (currentRisk?.is_trading_halted ?? false);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-sm animate-fade-in">
      <div className="bg-white dark:bg-[#202528] rounded-2xl w-full max-w-lg shadow-2xl border border-slate-100 dark:border-slate-800 overflow-hidden transform transition-all max-h-[92vh] flex flex-col">
        {/* Header */}
        <div className="p-5 border-b border-slate-100 dark:border-slate-800/80 flex items-center justify-between shrink-0">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-emerald-50 dark:bg-emerald-950/40 text-emerald-500 flex items-center justify-center shadow-sm">
              <Wallet className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-base font-extrabold text-[#363949] dark:text-white">
                模拟资金与风控熔断设置
              </h3>
              <p className="text-xs text-[#7d8da1]">
                自定义初始本金、单日亏损上限及熔断机制
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 text-[#7d8da1] hover:text-[#363949] dark:hover:text-white rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 transition"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Form Body with scroll */}
        <form onSubmit={handleSubmit} className="p-6 space-y-4 overflow-y-auto">
          {error && (
            <div className="p-3 bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/50 rounded-xl flex items-center gap-2.5 text-xs text-rose-600 dark:text-rose-400">
              <AlertCircle className="w-4 h-4 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {/* Quick Presets */}
          <div>
            <label className="block text-xs font-bold text-[#7d8da1] mb-2 uppercase tracking-wider">
              快捷本金预设 (USDC)
            </label>
            <div className="grid grid-cols-5 gap-2">
              {quickPresets.map((val) => (
                <button
                  type="button"
                  key={val}
                  onClick={() => handleSelectPreset(val)}
                  className={`py-2 px-1 text-xs font-mono font-bold rounded-xl border transition ${
                    parseFloat(amount) === val
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
                初始活跃本金 (USDC)
              </label>
              <div className="relative">
                <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[#7d8da1] font-mono font-bold text-xs">
                  $
                </span>
                <input
                  type="number"
                  step="any"
                  min="0.1"
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  required
                  placeholder="例如: 50.00"
                  className="w-full pl-7 pr-3 py-2 bg-slate-50 dark:bg-[#181a1e] border border-slate-200 dark:border-slate-700 rounded-xl font-mono text-sm font-bold text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#1b9c85]"
                />
              </div>
            </div>

            <div>
              <label className="block text-xs font-bold text-[#363949] dark:text-slate-200 mb-1">
                动态硬顶 (USDC)
              </label>
              <div className="relative">
                <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[#7d8da1] font-mono font-bold text-xs">
                  $
                </span>
                <input
                  type="number"
                  step="any"
                  min="0.1"
                  value={cap}
                  onChange={(e) => setCap(e.target.value)}
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
                🛡️ 单日最大亏损限额 (USDC)
              </label>
              <span className="text-[10px] text-[#7d8da1] font-mono">
                达到此亏损后自动触发熔断暂停买入
              </span>
            </div>

            {/* Daily loss presets */}
            <div className="grid grid-cols-5 gap-1.5">
              {lossPresets.map((val) => (
                <button
                  type="button"
                  key={val}
                  onClick={() => setDailyLossLimit(val.toString())}
                  className={`py-1.5 px-1 text-xs font-mono font-bold rounded-lg border transition ${
                    parseFloat(dailyLossLimit) === val
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
                    value={dailyLossLimit}
                    onChange={(e) => setDailyLossLimit(e.target.value)}
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
                  value={maxConsecutive}
                  onChange={(e) => setMaxConsecutive(e.target.value)}
                  placeholder="默认: 5"
                  className="w-full px-3 py-1.5 bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 rounded-lg font-mono text-sm text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#6c9bcf]"
                />
              </div>
            </div>
          </div>

          {/* If halted, show unhalt button */}
          {isHalted && (
            <div className="p-3 bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900/60 rounded-xl flex items-center justify-between text-xs text-rose-700 dark:text-rose-300">
              <div className="flex items-center gap-2">
                <ShieldAlert className="w-4 h-4 shrink-0 text-[#ff0060]" />
                <span>当前处于<strong>熔断停机</strong>状态</span>
              </div>
              <button
                type="button"
                onClick={handleUnhalt}
                disabled={isUnhalting}
                className="px-3 py-1.5 bg-[#1b9c85] hover:bg-[#178572] text-white font-bold rounded-lg shadow transition flex items-center gap-1.5 cursor-pointer"
              >
                <RotateCcw className={`w-3.5 h-3.5 ${isUnhalting ? 'animate-spin' : ''}`} />
                {isUnhalting ? '恢复中...' : '立即解除熔断'}
              </button>
            </div>
          )}

          {/* Protection Notice */}
          <div className="p-3 bg-emerald-50/60 dark:bg-emerald-950/20 border border-emerald-200/60 dark:border-emerald-900/40 rounded-xl text-xs text-emerald-700 dark:text-emerald-400 flex items-start gap-2">
            <ShieldCheck className="w-4 h-4 shrink-0 mt-0.5" />
            <span>
              保存后，系统将自动应用新限额。若调高了单日亏损限额，系统将自动清除超限熔断，交易将自动恢复。
            </span>
          </div>

          {/* Submit Button */}
          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 py-2.5 border border-slate-200 dark:border-slate-700 text-[#7d8da1] font-bold rounded-xl hover:bg-slate-50 dark:hover:bg-slate-800 transition"
            >
              取消
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="flex-1 py-2.5 bg-[#1b9c85] hover:bg-[#178572] text-white font-extrabold rounded-xl shadow-lg shadow-[#1b9c85]/20 flex items-center justify-center gap-2 transition disabled:opacity-50"
            >
              {isSubmitting ? (
                <>
                  <RefreshCw className="w-4 h-4 animate-spin" />
                  <span>正在保存...</span>
                </>
              ) : (
                <>
                  <DollarSign className="w-4 h-4" />
                  <span>确认并应用设置</span>
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
