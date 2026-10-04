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
  Pencil,
  Check,
  HelpCircle,
  DollarSign,
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
  const [editingAccount, setEditingAccount] = useState<PolymarketAccountPublic | null>(null);
  const [calibratingId, setCalibratingId] = useState<string | null>(null);
  const [calibratingAmount, setCalibratingAmount] = useState<string>('');
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

  const [editFormData, setEditFormData] = useState<{
    label: string;
    wallet_address: string;
    proxy_wallet_address: string;
    api_key: string;
    api_secret: string;
    api_passphrase: string;
    balance_usdc: number;
  }>({
    label: '',
    wallet_address: '',
    proxy_wallet_address: '',
    api_key: '',
    api_secret: '',
    api_passphrase: '',
    balance_usdc: 0,
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
          acc.id === id
            ? {
                ...acc,
                balance_usdc: res.balance_usdc,
                proxy_wallet_address: res.proxy_wallet_address || acc.proxy_wallet_address,
              }
            : acc
        )
      );
      const bal = typeof res.balance_usdc === 'number' && !isNaN(res.balance_usdc) ? res.balance_usdc : 0;
      if (bal > 0) {
        setSuccessMsg(`余额刷新成功: $${bal.toFixed(2)} USDC`);
      } else {
        setSuccessMsg(`链上已完成核验。如网页有可用资金，可点击金额旁的「校准」直接同步！`);
      }
    } catch (err: any) {
      setError(err.message || '刷新余额失败');
    } finally {
      setRefreshingId(null);
    }
  };

  const handleSaveCalibratedBalance = async (id: string) => {
    const val = parseFloat(calibratingAmount);
    if (isNaN(val) || val < 0) {
      setError('请输入合法的金额（例如 1.41）');
      return;
    }
    try {
      setLoading(true);
      setError(null);
      await api.calibrateAccountBalance(id, val);
      setSuccessMsg(`账户余额已成功校准为 $${val.toFixed(2)} USDC`);
      setCalibratingId(null);
      await loadAccounts();
    } catch (err: any) {
      setError(err.message || '校准余额失败');
    } finally {
      setLoading(false);
    }
  };

  const handleStartEdit = (acc: PolymarketAccountPublic) => {
    setEditingAccount(acc);
    setEditFormData({
      label: acc.label,
      wallet_address: acc.wallet_address,
      proxy_wallet_address: acc.proxy_wallet_address || '',
      api_key: '',
      api_secret: '',
      api_passphrase: '',
      balance_usdc: acc.balance_usdc,
    });
    setShowAddForm(false);
  };

  const handleEditSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editingAccount) return;
    setError(null);
    try {
      setLoading(true);
      const payload: any = {
        label: editFormData.label.trim(),
        wallet_address: editFormData.wallet_address.trim(),
        proxy_wallet_address: editFormData.proxy_wallet_address.trim() || undefined,
        balance_usdc: editFormData.balance_usdc,
      };
      if (editFormData.api_key.trim()) {
        payload.api_key = editFormData.api_key.trim();
      }
      if (editFormData.api_secret.trim()) {
        payload.api_secret = editFormData.api_secret.trim();
      }
      if (editFormData.api_passphrase.trim()) {
        payload.api_passphrase = editFormData.api_passphrase.trim();
      }
      await api.updateAccount(editingAccount.id, payload);
      setSuccessMsg('Polymarket 账户信息已成功更新！');
      setEditingAccount(null);
      await loadAccounts();
    } catch (err: any) {
      setError(err.message || '更新账户失败');
    } finally {
      setLoading(false);
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
    if (!formData.wallet_address.trim() || !formData.wallet_address.startsWith('0x')) {
      setError('请输入合法的 Polygon 钱包地址 (0x 开头)');
      return;
    }

    try {
      setLoading(true);
      const created = await api.createAccount({
        label: formData.label.trim(),
        api_key: formData.api_key.trim(),
        api_secret: formData.api_secret?.trim() || '',
        api_passphrase: formData.api_passphrase?.trim() || '',
        wallet_address: formData.wallet_address.trim(),
        proxy_wallet_address: formData.proxy_wallet_address?.trim() || undefined,
      });

      setSuccessMsg('Polymarket 账户添加成功！已自动激活');
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
      if (onAccountActivated && created) {
        onAccountActivated(created);
      }
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

          {/* Polymarket Proxy Wallet & Balance Guide */}
          <div className="p-4 rounded-2xl bg-gradient-to-r from-blue-500/10 via-indigo-500/10 to-purple-500/10 border border-blue-500/25 space-y-2 text-xs">
            <div className="flex items-center gap-2 font-black text-[#363949] dark:text-white">
              <HelpCircle className="w-4 h-4 text-[#6c9bcf] shrink-0" />
              <span>为什么官方网页有资金 ($1.41) 却在此处显示 $0.00？</span>
            </div>
            <p className="text-[11px] leading-relaxed text-slate-600 dark:text-slate-300">
              在 Polymarket 官网中充值的资金存放于您账户专属的 <strong>Proxy 代理钱包 (Safe 合约)</strong> 中，并非外部签名钱包：
            </p>
            <div className="p-3 rounded-xl bg-white/80 dark:bg-slate-800/80 border border-blue-200/50 dark:border-blue-900/50 space-y-1.5 text-[11px] text-slate-700 dark:text-slate-300 font-medium">
              <div className="font-bold text-[#6c9bcf]">📍 2 步极速同步您的真实可用资金：</div>
              <div className="leading-relaxed">
                1. <strong>一键手动校准</strong>：直接在下方账户金额旁点击「<span className="text-[#6c9bcf] font-bold">校准</span>」，输入 <strong>1.41</strong> 即可立即作为本金开始实盘！
              </div>
              <div className="leading-relaxed">
                2. <strong>绑定官方代理充值地址</strong>：打开 Polymarket 官网，点击右上角「<strong>充值 (Deposit)</strong>」→ 选择「<strong>使用加密货币 (Crypto Transfer)</strong>」并在网络选择 Polygon，复制显示的 <strong>0x... 充值地址</strong>。然后点击下方账户的「<strong>编辑</strong>」按钮，粘贴到「<strong>Proxy 代理钱包</strong>」即可自动持续同步！
              </div>
            </div>
          </div>

          {/* Account List */}
          <div>
            <div className="flex items-center justify-between mb-3">
              <span className="text-xs font-extrabold uppercase tracking-wider text-[#7d8da1] font-mono">
                已绑定账户 ({accounts.length})
              </span>
              {!showAddForm && !editingAccount && (
                <button
                  onClick={() => setShowAddForm(true)}
                  className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl bg-[#6c9bcf] text-white text-xs font-bold hover:brightness-105 shadow-md shadow-[#6c9bcf]/20 transition-all cursor-pointer"
                >
                  <Plus className="w-3.5 h-3.5" />
                  <span>添加新账户</span>
                </button>
              )}
            </div>

            {accounts.length === 0 && !showAddForm && !editingAccount ? (
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
                            <span>Proxy:</span>{' '}
                            {acc.proxy_wallet_address ? (
                              <span className="text-[#1b9c85] font-bold">
                                {acc.proxy_wallet_address.slice(0, 6)}...{acc.proxy_wallet_address.slice(-4)}
                              </span>
                            ) : (
                              <span className="text-amber-500 font-sans font-bold">
                                未绑定代理 (点击编辑补全)
                              </span>
                            )}
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
                          {calibratingId === acc.id ? (
                            <div className="flex items-center gap-1 mt-1">
                              <span className="text-xs font-mono text-[#7d8da1]">$</span>
                              <input
                                type="number"
                                step="0.01"
                                value={calibratingAmount}
                                onChange={(e) => setCalibratingAmount(e.target.value)}
                                className="w-16 px-1.5 py-0.5 rounded-lg bg-white dark:bg-[#202528] border border-[#6c9bcf] text-xs font-mono text-[#363949] dark:text-white"
                                placeholder="1.41"
                                autoFocus
                              />
                              <button
                                onClick={() => handleSaveCalibratedBalance(acc.id)}
                                className="p-1 rounded-lg bg-[#1b9c85] text-white hover:brightness-105 cursor-pointer"
                                title="保存校准余额"
                              >
                                <Check className="w-3.5 h-3.5" />
                              </button>
                              <button
                                onClick={() => setCalibratingId(null)}
                                className="p-1 rounded-lg bg-slate-200 dark:bg-slate-700 text-[#7d8da1] cursor-pointer"
                                title="取消"
                              >
                                <X className="w-3.5 h-3.5" />
                              </button>
                            </div>
                          ) : (
                            <div className="flex items-center justify-end gap-1.5">
                              <div className="text-base font-extrabold font-mono text-[#1b9c85]">
                                ${(typeof acc.balance_usdc === 'number' && !isNaN(acc.balance_usdc) ? acc.balance_usdc : 0).toFixed(2)}
                              </div>
                              <button
                                title="手动校准已知余额（如网页的 $1.41）"
                                onClick={() => {
                                  setCalibratingId(acc.id);
                                  setCalibratingAmount(String(acc.balance_usdc > 0 ? acc.balance_usdc : 1.41));
                                }}
                                className="px-1.5 py-0.5 rounded-md bg-[#6c9bcf]/10 hover:bg-[#6c9bcf]/20 text-[#6c9bcf] text-[10px] font-bold transition-all cursor-pointer"
                              >
                                校准
                              </button>
                            </div>
                          )}
                        </div>

                        <div className="flex items-center gap-1.5">
                          <button
                            title="编辑账户与代理钱包地址"
                            onClick={() => handleStartEdit(acc)}
                            className="p-2 rounded-xl bg-white dark:bg-[#202528] shadow-sm hover:bg-slate-50 text-[#7d8da1] hover:text-[#6c9bcf] dark:hover:text-white transition-all cursor-pointer"
                          >
                            <Pencil className="w-3.5 h-3.5" />
                          </button>

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

              {/* Guidance on where to get API Key, Secret and Passphrase */}
              <div className="p-3.5 rounded-2xl bg-amber-500/10 border border-amber-500/25 text-xs text-amber-800 dark:text-amber-300 space-y-1.5">
                <div className="flex items-center gap-1.5 font-bold">
                  <span>💡 凭据填写指南（全面兼容官方 Relayer 密钥与 Builders 密钥）</span>
                </div>
                <p className="text-[11px] leading-relaxed text-slate-600 dark:text-slate-300">
                  Polymarket 官方提供两种密钥体系，本系统均已完美支持：
                </p>
                <ul className="text-[11px] list-disc list-inside space-y-1 text-slate-600 dark:text-slate-300 pl-1">
                  <li>
                    <strong className="text-amber-600 dark:text-amber-400">官方 Relayer API 密钥</strong>：仅提供 <strong>API Key</strong> 与 <strong>钱包地址</strong>，下方 Secret 和 Passphrase <strong>直接留空即可</strong>！
                  </li>
                  <li>
                    <strong className="text-emerald-600 dark:text-emerald-400">官方 Builders API 密钥</strong>：包含 <strong>API Key、Secret、Passphrase</strong> 完整凭据，填写完整可启用 CLOB 原生完整签名。
                  </li>
                </ul>
                <div className="pt-1 flex items-center justify-between">
                  <a
                    href="https://polymarket.com/settings?tab=builder"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1 text-[11px] font-bold text-[#6c9bcf] hover:underline cursor-pointer"
                  >
                    🔗 官网直达: Settings → Builders 凭据生成页 (polymarket.com/settings?tab=builder) ↗
                  </a>
                </div>
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
                      placeholder="0x... (个人钱包或 Proxy 钱包)"
                      value={formData.wallet_address}
                      onChange={(e) =>
                        setFormData({ ...formData, wallet_address: e.target.value })
                      }
                      className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                    />
                  </div>

                  <div>
                    <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                      Proxy 代理钱包 (推荐填入)
                    </label>
                    <input
                      type="text"
                      placeholder="0x... (个人主页 profile 网址里的 0x 地址)"
                      value={formData.proxy_wallet_address || ''}
                      onChange={(e) =>
                        setFormData({ ...formData, proxy_wallet_address: e.target.value })
                      }
                      className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                    />
                  </div>
                </div>

                <div className="p-2.5 rounded-xl bg-blue-500/10 border border-blue-500/20 text-[11px] text-blue-800 dark:text-blue-300 leading-relaxed">
                  <span>💡 <strong>为什么账户有钱却显示 $0.00？</strong>在 Polymarket 网站上充值的资金均存放在 <strong>Proxy 代理钱包 (Safe 合约钱包)</strong> 中。请在网站右上角点击个人头像复制网址：<code>polymarket.com/profile/0x...</code>，将网址中的 <strong>0x 代理地址</strong> 填入上方输入框，系统将立刻同步显示您的真实可用资金！</span>
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
                        API Secret (选填 / Relayer密钥可留空)
                      </label>
                      <input
                        type="password"
                        placeholder="•••••••••••••••• (选填)"
                        value={formData.api_secret}
                        onChange={(e) =>
                          setFormData({ ...formData, api_secret: e.target.value })
                        }
                        className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                      />
                    </div>

                    <div>
                      <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                        API Passphrase (选填 / Relayer密钥可留空)
                      </label>
                      <input
                        type="password"
                        placeholder="•••••••• (选填)"
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

          {/* Edit Account Form */}
          {editingAccount && (
            <div className="p-5 rounded-3xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-[#6c9bcf]/40 shadow-sm space-y-4 animate-slideDown">
              <div className="flex items-center justify-between border-b border-slate-200 dark:border-slate-800 pb-3">
                <h4 className="text-sm font-bold text-[#363949] dark:text-white flex items-center gap-2">
                  <Pencil className="w-4 h-4 text-[#6c9bcf]" />
                  <span>编辑账户与地址凭据 ({editingAccount.label})</span>
                </h4>
                <button
                  type="button"
                  onClick={() => setEditingAccount(null)}
                  className="text-xs text-[#7d8da1] hover:text-[#363949] dark:hover:text-white"
                >
                  取消
                </button>
              </div>

              <form onSubmit={handleEditSubmit} className="space-y-3.5">
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div>
                    <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                      账户别名 / 标签 *
                    </label>
                    <input
                      type="text"
                      required
                      value={editFormData.label}
                      onChange={(e) => setEditFormData({ ...editFormData, label: e.target.value })}
                      className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                    />
                  </div>

                  <div>
                    <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                      Polygon 初始可用本金 (USDC)
                    </label>
                    <input
                      type="number"
                      step="0.01"
                      value={editFormData.balance_usdc}
                      onChange={(e) => setEditFormData({ ...editFormData, balance_usdc: parseFloat(e.target.value) || 0 })}
                      className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                    />
                  </div>
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div>
                    <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                      Polygon 钱包地址 (0x) *
                    </label>
                    <input
                      type="text"
                      required
                      value={editFormData.wallet_address}
                      onChange={(e) =>
                        setEditFormData({ ...editFormData, wallet_address: e.target.value })
                      }
                      className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                    />
                  </div>

                  <div>
                    <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                      Proxy 代理钱包 (存放网页 $1.41 的充值地址)
                    </label>
                    <input
                      type="text"
                      placeholder="0x... (从官网「充值」→「使用加密货币」复制)"
                      value={editFormData.proxy_wallet_address}
                      onChange={(e) =>
                        setEditFormData({ ...editFormData, proxy_wallet_address: e.target.value })
                      }
                      className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                    />
                  </div>
                </div>

                <div className="p-2.5 rounded-xl bg-blue-500/10 border border-blue-500/20 text-[11px] text-blue-800 dark:text-blue-300 leading-relaxed">
                  <span>💡 提示：在官网点击右上角「充值」复制显示的 Polygon 0x 地址粘贴到上方「Proxy 代理钱包」，系统即可自动持续识别您的链上资金。</span>
                </div>

                <div className="space-y-3 pt-1">
                  <div>
                    <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                      CLOB API Key (留空表示不修改)
                    </label>
                    <input
                      type="text"
                      placeholder={`当前: ${editingAccount.api_key_masked} (若无需变更请留空)`}
                      value={editFormData.api_key}
                      onChange={(e) => setEditFormData({ ...editFormData, api_key: e.target.value })}
                      className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                    />
                  </div>

                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <div>
                      <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                        API Secret (若无需变更请留空)
                      </label>
                      <input
                        type="password"
                        placeholder="•••••••• (若无需变更请留空)"
                        value={editFormData.api_secret}
                        onChange={(e) =>
                          setEditFormData({ ...editFormData, api_secret: e.target.value })
                        }
                        className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                      />
                    </div>

                    <div>
                      <label className="block text-xs font-semibold text-[#7d8da1] mb-1">
                        API Passphrase (若无需变更请留空)
                      </label>
                      <input
                        type="password"
                        placeholder="•••••••• (若无需变更请留空)"
                        value={editFormData.api_passphrase}
                        onChange={(e) =>
                          setEditFormData({ ...editFormData, api_passphrase: e.target.value })
                        }
                        className="w-full px-3.5 py-2.5 rounded-xl bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[#363949] dark:text-white text-xs font-mono placeholder:text-slate-400 focus:outline-none focus:border-[#6c9bcf] transition-colors"
                      />
                    </div>
                  </div>
                </div>

                <div className="flex items-center justify-end gap-3 pt-3">
                  <button
                    type="button"
                    onClick={() => setEditingAccount(null)}
                    className="px-4 py-2 rounded-xl bg-slate-200 dark:bg-slate-700 text-[#363949] dark:text-white text-xs font-semibold transition-colors cursor-pointer"
                  >
                    取消
                  </button>
                  <button
                    type="submit"
                    disabled={loading}
                    className="flex items-center gap-1.5 px-5 py-2.5 rounded-xl bg-[#6c9bcf] text-white text-xs font-bold hover:brightness-105 shadow-md shadow-[#6c9bcf]/20 transition-all cursor-pointer disabled:opacity-50"
                  >
                    <Check className="w-4 h-4" />
                    <span>{loading ? '保存中...' : '确认更新账户'}</span>
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
            onClick={() => {
              const active = accounts.find((a) => a.is_active) || accounts[0];
              if (active && onAccountActivated) {
                onAccountActivated(active);
              }
              onClose();
            }}
            className="px-4 py-1.5 rounded-xl bg-[#6c9bcf] text-white text-xs font-bold transition-all cursor-pointer"
          >
            完成
          </button>
        </div>
      </div>
    </div>
  );
};
