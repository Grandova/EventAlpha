import React, { useState } from 'react';
import { Play, RotateCcw, TrendingUp, Award, AlertTriangle, ShieldCheck } from 'lucide-react';
import { Asset, BacktestRequest, BacktestResult } from '../types';
import { api } from '../services/api';

interface BacktestConsoleProps {
  defaultAsset: Asset;
}

export const BacktestConsole: React.FC<BacktestConsoleProps> = ({ defaultAsset }) => {
  const [asset, setAsset] = useState<Asset>(defaultAsset);
  const [mode, setMode] = useState<'capital_recovery' | 'profit_isolation'>('capital_recovery');
  const [initialBankroll, setInitialBankroll] = useState(10.0);
  const [bankrollCap, setBankrollCap] = useState(10.0);
  const [stake, setStake] = useState(1.0);
  const [minProb, setMinProb] = useState(0.70);
  const [minNetEdge, setMinNetEdge] = useState(0.05);
  const [feeRate, setFeeRate] = useState(0.012);
  const [slippageRate, setSlippageRate] = useState(0.005);

  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<BacktestResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleRunBacktest = async () => {
    setLoading(true);
    setError(null);
    try {
      const req: BacktestRequest = {
        assets: [asset],
        initial_bankroll: initialBankroll,
        bankroll_cap: bankrollCap,
        mode,
        stake,
        min_prob: minProb,
        min_net_edge: minNetEdge,
        fee_rate: feeRate,
        slippage_rate: slippageRate,
      };
      const res = await api.runBacktest(req);
      setResult(res);
    } catch (err: any) {
      setError(err?.message || 'Failed to run backtest');
    } finally {
      setLoading(false);
    }
  };

  const metrics = result?.metrics;
  const equityCurve = result?.equity_curve ?? [];

  // Build SVG path for equity curve
  const renderEquitySvg = () => {
    if (equityCurve.length < 2) return null;

    const width = 800;
    const height = 240;
    const padding = 30;

    let minVal = Math.min(...equityCurve.map((p) => Math.min(p.active_bankroll, p.total_equity)));
    let maxVal = Math.max(...equityCurve.map((p) => Math.max(p.total_equity, p.active_bankroll + p.locked_profit)));
    minVal = Math.floor(Math.min(minVal, initialBankroll) * 0.95);
    maxVal = Math.ceil(Math.max(maxVal, initialBankroll) * 1.05);

    const getX = (i: number) => padding + (i / (equityCurve.length - 1)) * (width - padding * 2);
    const getY = (val: number) => height - padding - ((val - minVal) / (maxVal - minVal || 1)) * (height - padding * 2);

    const activePoints = equityCurve.map((p, i) => `${getX(i)},${getY(p.active_bankroll)}`).join(' ');
    const totalPoints = equityCurve.map((p, i) => `${getX(i)},${getY(p.total_equity)}`).join(' ');

    return (
      <svg viewBox={`0 0 ${width} ${height}`} className="w-full h-56 bg-slate-950/60 rounded-lg border border-slate-800">
        {/* Horizontal grid lines */}
        {[0, 0.25, 0.5, 0.75, 1].map((ratio) => {
          const val = minVal + ratio * (maxVal - minVal);
          const y = getY(val);
          return (
            <g key={ratio}>
              <line x1={padding} y1={y} x2={width - padding} y2={y} stroke="#1e293b" strokeDasharray="3 3" />
              <text x={padding - 5} y={y + 3} textAnchor="end" fill="#64748b" fontSize="9" fontFamily="monospace">
                ${val.toFixed(1)}
              </text>
            </g>
          );
        })}

        {/* Total Equity line (Purple) */}
        <polyline fill="none" stroke="#a855f7" strokeWidth="2.5" points={totalPoints} />

        {/* Active Bankroll line (Cyan) */}
        <polyline fill="none" stroke="#06b6d4" strokeWidth="2" strokeDasharray="4 2" points={activePoints} />

        {/* Legend */}
        <g transform="translate(40, 20)">
          <line x1="0" y1="0" x2="20" y2="0" stroke="#a855f7" strokeWidth="2.5" />
          <text x="25" y="4" fill="#a855f7" fontSize="10" fontWeight="bold">Total Equity</text>

          <line x1="120" y1="0" x2="140" y2="0" stroke="#06b6d4" strokeWidth="2" strokeDasharray="4 2" />
          <text x="145" y="4" fill="#06b6d4" fontSize="10" fontWeight="bold">Active Bankroll</text>
        </g>
      </svg>
    );
  };

  return (
    <div className="quant-card p-4 mb-4">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <TrendingUp className="h-4 w-4 text-cyan-400" />
          <h2 className="text-xs font-bold text-slate-400 uppercase tracking-wider">
            高性能事件驱动量化回测引擎
          </h2>
        </div>
        <span className="text-[10px] text-slate-500 font-mono">
          严格防未来函数 &bull; 真实订单簿深度穿透与滑点 &bull; 资金模式 A/B 模拟
        </span>
      </div>

      {/* Backtest Config Panel */}
      <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 mb-4">
        <div className="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-8 gap-3 text-xs mb-3">
          <div>
            <label className="text-[10px] text-slate-400 block mb-1">标的资产</label>
            <select
              value={asset}
              onChange={(e) => setAsset(e.target.value as Asset)}
              className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 text-white font-mono"
            >
              <option value="BTC">BTC</option>
              <option value="ETH">ETH</option>
              <option value="SOL">SOL</option>
            </select>
          </div>

          <div>
            <label className="text-[10px] text-slate-400 block mb-1">资金管理模式</label>
            <select
              value={mode}
              onChange={(e) => setMode(e.target.value as any)}
              className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 text-white font-mono"
            >
              <option value="capital_recovery">模式 B (本金回收)</option>
              <option value="profit_isolation">模式 A (利润隔离)</option>
            </select>
          </div>

          <div>
            <label className="text-[10px] text-slate-400 block mb-1">初始资金 ($)</label>
            <input
              type="number"
              step="1"
              value={initialBankroll}
              onChange={(e) => setInitialBankroll(parseFloat(e.target.value))}
              className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 text-white font-mono"
            />
          </div>

          <div>
            <label className="text-[10px] text-slate-400 block mb-1">资金硬顶 ($)</label>
            <input
              type="number"
              step="1"
              value={bankrollCap}
              onChange={(e) => setBankrollCap(parseFloat(e.target.value))}
              className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 text-white font-mono"
            />
          </div>

          <div>
            <label className="text-[10px] text-slate-400 block mb-1">单笔注额 ($)</label>
            <input
              type="number"
              step="0.1"
              value={stake}
              onChange={(e) => setStake(parseFloat(e.target.value))}
              className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 text-white font-mono"
            />
          </div>

          <div>
            <label className="text-[10px] text-slate-400 block mb-1">胜率硬门槛</label>
            <input
              type="number"
              step="0.05"
              value={minProb}
              onChange={(e) => setMinProb(parseFloat(e.target.value))}
              className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 text-white font-mono"
            />
          </div>

          <div>
            <label className="text-[10px] text-slate-400 block mb-1">净期望门槛</label>
            <input
              type="number"
              step="0.01"
              value={minNetEdge}
              onChange={(e) => setMinNetEdge(parseFloat(e.target.value))}
              className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 text-white font-mono"
            />
          </div>

          <div>
            <label className="text-[10px] text-slate-400 block mb-1">综合手续费率</label>
            <input
              type="number"
              step="0.001"
              value={feeRate}
              onChange={(e) => setFeeRate(parseFloat(e.target.value))}
              className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 text-white font-mono"
            />
          </div>
        </div>

        <div className="flex items-center justify-between pt-2 border-t border-slate-800">
          <span className="text-xs text-slate-500 font-mono">
            完整模拟逐档吃单滑点深度、手续费扣除与模式 B 利润隔离机制。
          </span>
          <button
            onClick={handleRunBacktest}
            disabled={loading}
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-bold text-xs shadow-lg shadow-cyan-500/20 transition-all disabled:opacity-50 cursor-pointer"
          >
            <Play className="h-3.5 w-3.5 fill-slate-950" />
            {loading ? '正在模拟推演中...' : '执行事件回测'}
          </button>
        </div>
      </div>

      {error && (
        <div className="mb-4 p-3 rounded-lg bg-rose-950/40 border border-rose-800/80 text-rose-400 text-xs font-mono">
          {error}
        </div>
      )}

      {/* Backtest Results Display */}
      {metrics && (
        <div>
          {/* Key Quantitative Metrics Grid */}
          <div className="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-8 gap-2.5 mb-4">
            <div className="bg-slate-900 p-2.5 rounded border border-slate-800">
              <span className="text-[10px] text-slate-400 block">夏普比率 (Sharpe)</span>
              <span className="text-lg font-bold font-mono-num text-cyan-400">
                {(typeof metrics.sharpe_ratio === 'number' && !isNaN(metrics.sharpe_ratio) ? metrics.sharpe_ratio : 0).toFixed(2)}
              </span>
            </div>
            <div className="bg-slate-900 p-2.5 rounded border border-slate-800">
              <span className="text-[10px] text-slate-400 block">索提诺比率 (Sortino)</span>
              <span className="text-lg font-bold font-mono-num text-emerald-400">
                {(typeof metrics.sortino_ratio === 'number' && !isNaN(metrics.sortino_ratio) ? metrics.sortino_ratio : 0).toFixed(2)}
              </span>
            </div>
            <div className="bg-slate-900 p-2.5 rounded border border-slate-800">
              <span className="text-[10px] text-slate-400 block">卡尔玛比率 (Calmar)</span>
              <span className="text-lg font-bold font-mono-num text-purple-400">
                {(typeof metrics.calmar_ratio === 'number' && !isNaN(metrics.calmar_ratio) ? metrics.calmar_ratio : 0).toFixed(2)}
              </span>
            </div>
            <div className="bg-slate-900 p-2.5 rounded border border-slate-800">
              <span className="text-[10px] text-slate-400 block">最大历史回撤</span>
              <span className="text-lg font-bold font-mono-num text-rose-400">
                {((typeof metrics.max_drawdown_pct === 'number' && !isNaN(metrics.max_drawdown_pct) ? metrics.max_drawdown_pct : 0) * 100).toFixed(1)}% (${(typeof metrics.max_drawdown_usdc === 'number' && !isNaN(metrics.max_drawdown_usdc) ? metrics.max_drawdown_usdc : 0).toFixed(2)})
              </span>
            </div>
            <div className="bg-slate-900 p-2.5 rounded border border-slate-800">
              <span className="text-[10px] text-slate-400 block">综合胜率</span>
              <span className="text-lg font-bold font-mono-num text-emerald-400">
                {(typeof metrics.win_rate === 'number' && !isNaN(metrics.win_rate) ? metrics.win_rate : 0).toFixed(1)}%
              </span>
            </div>
            <div className="bg-slate-900 p-2.5 rounded border border-slate-800">
              <span className="text-[10px] text-slate-400 block">盈亏比 (Profit Factor)</span>
              <span className="text-lg font-bold font-mono-num text-white">
                {(typeof metrics.profit_factor === 'number' && !isNaN(metrics.profit_factor) ? metrics.profit_factor : 0).toFixed(2)}
              </span>
            </div>
            <div className="bg-slate-900 p-2.5 rounded border border-slate-800">
              <span className="text-[10px] text-slate-400 block">净盈亏金额</span>
              <span className={`text-lg font-bold font-mono-num ${metrics.net_pnl >= 0 ? 'text-emerald-400' : 'text-rose-400'}`}>
                {metrics.net_pnl >= 0 ? '+' : ''}${(typeof metrics.net_pnl === 'number' && !isNaN(metrics.net_pnl) ? metrics.net_pnl : 0).toFixed(2)}
              </span>
            </div>
            <div className="bg-slate-900 p-2.5 rounded border border-slate-800">
              <span className="text-[10px] text-slate-400 block">最终锁定利润</span>
              <span className="text-lg font-bold font-mono-num text-emerald-400">
                ${(typeof metrics.final_locked_profit === 'number' && !isNaN(metrics.final_locked_profit) ? metrics.final_locked_profit : 0).toFixed(2)}
              </span>
            </div>
          </div>

          {/* Interactive SVG Equity Curve */}
          <div className="mb-4">
            <h3 className="text-xs font-bold text-slate-300 mb-2 uppercase tracking-wide">
              资产净值动态演进曲线 (全真走势推演)
            </h3>
            {renderEquitySvg()}
          </div>
        </div>
      )}
    </div>
  );
};
