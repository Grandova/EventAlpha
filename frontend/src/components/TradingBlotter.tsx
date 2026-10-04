import React, { useState } from 'react';
import {
  X,
  FileText,
  CheckCircle,
  Clock,
  Zap,
  ArrowUpRight,
  ArrowDownRight,
  Layers,
  ShieldCheck,
  TrendingUp,
  TrendingDown,
  Info,
  Hash,
} from 'lucide-react';
import { PaperOrder, PaperPosition, PaperResult, TradeStatistics } from '../types';

interface TradingBlotterProps {
  activePositions: PaperPosition[];
  recentOrders: PaperOrder[];
  settledResults: PaperResult[];
  statistics: TradeStatistics | null;
}

type DetailItem =
  | { type: 'position'; data: PaperPosition }
  | { type: 'result'; data: PaperResult }
  | { type: 'order'; data: PaperOrder };

export const TradingBlotter: React.FC<TradingBlotterProps> = ({
  activePositions,
  recentOrders,
  settledResults,
  statistics,
}) => {
  const [activeTab, setActiveTab] = useState<'positions' | 'results' | 'orders'>('positions');
  const [showAll, setShowAll] = useState(false);
  const [selectedItem, setSelectedItem] = useState<DetailItem | null>(null);

  const formatTime = (ts?: number) => {
    if (!ts || isNaN(ts)) return '--:--:--';
    return new Date(ts).toLocaleTimeString([], { hour12: false });
  };

  const formatFullTime = (ts?: number) => {
    if (!ts || isNaN(ts)) return '--:--:--';
    return new Date(ts).toLocaleString('zh-CN', {
      hour12: false,
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };

  const displayPositions = showAll ? activePositions : activePositions.slice(0, 10);
  const displayResults = showAll ? settledResults : settledResults.slice(0, 10);
  const displayOrders = showAll ? recentOrders : recentOrders.slice(0, 10);

  const currentTotal =
    activeTab === 'positions'
      ? activePositions.length
      : activeTab === 'results'
      ? settledResults.length
      : recentOrders.length;

  return (
    <div className="asmr-card p-6 lg:p-8 space-y-4">
      {/* Header (Matches screenshot "Recent Orders" title) */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-slate-100 dark:border-slate-800 pb-4">
        <div>
          <h2 className="text-xl font-extrabold text-[#363949] dark:text-white tracking-tight">
            近期交易与委托
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
            在途持仓 ({activePositions.length})
          </button>
          <button
            onClick={() => setActiveTab('results')}
            className={`px-3.5 py-1.5 rounded-xl text-xs font-bold transition-all cursor-pointer ${
              activeTab === 'results'
                ? 'bg-white dark:bg-[#202528] text-[#363949] dark:text-white shadow-sm'
                : 'text-[#7d8da1] hover:text-[#363949]'
            }`}
          >
            已结算 ({settledResults.length})
          </button>
          <button
            onClick={() => setActiveTab('orders')}
            className={`px-3.5 py-1.5 rounded-xl text-xs font-bold transition-all cursor-pointer ${
              activeTab === 'orders'
                ? 'bg-white dark:bg-[#202528] text-[#363949] dark:text-white shadow-sm'
                : 'text-[#7d8da1] hover:text-[#363949]'
            }`}
          >
            全部委托 ({recentOrders.length})
          </button>
        </div>
      </div>

      {/* Table Content (Matches screenshot exact columns & text styling) */}
      <div className="overflow-x-auto">
        {activeTab === 'positions' && (
          <table className="w-full text-center text-xs">
            <thead>
              <tr className="text-[#7d8da1] dark:text-slate-400 font-semibold border-b border-slate-100 dark:border-slate-800">
                <th className="pb-3 text-left pl-3">标的 & 方向</th>
                <th className="pb-3">开仓时间</th>
                <th className="pb-3 font-mono">持仓编号</th>
                <th className="pb-3 font-mono">本金 / 入场价</th>
                <th className="pb-3">状态</th>
                <th className="pb-3 text-right pr-3">详情</th>
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
                      {formatTime(pos.created_at_ms || pos.entry_time_ms)}
                    </td>
                    <td className="py-3.5 font-mono text-[#7d8da1] dark:text-slate-400">
                      {pos.position_id.slice(0, 10)}
                    </td>
                    <td className="py-3.5 font-mono font-bold text-[#363949] dark:text-white">
                      ${(typeof pos.stake === 'number' && !isNaN(pos.stake) ? pos.stake : 0).toFixed(2)} @ ${(typeof pos.entry_price === 'number' && !isNaN(pos.entry_price) ? pos.entry_price : 0.5).toFixed(3)}
                    </td>
                    <td className="py-3.5">
                      <span className="text-[#1b9c85] font-extrabold text-xs">
                        持仓中
                      </span>
                    </td>
                    <td className="py-3.5 text-right pr-3">
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedItem({ type: 'position', data: pos });
                        }}
                        className="px-2.5 py-1 text-[11px] font-bold rounded-lg bg-[#6c9bcf]/15 text-[#6c9bcf] hover:bg-[#6c9bcf]/25 border border-[#6c9bcf]/30 transition cursor-pointer"
                      >
                        查看详情
                      </button>
                    </td>
                  </tr>
                ))
              ) : (
                <tr>
                  <td colSpan={6} className="py-8 text-center text-[#7d8da1] font-mono">
                    暂无运行中的活跃在途持仓。
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
                <th className="pb-3 text-left pl-3">标的 & 方向</th>
                <th className="pb-3">结算时间</th>
                <th className="pb-3 font-mono">结算编号</th>
                <th className="pb-3 font-mono">盈亏 / 本金</th>
                <th className="pb-3">结果</th>
                <th className="pb-3 text-right pr-3">详情</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800/60 font-medium">
              {displayResults.length > 0 ? (
                displayResults.map((res) => {
                  const isWin = res.outcome === 'WIN';
                  const pnlVal = typeof res.pnl === 'number' && !isNaN(res.pnl) ? res.pnl : 0;
                  const stakeVal = typeof res.stake === 'number' && !isNaN(res.stake) ? res.stake : 0;
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
                        {formatTime(res.created_at_ms || res.entry_time_ms)}
                      </td>
                      <td className="py-3.5 font-mono text-[#7d8da1] dark:text-slate-400">
                        {res.result_id.slice(0, 10)}
                      </td>
                      <td className="py-3.5 font-mono font-bold">
                        <span className={pnlVal >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}>
                          {pnlVal >= 0 ? '+' : ''}${pnlVal.toFixed(2)}
                        </span>
                        <span className="text-[#7d8da1] text-[10px] ml-1">(${stakeVal.toFixed(2)})</span>
                      </td>
                      <td className="py-3.5">
                        <span
                          className={`font-extrabold text-xs ${
                            isWin ? 'text-[#1b9c85]' : 'text-[#ff0060]'
                          }`}
                        >
                          {isWin ? '盈利 (WIN)' : '亏损 (LOSS)'}
                        </span>
                      </td>
                      <td className="py-3.5 text-right pr-3">
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            setSelectedItem({ type: 'result', data: res });
                          }}
                          className="px-2.5 py-1 text-[11px] font-bold rounded-lg bg-[#6c9bcf]/15 text-[#6c9bcf] hover:bg-[#6c9bcf]/25 border border-[#6c9bcf]/30 transition cursor-pointer"
                        >
                          查看详情
                        </button>
                      </td>
                    </tr>
                  );
                })
              ) : (
                <tr>
                  <td colSpan={6} className="py-8 text-center text-[#7d8da1] font-mono">
                    暂无已结算历史记录。
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
                <th className="pb-3 text-left pl-3">标的 & 方向</th>
                <th className="pb-3">委托时间</th>
                <th className="pb-3 font-mono">委托单号</th>
                <th className="pb-3 font-mono">本金 / 价格</th>
                <th className="pb-3">状态</th>
                <th className="pb-3 text-right pr-3">详情</th>
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
                      {formatTime(ord.timestamp_ms)}
                    </td>
                    <td className="py-3.5 font-mono text-[#7d8da1] dark:text-slate-400">
                      {ord.order_id.slice(0, 10)}
                    </td>
                    <td className="py-3.5 font-mono font-bold text-[#363949] dark:text-white">
                      ${(typeof ord.stake === 'number' && !isNaN(ord.stake) ? ord.stake : 0).toFixed(2)} (${(typeof ord.fill_price === 'number' && !isNaN(ord.fill_price) ? ord.fill_price : 0.5).toFixed(3)})
                    </td>
                    <td className="py-3.5">
                      <span className="text-[#1b9c85] font-extrabold text-xs">
                        {ord.status === 'FILLED' ? '已成交' : ord.status === 'PENDING' ? '待撮合' : ord.status}
                      </span>
                    </td>
                    <td className="py-3.5 text-right pr-3">
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedItem({ type: 'order', data: ord });
                        }}
                        className="px-2.5 py-1 text-[11px] font-bold rounded-lg bg-[#6c9bcf]/15 text-[#6c9bcf] hover:bg-[#6c9bcf]/25 border border-[#6c9bcf]/30 transition cursor-pointer"
                      >
                        查看详情
                      </button>
                    </td>
                  </tr>
                ))
              ) : (
                <tr>
                  <td colSpan={6} className="py-8 text-center text-[#7d8da1] font-mono">
                    暂无委托记录。
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        )}
      </div>

      {/* Show All / Expand link */}
      {currentTotal > 10 ? (
        <div className="pt-2 text-center">
          <button
            onClick={() => setShowAll(!showAll)}
            className="text-xs font-bold text-[#6c9bcf] hover:underline cursor-pointer transition-colors"
          >
            {showAll ? '收起列表' : `展开查看全部记录 (共 ${currentTotal} 笔)`}
          </button>
        </div>
      ) : currentTotal > 0 ? (
        <div className="pt-2 text-center text-[11px] text-[#7d8da1] font-mono">
          已显示全部 {currentTotal} 笔记录
        </div>
      ) : null}

      {/* Transaction & Settlement Details Modal */}
      {selectedItem && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-sm animate-fadeIn">
          <div className="bg-white dark:bg-[#1f2229] border border-slate-200 dark:border-slate-800 rounded-3xl max-w-lg w-full p-6 shadow-2xl space-y-5 animate-scaleUp max-h-[90vh] overflow-y-auto">
            {/* Modal Header */}
            <div className="flex items-center justify-between pb-3.5 border-b border-slate-100 dark:border-slate-800">
              <div className="flex items-center gap-3">
                <div className="p-2.5 rounded-2xl bg-[#6c9bcf]/15 text-[#6c9bcf]">
                  <FileText className="w-5 h-5" />
                </div>
                <div>
                  <h3 className="text-base font-extrabold text-[#363949] dark:text-white">
                    {selectedItem.type === 'position' && '在途持仓详细审计'}
                    {selectedItem.type === 'result' && '交易结算与收益归因'}
                    {selectedItem.type === 'order' && '委托订单撮合审计'}
                  </h3>
                  <div className="flex items-center gap-2 mt-1">
                    <span className="text-xs font-black text-[#363949] dark:text-white">
                      {selectedItem.data.asset}
                    </span>
                    <span
                      className={`px-2 py-0.5 rounded-md text-[10px] font-extrabold ${
                        selectedItem.data.side === 'UP'
                          ? 'bg-[#1b9c85]/15 text-[#1b9c85]'
                          : 'bg-[#ff0060]/15 text-[#ff0060]'
                      }`}
                    >
                      {selectedItem.data.side} 看{selectedItem.data.side === 'UP' ? '涨' : '跌'}
                    </span>
                    {selectedItem.type === 'result' && (
                      <span
                        className={`px-2 py-0.5 rounded-md text-[10px] font-extrabold ${
                          (selectedItem.data as PaperResult).outcome === 'WIN'
                            ? 'bg-[#1b9c85]/15 text-[#1b9c85]'
                            : 'bg-[#ff0060]/15 text-[#ff0060]'
                        }`}
                      >
                        {(selectedItem.data as PaperResult).outcome === 'WIN' ? '盈利 (WIN)' : '亏损 (LOSS)'}
                      </span>
                    )}
                  </div>
                </div>
              </div>
              <button
                onClick={() => setSelectedItem(null)}
                className="p-2 rounded-xl bg-slate-100 dark:bg-slate-800 text-[#7d8da1] hover:text-[#363949] dark:hover:text-white transition-colors cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Modal Body - Position */}
            {selectedItem.type === 'position' && (() => {
              const pos = selectedItem.data as PaperPosition;
              const stakeVal = typeof pos.stake === 'number' && !isNaN(pos.stake) ? pos.stake : 0;
              const priceVal = typeof pos.entry_price === 'number' && !isNaN(pos.entry_price) ? pos.entry_price : 0.5;
              const sharesVal = typeof pos.shares === 'number' && !isNaN(pos.shares) ? pos.shares : 0;
              return (
                <div className="space-y-4 text-xs">
                  <div className="grid grid-cols-2 gap-3">
                    <div className="p-3.5 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60">
                      <div className="text-[11px] text-[#7d8da1] font-medium">投入本金</div>
                      <div className="text-base font-extrabold font-mono text-[#363949] dark:text-white mt-0.5">
                        ${stakeVal.toFixed(2)} USDC
                      </div>
                    </div>
                    <div className="p-3.5 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60">
                      <div className="text-[11px] text-[#7d8da1] font-medium">入场价格</div>
                      <div className="text-base font-extrabold font-mono text-[#6c9bcf] mt-0.5">
                        ${priceVal.toFixed(3)}
                      </div>
                    </div>
                    <div className="p-3.5 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60">
                      <div className="text-[11px] text-[#7d8da1] font-medium">持有份额</div>
                      <div className="text-base font-extrabold font-mono text-[#363949] dark:text-white mt-0.5">
                        {sharesVal.toFixed(2)} 份
                      </div>
                    </div>
                    <div className="p-3.5 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60">
                      <div className="text-[11px] text-[#7d8da1] font-medium">运行状态</div>
                      <div className="text-xs font-bold text-[#1b9c85] mt-1 flex items-center gap-1.5">
                        <span className="w-2 h-2 rounded-full bg-[#1b9c85] animate-ping" />
                        持仓在途 (5分钟自动到期兑现)
                      </div>
                    </div>
                  </div>

                  <div className="p-4 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60 space-y-2.5 font-mono">
                    <div className="flex items-center justify-between text-[#7d8da1]">
                      <span>持仓编号</span>
                      <span className="text-[#363949] dark:text-slate-200 font-bold">{pos.position_id}</span>
                    </div>
                    <div className="flex items-center justify-between text-[#7d8da1]">
                      <span>标的合约</span>
                      <span className="text-[#363949] dark:text-slate-200 truncate max-w-[220px]" title={pos.market_id}>{pos.market_id}</span>
                    </div>
                    <div className="flex items-center justify-between text-[#7d8da1]">
                      <span>开仓时间</span>
                      <span className="text-[#363949] dark:text-slate-200">{formatFullTime(pos.entry_time_ms || pos.created_at_ms)}</span>
                    </div>
                    <div className="flex items-center justify-between text-[#7d8da1]">
                      <span>预定结算</span>
                      <span className="text-[#1b9c85] font-sans font-bold">5分钟到期由结算引擎自动兑现</span>
                    </div>
                  </div>
                </div>
              );
            })()}

            {/* Modal Body - Result */}
            {selectedItem.type === 'result' && (() => {
              const res = selectedItem.data as PaperResult;
              const isWin = res.outcome === 'WIN';
              const pnlVal = typeof res.pnl === 'number' && !isNaN(res.pnl) ? res.pnl : 0;
              const stakeVal = typeof res.stake === 'number' && !isNaN(res.stake) ? res.stake : 0;
              const payoutVal = typeof res.payout === 'number' && !isNaN(res.payout) ? res.payout : 0;
              const probVal = typeof res.predicted_probability === 'number' && !isNaN(res.predicted_probability) ? res.predicted_probability : 0.5;
              const netEdgeVal = typeof res.net_edge === 'number' && !isNaN(res.net_edge) ? res.net_edge : 0;
              const grossEdgeVal = typeof res.gross_edge === 'number' && !isNaN(res.gross_edge) ? res.gross_edge : 0;
              const feeVal = typeof res.fee === 'number' && !isNaN(res.fee) ? res.fee : 0;
              const slippageVal = typeof res.slippage === 'number' && !isNaN(res.slippage) ? res.slippage : 0;

              return (
                <div className="space-y-4 text-xs">
                  {/* Top Highlight Cards */}
                  <div className="grid grid-cols-3 gap-2.5">
                    <div className={`p-3 rounded-2xl border text-center ${
                      isWin ? 'bg-[#1b9c85]/10 border-[#1b9c85]/30' : 'bg-[#ff0060]/10 border-[#ff0060]/30'
                    }`}>
                      <div className="text-[10px] text-[#7d8da1] font-medium">最终盈亏</div>
                      <div className={`text-base font-black font-mono mt-0.5 ${
                        pnlVal >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'
                      }`}>
                        {pnlVal >= 0 ? '+' : ''}${pnlVal.toFixed(2)}
                      </div>
                    </div>
                    <div className="p-3 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60 text-center">
                      <div className="text-[10px] text-[#7d8da1] font-medium">投入本金</div>
                      <div className="text-base font-extrabold font-mono text-[#363949] dark:text-white mt-0.5">
                        ${stakeVal.toFixed(2)}
                      </div>
                    </div>
                    <div className="p-3 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60 text-center">
                      <div className="text-[10px] text-[#7d8da1] font-medium">最终兑付</div>
                      <div className="text-base font-extrabold font-mono text-[#363949] dark:text-white mt-0.5">
                        ${payoutVal.toFixed(2)}
                      </div>
                    </div>
                  </div>

                  {/* Pricing & Execution Grid */}
                  <div className="p-4 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60 space-y-2.5">
                    <div className="text-[11px] font-bold text-[#363949] dark:text-white flex items-center gap-1.5">
                      <TrendingUp className="w-3.5 h-3.5 text-[#6c9bcf]" />
                      <span>交易撮合与执行明细</span>
                    </div>
                    <div className="grid grid-cols-2 gap-2 font-mono text-[#7d8da1]">
                      <div>初始入场价: <span className="text-[#363949] dark:text-slate-200 font-bold">${res.entry_price?.toFixed(3) || '0.500'}</span></div>
                      <div>撮合成交价: <span className="text-[#363949] dark:text-slate-200 font-bold">${res.fill_price?.toFixed(3) || '0.500'}</span></div>
                      <div>成交份额: <span className="text-[#363949] dark:text-slate-200 font-bold">{res.shares?.toFixed(2) || '0.00'} 份</span></div>
                      <div>手续费: <span className="text-[#363949] dark:text-slate-200 font-bold">${feeVal.toFixed(4)}</span></div>
                      <div>滑点损失: <span className="text-[#363949] dark:text-slate-200 font-bold">${slippageVal.toFixed(4)}</span></div>
                      <div>投资回报率 (ROI): <span className={`font-bold ${pnlVal >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}`}>
                        {stakeVal > 0 ? ((pnlVal / stakeVal) * 100).toFixed(1) : '0.0'}%
                      </span></div>
                    </div>
                  </div>

                  {/* Quantitative Model Attribution */}
                  <div className="p-4 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60 space-y-2.5">
                    <div className="text-[11px] font-bold text-[#363949] dark:text-white flex items-center gap-1.5">
                      <Zap className="w-3.5 h-3.5 text-amber-500" />
                      <span>量化模型归因与信号</span>
                    </div>
                    <div className="grid grid-cols-2 gap-2 font-mono text-[#7d8da1]">
                      <div>预测胜率: <span className="text-[#6c9bcf] font-bold">{(probVal * 100).toFixed(1)}%</span></div>
                      <div>置信度评级: <span className="text-[#363949] dark:text-slate-200 font-bold">{res.model_confidence || 'HIGH'}</span></div>
                      <div>净边缘 (Net Edge): <span className="text-[#1b9c85] font-bold">{(netEdgeVal * 100).toFixed(2)}%</span></div>
                      <div>毛边缘 (Gross): <span className="text-[#363949] dark:text-slate-200 font-bold">{(grossEdgeVal * 100).toFixed(2)}%</span></div>
                      <div>策略版本: <span className="text-[#363949] dark:text-slate-200">{res.strategy_version || 'v1.0.0'}</span></div>
                      <div>模型版本: <span className="text-[#363949] dark:text-slate-200">{res.model_version || 'ensemble-v1'}</span></div>
                    </div>
                  </div>

                  {/* Bankroll Changes */}
                  <div className="p-3.5 rounded-2xl bg-[#6c9bcf]/5 border border-[#6c9bcf]/20 space-y-1.5 font-mono">
                    <div className="flex items-center justify-between text-[#7d8da1]">
                      <span>活跃本金池变动:</span>
                      <span className="text-[#363949] dark:text-white font-bold">
                        ${(res.bankroll_before || 0).toFixed(2)} → ${(res.bankroll_after || 0).toFixed(2)} USDC
                      </span>
                    </div>
                    <div className="flex items-center justify-between text-[#7d8da1]">
                      <span>锁定利润池变动:</span>
                      <span className="text-[#1b9c85] font-bold">
                        ${(res.locked_profit_before || 0).toFixed(2)} → ${(res.locked_profit_after || 0).toFixed(2)} USDC
                      </span>
                    </div>
                  </div>

                  {/* Identifiers & Timing */}
                  <div className="p-3.5 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60 space-y-1.5 font-mono text-[11px] text-[#7d8da1]">
                    <div className="flex justify-between">
                      <span>结算编号:</span>
                      <span className="text-[#363949] dark:text-slate-200">{res.result_id}</span>
                    </div>
                    <div className="flex justify-between">
                      <span>开仓时间:</span>
                      <span className="text-[#363949] dark:text-slate-200">{formatFullTime(res.entry_time_ms)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span>结算时间:</span>
                      <span className="text-[#363949] dark:text-slate-200">{formatFullTime(res.created_at_ms)}</span>
                    </div>
                  </div>
                </div>
              );
            })()}

            {/* Modal Body - Order */}
            {selectedItem.type === 'order' && (() => {
              const ord = selectedItem.data as PaperOrder;
              const stakeVal = typeof ord.stake === 'number' && !isNaN(ord.stake) ? ord.stake : 0;
              const quotePrice = typeof ord.quote_price === 'number' && !isNaN(ord.quote_price) ? ord.quote_price : 0.5;
              const fillPrice = typeof ord.fill_price === 'number' && !isNaN(ord.fill_price) ? ord.fill_price : 0.5;
              const sharesVal = typeof ord.shares === 'number' && !isNaN(ord.shares) ? ord.shares : 0;

              return (
                <div className="space-y-4 text-xs">
                  <div className="grid grid-cols-2 gap-3">
                    <div className="p-3.5 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60">
                      <div className="text-[11px] text-[#7d8da1] font-medium">委托本金</div>
                      <div className="text-base font-extrabold font-mono text-[#363949] dark:text-white mt-0.5">
                        ${stakeVal.toFixed(2)} USDC
                      </div>
                    </div>
                    <div className="p-3.5 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60">
                      <div className="text-[11px] text-[#7d8da1] font-medium">申报价格 / 撮合价格</div>
                      <div className="text-sm font-extrabold font-mono text-[#6c9bcf] mt-0.5">
                        ${quotePrice.toFixed(3)} → ${fillPrice.toFixed(3)}
                      </div>
                    </div>
                    <div className="p-3.5 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60">
                      <div className="text-[11px] text-[#7d8da1] font-medium">成交份额</div>
                      <div className="text-base font-extrabold font-mono text-[#363949] dark:text-white mt-0.5">
                        {sharesVal.toFixed(2)} 份
                      </div>
                    </div>
                    <div className="p-3.5 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60">
                      <div className="text-[11px] text-[#7d8da1] font-medium">撮合状态</div>
                      <div className="text-xs font-bold text-[#1b9c85] mt-1 flex items-center gap-1.5">
                        <CheckCircle className="w-3.5 h-3.5 text-[#1b9c85]" />
                        {ord.status === 'FILLED' ? '撮合成交完成' : ord.status}
                      </div>
                    </div>
                  </div>

                  <div className="p-4 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200/60 dark:border-slate-800/60 space-y-2.5 font-mono">
                    <div className="flex items-center justify-between text-[#7d8da1]">
                      <span>委托单号</span>
                      <span className="text-[#363949] dark:text-slate-200 font-bold">{ord.order_id}</span>
                    </div>
                    <div className="flex items-center justify-between text-[#7d8da1]">
                      <span>市场合约</span>
                      <span className="text-[#363949] dark:text-slate-200 truncate max-w-[220px]" title={ord.market_id}>{ord.market_id}</span>
                    </div>
                    <div className="flex items-center justify-between text-[#7d8da1]">
                      <span>驱动信号</span>
                      <span className="text-[#363949] dark:text-slate-200">{ord.signal_id || 'manual_trigger'}</span>
                    </div>
                    <div className="flex items-center justify-between text-[#7d8da1]">
                      <span>手续费与滑点</span>
                      <span className="text-[#363949] dark:text-slate-200">费 ${ord.fee?.toFixed(4) || '0.0000'} / 滑 ${ord.slippage?.toFixed(4) || '0.0000'}</span>
                    </div>
                    <div className="flex items-center justify-between text-[#7d8da1]">
                      <span>委托时间</span>
                      <span className="text-[#363949] dark:text-slate-200">{formatFullTime(ord.timestamp_ms)}</span>
                    </div>
                  </div>
                </div>
              );
            })()}

            {/* Modal Footer */}
            <div className="pt-2 flex justify-end">
              <button
                onClick={() => setSelectedItem(null)}
                className="w-full py-2.5 rounded-xl bg-[#6c9bcf] text-white text-xs font-bold hover:brightness-105 transition-all cursor-pointer shadow-md shadow-[#6c9bcf]/20"
              >
                关闭审计详情
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
