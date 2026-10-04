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
  FileText,
  X,
  ExternalLink,
  Copy,
  Check,
  Info,
  KeyRound,
  ShieldCheck,
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
  const [selectedOrder, setSelectedOrder] = useState<RealOrder | null>(null);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const loadRealOrders = async () => {
    try {
      setLoading(true);
      const list = await api.getRealOrders(50);
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

  const handleCopy = (text: string, key: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

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
                订单直连 Polymarket L2 订单簿，单笔下注受 $10.00 Mode B 资金硬顶与连续亏损熔断保护。点击任一行可展开错误诊断详情。
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
                      ${(typeof activeAccount.balance_usdc === 'number' && !isNaN(activeAccount.balance_usdc) ? activeAccount.balance_usdc : 0).toFixed(2)} USDC
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
              展示经由 HMAC-SHA256 签名提交至 CLOB 的真实撮合委托（点击任意订单查看诊断原因）
            </p>
          </div>

          <button
            onClick={loadRealOrders}
            className="p-2 rounded-xl bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 text-[#7d8da1] hover:text-[#363949] dark:hover:text-white transition-all cursor-pointer"
            title="刷新实盘订单"
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
                <th className="pb-3 font-semibold">状态与拒单原因</th>
                <th className="pb-3 font-semibold">手续费</th>
                <th className="pb-3 font-semibold">实盘盈亏</th>
                <th className="pb-3 font-semibold">时间</th>
                <th className="pb-3 font-semibold text-right">操作</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800/60">
              {orders.length > 0 ? (
                orders.map((o) => {
                  const isUp = o.outcome.toUpperCase() === 'UP';
                  const isFilled = o.status.toLowerCase() === 'filled';
                  const isFailed = o.status.toLowerCase() === 'failed' || o.status.toLowerCase() === 'rejected';

                  return (
                    <tr
                      key={o.id}
                      onClick={() => setSelectedOrder(o)}
                      className="hover:bg-slate-50 dark:hover:bg-slate-800/40 transition-colors cursor-pointer group"
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
                        ${(typeof o.price === 'number' && !isNaN(o.price) ? o.price : 0).toFixed(3)}
                      </td>

                      <td className="py-3 text-[#7d8da1] font-mono">
                        {(typeof o.filled_size === 'number' && !isNaN(o.filled_size) ? o.filled_size : 0).toFixed(1)} / {(typeof o.size === 'number' && !isNaN(o.size) ? o.size : 0).toFixed(1)}
                      </td>

                      <td className="py-3 max-w-[220px]">
                        <div className="flex flex-col gap-1 items-start">
                          <span
                            className={`inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[10px] font-extrabold ${
                              isFilled
                                ? 'bg-[#1b9c85]/15 text-[#1b9c85]'
                                : isFailed
                                ? 'bg-rose-100 dark:bg-rose-950 text-[#ff0060] border border-rose-200 dark:border-rose-900/60'
                                : 'bg-[#f7d154]/20 text-amber-600'
                            }`}
                          >
                            {isFilled && <CheckCircle2 className="w-3 h-3" />}
                            {isFailed && <XCircle className="w-3 h-3" />}
                            {o.status.toUpperCase()}
                          </span>

                          {/* Error snippet if failed */}
                          {isFailed && o.error_message && (
                            <span
                              className="text-[10px] text-rose-600 dark:text-rose-400 truncate max-w-[200px] font-sans"
                              title={o.error_message}
                            >
                              {o.error_message}
                            </span>
                          )}
                        </div>
                      </td>

                      <td className="py-3 text-[#7d8da1] font-mono">
                        ${(typeof o.fee === 'number' && !isNaN(o.fee) ? o.fee : 0).toFixed(3)}
                      </td>

                      <td className="py-3 font-mono font-bold">
                        {typeof o.pnl === 'number' && !isNaN(o.pnl) ? (
                          <span className={o.pnl >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}>
                            {o.pnl >= 0 ? '+' : ''}${o.pnl.toFixed(2)}
                          </span>
                        ) : (
                          <span className="text-[#7d8da1]">-</span>
                        )}
                      </td>

                      <td className="py-3 text-[#7d8da1] font-mono">
                        {new Date(o.created_at).toLocaleTimeString([], { hour12: false })}
                      </td>

                      <td className="py-3 text-right">
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            setSelectedOrder(o);
                          }}
                          className="px-2.5 py-1 text-[11px] font-semibold rounded-lg bg-indigo-50 dark:bg-indigo-950/60 text-[var(--color-primary)] border border-indigo-200 dark:border-indigo-800/80 hover:bg-indigo-100 dark:hover:bg-indigo-900 transition cursor-pointer"
                        >
                          查看详情
                        </button>
                      </td>
                    </tr>
                  );
                })
              ) : (
                <tr>
                  <td colSpan={10} className="py-8 text-center text-[#7d8da1]">
                    暂无实盘订单记录。切换至实盘模式并激活账户后，策略信号将自动下单撮合。
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Real Order Detail & Diagnostic Modal */}
      {selectedOrder && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-sm animate-fade-in">
          <div className="bg-white dark:bg-[#202528] rounded-2xl w-full max-w-2xl shadow-2xl border border-slate-100 dark:border-slate-800 overflow-hidden transform transition-all max-h-[92vh] flex flex-col">
            {/* Modal Header */}
            <div className="p-5 border-b border-slate-100 dark:border-slate-800/80 flex items-center justify-between shrink-0">
              <div className="flex items-center gap-3">
                <div
                  className={`w-10 h-10 rounded-xl flex items-center justify-center shadow-sm ${
                    selectedOrder.status.toLowerCase() === 'filled'
                      ? 'bg-emerald-50 dark:bg-emerald-950/40 text-emerald-500'
                      : 'bg-rose-50 dark:bg-rose-950/40 text-[#ff0060]'
                  }`}
                >
                  <FileText className="w-5 h-5" />
                </div>
                <div>
                  <div className="flex items-center gap-2">
                    <h3 className="text-base font-extrabold text-[#363949] dark:text-white">
                      实盘订单审计与诊断详情
                    </h3>
                    <span
                      className={`px-2 py-0.5 text-[10px] font-extrabold rounded-full font-mono ${
                        selectedOrder.status.toLowerCase() === 'filled'
                          ? 'bg-[#1b9c85]/15 text-[#1b9c85] border border-[#1b9c85]/30'
                          : 'bg-[#ff0060]/15 text-[#ff0060] border border-[#ff0060]/30'
                      }`}
                    >
                      {selectedOrder.status.toUpperCase()}
                    </span>
                  </div>
                  <p className="text-xs text-[#7d8da1] font-mono mt-0.5">
                    ID: {selectedOrder.id}
                  </p>
                </div>
              </div>

              <button
                onClick={() => setSelectedOrder(null)}
                className="p-2 text-[#7d8da1] hover:text-[#363949] dark:hover:text-white rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 transition cursor-pointer"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            {/* Modal Body */}
            <div className="p-6 space-y-4 overflow-y-auto">
              {/* If FAILED: Prominent Diagnostic Banner */}
              {(selectedOrder.status.toLowerCase() === 'failed' || selectedOrder.status.toLowerCase() === 'rejected') && (
                <div className="p-4 bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900/60 rounded-xl space-y-3">
                  <div className="flex items-start gap-2.5 text-rose-800 dark:text-rose-200">
                    <OctagonAlert className="w-5 h-5 shrink-0 text-[#ff0060] mt-0.5" />
                    <div>
                      <h4 className="text-xs font-bold text-[#ff0060]">
                        Polymarket CLOB 撮合拒绝原因 (Rejection Reason)
                      </h4>
                      <p className="text-xs font-mono font-bold mt-1 break-all bg-white/70 dark:bg-black/30 p-2 rounded-lg border border-rose-200 dark:border-rose-900/40">
                        {selectedOrder.error_message || 'Polymarket CLOB 拒绝了该笔委托 (无详细错误回包)'}
                      </p>
                    </div>
                  </div>

                  {/* Root cause analysis breakdown */}
                  <div className="pt-2 border-t border-rose-200/60 dark:border-rose-900/40 text-xs text-rose-700 dark:text-rose-300 space-y-2">
                    <p className="font-bold">💡 为什么会买入失败？主要排查要点：</p>
                    <ul className="list-disc pl-5 space-y-1.5 text-[11px] leading-relaxed text-slate-600 dark:text-slate-300">
                      <li>
                        <strong>1. 标的 Token ID 映射错误</strong>：Polymarket 链上只接受真实的 CTF ERC1155 Token ID（一串256位大整数数字）。如果当前轮次是本地高频模拟生成的标的 ID（例如 <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded font-mono">{selectedOrder.token_id}</code>），Polymarket 服务器会返回 <code className="text-rose-600 font-mono">invalid tokenID</code> 或 <code className="text-rose-600 font-mono">market not found</code>。
                      </li>
                      <li>
                        <strong>2. 缺少以太坊 EIP-712 签名</strong>：Polymarket CLOB 的 API Key（L2）仅用于行情查询与撤单；而在真实下单（POST /order）时，Polymarket 智能合约要求报文内必须包含由钱包私钥签署的 <strong>EIP-712 结构化签名</strong>（或者通过 Proxy 代理合约预先授权签名），仅靠 API Key 无法由交易所代为扣划真实链上资产。
                      </li>
                      <li>
                        <strong>3. Polygon USDC 余额与授权 (Allowance)</strong>：Polymarket CTF 交易所要求钱包已向交易所主合约执行过 USDC <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded font-mono">approve</code> 授权，且钱包中须有足够的 Polygon 原生 USDC。
                      </li>
                    </ul>
                  </div>
                </div>
              )}

              {/* Order Key Parameters Grid */}
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                <div className="p-3 bg-slate-50 dark:bg-[#181a1e] rounded-xl border border-slate-200 dark:border-slate-800">
                  <span className="text-[10px] text-[#7d8da1] block font-semibold">标的 / 方向</span>
                  <span className="text-sm font-bold text-[#363949] dark:text-white flex items-center gap-1 mt-0.5">
                    {selectedOrder.asset}{' '}
                    <span
                      className={`text-xs px-1.5 py-0.2 rounded font-extrabold ${
                        selectedOrder.outcome.toUpperCase() === 'UP'
                          ? 'text-[#1b9c85]'
                          : 'text-[#ff0060]'
                      }`}
                    >
                      {selectedOrder.outcome}
                    </span>
                  </span>
                </div>

                <div className="p-3 bg-slate-50 dark:bg-[#181a1e] rounded-xl border border-slate-200 dark:border-slate-800">
                  <span className="text-[10px] text-[#7d8da1] block font-semibold">委托价格</span>
                  <span className="text-sm font-bold font-mono text-[#363949] dark:text-white mt-0.5 block">
                    ${selectedOrder.price.toFixed(3)}
                  </span>
                </div>

                <div className="p-3 bg-slate-50 dark:bg-[#181a1e] rounded-xl border border-slate-200 dark:border-slate-800">
                  <span className="text-[10px] text-[#7d8da1] block font-semibold">委托 / 成交量</span>
                  <span className="text-sm font-bold font-mono text-[#363949] dark:text-white mt-0.5 block">
                    {selectedOrder.filled_size.toFixed(1)} / {selectedOrder.size.toFixed(1)} 股
                  </span>
                </div>

                <div className="p-3 bg-slate-50 dark:bg-[#181a1e] rounded-xl border border-slate-200 dark:border-slate-800">
                  <span className="text-[10px] text-[#7d8da1] block font-semibold">委托类型</span>
                  <span className="text-sm font-bold font-mono text-[var(--color-primary)] mt-0.5 block">
                    {selectedOrder.order_type}
                  </span>
                </div>
              </div>

              {/* Technical Audit Properties */}
              <div className="p-3.5 bg-slate-50 dark:bg-[#181a1e] rounded-xl border border-slate-200 dark:border-slate-800 space-y-2.5 text-xs font-mono">
                <div className="flex items-center justify-between">
                  <span className="text-[#7d8da1]">本地订单号:</span>
                  <div className="flex items-center gap-1.5 text-[#363949] dark:text-white">
                    <span>{selectedOrder.id}</span>
                    <button
                      onClick={() => handleCopy(selectedOrder.id, 'id')}
                      className="p-1 hover:text-[#1b9c85] transition"
                      title="复制订单号"
                    >
                      {copiedKey === 'id' ? <Check className="w-3.5 h-3.5 text-emerald-500" /> : <Copy className="w-3.5 h-3.5" />}
                    </button>
                  </div>
                </div>

                {selectedOrder.clob_order_id && (
                  <div className="flex items-center justify-between">
                    <span className="text-[#7d8da1]">CLOB 远端订单号:</span>
                    <span className="text-[#363949] dark:text-white font-bold">{selectedOrder.clob_order_id}</span>
                  </div>
                )}

                <div className="flex items-center justify-between">
                  <span className="text-[#7d8da1]">Token ID:</span>
                  <div className="flex items-center gap-1.5 text-[#363949] dark:text-white">
                    <span className="truncate max-w-[280px]" title={selectedOrder.token_id}>
                      {selectedOrder.token_id}
                    </span>
                    <button
                      onClick={() => handleCopy(selectedOrder.token_id, 'token')}
                      className="p-1 hover:text-[#1b9c85] transition"
                      title="复制 Token ID"
                    >
                      {copiedKey === 'token' ? <Check className="w-3.5 h-3.5 text-emerald-500" /> : <Copy className="w-3.5 h-3.5" />}
                    </button>
                  </div>
                </div>

                <div className="flex items-center justify-between">
                  <span className="text-[#7d8da1]">Market ID:</span>
                  <span className="text-[#363949] dark:text-white">{selectedOrder.market_id}</span>
                </div>

                <div className="flex items-center justify-between">
                  <span className="text-[#7d8da1]">下单时间:</span>
                  <span className="text-[#363949] dark:text-white">
                    {new Date(selectedOrder.created_at).toLocaleString('zh-CN', { hour12: false })}
                  </span>
                </div>

                <div className="flex items-center justify-between">
                  <span className="text-[#7d8da1]">预估手续费:</span>
                  <span className="text-[#363949] dark:text-white">${selectedOrder.fee.toFixed(3)} USDC</span>
                </div>

                {activeAccount && (
                  <div className="flex items-center justify-between border-t border-slate-200 dark:border-slate-800 pt-2">
                    <span className="text-[#7d8da1]">执行账户:</span>
                    <span className="text-[#363949] dark:text-white font-bold">
                      {activeAccount.label} ({activeAccount.wallet_address.slice(0, 6)}...{activeAccount.wallet_address.slice(-4)})
                    </span>
                  </div>
                )}
              </div>

              {/* Action Buttons */}
              <div className="flex justify-end pt-2">
                <button
                  type="button"
                  onClick={() => setSelectedOrder(null)}
                  className="px-5 py-2.5 bg-slate-100 hover:bg-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700 text-[#363949] dark:text-white font-bold rounded-xl transition cursor-pointer text-xs"
                >
                  关闭详情
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
