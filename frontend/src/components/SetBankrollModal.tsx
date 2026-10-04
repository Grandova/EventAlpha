import React, { useState } from 'react';
import { X, DollarSign, Wallet, ShieldCheck, RefreshCw, AlertCircle } from 'lucide-react';
import { api } from '../services/api';
import { BankrollState } from '../types';

interface SetBankrollModalProps {
  isOpen: boolean;
  onClose: () => void;
  currentBankroll: BankrollState | null;
  onSuccess: (newBankroll: BankrollState) => void;
}

export const SetBankrollModal: React.FC<SetBankrollModalProps> = ({
  isOpen,
  onClose,
  currentBankroll,
  onSuccess,
}) => {
  const [amount, setAmount] = useState<string>(
    currentBankroll ? currentBankroll.active_bankroll.toString() : '100'
  );
  const [cap, setCap] = useState<string>(
    currentBankroll ? currentBankroll.bankroll_cap.toString() : '100'
  );
  const [minFloor, setMinFloor] = useState<string>('0');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const quickPresets = [10, 50, 100, 500, 1000, 5000];

  const handleSelectPreset = (val: number) => {
    setAmount(val.toString());
    setCap(val.toString());
    setMinFloor('0');
    setError(null);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const numAmount = parseFloat(amount);
    const numCap = parseFloat(cap);
    const numMin = parseFloat(minFloor);

    if (isNaN(numAmount) || numAmount <= 0) {
      setError('请输入有效的本金金额 (大于 0)');
      return;
    }

    try {
      setIsSubmitting(true);
      setError(null);
      const res = await api.setBankrollFunds({
        active_bankroll: numAmount,
        bankroll_cap: isNaN(numCap) ? numAmount : numCap,
        minimum_bankroll: isNaN(numMin) ? 0 : numMin,
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

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-sm animate-fade-in">
      <div className="bg-white dark:bg-[#202528] rounded-2xl w-full max-w-lg shadow-2xl border border-slate-100 dark:border-slate-800 overflow-hidden transform transition-all">
        {/* Header */}
        <div className="p-6 border-b border-slate-100 dark:border-slate-800/80 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-emerald-50 dark:bg-emerald-950/40 text-emerald-500 flex items-center justify-center shadow-sm">
              <Wallet className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-lg font-extrabold text-[#363949] dark:text-white">
                自定义模拟盘资金
              </h3>
              <p className="text-xs text-[#7d8da1]">
                设置或重置虚拟本金 · 自动解除熔断停机状态
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

        {/* Form Body */}
        <form onSubmit={handleSubmit} className="p-6 space-y-5">
          {error && (
            <div className="p-3.5 bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/50 rounded-xl flex items-center gap-2.5 text-xs text-rose-600 dark:text-rose-400">
              <AlertCircle className="w-4 h-4 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {/* Quick Presets */}
          <div>
            <label className="block text-xs font-bold text-[#7d8da1] mb-2 uppercase tracking-wider">
              快捷金额预设
            </label>
            <div className="grid grid-cols-3 sm:grid-cols-6 gap-2">
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
          <div>
            <label className="block text-xs font-bold text-[#363949] dark:text-slate-200 mb-1.5">
              活跃初始本金 (USDC)
            </label>
            <div className="relative">
              <span className="absolute left-3.5 top-1/2 -translate-y-1/2 text-[#7d8da1] font-mono font-bold">
                $
              </span>
              <input
                type="number"
                step="any"
                min="0.1"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                required
                placeholder="例如: 100.00"
                className="w-full pl-8 pr-4 py-2.5 bg-slate-50 dark:bg-[#181a1e] border border-slate-200 dark:border-slate-700 rounded-xl font-mono text-base font-bold text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#1b9c85]"
              />
            </div>
          </div>

          {/* Cap & Min Floor in grid */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-xs font-bold text-[#363949] dark:text-slate-200 mb-1.5">
                模式 B 动态硬顶 (USDC)
              </label>
              <input
                type="number"
                step="any"
                min="0.1"
                value={cap}
                onChange={(e) => setCap(e.target.value)}
                placeholder="默认等于初始本金"
                className="w-full px-3.5 py-2.5 bg-slate-50 dark:bg-[#181a1e] border border-slate-200 dark:border-slate-700 rounded-xl font-mono text-sm text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#1b9c85]"
              />
              <span className="text-[10px] text-[#7d8da1] mt-1 block">超过硬顶的盈利将自动划入锁定利润金库</span>
            </div>

            <div>
              <label className="block text-xs font-bold text-[#363949] dark:text-slate-200 mb-1.5">
                最低停机底线 (Floor)
              </label>
              <input
                type="number"
                step="any"
                min="0"
                value={minFloor}
                onChange={(e) => setMinFloor(e.target.value)}
                placeholder="设为 0 表示不限制"
                className="w-full px-3.5 py-2.5 bg-slate-50 dark:bg-[#181a1e] border border-slate-200 dark:border-slate-700 rounded-xl font-mono text-sm text-[#363949] dark:text-white focus:outline-none focus:ring-2 focus:ring-[#1b9c85]"
              />
              <span className="text-[10px] text-[#7d8da1] mt-1 block">本金触及此值将自动停机熔断，推荐设为 0</span>
            </div>
          </div>

          {/* Notice */}
          <div className="p-3 bg-emerald-50/60 dark:bg-emerald-950/20 border border-emerald-200/60 dark:border-emerald-900/40 rounded-xl text-xs text-emerald-700 dark:text-emerald-400 flex items-start gap-2">
            <ShieldCheck className="w-4 h-4 shrink-0 mt-0.5" />
            <span>注入资金后，系统将自动清除熔断停机标志并重置回撤记录，模拟盘将即可恢复自动扫描与买入撮合。</span>
          </div>

          {/* Submit Button */}
          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 py-3 border border-slate-200 dark:border-slate-700 text-[#7d8da1] font-bold rounded-xl hover:bg-slate-50 dark:hover:bg-slate-800 transition"
            >
              取消
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="flex-1 py-3 bg-[#1b9c85] hover:bg-[#178572] text-white font-extrabold rounded-xl shadow-lg shadow-[#1b9c85]/20 flex items-center justify-center gap-2 transition disabled:opacity-50"
            >
              {isSubmitting ? (
                <>
                  <RefreshCw className="w-4 h-4 animate-spin" />
                  <span>正在更新本金...</span>
                </>
              ) : (
                <>
                  <DollarSign className="w-4 h-4" />
                  <span>确认注入本金</span>
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
