import React from 'react';
import { Cpu, Activity, Clock, Zap, BarChart } from 'lucide-react';
import { FeatureSnapshot, Asset } from '../types';

interface FeatureMonitorProps {
  asset: Asset;
  features: FeatureSnapshot | null;
}

export const FeatureMonitor: React.FC<FeatureMonitorProps> = ({
  asset,
  features,
}) => {
  if (!features) {
    return (
      <div className="quant-card p-8 text-center text-xs text-slate-500 font-mono">
        Loading real-time 37-dimensional feature vectors for {asset}...
      </div>
    );
  }

  const renderMetric = (label: string, val: number | string, unit = '', isPct = false, colorize = false) => {
    let numVal = typeof val === 'number' ? val : parseFloat(val);
    let displayVal = typeof val === 'number' ? (isPct ? (val * 100).toFixed(4) : val.toFixed(4)) : val;
    let colorClass = 'text-white';
    if (colorize && typeof val === 'number') {
      colorClass = numVal > 0 ? 'text-emerald-400' : numVal < 0 ? 'text-rose-400' : 'text-slate-300';
    }

    return (
      <div className="bg-slate-900/60 p-2.5 rounded border border-slate-800/80">
        <span className="text-[10px] text-slate-400 block truncate">{label}</span>
        <span className={`text-sm font-bold font-mono-num ${colorClass}`}>
          {colorize && numVal > 0 ? '+' : ''}
          {displayVal}
          {unit}
        </span>
      </div>
    );
  };

  return (
    <div className="quant-card p-4 mb-4">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Cpu className="h-4 w-4 text-cyan-400" />
          <h2 className="text-xs font-bold text-slate-400 uppercase tracking-wider">
            37-Dimensional Real-Time Feature Matrix &bull; {asset}
          </h2>
        </div>
        <span className="text-[10px] text-slate-500 font-mono">
          Last Calculation: {new Date(features.timestamp_ms).toLocaleTimeString()} &bull; 100ms Interval
        </span>
      </div>

      <div className="space-y-4">
        {/* Section 1: Multi-scale Returns & Velocity */}
        <div>
          <h3 className="text-xs font-bold text-slate-300 mb-2 flex items-center gap-1.5 uppercase tracking-wide">
            <Zap className="h-3.5 w-3.5 text-cyan-400" />
            Momentum, Multi-Scale Returns & Dynamics
          </h3>
          <div className="grid grid-cols-3 sm:grid-cols-6 lg:grid-cols-9 gap-2">
            {renderMetric('Return (1s)', features.return_1s, '%', true, true)}
            {renderMetric('Return (3s)', features.return_3s, '%', true, true)}
            {renderMetric('Return (5s)', features.return_5s, '%', true, true)}
            {renderMetric('Return (10s)', features.return_10s, '%', true, true)}
            {renderMetric('Return (30s)', features.return_30s, '%', true, true)}
            {renderMetric('Return (60s)', features.return_60s, '%', true, true)}
            {renderMetric('Velocity (5s)', features.velocity_5s, '', false, true)}
            {renderMetric('Velocity (15s)', features.velocity_15s, '', false, true)}
            {renderMetric('Acceleration', features.acceleration_5s_15s, '', false, true)}
          </div>
        </div>

        {/* Section 2: Realized Volatility */}
        <div>
          <h3 className="text-xs font-bold text-slate-300 mb-2 flex items-center gap-1.5 uppercase tracking-wide">
            <Activity className="h-3.5 w-3.5 text-purple-400" />
            Realized Volatility Multi-Horizon
          </h3>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
            {renderMetric('Realized Vol (5s)', features.realized_vol_5s, '%', true)}
            {renderMetric('Realized Vol (10s)', features.realized_vol_10s, '%', true)}
            {renderMetric('Realized Vol (30s)', features.realized_vol_30s, '%', true)}
            {renderMetric('Realized Vol (60s)', features.realized_vol_60s, '%', true)}
          </div>
        </div>

        {/* Section 3: Trade Flow CVD & Imbalance */}
        <div>
          <h3 className="text-xs font-bold text-slate-300 mb-2 flex items-center gap-1.5 uppercase tracking-wide">
            <BarChart className="h-3.5 w-3.5 text-emerald-400" />
            Trade Flow Cumulative Volume Delta (CVD) & Order Imbalance
          </h3>
          <div className="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-8 gap-2">
            {renderMetric('CVD (5s)', features.cvd_5s, '', false, true)}
            {renderMetric('CVD (15s)', features.cvd_15s, '', false, true)}
            {renderMetric('CVD (30s)', features.cvd_30s, '', false, true)}
            {renderMetric('CVD (60s)', features.cvd_60s, '', false, true)}
            {renderMetric('Trade Imb (5s)', features.trade_imbalance_5s, '', false, true)}
            {renderMetric('Trade Imb (15s)', features.trade_imbalance_15s, '', false, true)}
            {renderMetric('Trade Imb (30s)', features.trade_imbalance_30s, '', false, true)}
            {renderMetric('Trade Imb (60s)', features.trade_imbalance_60s, '', false, true)}
          </div>
        </div>

        {/* Section 4: Polymarket OrderBook Microstructure */}
        <div>
          <h3 className="text-xs font-bold text-slate-300 mb-2 flex items-center gap-1.5 uppercase tracking-wide">
            <Cpu className="h-3.5 w-3.5 text-indigo-400" />
            Polymarket Microstructure & Orderbook Imbalance (OBI)
          </h3>
          <div className="grid grid-cols-2 sm:grid-cols-6 gap-2">
            {renderMetric('OBI Top 5', features.poly_obi_top5, '', false, true)}
            {renderMetric('OBI Top 10', features.poly_obi_top10, '', false, true)}
            {renderMetric('OBI Top 20', features.poly_obi_top20, '', false, true)}
            {renderMetric('Polymarket Spread', `$${features.poly_spread.toFixed(3)}`)}
            {renderMetric('Total Liquidity', `$${features.poly_total_liquidity.toFixed(0)}`)}
            {renderMetric('Implied Prob (Up)', `${(features.poly_implied_prob * 100).toFixed(1)}%`)}
          </div>
        </div>

        {/* Section 5: Round Progress, Distance to Open & Spreads */}
        <div>
          <h3 className="text-xs font-bold text-slate-300 mb-2 flex items-center gap-1.5 uppercase tracking-wide">
            <Clock className="h-3.5 w-3.5 text-amber-400" />
            5-Minute Round State, Open Distance & Inter-Exchange Dispersion
          </h3>
          <div className="grid grid-cols-2 sm:grid-cols-6 gap-2">
            {renderMetric('Distance to Open', `$${features.distance_from_open.toFixed(2)}`, '', false, true)}
            {renderMetric('Dist Percent', features.distance_percent, '%', true, true)}
            {renderMetric('Dist / Vol Ratio', features.distance_to_vol_ratio.toFixed(2), 'x', false, true)}
            {renderMetric('Remaining Secs', `${features.remaining_seconds.toFixed(0)}s`)}
            {renderMetric('Decay Factor', features.time_decay_factor.toFixed(3))}
            {renderMetric('Binance-OKX Spread', `$${features.spread_binance_okx.toFixed(2)}`)}
          </div>
        </div>
      </div>
    </div>
  );
};
