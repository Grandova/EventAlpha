import React, { useState } from 'react';
import { PaperOrder, PaperPosition, PaperResult, TradeStatistics } from '../types';

interface TradingBlotterProps {
  activePositions: PaperPosition[];
  recentOrders: PaperOrder[];
  settledResults: PaperResult[];
  statistics: TradeStatistics | null;
}

export const TradingBlotter: React.FC<TradingBlotterProps> = ({
  activePositions,
  recentOrders,
  settledResults,
  statistics,
}) => {
  const [activeTab, setActiveTab] = useState<'positions' | 'results' | 'orders'>('positions');
  const [showAll, setShowAll] = useState(false);

  const displayPositions = showAll ? activePositions : activePositions.slice(0, 5);
  const displayResults = showAll ? settledResults : settledResults.slice(0, 5);
  const displayOrders = showAll ? recentOrders : recentOrders.slice(0, 5);

  return (
    <div className="asmr-card p-6 lg:p-8 space-y-4">
      {/* Header (Matches screenshot "Recent Orders" title) */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-slate-100 dark:border-slate-800 pb-4">
        <div>
          <h2 className="text-xl font-extrabold text-[#363949] dark:text-white tracking-tight">
            Recent Orders
          </h2>
          <p className="text-xs text-[#7d8da1] dark:text-slate-400 mt-0.5">
            Polymarket 5M 交易执行与结算审计
          </p>
        </div>

        {/* Tab Switchers styled as AsmrProg pills */}
        <div className="flex items-center p-1 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e]">
          <button
            onClick={() => setActiveTab('positions')}
            className={`px-3.5 py-1.5 rounded-xl text-xs font-bold transition-all cursor-pointer ${
              activeTab === 'positions'
                ? 'bg-white dark:bg-[#202528] text-[#363949] dark:text-white shadow-sm'
                : 'text-[#7d8da1] hover:text-[#363949]'
            }`}
          >
            Active Positions ({activePositions.length})
          </button>
          <button
            onClick={() => setActiveTab('results')}
            className={`px-3.5 py-1.5 rounded-xl text-xs font-bold transition-all cursor-pointer ${
              activeTab === 'results'
                ? 'bg-white dark:bg-[#202528] text-[#363949] dark:text-white shadow-sm'
                : 'text-[#7d8da1] hover:text-[#363949]'
            }`}
          >
            Settled ({settledResults.length})
          </button>
          <button
            onClick={() => setActiveTab('orders')}
            className={`px-3.5 py-1.5 rounded-xl text-xs font-bold transition-all cursor-pointer ${
              activeTab === 'orders'
                ? 'bg-white dark:bg-[#202528] text-[#363949] dark:text-white shadow-sm'
                : 'text-[#7d8da1] hover:text-[#363949]'
            }`}
          >
            All Orders ({recentOrders.length})
          </button>
        </div>
      </div>

      {/* Table Content (Matches screenshot exact columns & text styling) */}
      <div className="overflow-x-auto">
        {activeTab === 'positions' && (
          <table className="w-full text-center text-xs">
            <thead>
              <tr className="text-[#7d8da1] dark:text-slate-400 font-semibold border-b border-slate-100 dark:border-slate-800">
                <th className="pb-3 text-left pl-3">Asset & Side</th>
                <th className="pb-3 font-mono">Order Number</th>
                <th className="pb-3 font-mono">Stake / Fill</th>
                <th className="pb-3">Status</th>
                <th className="pb-3 text-right pr-3">Details</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800/60 font-medium">
              {displayPositions.length > 0 ? (
                displayPositions.map((pos) => (
                  <tr key={pos.position_id} className="hover:bg-slate-50/80 dark:hover:bg-slate-800/40 transition-colors">
                    <td className="py-3.5 text-left pl-3 font-bold text-[#363949] dark:text-white">
                      <div className="flex items-center gap-2">
                        <span className="w-2 h-2 rounded-full bg-[#1b9c85]" />
                        <span>{pos.asset}</span>
                        <span
                          className={`px-2 py-0.5 rounded-md text-[10px] font-extrabold ${
                            pos.side === 'UP' ? 'bg-[#1b9c85]/15 text-[#1b9c85]' : 'bg-[#ff0060]/15 text-[#ff0060]'
                          }`}
                        >
                          {pos.side}
                        </span>
                      </div>
                    </td>
                    <td className="py-3.5 font-mono text-[#7d8da1] dark:text-slate-400">
                      {pos.position_id.slice(0, 10)}
                    </td>
                    <td className="py-3.5 font-mono font-bold text-[#363949] dark:text-white">
                      ${pos.stake.toFixed(2)} @ ${pos.entry_price.toFixed(3)}
                    </td>
                    <td className="py-3.5">
                      <span className="text-[#1b9c85] font-extrabold text-xs">
                        Active
                      </span>
                    </td>
                    <td className="py-3.5 text-right pr-3 font-mono text-[#6c9bcf] hover:underline cursor-pointer">
                      Details
                    </td>
                  </tr>
                ))
              ) : (
                <tr>
                  <td colSpan={5} className="py-8 text-center text-[#7d8da1] font-mono">
                    No active positions currently running.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        )}

        {activeTab === 'results' && (
          <table className="w-full text-center text-xs">
            <thead>
              <tr className="text-[#7d8da1] dark:text-slate-400 font-semibold border-b border-slate-100 dark:border-slate-800">
                <th className="pb-3 text-left pl-3">Asset & Side</th>
                <th className="pb-3 font-mono">Result ID</th>
                <th className="pb-3 font-mono">PnL / Stake</th>
                <th className="pb-3">Status</th>
                <th className="pb-3 text-right pr-3">Details</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800/60 font-medium">
              {displayResults.length > 0 ? (
                displayResults.map((res) => {
                  const isWin = res.outcome === 'WIN';
                  return (
                    <tr key={res.result_id} className="hover:bg-slate-50/80 dark:hover:bg-slate-800/40 transition-colors">
                      <td className="py-3.5 text-left pl-3 font-bold text-[#363949] dark:text-white">
                        <div className="flex items-center gap-2">
                          <span
                            className={`w-2 h-2 rounded-full ${isWin ? 'bg-[#1b9c85]' : 'bg-[#ff0060]'}`}
                          />
                          <span>{res.asset}</span>
                          <span
                            className={`px-2 py-0.5 rounded-md text-[10px] font-extrabold ${
                              res.side === 'UP' ? 'bg-[#1b9c85]/15 text-[#1b9c85]' : 'bg-[#ff0060]/15 text-[#ff0060]'
                            }`}
                          >
                            {res.side}
                          </span>
                        </div>
                      </td>
                      <td className="py-3.5 font-mono text-[#7d8da1] dark:text-slate-400">
                        {res.result_id.slice(0, 10)}
                      </td>
                      <td className="py-3.5 font-mono font-bold">
                        <span className={res.pnl >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}>
                          {res.pnl >= 0 ? '+' : ''}${res.pnl.toFixed(2)}
                        </span>
                        <span className="text-[#7d8da1] text-[10px] ml-1">(${res.stake.toFixed(2)})</span>
                      </td>
                      <td className="py-3.5">
                        <span
                          className={`font-extrabold text-xs ${
                            isWin ? 'text-[#1b9c85]' : 'text-[#ff0060]'
                          }`}
                        >
                          {isWin ? 'Active' : 'Declined'}
                        </span>
                      </td>
                      <td className="py-3.5 text-right pr-3 font-mono text-[#6c9bcf] hover:underline cursor-pointer">
                        Details
                      </td>
                    </tr>
                  );
                })
              ) : (
                <tr>
                  <td colSpan={5} className="py-8 text-center text-[#7d8da1] font-mono">
                    No settled results yet.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        )}

        {activeTab === 'orders' && (
          <table className="w-full text-center text-xs">
            <thead>
              <tr className="text-[#7d8da1] dark:text-slate-400 font-semibold border-b border-slate-100 dark:border-slate-800">
                <th className="pb-3 text-left pl-3">Asset & Side</th>
                <th className="pb-3 font-mono">Order Number</th>
                <th className="pb-3 font-mono">Payment / Fill</th>
                <th className="pb-3">Status</th>
                <th className="pb-3 text-right pr-3">Details</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800/60 font-medium">
              {displayOrders.length > 0 ? (
                displayOrders.map((ord) => (
                  <tr key={ord.order_id} className="hover:bg-slate-50/80 dark:hover:bg-slate-800/40 transition-colors">
                    <td className="py-3.5 text-left pl-3 font-bold text-[#363949] dark:text-white">
                      <div className="flex items-center gap-2">
                        <span className="w-2 h-2 rounded-full bg-[#6c9bcf]" />
                        <span>{ord.asset}</span>
                        <span
                          className={`px-2 py-0.5 rounded-md text-[10px] font-extrabold ${
                            ord.side === 'UP' ? 'bg-[#1b9c85]/15 text-[#1b9c85]' : 'bg-[#ff0060]/15 text-[#ff0060]'
                          }`}
                        >
                          {ord.side}
                        </span>
                      </div>
                    </td>
                    <td className="py-3.5 font-mono text-[#7d8da1] dark:text-slate-400">
                      {ord.order_id.slice(0, 10)}
                    </td>
                    <td className="py-3.5 font-mono font-bold text-[#363949] dark:text-white">
                      ${ord.stake.toFixed(2)} (${ord.fill_price.toFixed(3)})
                    </td>
                    <td className="py-3.5">
                      <span className="text-[#f7d154] font-extrabold text-xs">
                        {ord.status}
                      </span>
                    </td>
                    <td className="py-3.5 text-right pr-3 font-mono text-[#6c9bcf] hover:underline cursor-pointer">
                      Details
                    </td>
                  </tr>
                ))
              ) : (
                <tr>
                  <td colSpan={5} className="py-8 text-center text-[#7d8da1] font-mono">
                    No orders recorded yet.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        )}
      </div>

      {/* AsmrProg Iconic "Show All" Link at bottom (Exact screenshot!) */}
      <div className="pt-2 text-center">
        <button
          onClick={() => setShowAll(!showAll)}
          className="text-xs font-bold text-[#6c9bcf] hover:underline cursor-pointer transition-colors"
        >
          {showAll ? 'Show Less' : 'Show All'}
        </button>
      </div>
    </div>
  );
};
