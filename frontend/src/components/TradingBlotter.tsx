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
                    <td className="py-3.5 text-right pr-3 font-mono text-[#6c9bcf] hover:underline cursor-pointer">
                      详情
                    </td>
                  </tr>
                ))
              ) : (
                <tr>
                  <td colSpan={5} className="py-8 text-center text-[#7d8da1] font-mono">
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
                      <td className="py-3.5 text-right pr-3 font-mono text-[#6c9bcf] hover:underline cursor-pointer">
                        详情
                      </td>
                    </tr>
                  );
                })
              ) : (
                <tr>
                  <td colSpan={5} className="py-8 text-center text-[#7d8da1] font-mono">
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
                    <td className="py-3.5 text-right pr-3 font-mono text-[#6c9bcf] hover:underline cursor-pointer">
                      详情
                    </td>
                  </tr>
                ))
              ) : (
                <tr>
                  <td colSpan={5} className="py-8 text-center text-[#7d8da1] font-mono">
                    暂无委托记录。
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
          {showAll ? '收起列表' : '展开查看全部记录'}
        </button>
      </div>
    </div>
  );
};
