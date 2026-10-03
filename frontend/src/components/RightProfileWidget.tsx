import React from 'react';
import {
  Bell,
  MoreVertical,
  Plus,
  ShieldCheck,
  ShieldAlert,
  Brain,
  Clock,
  Wallet,
  Zap,
} from 'lucide-react';
import { PolymarketAccountPublic, TradingMode } from '../types';

interface RightProfileWidgetProps {
  activeAccount: PolymarketAccountPublic | null;
  tradingMode: TradingMode;
  onOpenAccountManager: () => void;
  onEmergencyHalt?: () => void;
}

export const RightProfileWidget: React.FC<RightProfileWidgetProps> = ({
  activeAccount,
  tradingMode,
  onOpenAccountManager,
  onEmergencyHalt,
}) => {
  const isPaper = tradingMode === 'paper';

  return (
    <div className="w-full xl:w-72 shrink-0 space-y-6">
      {/* Top Profile Card (Matches exact screenshot AP card!) */}
      <div className="asmr-card p-6 flex flex-col items-center text-center">
        {/* Large Iconic Double-Ring AP Badge */}
        <div className="relative flex items-center justify-center w-24 h-24 rounded-full border-4 border-[#ff0060] p-1.5 bg-white dark:bg-[#202528] shadow-lg shadow-[#ff0060]/10 mb-4 animate-pulse-ring">
          <div className="w-full h-full rounded-full border-4 border-[#ff0060] flex items-center justify-center">
            <span className="font-black text-[#ff0060] text-3xl tracking-tighter">
              AP
            </span>
          </div>
        </div>

        <h3 className="text-lg font-extrabold text-[#363949] dark:text-white tracking-tight">
          AsmrProg<span className="text-[#ff0060]">.5M</span>
        </h3>
        <p className="text-xs text-[#7d8da1] dark:text-slate-400 font-medium mt-0.5">
          Autonomous Quant Engine
        </p>

        {/* Current Account Status Pill */}
        <div className="mt-4 w-full p-3 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800 text-left">
          <div className="flex items-center justify-between text-[11px] font-mono text-[#7d8da1]">
            <span>ACTIVE TRADER</span>
            <span
              className={`font-bold ${
                isPaper ? 'text-[#1b9c85]' : 'text-[#ff0060]'
              }`}
            >
              {isPaper ? 'PAPER' : 'LIVE'}
            </span>
          </div>
          <div className="mt-1 flex items-center justify-between">
            <span className="text-xs font-bold text-[#363949] dark:text-white truncate max-w-[120px]">
              {activeAccount ? activeAccount.label : 'Guest Simulation'}
            </span>
            <span className="text-xs font-bold font-mono text-[#1b9c85]">
              ${activeAccount ? activeAccount.balance_usdc.toFixed(2) : '10.00'} U
            </span>
          </div>
        </div>
      </div>

      {/* Reminders / Safeguards Section (Matches exact screenshot Reminders!) */}
      <div className="space-y-3">
        <div className="flex items-center justify-between px-2">
          <h3 className="text-base font-extrabold text-[#363949] dark:text-white">
            Reminders
          </h3>
          <button className="p-1.5 rounded-full hover:bg-slate-100 dark:hover:bg-slate-800 text-[#7d8da1] transition-colors">
            <Bell className="w-4 h-4" />
          </button>
        </div>

        {/* Reminder Item 1: Mode B Guard */}
        <div className="asmr-card p-4 flex items-center justify-between transition-all hover:translate-x-1">
          <div className="flex items-center gap-3 min-w-0">
            <div className="p-2.5 rounded-2xl bg-[#1b9c85] text-white shrink-0 shadow-md shadow-[#1b9c85]/20">
              <ShieldCheck className="w-4 h-4" />
            </div>
            <div className="truncate">
              <h4 className="text-xs font-bold text-[#363949] dark:text-white truncate">
                Mode B Capital Guard
              </h4>
              <p className="text-[10px] text-[#7d8da1] dark:text-slate-400 font-mono">
                Floor: $2.00 | Cap: $10.00
              </p>
            </div>
          </div>
          <button className="text-[#7d8da1] hover:text-[#363949] p-1">
            <MoreVertical className="w-4 h-4" />
          </button>
        </div>

        {/* Reminder Item 2: Emergency Halt */}
        <div
          onClick={onEmergencyHalt}
          className="asmr-card p-4 flex items-center justify-between transition-all hover:translate-x-1 cursor-pointer group"
        >
          <div className="flex items-center gap-3 min-w-0">
            <div className="p-2.5 rounded-2xl bg-[#ff0060] text-white shrink-0 shadow-md shadow-[#ff0060]/20 group-hover:scale-105 transition-transform">
              <ShieldAlert className="w-4 h-4" />
            </div>
            <div className="truncate">
              <h4 className="text-xs font-bold text-[#363949] dark:text-white truncate group-hover:text-[#ff0060] transition-colors">
                Emergency Kill Switch
              </h4>
              <p className="text-[10px] text-[#7d8da1] dark:text-slate-400 font-mono">
                Instant CLOB Abort
              </p>
            </div>
          </div>
          <button className="text-[#7d8da1] group-hover:text-[#ff0060] p-1">
            <MoreVertical className="w-4 h-4" />
          </button>
        </div>

        {/* Reminder Item 3: Auto-Learning Engine */}
        <div className="asmr-card p-4 flex items-center justify-between transition-all hover:translate-x-1">
          <div className="flex items-center gap-3 min-w-0">
            <div className="p-2.5 rounded-2xl bg-[#6c9bcf] text-white shrink-0 shadow-md shadow-[#6c9bcf]/20">
              <Brain className="w-4 h-4" />
            </div>
            <div className="truncate">
              <h4 className="text-xs font-bold text-[#363949] dark:text-white truncate">
                Continuous Online SGD
              </h4>
              <p className="text-[10px] text-[#7d8da1] dark:text-slate-400 font-mono">
                Auto-Adapt Every 5M Round
              </p>
            </div>
          </div>
          <button className="text-[#7d8da1] hover:text-[#363949] p-1">
            <MoreVertical className="w-4 h-4" />
          </button>
        </div>

        {/* Add Account / Reminder Dashed Button (Exact screenshot button!) */}
        <button
          onClick={onOpenAccountManager}
          className="w-full py-3.5 px-4 rounded-3xl border-2 border-dashed border-[#6c9bcf] text-[#6c9bcf] text-xs font-bold flex items-center justify-center gap-2 hover:bg-[#6c9bcf]/10 hover:border-solid transition-all cursor-pointer shadow-sm"
        >
          <Plus className="w-4 h-4" />
          <span>Add Polymarket Account</span>
        </button>
      </div>
    </div>
  );
};
