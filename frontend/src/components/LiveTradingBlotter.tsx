import React, { useState, useEffect } from 'react';
import {
  ShieldAlert,
  OctagonAlert,
  RefreshCw,
  Wallet,
  Clock,
  ArrowUpRight,
  ArrowDownRight,
  CheckCircle2,
  XCircle,
  AlertTriangle,
} from 'lucide-react';
import { api } from '../services/api';
import { RealOrder, PolymarketAccountPublic } from '../types';

interface LiveTradingBlotterProps {
  activeAccount: PolymarketAccountPublic | null;
  onOpenAccountManager: () => void;
}

export const LiveTradingBlotter: React.FC<LiveTradingBlotterProps> = ({
  activeAccount,
  onOpenAccountManager,
}) => {
  const [orders, setOrders] = useState<RealOrder[]>([]);
  const [loading, setLoading] = useState(false);
  const [haltFeedback, setHaltFeedback] = useState<string | null>(null);
  const [isHalting, setIsHalting] = useState(false);

  const loadRealOrders = async () => {
    try {
      setLoading(true);
      const list = await api.getRealOrders(30);
      setOrders(list);
    } catch (err) {
      // ignore
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadRealOrders();
    const interval = setInterval(loadRealOrders, 2500);
    return () => clearInterval(interval);
  }, []);

  const handleEmergencyHalt = async () => {
    if (
      !window.confirm(
        '⚠️ 紧急熔断确认：将立即撤销所有在途订单并强制切回模拟盘（Paper Trading）！是否执行？'
      )
    ) {
      return;
    }

    try {
      setIsHalting(true);
      const res = await api.emergencyHalt();
      setHaltFeedback(`紧急熔断已触发：${res.message}`);
      await loadRealOrders();
      setTimeout(() => setHaltFeedback(null), 6000);
    } catch (err: any) {
      setHaltFeedback(`熔断失败: ${err.message || '未知错误'}`);
    } finally {
      setIsHalting(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Top Banner: Active Account & Real Trading Safeguards */}
      <div className="asmr-card p-6 lg:p-8">
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
          <div className="flex items-center gap-4">
            <div className="p-3 rounded-2xl bg-[#ff0060]/10 text-[#ff0060] border border-[#ff0060]/20">
              <ShieldAlert className="w-6 h-6" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-base font-extrabold text-[#363949] dark:text-white tracking-wide">
                  Polymarket 实盘交易监控 (CLOB Live Execution)
                </h3>
                <span className="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-[#ff0060]/15 text-[#ff0060] border border-[#ff0060]/30 font-mono">
                  LIVE CLOB
                </span>
              </div>
              <p className="text-xs text-[#7d8da1] dark:text-slate-400 mt-1">
                订单直连 Polymarket L2 订单簿，单笔下注受 $10.00 Mode B 资金硬顶与连续亏损熔断保护。
              </p>
            </div>
          </div>

          {/* Account & Emergency Controls */}
          <div className="flex flex-wrap items-center gap-3">
            {activeAccount ? (
              <div
                onClick={onOpenAccountManager}
                className="flex items-center gap-3 px-4 py-2.5 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-200 dark:border-slate-800 hover:border-[#6c9bcf] cursor-pointer transition-all"
              >
                <Wallet className="w-4 h-4 text-[#6c9bcf]" />
                <div className="text-left font-mono">
                  <div className="text-[10px] text-[#7d8da1]">当前活跃实盘账户</div>
                  <div className="text-xs font-bold text-[#363949] dark:text-white flex items-center gap-2">
                    <span>{activeAccount.label}</span>
                    <span className="text-[#1b9c85] font-bold">
                      ${activeAccount.balance_usdc.toFixed(2)} USDC
                    </span>
                  </div>
                </div>
              </div>
            ) : (
              <button
                onClick={onOpenAccountManager}
                className="flex items-center gap-2 px-4 py-2.5 rounded-2xl bg-[#f7d154]/20 border border-amber-300 text-amber-700 dark:text-amber-300 text-xs font-bold hover:brightness-105 cursor-pointer"
              >
                <AlertTriangle className="w-4 h-4 text-amber-500" />
                <span>未激活实盘账户 (点击绑定)</span>
              </button>
            )}

            {/* Emergency Halt Button */}
            <button
              disabled={isHalting}
              onClick={handleEmergencyHalt}
              className="flex items-center gap-2 px-4 py-2.5 rounded-2xl bg-[#ff0060] text-white text-xs font-bold shadow-md shadow-[#ff0060]/20 hover:brightness-110 transition-all cursor-pointer disabled:opacity-50"
            >
              <OctagonAlert className="w-4 h-4" />
              <span>{isHalting ? '熔断撤单中...' : '紧急熔断 (KILL SWITCH)'}</span>
            </button>
          </div>
        </div>

        {haltFeedback && (
          <div className="mt-4 p-3.5 rounded-2xl bg-[#ff0060]/10 border border-[#ff0060]/20 text-[#ff0060] text-xs font-mono animate-fadeIn flex items-center gap-2">
            <OctagonAlert className="w-4 h-4 text-[#ff0060] shrink-0" />
            <span>{haltFeedback}</span>
          </div>
        )}
      </div>

      {/* Real Orders Table */}
      <div className="asmr-card p-6 lg:p-8">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h4 className="text-base font-extrabold text-[#363949] dark:text-white tracking-wide flex items-center gap-2">
              <Clock className="w-4 h-4 text-[#6c9bcf]" />
              <span>Polymarket 实盘订单成交审计 (Real Orders Blotter)</span>
            </h4>
            <p className="text-xs text-[#7d8da1] dark:text-slate-400 mt-0.5">
              展示经由 HMAC-SHA256 签名提交至 CLOB 的真实撮合委托
            </p>
          </div>

          <button
            onClick={loadRealOrders}
            className="p-2 rounded-xl bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 text-[#7d8da1] hover:text-[#363949] dark:hover:text-white transition-all cursor-pointer"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
          </button>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-slate-100 dark:border-slate-800 text-[#7d8da1]">
                <th className="pb-3 font-semibold">本地订单号</th>
                <th className="pb-3 font-semibold">标的 / 方向</th>
                <th className="pb-3 font-semibold">类型</th>
                <th className="pb-3 font-semibold">委托价</th>
                <th className="pb-3 font-semibold">委托数量 / 成交</th>
                <th className="pb-3 font-semibold">状态</th>
                <th className="pb-3 font-semibold">手续费</th>
                <th className="pb-3 font-semibold">实盘盈亏</th>
                <th className="pb-3 font-semibold text-right">时间</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800/60">
              {orders.length > 0 ? (
                orders.map((o) => {
                  const isUp = o.outcome.toUpperCase() === 'UP';
                  const isFilled = o.status.toLowerCase() === 'filled';
                  const isRejected = o.status.toLowerCase() === 'rejected';

                  return (
                    <tr
                      key={o.id}
                      className="hover:bg-slate-50 dark:hover:bg-slate-800/40 transition-colors"
                    >
                      <td className="py-3 text-[#363949] dark:text-white font-semibold">
                        <div className="truncate max-w-[120px]" title={o.id}>
                          {o.id}
                        </div>
                        {o.clob_order_id && (
                          <div
                            className="text-[10px] text-[#7d8da1] truncate max-w-[120px]"
                            title={o.clob_order_id}
                          >
                            CLOB: {o.clob_order_id.slice(0, 8)}...
                          </div>
                        )}
                      </td>
                      <td className="py-3">
                        <div className="flex items-center gap-1.5 font-bold">
                          <span className="text-[#363949] dark:text-white">{o.asset}</span>
                          <span
                            className={`flex items-center gap-0.5 px-2 py-0.5 rounded text-[10px] ${
                              isUp
                                ? 'bg-[#1b9c85]/15 text-[#1b9c85]'
                                : 'bg-[#ff0060]/15 text-[#ff0060]'
                            }`}
                          >
                            {isUp ? (
                              <ArrowUpRight className="w-3 h-3" />
                            ) : (
                              <ArrowDownRight className="w-3 h-3" />
                            )}
                            {o.outcome}
                          </span>
                        </div>
                      </td>
                      <td className="py-3 text-[#7d8da1] uppercase">{o.order_type}</td>
                      <td className="py-3 text-[#363949] dark:text-white font-bold font-mono">
                        ${o.price.toFixed(3)}
                      </td>
                      <td className="py-3 text-[#7d8da1] font-mono">
                        {o.filled_size.toFixed(1)} / {o.size.toFixed(1)}
                      </td>
                      <td className="py-3">
                        <span
                          className={`inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[10px] font-extrabold ${
                            isFilled
                              ? 'bg-[#1b9c85]/15 text-[#1b9c85]'
                              : isRejected
                              ? 'bg-[#ff0060]/15 text-[#ff0060]'
                              : 'bg-[#f7d154]/20 text-amber-600'
                          }`}
                        >
                          {isFilled && <CheckCircle2 className="w-3 h-3" />}
                          {isRejected && <XCircle className="w-3 h-3" />}
                          {o.status.toUpperCase()}
                        </span>
                      </td>
                      <td className="py-3 text-[#7d8da1] font-mono">${o.fee.toFixed(3)}</td>
                      <td className="py-3 font-mono font-bold">
                        {o.pnl !== undefined && o.pnl !== null ? (
                          <span
                            className={o.pnl >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}
                          >
                            {o.pnl >= 0 ? '+' : ''}${o.pnl.toFixed(2)}
                          </span>
                        ) : (
                          <span className="text-[#7d8da1]">-</span>
                        )}
                      </td>
                      <td className="py-3 text-right text-[#7d8da1] font-mono">
                        {new Date(o.created_at).toLocaleTimeString()}
                      </td>
                    </tr>
                  );
                })
              ) : (
                <tr>
                  <td colSpan={9} className="py-8 text-center text-[#7d8da1]">
                    暂无实盘订单记录。切换至实盘模式并激活账户后，策略信号将自动下单撮合。
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
