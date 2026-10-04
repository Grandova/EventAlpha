import React from 'react';
import {
  Sun,
  Moon,
  ShieldCheck,
  ShieldAlert,
  Wallet,
  OctagonAlert,
  Activity,
  Zap,
  Play,
  Pause,
} from 'lucide-react';
import { HealthResponse, PriceSummary, Asset, TradingMode, PolymarketAccountPublic } from '../types';

interface HeaderProps {
  health: HealthResponse | null;
  activeAsset: Asset;
  onSelectAsset: (asset: Asset) => void;
  compositePrice?: number;
  spotPrices?: PriceSummary[];
  isWsConnected?: boolean;
  tradingMode?: TradingMode;
  onToggleTradingMode?: (mode: TradingMode) => void;
  isAutoTradingEnabled?: boolean;
  onToggleAutoTrading?: (enabled: boolean) => void;
  activeAccount?: PolymarketAccountPublic | null;
  onOpenAccountManager?: () => void;
  onEmergencyHalt?: () => void;
  isDarkMode?: boolean;
  onToggleTheme?: () => void;
  currentUser?: string;
  onLogout?: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  health,
  activeAsset,
  onSelectAsset,
  compositePrice,
  spotPrices,
  isWsConnected = true,
  tradingMode = 'paper',
  onToggleTradingMode,
  isAutoTradingEnabled = true,
  onToggleAutoTrading,
  activeAccount,
  onOpenAccountManager,
  onEmergencyHalt,
  isDarkMode = false,
  onToggleTheme,
  currentUser = 'admin',
  onLogout,
}) => {
  const isPaper = tradingMode === 'paper';

  return (
    <header className="mb-6 flex flex-col md:flex-row md:items-center md:justify-between gap-4">
      {/* Left: Big Bold Title (Matches screenshot "Analytics") */}
      <div className="flex items-center gap-4">
        <div>
          <h1 className="text-2xl lg:text-3xl font-extrabold text-[#363949] dark:text-white tracking-tight">
            量化监控总览
          </h1>
          <div className="flex items-center gap-2 mt-1">
            <span className="text-xs font-bold text-[#7d8da1] dark:text-slate-400 uppercase tracking-wider font-mono">
              POLYMARKET 5M QUANT
            </span>
            <span className="text-slate-300 dark:text-slate-700">&bull;</span>
            <span className="text-xs font-extrabold text-[#6c9bcf] font-mono tracking-wide">
              {activeAsset} 现货加权基准
            </span>
            {typeof compositePrice === 'number' && !isNaN(compositePrice) && compositePrice > 0 && (
              <span className="ml-1 px-2.5 py-0.5 rounded-xl bg-white dark:bg-[#202528] shadow-sm font-mono-num font-bold text-[#1b9c85] text-xs">
                ${compositePrice.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
              </span>
            )}
          </div>
        </div>
      </div>

      {/* Right: Controls, Theme Switcher & User Profile */}
      <div className="flex flex-wrap items-center gap-3">
        {/* Global Auto-Trading Master Switch */}
        {onToggleAutoTrading && (
          <button
            onClick={() => onToggleAutoTrading(!isAutoTradingEnabled)}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-2xl text-xs font-extrabold shadow-sm transition-all cursor-pointer ${
              isAutoTradingEnabled
                ? 'bg-emerald-50 dark:bg-emerald-950/60 border border-emerald-300 dark:border-emerald-800 text-[#1b9c85] hover:bg-emerald-100'
                : 'bg-amber-50 dark:bg-amber-950/60 border border-amber-300 dark:border-amber-800 text-amber-600 dark:text-amber-400 hover:bg-amber-100 animate-pulse'
            }`}
            title={isAutoTradingEnabled ? '点击暂停策略自动交易（保留行情监控与风控）' : '点击恢复策略自动交易'}
          >
            {isAutoTradingEnabled ? (
              <>
                <Play className="w-3.5 h-3.5 fill-[#1b9c85] text-[#1b9c85]" />
                <span>自动交易: 运行中</span>
              </>
            ) : (
              <>
                <Pause className="w-3.5 h-3.5 fill-amber-500 text-amber-500" />
                <span>自动交易: 已暂停</span>
              </>
            )}
          </button>
        )}

        {/* Trading Mode Switcher */}
        <div className="flex items-center p-1 rounded-2xl bg-white dark:bg-[#202528] shadow-[0_0.5rem_1rem_rgba(132,139,200,0.1)] dark:shadow-none">
          <button
            onClick={() => onToggleTradingMode && onToggleTradingMode('paper')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-bold transition-all cursor-pointer ${
              isPaper
                ? 'bg-[#1b9c85] text-white shadow-md shadow-[#1b9c85]/20'
                : 'text-[#7d8da1] hover:text-[#363949]'
            }`}
          >
            <ShieldCheck className="w-3.5 h-3.5" />
            <span>模拟盘</span>
          </button>
          <button
            onClick={() => onToggleTradingMode && onToggleTradingMode('live')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-bold transition-all cursor-pointer ${
              !isPaper
                ? 'bg-[#ff0060] text-white shadow-md shadow-[#ff0060]/20 animate-pulse'
                : 'text-[#7d8da1] hover:text-[#ff0060]'
            }`}
          >
            <ShieldAlert className="w-3.5 h-3.5" />
            <span>实盘</span>
          </button>
        </div>

        {/* Asset Selector */}
        <div className="flex items-center p-1 rounded-2xl bg-white dark:bg-[#202528] shadow-[0_0.5rem_1rem_rgba(132,139,200,0.1)] dark:shadow-none">
          {(['BTC', 'ETH', 'SOL'] as Asset[]).map((asset) => (
            <button
              key={asset}
              onClick={() => onSelectAsset(asset)}
              className={`px-3 py-1.5 rounded-xl text-xs font-bold transition-all cursor-pointer ${
                activeAsset === asset
                  ? 'bg-[#6c9bcf] text-white shadow-md shadow-[#6c9bcf]/20'
                  : 'text-[#7d8da1] hover:text-[#363949]'
              }`}
            >
              {asset}
            </button>
          ))}
        </div>

        {/* AsmrProg Iconic Dark/Light Mode Toggle Switch */}
        {onToggleTheme && (
          <div
            onClick={onToggleTheme}
            className="flex items-center justify-between w-14 h-8 p-1 rounded-full bg-slate-200 dark:bg-slate-700 cursor-pointer transition-colors shadow-inner"
            title="切换深色/浅色模式"
          >
            <div
              className={`flex items-center justify-center w-6 h-6 rounded-full bg-white dark:bg-[#202528] text-amber-500 shadow-sm transition-transform duration-300 ${
                isDarkMode ? 'translate-x-6 text-[#6c9bcf]' : 'translate-x-0'
              }`}
            >
              {isDarkMode ? <Moon className="w-3.5 h-3.5" /> : <Sun className="w-3.5 h-3.5" />}
            </div>
          </div>
        )}

        {/* User / Profile Avatar Badge */}
        <div
          onClick={onOpenAccountManager}
          className="flex items-center gap-3 p-1.5 pr-4 rounded-full bg-white dark:bg-[#202528] shadow-[0_0.5rem_1rem_rgba(132,139,200,0.1)] dark:shadow-none cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/60 transition-all"
        >
          <div className="w-8 h-8 rounded-full bg-gradient-to-tr from-[#6c9bcf] to-[#1b9c85] p-0.5 flex items-center justify-center text-white font-bold text-xs shadow-sm">
            {currentUser ? currentUser.slice(0, 2).toUpperCase() : (activeAccount ? activeAccount.label.slice(0, 2).toUpperCase() : 'AP')}
          </div>
          <div className="text-left">
            <p className="text-xs font-bold text-[#363949] dark:text-white leading-tight">
              你好，{currentUser || (activeAccount ? activeAccount.label : '交易员')}
            </p>
            <p className="text-[10px] text-[#7d8da1] dark:text-slate-400 font-medium">
              {isPaper ? '模拟盘安全运行' : 'Mode B 实盘防护'}
            </p>
          </div>
        </div>
      </div>
    </header>
  );
};
