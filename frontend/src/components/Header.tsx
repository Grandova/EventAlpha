import React from 'react';
import { ShieldCheck, ShieldAlert, Activity, Wifi, Clock, Database } from 'lucide-react';
import { HealthResponse } from '../types';

interface HeaderProps {
  health: HealthResponse | null;
  activeAsset: string;
  onSelectAsset: (asset: any) => void;
  activeTab: string;
  onSelectTab: (tab: string) => void;
}

export const Header: React.FC<HeaderProps> = ({
  health,
  activeAsset,
  onSelectAsset,
  activeTab,
  onSelectTab,
}) => {
  const isPaper = health?.safety_status?.paper_trading_only ?? true;
  const isSafetyLocked = health?.safety_status?.safety_lock_engaged ?? true;
  const freshness = health?.exchange_freshness;

  const uptimeStr = React.useMemo(() => {
    if (!health?.uptime_secs) return '0s';
    const hrs = Math.floor(health.uptime_secs / 3600);
    const mins = Math.floor((health.uptime_secs % 3600) / 60);
    const secs = health.uptime_secs % 60;
    return `${hrs}h ${mins}m ${secs}s`;
  }, [health?.uptime_secs]);

  return (
    <header className="quant-card mb-4 px-4 py-3 border-b border-slate-800">
      <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
        {/* Brand & Safety Status */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <div className="h-9 w-9 rounded-lg bg-gradient-to-tr from-cyan-600 to-emerald-400 flex items-center justify-center font-bold text-white shadow-lg shadow-cyan-900/40">
              PQ
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-lg font-black tracking-wider text-white">POLYQUANT 5M</h1>
                <span className="text-[10px] bg-cyan-950/80 text-cyan-400 border border-cyan-800/60 px-1.5 py-0.5 rounded font-mono font-bold">
                  v0.1.0
                </span>
              </div>
              <p className="text-xs text-slate-400 font-medium">Polymarket 5-Minute Crypto Up/Down Quant System</p>
            </div>
          </div>

          {/* Critical Safety Lock Banner */}
          <div className="hidden sm:flex items-center gap-2 px-3 py-1.5 rounded-lg border border-emerald-500/30 bg-emerald-950/30 text-emerald-400 text-xs font-semibold">
            {isPaper && isSafetyLocked ? (
              <>
                <ShieldCheck className="h-4 w-4 text-emerald-400 shrink-0" />
                <span className="tracking-wide">PAPER TRADING ONLY &bull; ZERO LIVE ORDERS PERMITTED</span>
              </>
            ) : (
              <>
                <ShieldAlert className="h-4 w-4 text-rose-500 shrink-0" />
                <span className="text-rose-400">SAFETY LOCK WARNING</span>
              </>
            )}
          </div>
        </div>

        {/* Exchange Freshness & Health Badges */}
        <div className="flex flex-wrap items-center gap-2 sm:gap-3 text-xs">
          {freshness?.exchanges &&
            Object.entries(freshness.exchanges).map(([exch, data]) => {
              const isOk = data.is_fresh;
              return (
                <div
                  key={exch}
                  className={`flex items-center gap-1.5 px-2.5 py-1 rounded border font-mono ${
                    isOk
                      ? 'bg-slate-900/80 border-slate-700/60 text-slate-300'
                      : 'bg-rose-950/40 border-rose-800/60 text-rose-400'
                  }`}
                  title={`Status: ${data.status}, Age: ${data.age_ms}ms`}
                >
                  <span
                    className={`h-1.5 w-1.5 rounded-full ${
                      isOk ? 'bg-emerald-400 animate-pulse' : 'bg-rose-500'
                    }`}
                  />
                  <span className="uppercase font-semibold">{exch}</span>
                  <span className="text-[10px] text-slate-400">{data.age_ms}ms</span>
                </div>
              );
            })}

          {/* System Uptime */}
          <div className="hidden md:flex items-center gap-1.5 px-2.5 py-1 rounded border border-slate-800 bg-slate-900/50 text-slate-400 font-mono text-[11px]">
            <Clock className="h-3 w-3 text-slate-500" />
            <span>Up: {uptimeStr}</span>
          </div>
        </div>
      </div>

      {/* Navigation Sub-Bar & Asset Switcher */}
      <div className="mt-3 pt-3 border-t border-slate-800/80 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        {/* Navigation Tabs */}
        <div className="flex items-center gap-1">
          {[
            { id: 'dashboard', label: 'Dashboard 概览' },
            { id: 'microstructure', label: 'OrderBook 盘口深度' },
            { id: 'features', label: '37-Dim 特征监控' },
            { id: 'tuning', label: 'Strategy 调优与训练' },
            { id: 'backtest', label: 'Backtest 回测引擎' },
            { id: 'replay', label: 'Replay 逐帧回放' },
            { id: 'events', label: 'Audit Logs 审计日志' },
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => onSelectTab(tab.id)}
              className={`px-3 py-1.5 rounded-md text-xs font-semibold transition-all ${
                activeTab === tab.id
                  ? 'bg-cyan-500/15 text-cyan-400 border border-cyan-500/30'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        {/* Asset Selector */}
        <div className="flex items-center gap-1.5 bg-slate-900/80 p-1 rounded-lg border border-slate-800">
          <span className="text-[11px] font-semibold text-slate-500 px-2 uppercase tracking-wider">Asset:</span>
          {['BTC', 'ETH', 'SOL'].map((asset) => (
            <button
              key={asset}
              onClick={() => onSelectAsset(asset)}
              className={`px-3 py-1 rounded text-xs font-bold transition-all ${
                activeAsset === asset
                  ? 'bg-cyan-500 text-slate-950 shadow-md shadow-cyan-500/30'
                  : 'text-slate-400 hover:text-white hover:bg-slate-800'
              }`}
            >
              {asset}
            </button>
          ))}
        </div>
      </div>
    </header>
  );
};
