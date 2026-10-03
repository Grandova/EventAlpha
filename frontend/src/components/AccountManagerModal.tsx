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
  ExternalLink,
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
      setSuccessMsg(`余额刷新成功: $${res.balance_usdc.toFixed(2)} USDC`);
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
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-md animate-fadeIn">
      <div className="relative w-full max-w-2xl max-h-[90vh] flex flex-col bg-slate-900/95 border border-slate-700/80 rounded-3xl shadow-2xl shadow-cyan-950/40 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-800 bg-slate-950/50">
          <div className="flex items-center gap-3">
            <div className="p-2.5 rounded-2xl bg-gradient-to-tr from-cyan-500/20 to-indigo-500/20 text-cyan-400 border border-cyan-500/30">
              <Wallet className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-base font-extrabold text-white tracking-wide flex items-center gap-2">
                Polymarket 账户与实盘授权
              </h3>
              <p className="text-xs text-slate-400">
                管理已绑定的 Polygon L2 CLOB 交易账户及资金状态
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 rounded-xl text-slate-400 hover:text-white hover:bg-slate-800/80 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content Area */}
        <div className="p-6 overflow-y-auto space-y-5 flex-1">
          {/* Notifications */}
          {error && (
            <div className="flex items-center gap-2 p-3 rounded-2xl bg-rose-950/50 border border-rose-800/60 text-rose-300 text-xs font-mono">
              <AlertTriangle className="w-4 h-4 shrink-0 text-rose-400" />
              <span>{error}</span>
            </div>
          )}

          {successMsg && (
            <div className="flex items-center gap-2 p-3 rounded-2xl bg-emerald-950/50 border border-emerald-800/60 text-emerald-300 text-xs font-mono">
              <CheckCircle className="w-4 h-4 shrink-0 text-emerald-400" />
              <span>{successMsg}</span>
            </div>
          )}

          {/* Security Notice */}
          <div className="p-4 rounded-2xl bg-cyan-950/20 border border-cyan-800/40 text-xs text-slate-300 space-y-1.5">
            <div className="flex items-center gap-2 font-bold text-cyan-400">
              <Lock className="w-4 h-4" />
              <span>安全隔离与本地私钥保护机制</span>
            </div>
            <p className="text-[11px] text-slate-400 leading-relaxed">
              API Secret 与 Passphrase 仅存储于宿主机本地 SQLite 数据库中，用于计算 Polymarket CLOB HMAC-SHA256 签名，前台展示自动掩码掩盖。
            </p>
          </div>

          {/* Account List */}
          <div>
            <div className="flex items-center justify-between mb-3">
              <span className="text-xs font-bold uppercase tracking-wider text-slate-400 font-mono">
                已绑定账户 ({accounts.length})
              </span>
              {!showAddForm && (
                <button
                  onClick={() => setShowAddForm(true)}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-gradient-to-r from-cyan-500 to-indigo-500 text-white text-xs font-bold hover:brightness-110 shadow-md shadow-cyan-500/20 transition-all cursor-pointer"
                >
                  <Plus className="w-3.5 h-3.5" />
                  <span>添加新账户</span>
                </button>
              )}
            </div>

            {accounts.length === 0 && !showAddForm ? (
              <div className="text-center py-10 px-4 rounded-2xl border border-dashed border-slate-800 bg-slate-950/40">
                <Wallet className="w-10 h-10 text-slate-600 mx-auto mb-2" />
                <p className="text-sm font-semibold text-slate-300">尚未添加任何 Polymarket 账户</p>
                <p className="text-xs text-slate-500 mt-1 max-w-sm mx-auto">
                  实盘交易必须至少绑定一个具有 Polygon USDC 余额的账户方可解锁。
                </p>
                <button
                  onClick={() => setShowAddForm(true)}
                  className="mt-4 inline-flex items-center gap-2 px-4 py-2 rounded-xl bg-cyan-500 text-white text-xs font-bold hover:bg-cyan-400 transition-all cursor-pointer"
                >
                  <Plus className="w-4 h-4" />
                  <span>立即添加 Polymarket 账户</span>
                </button>
              </div>
            ) : (
              <div className="space-y-3">
                {accounts.map((acc) => (
                  <div
                    key={acc.id}
                    className={`p-4 rounded-2xl border transition-all duration-200 ${
                      acc.is_active
                        ? 'bg-slate-900/90 border-cyan-500/50 shadow-lg shadow-cyan-950/30 ring-1 ring-cyan-500/30'
                        : 'bg-slate-950/60 border-slate-800 hover:border-slate-700'
                    }`}
                  >
                    <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                      <div>
                        <div className="flex items-center gap-2.5">
                          <h4 className="text-sm font-bold text-white tracking-wide">
                            {acc.label}
                          </h4>
                          {acc.is_active ? (
                            <span className="flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-cyan-500/20 text-cyan-400 border border-cyan-500/30 font-mono">
                              <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 animate-pulse"></span>
                              当前活跃
                            </span>
                          ) : (
                            <span className="px-2 py-0.5 rounded-full text-[10px] font-medium bg-slate-800 text-slate-400 font-mono">
                              待命
                            </span>
                          )}
                        </div>

                        <div className="mt-2 flex flex-wrap items-center gap-3 text-xs font-mono text-slate-400">
                          <div>
                            <span className="text-slate-500">钱包:</span>{' '}
                            <span className="text-slate-300">
                              {acc.wallet_address.slice(0, 6)}...{acc.wallet_address.slice(-4)}
                            </span>
                          </div>
                          <div>
                            <span className="text-slate-500">API Key:</span>{' '}
                            <span className="text-slate-300">{acc.api_key_masked}</span>
                          </div>
                          {acc.proxy_wallet_address && (
                            <div>
                              <span className="text-slate-500">代理:</span>{' '}
                              <span className="text-slate-300">
                                {acc.proxy_wallet_address.slice(0, 6)}...
                              </span>
                            </div>
                          )}
                        </div>
                      </div>

                      {/* Right: Balance & Actions */}
                      <div className="flex items-center gap-3">
                        <div className="text-right">
                          <div className="text-[10px] text-slate-500 uppercase font-mono">
                            Polygon USDC
                          </div>
                          <div className="text-sm font-bold font-mono text-cyan-400">
                            ${acc.balance_usdc.toFixed(2)}
                          </div>
                        </div>

                        <div className="flex items-center gap-1.5">
                          <button
                            title="刷新链上余额"
                            disabled={refreshingId === acc.id}
                            onClick={() => handleRefreshBalance(acc.id)}
                            className="p-2 rounded-xl bg-slate-800/80 hover:bg-slate-700 text-slate-300 hover:text-white transition-all cursor-pointer disabled:opacity-50"
                          >
                            <RefreshCw
                              className={`w-3.5 h-3.5 ${
                                refreshingId === acc.id ? 'animate-spin text-cyan-400' : ''
                              }`}
                            />
                          </button>

                          {!acc.is_active && (
                            <button
                              onClick={() => handleActivate(acc.id)}
                              className="px-2.5 py-1.5 rounded-xl bg-slate-800 hover:bg-cyan-500/20 text-slate-300 hover:text-cyan-400 border border-slate-700 hover:border-cyan-500/40 text-xs font-bold transition-all cursor-pointer"
                            >
                              激活
                            </button>
                          )}

                          <button
                            title="删除账户"
                            onClick={() => handleDelete(acc.id, acc.label)}
                            className="p-2 rounded-xl bg-slate-800/80 hover:bg-rose-950/60 text-slate-400 hover:text-rose-400 border border-slate-700/60 hover:border-rose-800/50 transition-all cursor-pointer"
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
            <div className="p-5 rounded-2xl bg-slate-950/80 border border-slate-700/80 space-y-4 animate-slideDown">
              <div className="flex items-center justify-between border-b border-slate-800/80 pb-3">
                <h4 className="text-sm font-bold text-white flex items-center gap-2">
                  <Key className="w-4 h-4 text-cyan-400" />
                  <span>绑定 Polymarket CLOB 凭据</span>
                </h4>
                <button
                  type="button"
                  onClick={() => setShowAddForm(false)}
                  className="text-xs text-slate-400 hover:text-white"
                >
                  取消
                </button>
              </div>

              <form onSubmit={handleFormSubmit} className="space-y-3.5">
                <div>
                  <label className="block text-xs font-semibold text-slate-400 mb-1">
                    账户别名 / 标签 *
                  </label>
                  <input
                    type="text"
                    required
                    placeholder="例: Polymarket 主力实盘账户 01"
                    value={formData.label}
                    onChange={(e) => setFormData({ ...formData, label: e.target.value })}
                    className="w-full px-3.5 py-2 rounded-xl bg-slate-900 border border-slate-700/80 text-white text-xs placeholder:text-slate-600 focus:outline-none focus:border-cyan-500 transition-colors"
                  />
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div>
                    <label className="block text-xs font-semibold text-slate-400 mb-1">
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
                      className="w-full px-3.5 py-2 rounded-xl bg-slate-900 border border-slate-700/80 text-white text-xs font-mono placeholder:text-slate-600 focus:outline-none focus:border-cyan-500 transition-colors"
                    />
                  </div>

                  <div>
                    <label className="block text-xs font-semibold text-slate-400 mb-1">
                      Proxy 代理钱包 (可选)
                    </label>
                    <input
                      type="text"
                      placeholder="0x... (若使用 Safe 或 Email 登录)"
                      value={formData.proxy_wallet_address || ''}
                      onChange={(e) =>
                        setFormData({ ...formData, proxy_wallet_address: e.target.value })
                      }
                      className="w-full px-3.5 py-2 rounded-xl bg-slate-900 border border-slate-700/80 text-white text-xs font-mono placeholder:text-slate-600 focus:outline-none focus:border-cyan-500 transition-colors"
                    />
                  </div>
                </div>

                <div className="space-y-3 pt-1">
                  <div>
                    <label className="block text-xs font-semibold text-slate-400 mb-1">
                      Polymarket CLOB API Key *
                    </label>
                    <input
                      type="text"
                      required
                      placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
                      value={formData.api_key}
                      onChange={(e) => setFormData({ ...formData, api_key: e.target.value })}
                      className="w-full px-3.5 py-2 rounded-xl bg-slate-900 border border-slate-700/80 text-white text-xs font-mono placeholder:text-slate-600 focus:outline-none focus:border-cyan-500 transition-colors"
                    />
                  </div>

                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <div>
                      <label className="block text-xs font-semibold text-slate-400 mb-1">
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
                        className="w-full px-3.5 py-2 rounded-xl bg-slate-900 border border-slate-700/80 text-white text-xs font-mono placeholder:text-slate-600 focus:outline-none focus:border-cyan-500 transition-colors"
                      />
                    </div>

                    <div>
                      <label className="block text-xs font-semibold text-slate-400 mb-1">
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
                        className="w-full px-3.5 py-2 rounded-xl bg-slate-900 border border-slate-700/80 text-white text-xs font-mono placeholder:text-slate-600 focus:outline-none focus:border-cyan-500 transition-colors"
                      />
                    </div>
                  </div>
                </div>

                <div className="flex items-center justify-end gap-3 pt-3">
                  <button
                    type="button"
                    onClick={() => setShowAddForm(false)}
                    className="px-4 py-2 rounded-xl bg-slate-800 text-slate-300 hover:text-white text-xs font-semibold transition-colors cursor-pointer"
                  >
                    取消
                  </button>
                  <button
                    type="submit"
                    disabled={loading}
                    className="flex items-center gap-1.5 px-5 py-2 rounded-xl bg-gradient-to-r from-cyan-500 to-indigo-500 text-white text-xs font-bold hover:brightness-110 shadow-lg shadow-cyan-500/25 transition-all cursor-pointer disabled:opacity-50"
                  >
                    <ShieldCheck className="w-4 h-4" />
                    <span>{loading ? '验证并加密保存中...' : '确认安全添加'}</span>
                  </button>
                </div>
              </form>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-slate-800 bg-slate-950/60 flex items-center justify-between text-xs text-slate-500 font-mono">
          <div className="flex items-center gap-1.5">
            <span className="w-2 h-2 rounded-full bg-emerald-400"></span>
            <span>POLYGON NETWORK MAINNET</span>
          </div>
          <button
            onClick={onClose}
            className="px-4 py-1.5 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white text-xs font-bold transition-all cursor-pointer"
          >
            完成
          </button>
        </div>
      </div>
    </div>
  );
};
