import React, { useState } from 'react';
import { Lock, User, ShieldCheck, ArrowRight, AlertCircle, Sun, Moon } from 'lucide-react';
import { api, setStoredToken } from '../services/api';

interface LoginPageProps {
  onLoginSuccess: (username: string) => void;
  isDarkMode: boolean;
  onToggleTheme: () => void;
}

export const LoginPage: React.FC<LoginPageProps> = ({
  onLoginSuccess,
  isDarkMode,
  onToggleTheme,
}) => {
  const [username, setUsername] = useState('admin');
  const [password, setPassword] = useState('admin_polyquant');
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username.trim() || !password) {
      setErrorMsg('请输入用户名和密码');
      return;
    }

    setLoading(true);
    setErrorMsg(null);

    try {
      const res = await api.login({ username: username.trim(), password });
      if (res.success && res.token) {
        setStoredToken(res.token);
        onLoginSuccess(res.username || username);
      } else {
        setErrorMsg(res.message || '登录失败，请检查账号密码');
      }
    } catch (err: any) {
      setErrorMsg(err.message || '网络或鉴权错误，无法连接后端');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-[#f6f6f9] dark:bg-[#181a1e] text-[#363949] dark:text-[#edeffd] flex items-center justify-center p-4 transition-colors duration-300">
      {/* Top Floating Theme Switcher */}
      <div className="absolute top-6 right-6">
        <button
          onClick={onToggleTheme}
          className="flex items-center gap-1.5 px-3.5 py-2 rounded-2xl bg-white dark:bg-[#202528] shadow-[0_0.5rem_1rem_rgba(132,139,200,0.1)] dark:shadow-none text-xs font-bold text-[#7d8da1] hover:text-[#363949] dark:hover:text-white transition-all cursor-pointer"
        >
          {isDarkMode ? <Sun className="w-4 h-4 text-[#ffbb55]" /> : <Moon className="w-4 h-4 text-[#6c9bcf]" />}
          <span>{isDarkMode ? '日间模式' : '深色模式'}</span>
        </button>
      </div>

      <div className="w-full max-w-md asmr-card p-8 sm:p-10 space-y-6">
        {/* Logo and Header */}
        <div className="flex flex-col items-center text-center space-y-3">
          <div className="relative flex items-center justify-center w-20 h-20 rounded-full border-4 border-[#ff0060] p-1 bg-white dark:bg-[#202528] shadow-lg shadow-[#ff0060]/20 animate-pulse-ring">
            <div className="w-full h-full rounded-full border-2 border-[#ff0060] flex items-center justify-center">
              <span className="font-black text-[#ff0060] text-2xl tracking-tighter">
                AP
              </span>
            </div>
          </div>

          <div>
            <h1 className="text-xl sm:text-2xl font-extrabold text-[#363949] dark:text-white tracking-tight">
              EventAlpha <span className="text-[#ff0060]">· 量化控制台</span>
            </h1>
            <p className="text-xs text-[#7d8da1] dark:text-slate-400 font-medium mt-1">
              Polymarket 5M 智能预测与自学习实盘系统
            </p>
          </div>
        </div>

        {/* Security Warning Badge */}
        <div className="p-3.5 rounded-2xl bg-emerald-500/10 border border-emerald-500/20 text-emerald-600 dark:text-emerald-400 flex items-start gap-2.5 text-xs">
          <ShieldCheck className="w-4 h-4 shrink-0 mt-0.5" />
          <div className="space-y-0.5">
            <p className="font-bold">公网安全访问控制已生效</p>
            <p className="text-[11px] opacity-90">
              为防止未授权访问与资金操作，控制台与全部敏感量化 API 已受密码防护。
            </p>
          </div>
        </div>

        {/* Error Alert */}
        {errorMsg && (
          <div className="p-3.5 rounded-2xl bg-rose-500/10 border border-rose-500/20 text-rose-500 flex items-center gap-2 text-xs font-semibold animate-shake">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span>{errorMsg}</span>
          </div>
        )}

        {/* Login Form */}
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-xs font-bold text-[#7d8da1] dark:text-slate-400 mb-1.5">
              管理员账号
            </label>
            <div className="relative flex items-center">
              <User className="absolute left-3.5 w-4 h-4 text-[#7d8da1]" />
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="请输入用户名 (默认: admin)"
                className="w-full pl-10 pr-4 py-3 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200 dark:border-slate-800 text-xs font-semibold text-[#363949] dark:text-white placeholder-[#7d8da1]/60 focus:outline-none focus:ring-2 focus:ring-[#6c9bcf] transition-all"
                disabled={loading}
              />
            </div>
          </div>

          <div>
            <label className="block text-xs font-bold text-[#7d8da1] dark:text-slate-400 mb-1.5">
              安全密码
            </label>
            <div className="relative flex items-center">
              <Lock className="absolute left-3.5 w-4 h-4 text-[#7d8da1]" />
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="请输入密码 (默认: admin_polyquant)"
                className="w-full pl-10 pr-4 py-3 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200 dark:border-slate-800 text-xs font-semibold text-[#363949] dark:text-white placeholder-[#7d8da1]/60 focus:outline-none focus:ring-2 focus:ring-[#6c9bcf] transition-all"
                disabled={loading}
              />
            </div>
          </div>

          <div className="pt-2">
            <button
              type="submit"
              disabled={loading}
              className="w-full py-3.5 px-4 rounded-2xl bg-[#6c9bcf] hover:bg-[#5b8ac0] active:scale-[0.99] text-white font-bold text-xs shadow-lg shadow-[#6c9bcf]/30 flex items-center justify-center gap-2 transition-all cursor-pointer disabled:opacity-50"
            >
              <span>{loading ? '正在验证凭证...' : '安全登录量化控制台'}</span>
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </form>

        {/* Footer Note */}
        <div className="pt-2 text-center text-[11px] text-[#7d8da1] dark:text-slate-500 font-mono">
          默认账号: <span className="font-bold text-[#363949] dark:text-slate-300">admin</span> | 默认密码: <span className="font-bold text-[#363949] dark:text-slate-300">admin_polyquant</span>
        </div>
      </div>
    </div>
  );
};
