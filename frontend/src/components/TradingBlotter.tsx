import React from 'react';
import { Briefcase, CheckCircle2, XCircle, Clock, Award, TrendingUp, Percent } from 'lucide-react';
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
  const [activeTab, setActiveTab] = React.useState<'positions' | 'results' | 'orders'>('positions');

  const winRate = statistics?.win_rate ?? 0.0;
  const totalTrades = statistics?.total_trades ?? 0;
  const netPnl = statistics?.net_pnl ?? 0.0;
  const profitFactor = statistics?.profit_factor ?? 0.0;

  return (
    <div className="asmr-card p-6">
      {/* Header & Mini Stats Bar */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-4">
        <div className="flex items-center gap-2">
          <Briefcase className="h-4 w-4 text-cyan-400" />
          <h2 className="text-xs font-bold text-slate-400 uppercase tracking-wider">
            Paper Trading Blotter & Performance Statistics
          </h2>
        </div>

        {/* 4 Mini Stats */}
        <div className="flex items-center gap-3 text-xs font-mono">
          <div className="bg-slate-900 px-2.5 py-1 rounded border border-slate-800">
            <span className="text-slate-500 mr-1.5">Win Rate:</span>
            <strong className="text-emerald-400 font-mono-num">{winRate.toFixed(1)}%</strong>
          </div>
          <div className="bg-slate-900 px-2.5 py-1 rounded border border-slate-800">
            <span className="text-slate-500 mr-1.5">Trades:</span>
            <strong className="text-white font-mono-num">{totalTrades}</strong>
          </div>
          <div className="bg-slate-900 px-2.5 py-1 rounded border border-slate-800">
            <span className="text-slate-500 mr-1.5">Net PnL:</span>
            <strong className={`font-mono-num ${netPnl >= 0 ? 'text-emerald-400' : 'text-rose-400'}`}>
              {netPnl >= 0 ? '+' : ''}${netPnl.toFixed(2)}
            </strong>
          </div>
          <div className="bg-slate-900 px-2.5 py-1 rounded border border-slate-800">
            <span className="text-slate-500 mr-1.5">Profit Factor:</span>
            <strong className="text-cyan-400 font-mono-num">{profitFactor.toFixed(2)}</strong>
          </div>
        </div>
      </div>

      {/* Blotter Tabs */}
      <div className="flex items-center gap-2 mb-3 border-b border-slate-800 pb-2">
        <button
          onClick={() => setActiveTab('positions')}
          className={`px-3 py-1 rounded text-xs font-bold transition-all ${
            activeTab === 'positions'
              ? 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/40'
              : 'text-slate-400 hover:text-white'
          }`}
        >
          Active Positions ({activePositions.length})
        </button>
        <button
          onClick={() => setActiveTab('results')}
          className={`px-3 py-1 rounded text-xs font-bold transition-all ${
            activeTab === 'results'
              ? 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/40'
              : 'text-slate-400 hover:text-white'
          }`}
        >
          Settled Results ({settledResults.length})
        </button>
        <button
          onClick={() => setActiveTab('orders')}
          className={`px-3 py-1 rounded text-xs font-bold transition-all ${
            activeTab === 'orders'
              ? 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/40'
              : 'text-slate-400 hover:text-white'
          }`}
        >
          Recent Orders ({recentOrders.length})
        </button>
      </div>

      {/* Tab 1: Active Open Positions */}
      {activeTab === 'positions' && (
        <div className="overflow-x-auto">
          {activePositions.length === 0 ? (
            <div className="py-8 text-center text-xs text-slate-500 font-mono">
              No active open positions. System is evaluating next 5-minute opportunities.
            </div>
          ) : (
            <table className="w-full text-left text-xs">
              <thead className="text-[10px] text-slate-500 uppercase border-b border-slate-800">
                <tr>
                  <th className="py-2 px-3">Position ID</th>
                  <th className="py-2 px-3">Asset</th>
                  <th className="py-2 px-3">Side</th>
                  <th className="py-2 px-3">Fill Price</th>
                  <th className="py-2 px-3">Stake</th>
                  <th className="py-2 px-3">Shares</th>
                  <th className="py-2 px-3">Status</th>
                  <th className="py-2 px-3">Entry Time</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/60 font-mono">
                {activePositions.map((pos) => (
                  <tr key={pos.position_id} className="hover:bg-slate-900/50">
                    <td className="py-2 px-3 text-cyan-400">{pos.position_id}</td>
                    <td className="py-2 px-3 font-bold text-white">{pos.asset}</td>
                    <td className="py-2 px-3">
                      <span
                        className={`px-2 py-0.5 rounded text-[10px] font-bold ${
                          pos.side === 'UP' ? 'bg-emerald-950 text-emerald-400' : 'bg-rose-950 text-rose-400'
                        }`}
                      >
                        {pos.side}
                      </span>
                    </td>
                    <td className="py-2 px-3 text-slate-300 font-mono-num">${pos.entry_price.toFixed(3)}</td>
                    <td className="py-2 px-3 text-slate-300 font-mono-num">${pos.stake.toFixed(2)}</td>
                    <td className="py-2 px-3 text-slate-300 font-mono-num">{pos.shares.toFixed(2)}</td>
                    <td className="py-2 px-3">
                      <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-cyan-950 text-cyan-400 border border-cyan-800/60">
                        {pos.status}
                      </span>
                    </td>
                    <td className="py-2 px-3 text-slate-500 text-[11px]">
                      {new Date(pos.entry_time_ms).toLocaleTimeString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}

      {/* Tab 2: Settled Results */}
      {activeTab === 'results' && (
        <div className="overflow-x-auto">
          {settledResults.length === 0 ? (
            <div className="py-8 text-center text-xs text-slate-500 font-mono">
              No settled trade results yet. Settlements execute automatically upon round resolution.
            </div>
          ) : (
            <table className="w-full text-left text-xs">
              <thead className="text-[10px] text-slate-500 uppercase border-b border-slate-800">
                <tr>
                  <th className="py-2 px-3">Result ID</th>
                  <th className="py-2 px-3">Asset</th>
                  <th className="py-2 px-3">Side</th>
                  <th className="py-2 px-3">Outcome</th>
                  <th className="py-2 px-3">Fill / Stake</th>
                  <th className="py-2 px-3">Net PnL</th>
                  <th className="py-2 px-3">Bankroll After</th>
                  <th className="py-2 px-3">Locked Profit After</th>
                  <th className="py-2 px-3">Settled At</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/60 font-mono">
                {settledResults.map((res) => (
                  <tr key={res.result_id} className="hover:bg-slate-900/50">
                    <td className="py-2 px-3 text-slate-400">{res.result_id.slice(0, 16)}...</td>
                    <td className="py-2 px-3 font-bold text-white">{res.asset}</td>
                    <td className="py-2 px-3">
                      <span
                        className={`px-1.5 py-0.5 rounded text-[10px] font-bold ${
                          res.side === 'UP' ? 'bg-emerald-950 text-emerald-400' : 'bg-rose-950 text-rose-400'
                        }`}
                      >
                        {res.side}
                      </span>
                    </td>
                    <td className="py-2 px-3">
                      <span
                        className={`px-2 py-0.5 rounded text-[10px] font-black ${
                          res.outcome === 'WIN'
                            ? 'bg-emerald-500 text-slate-950'
                            : res.outcome === 'LOSS'
                            ? 'bg-rose-500 text-white'
                            : 'bg-slate-800 text-slate-300'
                        }`}
                      >
                        {res.outcome}
                      </span>
                    </td>
                    <td className="py-2 px-3 text-slate-300 font-mono-num">
                      ${res.fill_price.toFixed(3)} / ${res.stake.toFixed(2)}
                    </td>
                    <td
                      className={`py-2 px-3 font-mono-num font-bold ${
                        res.pnl > 0 ? 'text-emerald-400' : res.pnl < 0 ? 'text-rose-400' : 'text-slate-400'
                      }`}
                    >
                      {res.pnl > 0 ? '+' : ''}${res.pnl.toFixed(2)}
                    </td>
                    <td className="py-2 px-3 text-cyan-400 font-mono-num">${res.bankroll_after.toFixed(2)}</td>
                    <td className="py-2 px-3 text-emerald-400 font-mono-num">${res.locked_profit_after.toFixed(2)}</td>
                    <td className="py-2 px-3 text-slate-500 text-[11px]">
                      {new Date(res.created_at_ms).toLocaleTimeString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}

      {/* Tab 3: Recent Orders */}
      {activeTab === 'orders' && (
        <div className="overflow-x-auto">
          {recentOrders.length === 0 ? (
            <div className="py-8 text-center text-xs text-slate-500 font-mono">
              No paper orders recorded yet.
            </div>
          ) : (
            <table className="w-full text-left text-xs">
              <thead className="text-[10px] text-slate-500 uppercase border-b border-slate-800">
                <tr>
                  <th className="py-2 px-3">Order ID</th>
                  <th className="py-2 px-3">Asset</th>
                  <th className="py-2 px-3">Side</th>
                  <th className="py-2 px-3">Stake</th>
                  <th className="py-2 px-3">Quote / Fill</th>
                  <th className="py-2 px-3">Slippage</th>
                  <th className="py-2 px-3">Fee</th>
                  <th className="py-2 px-3">Status</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/60 font-mono">
                {recentOrders.map((ord) => (
                  <tr key={ord.order_id} className="hover:bg-slate-900/50">
                    <td className="py-2 px-3 text-slate-400">{ord.order_id.slice(0, 16)}...</td>
                    <td className="py-2 px-3 font-bold text-white">{ord.asset}</td>
                    <td className="py-2 px-3">
                      <span
                        className={`px-1.5 py-0.5 rounded text-[10px] font-bold ${
                          ord.side === 'UP' ? 'bg-emerald-950 text-emerald-400' : 'bg-rose-950 text-rose-400'
                        }`}
                      >
                        {ord.side}
                      </span>
                    </td>
                    <td className="py-2 px-3 text-slate-300 font-mono-num">${ord.stake.toFixed(2)}</td>
                    <td className="py-2 px-3 text-slate-300 font-mono-num">
                      ${ord.quote_price.toFixed(3)} &rarr; ${ord.fill_price.toFixed(3)}
                    </td>
                    <td className="py-2 px-3 text-slate-400 font-mono-num">${ord.slippage.toFixed(4)}</td>
                    <td className="py-2 px-3 text-slate-400 font-mono-num">${ord.fee.toFixed(4)}</td>
                    <td className="py-2 px-3 text-emerald-400 font-semibold">{ord.status}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}
    </div>
  );
};
