import React, { useState, useEffect } from 'react';
import { X, Lock, Unlock, DollarSign, RefreshCw, CheckCircle2, ArrowRight } from 'lucide-react';
import { api } from '../services/api';
import { BankrollState } from '../types';

interface UnlockProfitModalProps {
  isOpen: boolean;
  onClose: () => void;
  currentBankroll: BankrollState | null;
  mode?: 'paper' | 'live';
  onSuccess: (newBankroll: BankrollState) => void;
}

export const UnlockProfitModal: React.FC<UnlockProfitModalProps> = ({
  isOpen,
  onClose,
  currentBankroll,
  mode = 'paper',
  onSuccess,
}) => {
  const lockedProfit = currentBankroll?.locked_profit ?? 0.0;
  const activeBankroll = currentBankroll?.active_bankroll ?? 10.0;

  const [amount, setAmount] = useState<string>('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      setAmount(lockedProfit > 0 ? lockedProfit.toFixed(2) : '0');
      setError(null);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const numAmount = parseFloat(amount) || 0;
  const projectedActive = activeBankroll + Math.min(numAmount, lockedProfit);
  const projectedLocked = Math.max(0, lockedProfit - numAmount);

  const presets = [1, 2, 5, 10];

  const handleSelectPreset = (val: number) => {
    const clamped = Math.min(val, lockedProfit);
    setAmount(clamped.toFixed(2));
    setError(null);
  };

  const handleSelectAll = () => {
    setAmount(lockedProfit.toFixed(2));
    setError(null);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (lockedProfit <= 0) {
      setError('锁定金库中暂无可用利润资金');
      return;
    }

    if (isNaN(numAmount) || numAmount <= 0) {
      setError('请输入大于 0 的提取金额');
      return;
    }

    if (numAmount > lockedProfit + 0.001) {
      setError(`提取金额不能超出锁定金库余额 ($${lockedProfit.toFixed(2)} USDC)`);
      return;
    }

    try {
      setIsSubmitting(true);
      setError(null);
      const res = await api.unlockProfit(numAmount, mode);
      if (res.success && res.bankroll) {
        onSuccess(res.bankroll);
        onClose();
      } else {
        setError(res.message || '提取利润失败');
      }
    } catch (err: any) {
      setError(err.message || '网络异常，提取失败');
    } finally {
      setIsSubmitting(false);
    }
  };

  const isLive = mode === 'live';

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-sm animate-fade-in">
      <div className="bg-white dark:bg-[#181a1e] border border-slate-200 dark:border-slate-800 rounded-3xl p-6 w-full max-w-md shadow-2xl relative">
        {/* Header */}
        <div className="flex items-center justify-between pb-4 border-b border-slate-100 dark:border-slate-800">
          <div className="flex items-center gap-3">
            <div className="p-2.5 bg-emerald-50 dark:bg-emerald-950/60 border border-emerald-200 dark:border-emerald-800/80 rounded-2xl">
              <Unlock className="w-5 h-5 text-[#1b9c85]" />
            </div>
            <div>
              <h2 className="text-lg font-black text-[#363949] dark:text-white">
                {isLive ? '提取实盘锁定利润至交易本金' : '提取模拟锁定利润至交易本金'}
              </h2>
              <p className="text-xs text-[#7d8da1] dark:text-slate-400">
                {isLive ? '将实盘隔离金库中的利润转为可用资金继续撮合' : '将隔离金库中的利润转为可用资金继续交易'}
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 rounded-xl text-[#7d8da1] hover:text-[#363949] hover:bg-slate-100 dark:hover:bg-slate-800 transition"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Error Alert */}
        {error && (
          <div className="mt-4 p-3 bg-rose-50 dark:bg-rose-950/50 border border-rose-200 dark:border-rose-900/60 rounded-xl text-xs text-[#ff0060] font-medium">
            {error}
          </div>
        )}

        <form onSubmit={handleSubmit} className="mt-4 space-y-4">
          {/* Fund Transition Preview */}
          <div className="p-3.5 bg-slate-50 dark:bg-[#202528] rounded-2xl border border-slate-200/60 dark:border-slate-800 flex items-center justify-between">
            <div className="text-center flex-1">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">金库可用利润</span>
              <span className="text-base font-extrabold font-mono text-[#1b9c85]">
                +${lockedProfit.toFixed(2)}
              </span>
              <span className="text-[10px] text-[#7d8da1] block font-mono">
                → ${projectedLocked.toFixed(2)}
              </span>
            </div>

            <div className="px-2 text-slate-300 dark:text-slate-600">
              <ArrowRight className="w-5 h-5" />
            </div>

            <div className="text-center flex-1">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">活跃交易资金</span>
              <span className="text-base font-extrabold font-mono text-[var(--color-primary)]">
                ${activeBankroll.toFixed(2)}
              </span>
              <span className="text-[10px] text-[#1b9c85] font-bold block font-mono">
                → ${projectedActive.toFixed(2)}
              </span>
            </div>
          </div>

          {/* Quick Presets */}
          <div>
            <label className="block text-xs font-bold text-[#363949] dark:text-slate-300 mb-1.5">
              快捷金额选择
            </label>
            <div className="grid grid-cols-5 gap-1.5">
              {presets.map((val) => (
                <button
                  type="button"
                  key={val}
                  disabled={lockedProfit < val}
                  onClick={() => handleSelectPreset(val)}
                  className={`py-1.5 px-1 text-xs font-mono font-bold rounded-xl border transition cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed ${
                    parseFloat(amount) === val
                      ? 'bg-[#1b9c85] text-white border-[#1b9c85] shadow-sm'
                      : 'border-slate-200 dark:border-slate-700 hover:border-[#1b9c85] text-[#363949] dark:text-slate-300'
                  }`}
                >
                  ${val}
                </button>
              ))}
              <button
                type="button"
                onClick={handleSelectAll}
                disabled={lockedProfit <= 0}
                className="py-1.5 px-1 text-xs font-bold rounded-xl border border-emerald-300 dark:border-emerald-800 bg-emerald-50 dark:bg-emerald-950/60 text-[#1b9c85] hover:bg-emerald-100 transition cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
              >
                全部
              </button>
            </div>
          </div>

          {/* Amount Input */}
          <div>
            <label className="block text-xs font-bold text-[#363949] dark:text-slate-300 mb-1.5">
              提取金额 (USDC)
            </label>
            <div className="relative">
              <span className="absolute left-3.5 top-1/2 -translate-y-1/2 text-[#7d8da1] font-mono text-sm">
                $
              </span>
              <input
                type="number"
                step="0.01"
                min="0.01"
                max={lockedProfit}
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                disabled={lockedProfit <= 0}
                required
                placeholder="0.00"
                className="w-full pl-8 pr-16 py-2.5 bg-slate-50 dark:bg-[#202528] border border-slate-200 dark:border-slate-700 rounded-xl font-mono font-bold text-base text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#1b9c85] disabled:opacity-50"
              />
              <span className="absolute right-3.5 top-1/2 -translate-y-1/2 text-xs font-mono font-semibold text-[#7d8da1]">
                USDC
              </span>
            </div>
          </div>

          {/* Rule Note */}
          <div className="p-3 bg-emerald-50/70 dark:bg-emerald-950/20 border border-emerald-200/60 dark:border-emerald-900/40 rounded-xl text-xs text-emerald-800 dark:text-emerald-400 flex items-start gap-2">
            <CheckCircle2 className="w-4 h-4 shrink-0 mt-0.5" />
            <span>
              提取成功后，资金将立即转入您的活跃本金，且系统会自动调高资金硬顶 (Cap)，避免资金被下次盈利再度误回收。
            </span>
          </div>

          {/* Actions */}
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
              disabled={isSubmitting || lockedProfit <= 0 || numAmount <= 0}
              className="flex-1 py-2.5 bg-[#1b9c85] hover:bg-[#178572] text-white font-extrabold rounded-xl shadow-lg shadow-[#1b9c85]/20 flex items-center justify-center gap-2 transition disabled:opacity-40 cursor-pointer"
            >
              {isSubmitting ? (
                <>
                  <RefreshCw className="w-4 h-4 animate-spin" />
                  <span>正在转入...</span>
                </>
              ) : (
                <>
                  <Unlock className="w-4 h-4" />
                  <span>确认提取至本金</span>
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
