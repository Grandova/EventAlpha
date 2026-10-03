import React from 'react';
import {
  ShieldCheck,
  ShieldAlert,
  Clock,
  Activity,
  Radio,
  Search,
  CheckCircle2,
  AlertTriangle,
} from 'lucide-react';
import { HealthResponse, PriceSummary, Asset } from '../types';

interface HeaderProps {
  health: HealthResponse | null;
  activeAsset: Asset;
  onSelectAsset: (asset: Asset) => void;
  compositePrice?: number;
  spotPrices?: PriceSummary[];
  isWsConnected?: boolean;
}

export const Header: React.FC<HeaderProps> = ({
  health,
  activeAsset,
  onSelectAsset,
  compositePrice,
  spotPrices,
  isWsConnected = true,
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
    <header className="mb-6 flex flex-col xl:flex-row xl:items-center xl:justify-between gap-4 bg-slate-900/60 p-4 lg:px-6 lg:py-3.5 rounded-2xl border border-slate-800/80 backdrop-blur-xl shadow-lg">
      {/* Left: View Breadcrumbs & Title */}
      <div className="flex items-center gap-4">
        <div>
          <div className="flex items-center gap-2">
            <span className="text-xs font-semibold text-slate-500 uppercase tracking-wider font-mono">
              POLYMARKET 5M QUANT
            </span>
            <span className="text-slate-600">/</span>
            <span className="text-xs font-bold text-cyan-400 font-mono tracking-wide">
              {activeAsset} PERPETUAL
            </span>
          </div>
          <div className="flex items-center gap-3 mt-1">
            <h2 className="text-xl font-extrabold text-white tracking-tight">
              {activeAsset} Up/Down 5-Minute Simulator
            </h2>
            {compositePrice && compositePrice > 0 && (
              <span className="px-2.5 py-0.5 rounded-lg bg-slate-800/80 border border-slate-700/60 font-mono-num font-bold text-cyan-400 text-sm shadow-inner">
                ${compositePrice.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
              </span>
            )}
          </div>
        </div>
      </div>

      {/* Center/Right: Asset Selector & Status Pills */}
      <div className="flex flex-wrap items-center gap-3">
        {/* Asset Selector Pills */}
        <div className="flex items-center p-1 rounded-xl bg-slate-950/80 border border-slate-800/90 shadow-inner">
          {(['BTC', 'ETH', 'SOL'] as Asset[]).map((asset) => (
            <button
              key={asset}
              onClick={() => onSelectAsset(asset)}
              className={`px-3.5 py-1.5 rounded-lg text-xs font-bold transition-all duration-200 cursor-pointer ${
                activeAsset === asset
                  ? 'bg-gradient-to-r from-cyan-500 to-indigo-500 text-white shadow-md shadow-cyan-500/25 scale-[1.02]'
                  : 'text-slate-400 hover:text-white hover:bg-slate-900/80'
              }`}
            >
              {asset}
            </button>
          ))}
        </div>

        {/* Exchange Freshness Indicators */}
        <div className="hidden lg:flex items-center gap-2 text-[11px] font-mono">
          {freshness?.exchanges &&
            Object.entries(freshness.exchanges).map(([exch, data]) => {
              const isOk = data.is_fresh;
              return (
                <div
                  key={exch}
                  className={`flex items-center gap-1.5 px-2.5 py-1 rounded-xl border transition-all ${
                    isOk
                      ? 'bg-slate-950/70 border-slate-800/80 text-slate-300'
                      : 'bg-rose-950/30 border-rose-900/50 text-rose-400'
                  }`}
                  title={`${exch.toUpperCase()}: ${data.age_ms}ms latency`}
                >
                  <span
                    className={`h-2 w-2 rounded-full ${
                      isOk ? 'bg-emerald-400 animate-pulse' : 'bg-rose-500'
                    }`}
                  />
                  <span className="font-semibold uppercase">{exch}</span>
                  <span className="text-[10px] text-slate-400">{data.age_ms}ms</span>
                </div>
              );
            })}
        </div>

        {/* WebSocket Live Telemetry Pill */}
        <div className="flex items-center gap-2 px-3 py-1.5 rounded-xl border border-slate-800/80 bg-slate-950/80 text-xs font-mono">
          <span className="relative flex h-2 w-2">
            <span className={`animate-ping absolute inline-flex h-full w-full rounded-full ${isWsConnected ? 'bg-emerald-400' : 'bg-amber-400'} opacity-75`}></span>
            <span className={`relative inline-flex rounded-full h-2 w-2 ${isWsConnected ? 'bg-emerald-500' : 'bg-amber-500'}`}></span>
          </span>
          <span className="text-slate-300 font-semibold">{isWsConnected ? 'WS 100ms' : 'Polling'}</span>
          <span className="text-slate-500 text-[10px]">({uptimeStr})</span>
        </div>
      </div>
    </header>
  );
};
