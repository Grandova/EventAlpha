import React from 'react';
import {
  LayoutDashboard,
  Layers,
  Cpu,
  Sliders,
  FlaskConical,
  PlayCircle,
  FileText,
  ShieldCheck,
  ShieldAlert,
  TrendingUp,
  Activity,
  Zap,
  Brain,
  Wallet,
} from 'lucide-react';
import { TradingMode } from '../types';

interface SidebarProps {
  activeTab: string;
  onSelectTab: (tab: string) => void;
  activePositionsCount?: number;
  isLive?: boolean;
  tradingMode?: TradingMode;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeTab,
  onSelectTab,
  activePositionsCount = 0,
  isLive = true,
  tradingMode = 'paper',
}) => {
  const isPaper = tradingMode === 'paper';

  const navItems = [
    {
      id: 'dashboard',
      label: 'Dashboard',
      sublabel: '实时盘面与信号',
      icon: LayoutDashboard,
      badge: activePositionsCount > 0 ? `${activePositionsCount} 活跃仓位` : undefined,
    },
    {
      id: 'self-learning',
      label: 'Auto-Learning',
      sublabel: '在线单轮增量自进化',
      icon: Brain,
      badge: '自主学习',
    },
    {
      id: 'live-orders',
      label: 'Live Orders',
      sublabel: 'Polymarket 实盘监控',
      icon: ShieldAlert,
      badge: !isPaper ? 'LIVE' : undefined,
    },
    {
      id: 'accounts',
      label: 'Accounts',
      sublabel: '钱包与 API 凭据',
      icon: Wallet,
    },
    {
      id: 'microstructure',
      label: 'OrderBook L2',
      sublabel: '深度与微观失衡',
      icon: Layers,
    },
    {
      id: 'features',
      label: 'Feature Engine',
      sublabel: '37维多尺度特征',
      icon: Cpu,
    },
    {
      id: 'tuning',
      label: 'Strategy & Train',
      sublabel: '动态调参与模型训练',
      icon: Sliders,
    },
    {
      id: 'backtest',
      label: 'Backtest Engine',
      sublabel: '资金隔离回测检验',
      icon: FlaskConical,
    },
    {
      id: 'replay',
      label: 'Replay Console',
      sublabel: '历史逐帧全息复盘',
      icon: PlayCircle,
    },
    {
      id: 'events',
      label: 'Audit & Risk Logs',
      sublabel: '熔断事件与风控审计',
      icon: FileText,
    },
  ];

  return (
    <aside className="w-full lg:w-64 shrink-0 flex flex-col justify-between bg-slate-950/70 border-r border-slate-800/80 p-4 lg:p-6 backdrop-blur-2xl">
      {/* Top Logo & App Brand */}
      <div>
        <div className="flex items-center gap-3 px-2 mb-8">
          <div className="relative flex items-center justify-center w-11 h-11 rounded-2xl bg-gradient-to-tr from-cyan-600 via-indigo-600 to-emerald-400 p-0.5 shadow-lg shadow-cyan-950/50">
            <div className="w-full h-full bg-slate-950 rounded-[14px] flex items-center justify-center">
              <TrendingUp className="w-6 h-6 text-cyan-400" />
            </div>
            <span className="absolute -top-1 -right-1 flex h-3 w-3">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
              <span className="relative inline-flex rounded-full h-3 w-3 bg-emerald-500"></span>
            </span>
          </div>

          <div>
            <div className="flex items-center gap-1.5">
              <h1 className="font-extrabold text-base tracking-wide text-white font-sans">
                POLYQUANT<span className="text-cyan-400">.5M</span>
              </h1>
            </div>
            <p className="text-[11px] font-medium text-slate-400">Crypto Quant & Self-Learning</p>
          </div>
        </div>

        {/* Navigation Section */}
        <div className="space-y-1">
          <p className="px-3 mb-2 text-[10px] font-bold uppercase tracking-wider text-slate-500 font-mono">
            Navigation Menu
          </p>

          <nav className="space-y-1.5">
            {navItems.map((item) => {
              const Icon = item.icon;
              const isActive = activeTab === item.id;

              return (
                <button
                  key={item.id}
                  onClick={() => onSelectTab(item.id)}
                  className={`w-full group relative flex items-center justify-between px-3.5 py-2.5 rounded-2xl text-left transition-all duration-200 cursor-pointer ${
                    isActive
                      ? 'bg-gradient-to-r from-cyan-500/15 via-indigo-500/10 to-transparent text-cyan-400 shadow-md shadow-cyan-950/20 font-semibold'
                      : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60 font-medium'
                  }`}
                >
                  {/* Left Active Glow Indicator */}
                  {isActive && (
                    <div className="absolute left-0 top-2 bottom-2 w-1 bg-gradient-to-b from-cyan-400 to-indigo-500 rounded-r-full shadow-[0_0_8px_#38bdf8]" />
                  )}

                  <div className="flex items-center gap-3 min-w-0">
                    <div
                      className={`p-2 rounded-xl transition-all ${
                        isActive
                          ? 'bg-cyan-500/20 text-cyan-400 shadow-inner'
                          : 'bg-slate-900/80 text-slate-400 group-hover:text-slate-200 group-hover:bg-slate-800'
                      }`}
                    >
                      <Icon className="w-4 h-4" />
                    </div>
                    <div className="truncate">
                      <div className="text-xs tracking-tight">{item.label}</div>
                      <div className="text-[10px] text-slate-500 truncate group-hover:text-slate-400">
                        {item.sublabel}
                      </div>
                    </div>
                  </div>

                  {item.badge && (
                    <span
                      className={`shrink-0 px-2 py-0.5 text-[10px] font-bold rounded-full border font-mono ${
                        item.badge === 'LIVE'
                          ? 'bg-rose-500/20 text-rose-400 border-rose-500/30 animate-pulse'
                          : item.badge === '自主学习'
                          ? 'bg-indigo-500/20 text-indigo-400 border-indigo-500/30'
                          : 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30'
                      }`}
                    >
                      {item.badge}
                    </span>
                  )}
                </button>
              );
            })}
          </nav>
        </div>
      </div>

      {/* Bottom Safety & System Status Card */}
      <div className="mt-8 pt-4 border-t border-slate-800/80 space-y-3">
        {/* Safety Lock Card */}
        <div
          className={`rounded-2xl p-3.5 border shadow-lg transition-all ${
            isPaper
              ? 'bg-gradient-to-b from-slate-900/90 to-slate-950 border-emerald-500/30'
              : 'bg-gradient-to-b from-rose-950/40 to-slate-950 border-rose-500/40 shadow-rose-950/20'
          }`}
        >
          <div
            className={`flex items-center gap-2 text-xs font-bold mb-1 ${
              isPaper ? 'text-emerald-400' : 'text-rose-400'
            }`}
          >
            {isPaper ? <ShieldCheck className="w-4 h-4" /> : <ShieldAlert className="w-4 h-4 animate-pulse" />}
            <span>{isPaper ? 'PAPER TRADING ONLY' : 'LIVE TRADING ACTIVE'}</span>
          </div>
          <p className="text-[10px] text-slate-400 leading-relaxed">
            {isPaper
              ? '模拟盘防线生效中：严禁向 Polymarket 提交真实订单，资金隔离 Mode B 已激活。'
              : '实盘撮合模式中：真实订单直连 Polymarket CLOB，单笔受 $10 资金硬顶保护。'}
          </p>

          <div className="mt-2.5 pt-2 border-t border-slate-800/60 flex items-center justify-between text-[10px] font-mono">
            <span className="text-slate-500">ENGINE FEED</span>
            <span className="inline-flex items-center gap-1 text-emerald-400">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
              {isLive ? '100ms LIVE WS' : 'POLLING'}
            </span>
          </div>
        </div>

        {/* System Version Footnote */}
        <div className="flex items-center justify-between px-2 text-[10px] text-slate-500 font-mono">
          <span>EVENT ALPHA</span>
          <span>v0.2.0-LANDING</span>
        </div>
      </div>
    </aside>
  );
};
