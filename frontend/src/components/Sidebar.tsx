import React from 'react';
import {
  LayoutDashboard,
  Brain,
  ShieldAlert,
  Wallet,
  Layers,
  Cpu,
  Sliders,
  FlaskConical,
  PlayCircle,
  FileText,
  LogOut,
  TrendingUp,
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
  tradingMode = 'paper',
}) => {
  const isPaper = tradingMode === 'paper';

  const navItems = [
    {
      id: 'dashboard',
      label: 'Dashboard',
      icon: LayoutDashboard,
      badge: activePositionsCount > 0 ? `${activePositionsCount}` : undefined,
    },
    {
      id: 'self-learning',
      label: 'Analytics & AI',
      icon: Brain,
      badge: 'Auto',
    },
    {
      id: 'live-orders',
      label: 'Live Orders',
      icon: ShieldAlert,
      badge: !isPaper ? 'LIVE' : undefined,
    },
    {
      id: 'accounts',
      label: 'Wallets & Keys',
      icon: Wallet,
    },
    {
      id: 'microstructure',
      label: 'OrderBook L2',
      icon: Layers,
    },
    {
      id: 'features',
      label: 'Feature Matrix',
      icon: Cpu,
    },
    {
      id: 'tuning',
      label: 'Strategy Tuner',
      icon: Sliders,
    },
    {
      id: 'backtest',
      label: 'Backtest Lab',
      icon: FlaskConical,
    },
    {
      id: 'replay',
      label: 'Replay Console',
      icon: PlayCircle,
    },
    {
      id: 'events',
      label: 'Risk & Audit',
      icon: FileText,
    },
  ];

  return (
    <aside className="w-full lg:w-60 shrink-0 flex flex-col justify-between bg-white dark:bg-[#202528] rounded-3xl lg:m-4 p-5 shadow-[0_1.5rem_2rem_rgba(132,139,200,0.18)] dark:shadow-none transition-all duration-300">
      {/* Top Logo (AsmrProg Iconic Circular Double Ring Badge) */}
      <div>
        <div className="flex items-center gap-3 px-3 py-2 mb-6">
          <div className="relative flex items-center justify-center w-11 h-11 rounded-full border-2 border-[#ff0060] p-1 bg-white dark:bg-[#202528] shadow-sm">
            <div className="w-full h-full rounded-full border-2 border-[#ff0060] flex items-center justify-center">
              <span className="font-extrabold text-[#ff0060] text-sm tracking-tighter">
                AP
              </span>
            </div>
          </div>
          <div>
            <h2 className="font-extrabold text-base tracking-tight text-[#363949] dark:text-white flex items-center">
              AsmrProg<span className="text-[#ff0060] text-xs ml-1 font-mono">.Quant</span>
            </h2>
            <p className="text-[10px] text-[#7d8da1] dark:text-slate-400 font-medium">
              Polymarket 5M Engine
            </p>
          </div>
        </div>

        {/* Navigation Items (Exact AsmrProg Left Blue Pill Accent) */}
        <nav className="space-y-1">
          {navItems.map((item) => {
            const Icon = item.icon;
            const isActive = activeTab === item.id;

            return (
              <button
                key={item.id}
                onClick={() => onSelectTab(item.id)}
                className={`relative w-full flex items-center justify-between px-4 py-3 rounded-2xl text-xs font-semibold transition-all duration-200 cursor-pointer ${
                  isActive
                    ? 'text-[#6c9bcf] bg-[#f6f6f9] dark:bg-[#181a1e] font-bold shadow-sm'
                    : 'text-[#7d8da1] hover:text-[#363949] dark:hover:text-white hover:bg-slate-50 dark:hover:bg-slate-800/40'
                }`}
              >
                {/* Left Active Accent Pill Bar */}
                {isActive && (
                  <span className="absolute left-0 top-2 bottom-2 w-1.5 bg-[#6c9bcf] rounded-r-full shadow-md shadow-[#6c9bcf]/50" />
                )}

                <div className="flex items-center gap-3.5 min-w-0 ml-1">
                  <Icon
                    className={`w-4 h-4 shrink-0 transition-transform ${
                      isActive ? 'text-[#6c9bcf] scale-110' : 'text-[#7d8da1]'
                    }`}
                  />
                  <span className="truncate">{item.label}</span>
                </div>

                {item.badge && (
                  <span
                    className={`shrink-0 px-2 py-0.5 text-[10px] font-extrabold rounded-full font-mono ${
                      item.badge === 'LIVE'
                        ? 'bg-[#ff0060] text-white animate-pulse'
                        : item.badge === 'Auto'
                        ? 'bg-[#1b9c85] text-white'
                        : 'bg-[#ff0060] text-white'
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

      {/* Bottom Mode Status & Logout */}
      <div className="pt-4 border-t border-slate-100 dark:border-slate-800 space-y-3">
        {/* Status Indicator Pill */}
        <div
          className={`p-3 rounded-2xl text-xs font-medium border flex items-center justify-between transition-all ${
            isPaper
              ? 'bg-[#1b9c85]/10 border-[#1b9c85]/20 text-[#1b9c85]'
              : 'bg-[#ff0060]/10 border-[#ff0060]/20 text-[#ff0060]'
          }`}
        >
          <div className="flex items-center gap-2">
            <span
              className={`w-2 h-2 rounded-full ${
                isPaper ? 'bg-[#1b9c85]' : 'bg-[#ff0060] animate-ping'
              }`}
            />
            <span className="font-bold uppercase tracking-wider text-[11px]">
              {isPaper ? 'Simulation' : 'CLOB Live'}
            </span>
          </div>
          <span className="text-[10px] font-mono opacity-80">v0.2.1</span>
        </div>

        {/* Logout / Reset Button */}
        <button
          onClick={() => onSelectTab('dashboard')}
          className="w-full flex items-center gap-3 px-4 py-2.5 rounded-2xl text-xs font-semibold text-[#7d8da1] hover:text-[#ff0060] hover:bg-rose-50 dark:hover:bg-rose-950/20 transition-all cursor-pointer"
        >
          <LogOut className="w-4 h-4" />
          <span>Exit Session</span>
        </button>
      </div>
    </aside>
  );
};
