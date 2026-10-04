import React, { useState, useEffect } from 'react';
import {
  X,
  Plus,
  Trash2,
  CheckCircle,
  RefreshCw,
  Wallet,
  Key,
  ShieldCheck,
  AlertTriangle,
  Lock,
} from 'lucide-react';
import { api } from '../services/api';
import { PolymarketAccountPublic, CreateAccountRequest } from '../types';

interface AccountManagerModalProps {
  isOpen: boolean;
  onClose: () => void;
  onAccountActivated?: (account: PolymarketAccountPublic) => void;
}

export const AccountManagerModal: React.FC<AccountManagerModalProps> = ({
  isOpen,
  onClose,
  onAccountActivated,
}) => {
  const [accounts, setAccounts] = useState<PolymarketAccountPublic[]>([]);
  const [loading, setLoading] = useState(false);
  const [refreshingId, setRefreshingId] = useState<string | null>(null);
  const [showAddForm, setShowAddForm] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  // Form state
  const [formData, setFormData] = useState<CreateAccountRequest>({
    label: '',
    api_key: '',
    api_secret: '',
    api_passphrase: '',
    wallet_address: '',
    proxy_wallet_address: '',
  });

  const loadAccounts = async () => {
    try {
      setLoading(true);
      setError(null);
      const list = await api.getAccounts();
      setAccounts(list);
    } catch (err: any) {
      setError(err.message || '加载账户列表失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      loadAccounts();
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleActivate = async (id: string) => {
    try {
      setError(null);
      await api.activateAccount(id);
      setSuccessMsg('已切换为当前活跃账户');
      await loadAccounts();
      const activated = accounts.find((a) => a.id === id);
      if (activated && onAccountActivated) {
        onAccountActivated({ ...activated, is_active: true });
      }
    } catch (err: any) {
      setError(err.message || '激活账户失败');
    }
  };

  const handleRefreshBalance = async (id: string) => {
    try {
      setRefreshingId(id);
      setError(null);
      const res = await api.getAccountBalance(id);
      setAccounts((prev) =>
        prev.map((acc) =>
          acc.id === id ? { ...acc, balance_usdc: res.balance_usdc } : acc
        )
      );
      const bal = typeof res.balance_usdc === 'number' && !isNaN(res.balance_usdc) ? res.balance_usdc : 0;
      setSuccessMsg(`余额刷新成功: $${bal.toFixed(2)} USDC`);
    } catch (err: any) {
      setError(err.message || '刷新余额失败');
    } finally {
      setRefreshingId(null);
    }
  };

  const handleDelete = async (id: string, label: string) => {
    if (!window.confirm(`确定要删除账户 "${label}" 吗？此操作无法撤销。`)) {
      return;
    }
    try {
      setError(null);
      await api.deleteAccount(id);
      setSuccessMsg(`账户 "${label}" 已删除`);
      await loadAccounts();
    } catch (err: any) {
      setError(err.message || '删除账户失败');
    }
  };

  const handleFormSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!formData.label.trim()) {
      setError('请输入账户标识名称');
      return;
    }
    if (!formData.api_key.trim()) {
      setError('请输入 Polymarket API Key');
      return;
    }
    if (!formData.api_secret.trim()) {
      setError('请输入 Polymarket API Secret');
      return;
    }
    if (!formData.api_passphrase.trim()) {
      setError('请输入 Polymarket API Passphrase');
      return;
    }
    if (!formData.wallet_address.trim() || !formData.wallet_address.startsWith('0x')) {
      setError('请输入合法的 Polygon 钱包地址 (0x 开头)');
      return;
    }

    try {
      setLoading(true);
      await api.createAccount({
        label: formData.label.trim(),
        api_key: formData.api_key.trim(),
        api_secret: formData.api_secret.trim(),
        api_passphrase: formData.api_passphrase.trim(),
        wallet_address: formData.wallet_address.trim(),
        proxy_wallet_address: formData.proxy_wallet_address?.trim() || undefined,
      });

      setSuccessMsg('Polymarket 账户添加成功！');
      setShowAddForm(false);
      setFormData({
        label: '',
        api_key: '',
        api_secret: '',
        api_passphrase: '',
        wallet_address: '',
        proxy_wallet_address: '',
      });
      await loadAccounts();
    } catch (err: any) {
      setError(err.message || '添加账户失败');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 dark:bg-black/70 backdrop-blur-sm animate-fadeIn">
      <div className="relative w-full max-w-2xl max-h-[90vh] flex flex-col bg-white dark:bg-[#202528] text-[#363949] dark:text-[#edeffd] rounded-3xl shadow-[0_2rem_3rem_rgba(132,139,200,0.3)] dark:shadow-2xl overflow-hidden border border-slate-100 dark:border-slate-800">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-5 border-b border-slate-100 dark:border-slate-800 bg-[#f6f6f9]/60 dark:bg-[#181a1e]/60">
          <div className="flex items-center gap-3">
            <div className="p-2.5 rounded-2xl bg-[#6c9bcf]/15 text-[#6c9bcf] border border-[#6c9bcf]/30">
              <Wallet className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-base font-extrabold text-[#363949] dark:text-white tracking-wide">
                Polymarket 账户授权与资金管理
              </h3>
              <p className="text-xs text-[#7d8da1] dark:text-slate-400">
                Polygon L2 CLOB 交易账户凭据与链上余额
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 rounded-xl text-[#7d8da1] hover:text-[#363949] dark:hover:text-white hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content Area */}
        <div className="p-6 overflow-y-auto space-y-5 flex-1">
          {error && (
            <div className="flex items-center gap-2 p-3.5 rounded-2xl bg-[#ff0060]/10 border border-[#ff0060]/20 text-[#ff0060] text-xs font-mono">
              <AlertTriangle className="w-4 h-4 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {successMsg && (
            <div className="flex items-center gap-2 p-3.5 rounded-2xl bg-[#1b9c85]/10 border border-[#1b9c85]/20 text-[#1b9c85] text-xs font-mono">
              <CheckCircle className="w-4 h-4 shrink-0" />
              <span>{successMsg}</span>
            </div>
          )}

          {/* Security Notice */}
          <div className="p-4 rounded-2xl bg-[#6c9bcf]/10 border border-[#6c9bcf]/20 text-xs text-[#363949] dark:text-slate-300 space-y-1">
            <div className="flex items-center gap-2 font-bold text-[#6c9bcf]">
              <Lock className="w-4 h-4" />
              <span>本地凭据隔离与 HMAC-SHA256 签名保护</span>
            </div>
            <p className="text-[11px] text-[#7d8da1] dark:text-slate-400 leading-relaxed">
              API Secret 与 Passphrase 仅存储于本地 SQLite 数据库中，用于执行 Polymarket CLOB 请求签名，前端展示已全面脱敏掩码。
            </p>
          </div>

          {/* Account List */}
          <div>
            <div className="flex items-center justify-between mb-3">
              <span className="text-xs font-extrabold uppercase tracking-wider text-[#7d8da1] font-mono">
                已绑定账户 ({accounts.length})
              </span>
              {!showAddForm && (
                <button
                  onClick={() => setShowAddForm(true)}
                  className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl bg-[#6c9bcf] text-white text-xs font-bold hover:brightness-105 shadow-md shadow-[#6c9bcf]/20 transition-all cursor-pointer"
                >
                  <Plus className="w-3.5 h-3.5" />
                  <span>添加新账户</span>
                </button>
              )}
            </div>

            {accounts.length === 0 && !showAddForm ? (
              <div className="text-center py-10 px-4 rounded-3xl border-2 border-dashed border-slate-200 dark:border-slate-800 bg-[#f6f6f9]/50 dark:bg-[#181a1e]/50">
                <Wallet className="w-10 h-10 text-slate-400 mx-auto mb-2" />
                <p className="text-sm font-bold text-[#363949] dark:text-white">尚未绑定任何 Polymarket 账户</p>
                <p className="text-xs text-[#7d8da1] mt-1 max-w-sm mx-auto">
                  实盘交易必须至少绑定一个具有 Polygon USDC 余额的账户。
                </p>
                <button
                  onClick={() => setShowAddForm(true)}
                  className="mt-4 inline-flex items-center gap-2 px-4 py-2 rounded-xl bg-[#6c9bcf] text-white text-xs font-bold hover:bg-[#5b89be] transition-all cursor-pointer"
                >
                  <Plus className="w-4 h-4" />
                  <span>立即添加账户</span>
                </button>
              </div>
            ) : (
              <div className="space-y-3">
                {accounts.map((acc) => (
                  <div
                    key={acc.id}
                    className={`p-4 rounded-2xl border transition-all duration-200 ${
                      acc.is_active
                        ? 'bg-[#6c9bcf]/5 border-[#6c9bcf] shadow-sm'
                        : 'bg-[#f6f6f9] dark:bg-[#181a1e] border-slate-200 dark:border-slate-800 hover:border-slate-300'
                    }`}
                  >
                    <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                      <div>
                        <div className="flex items-center gap-2.5">
                          <h4 className="text-sm font-extrabold text-[#363949] dark:text-white">
                            {acc.label}
                          </h4>
                          {acc.is_active ? (
                            <span className="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-[#1b9c85]/15 text-[#1b9c85] font-mono">
                              当前活跃
                            </span>
                          ) : (
                            <span className="px-2 py-0.5 rounded-full text-[10px] font-medium bg-slate-200 dark:bg-slate-700 text-[#7d8da1] font-mono">
                              待命
                            </span>
                          )}
                        </div>

                        <div className="mt-1.5 flex flex-wrap items-center gap-3 text-xs font-mono text-[#7d8da1]">
                          <div>
                            <span>钱包:</span>{' '}
                            <span className="text-[#363949] dark:text-slate-300 font-bold">
                              {acc.wallet_address.slice(0, 6)}...{acc.wallet_address.slice(-4)}
                            </span>
                          </div>
                          <div>
                            <span>API Key:</span>{' '}
                            <span className="text-[#363949] dark:text-slate-300">{acc.api_key_masked}</span>
                          </div>
                        </div>
                      </div>

                      {/* Right: Balance & Actions */}
                      <div className="flex items-center gap-3">
                        <div className="text-right">
                          <div className="text-[10px] text-[#7d8da1] uppercase font-mono">
                            Polygon USDC
                          </div>
                          <div className="text-base font-extrabold font-mono text-[#1b9c85]">
                            ${(typeof acc.balance_usdc === 'number' && !isNaN(acc.balance_usdc) ? acc.balance_usdc : 0).toFixed(2)}
                          </div>
                        </div>

                        <div className="flex items-center gap-1.5">
                          <button
                            title="刷新链上余额"
                            disabled={refreshingId === acc.id}
                            onClick={() => handleRefreshBalance(acc.id)}
                            className="p-2 rounded-xl bg-white dark:bg-[#202528] shadow-sm hover:bg-slate-50 text-[#7d8da1] hover:text-[#363949] dark:hover:text-white transition-all cursor-pointer disabled:opacity-50"
                          >
                            <RefreshCw
                              className={`w-3.5 h-3.5 ${
                                refreshingId === acc.id ? 'animate-spin text-[#6c9bcf]' : ''
                              }`}
                            />
                          </button>

                          {!acc.is_active && (
                            <button
                              onClick={() => handleActivate(acc.id)}
                              className="px-3 py-1.5 rounded-xl bg-[#6c9bcf] text-white text-xs font-bold hover:brightness-105 transition-all cursor-pointer"
                            >
                              激活
                            </button>
                          )}

                          <button
                            title="删除账户"
                            onClick={() => handleDelete(acc.id, acc.label)}
                            className="p-2 rounded-xl bg-white dark:bg-[#202528] shadow-sm hover:bg-[#ff0060]/10 text-[#7d8da1] hover:text-[#ff0060] transition-all cursor-pointer"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Add Account Form */}
          {showAddForm && (
            <div className="p-5 rounded-3xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200 dark:border-slate-800 space-y-4 animate-slideDown">
              <div className="flex items-center justify-between border-b border-slate-200 dark:border-slate-800 pb-3">
                <h4 className="text-sm font-bold text-[#363949] dark:text-white flex items-center gap-2">
                  <Key className="w-4 h-4 text-[#6c9bcf]" />
                  <span>绑定 Polymarket CLOB 凭据</span>
                </h4>
                <button
                  type="button"
                  onClick={() => setShowAddForm(false)}
                  className="text-xs text-[#7d8da1] hover:text-[#363949] dark:hover:text-white"
                >
                  取消
                </button>
              </div>

              <form onSubmit={handleFormSubmit} className="space-y-3.5">
                <div>
                  <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                    账户别名 / 标签 *
                  </label>
                  <input
                    type="text"
                    required
                    placeholder="例: Polymarket 主力实盘账户 01"
                    value={formData.label}
                    onChange={(e) => setFormData({ ...formData, label: e.target.value })}
                    className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                  />
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div>
                    <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                      Polygon 钱包地址 (0x) *
                    </label>
                    <input
                      type="text"
                      required
                      placeholder="0x..."
                      value={formData.wallet_address}
                      onChange={(e) =>
                        setFormData({ ...formData, wallet_address: e.target.value })
                      }
                      className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                    />
                  </div>

                  <div>
                    <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                      Proxy 代理钱包 (可选)
                    </label>
                    <input
                      type="text"
                      placeholder="0x... (若使用 Safe 或 Email 登录)"
                      value={formData.proxy_wallet_address || ''}
                      onChange={(e) =>
                        setFormData({ ...formData, proxy_wallet_address: e.target.value })
                      }
                      className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                    />
                  </div>
                </div>

                <div className="space-y-3 pt-1">
                  <div>
                    <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                      Polymarket CLOB API Key *
                    </label>
                    <input
                      type="text"
                      required
                      placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
                      value={formData.api_key}
                      onChange={(e) => setFormData({ ...formData, api_key: e.target.value })}
                      className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                    />
                  </div>

                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <div>
                      <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                        API Secret (Base64) *
                      </label>
                      <input
                        type="password"
                        required
                        placeholder="••••••••••••••••"
                        value={formData.api_secret}
                        onChange={(e) =>
                          setFormData({ ...formData, api_secret: e.target.value })
                        }
                        className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                      />
                    </div>

                    <div>
                      <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                        API Passphrase *
                      </label>
                      <input
                        type="password"
                        required
                        placeholder="••••••••"
                        value={formData.api_passphrase}
                        onChange={(e) =>
                          setFormData({ ...formData, api_passphrase: e.target.value })
                        }
                        className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                      />
                    </div>
                  </div>
                </div>

                <div className="flex items-center justify-end gap-3 pt-3">
                  <button
                    type="button"
                    onClick={() => setShowAddForm(false)}
                    className="px-4 py-2 rounded-xl bg-slate-200 dark:bg-slate-700 text-[#363949] dark:text-white text-xs font-semibold transition-colors cursor-pointer"
                  >
                    取消
                  </button>
                  <button
                    type="submit"
                    disabled={loading}
                    className="flex items-center gap-1.5 px-5 py-2.5 rounded-xl bg-[#6c9bcf] text-white text-xs font-bold hover:brightness-105 shadow-md shadow-[#6c9bcf]/20 transition-all cursor-pointer disabled:opacity-50"
                  >
                    <ShieldCheck className="w-4 h-4" />
                    <span>{loading ? '保存中...' : '确认安全绑定'}</span>
                  </button>
                </div>
              </form>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-slate-100 dark:border-slate-800 bg-[#f6f6f9]/60 dark:bg-[#181a1e]/60 flex items-center justify-between text-xs text-[#7d8da1] font-mono">
          <div className="flex items-center gap-1.5">
            <span className="w-2 h-2 rounded-full bg-[#1b9c85]"></span>
            <span>POLYGON MAINNET</span>
          </div>
          <button
            onClick={onClose}
            className="px-4 py-1.5 rounded-xl bg-[#6c9bcf] text-white text-xs font-bold transition-all cursor-pointer"
          >
            完成
          </button>
        </div>
      </div>
    </div>
  );
};
