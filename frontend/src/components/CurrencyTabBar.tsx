import React from 'react';
import {
  TrendingUp,
  TrendingDown,
  Timer,
  Zap,
  Check,
  X as XIcon,
  ShieldAlert,
  SlidersHorizontal,
  Activity,
  Layers,
} from 'lucide-react';
import { Asset, PriceSummary, CompositePriceSnapshot, PredictionSignal, ModelPrediction } from '../types';

interface CurrencyTabBarProps {
  activeAsset: Asset;
  onSelectAsset: (asset: Asset) => void;
  spotPrices?: PriceSummary[];
  allComposite?: Record<string, CompositePriceSnapshot> | null;
  allSignals?: Record<string, PredictionSignal> | null;
  allPredictions?: Record<string, ModelPrediction> | null;
  remainingSeconds: number;
  enabledAssets?: Asset[];
  onToggleEnabledAsset?: (asset: Asset) => void;
}

interface AssetMeta {
  asset: Asset;
  name: string;
  enName: string;
  symbol: string;
  iconBg: string;
  glowColor: string;
  accentColor: string;
}

const ASSET_METAS: AssetMeta[] = [
  {
    asset: 'BTC',
    name: '比特币',
    enName: 'Bitcoin',
    symbol: '₿',
    iconBg: 'bg-[#f7931a]',
    glowColor: 'shadow-[#f7931a]/25',
    accentColor: '#f7931a',
  },
  {
    asset: 'ETH',
    name: '以太坊',
    enName: 'Ethereum',
    symbol: 'Ξ',
    iconBg: 'bg-[#627eea]',
    glowColor: 'shadow-[#627eea]/25',
    accentColor: '#627eea',
  },
  {
    asset: 'SOL',
    name: '索拉纳',
    enName: 'Solana',
    symbol: '◎',
    iconBg: 'bg-[#14f195]',
    glowColor: 'shadow-[#14f195]/25',
    accentColor: '#14f195',
  },
];

export const CurrencyTabBar: React.FC<CurrencyTabBarProps> = ({
  activeAsset,
  onSelectAsset,
  spotPrices = [],
  allComposite,
  allSignals,
  allPredictions,
  remainingSeconds,
  enabledAssets = ['BTC', 'ETH', 'SOL'],
  onToggleEnabledAsset,
}) => {
  const mins = Math.floor(remainingSeconds / 60);
  const secs = remainingSeconds % 60;
  const timerStr = `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;

  const getPriceData = (asset: Asset) => {
    // 1. Check composite snapshot
    if (allComposite && allComposite[asset]) {
      const snap = allComposite[asset];
      return {
        price: snap.composite_price,
        openPrice: snap.open_price,
        distPct: snap.distance_percent,
        distFromOpen: snap.distance_from_open,
        return60s: snap.return_60s,
      };
    }

    // 2. Check spotPrices array
    if (Array.isArray(spotPrices)) {
      const found = spotPrices.find((sp) => sp && sp.asset === asset);
      if (found) {
        const p = found.price ?? found.mid ?? found.last;
        return {
          price: typeof p === 'number' ? p : undefined,
          openPrice: undefined,
          distPct: undefined,
          distFromOpen: undefined,
          return60s: undefined,
        };
      }
    }

    return {
      price: undefined,
      openPrice: undefined,
      distPct: undefined,
      distFromOpen: undefined,
      return60s: undefined,
    };
  };

  return (
    <div className="space-y-3">
      {/* Top Header Strip: Context & Explanation */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <span className="p-1 px-2 rounded-lg bg-[#6c9bcf]/15 text-[#6c9bcf] text-[10px] font-extrabold uppercase font-mono tracking-wider">
              MULTI-CURRENCY DESK
            </span>
            <h3 className="text-base font-extrabold text-[#363949] dark:text-white tracking-tight">
              货币币种独立交易控制台
            </h3>
          </div>
          <p className="text-xs text-[#7d8da1] mt-0.5">
            点击下方货币卡片即可切换到该币种专属的<strong>走势图表、技术指标、AI 预测与 5M 交易盘口</strong>
          </p>
        </div>

        {/* 5M Window Global Countdown */}
        <div className="flex items-center gap-2 self-start sm:self-auto bg-white dark:bg-[#202528] px-3.5 py-1.5 rounded-2xl border border-slate-200 dark:border-slate-800 shadow-sm font-mono text-xs">
          <span className="text-[#7d8da1] flex items-center gap-1">
            <Timer className="w-3.5 h-3.5 text-[#6c9bcf]" />
            5M 交割倒计时:
          </span>
          <span className={`font-black font-mono-num ${
            remainingSeconds <= 30 ? 'text-[#ff0060] animate-pulse' : remainingSeconds <= 60 ? 'text-amber-500' : 'text-[#1b9c85]'
          }`}>
            {timerStr}
          </span>
        </div>
      </div>

      {/* 3 Dedicated Currency Trading Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {ASSET_METAS.map((item) => {
          const isSelected = activeAsset === item.asset;
          const isEnabled = enabledAssets.includes(item.asset);
          const priceData = getPriceData(item.asset);
          const signal = allSignals ? allSignals[item.asset] : undefined;
          const prediction = allPredictions ? allPredictions[item.asset] : undefined;

          // Implied or calibrated win rate
          const pUp = prediction ? prediction.calibrated_p_up * 100 : undefined;
          const isUpSignal = signal?.action === 'BUY_UP' || (typeof pUp === 'number' && pUp >= 52);
          const isDownSignal = signal?.action === 'BUY_DOWN' || (typeof pUp === 'number' && pUp <= 48);

          return (
            <div
              key={item.asset}
              onClick={() => onSelectAsset(item.asset)}
              className={`relative asmr-card p-5 cursor-pointer transition-all duration-300 border-2 select-none group ${
                isSelected
                  ? 'border-[#6c9bcf] shadow-lg bg-gradient-to-b from-white to-slate-50/50 dark:from-[#202528] dark:to-[#181a1e] scale-[1.01]'
                  : 'border-transparent hover:border-slate-300 dark:hover:border-slate-700 hover:shadow-md'
              }`}
            >
              {/* Active Selection Glow Pill */}
              {isSelected && (
                <div className="absolute top-3 right-3 flex items-center gap-1 px-2 py-0.5 rounded-full bg-[#6c9bcf] text-white text-[10px] font-extrabold shadow-sm">
                  <span className="w-1.5 h-1.5 rounded-full bg-white animate-ping" />
                  <span>当前交易视窗</span>
                </div>
              )}

              {/* Card Header: Currency Icon, Name, and Auto-Trading Switch */}
              <div className="flex items-center justify-between gap-3">
                <div className="flex items-center gap-3">
                  <div className={`w-12 h-12 rounded-2xl ${item.iconBg} text-white flex items-center justify-center text-xl font-black shadow-md ${item.glowColor}`}>
                    {item.symbol}
                  </div>
                  <div>
                    <div className="flex items-center gap-1.5">
                      <h4 className="text-base font-extrabold text-[#363949] dark:text-white leading-tight">
                        {item.name}
                      </h4>
                      <span className="text-[10px] font-mono font-bold text-[#7d8da1] bg-slate-100 dark:bg-slate-800 px-1.5 py-0.5 rounded">
                        5M
                      </span>
                    </div>
                    <span className="text-xs text-[#7d8da1] font-mono">
                      {item.asset} / USDC
                    </span>
                  </div>
                </div>

                {/* Independent Auto-Trading Toggle for this currency */}
                {onToggleEnabledAsset && (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      onToggleEnabledAsset(item.asset);
                    }}
                    className={`p-1.5 px-2.5 rounded-xl text-[11px] font-bold font-mono transition-all flex items-center gap-1 cursor-pointer ${
                      isEnabled
                        ? 'bg-emerald-50 dark:bg-emerald-950/60 text-[#1b9c85] border border-emerald-300 dark:border-emerald-800 hover:bg-emerald-100'
                        : 'bg-slate-100 dark:bg-slate-800 text-[#7d8da1] border border-slate-300 dark:border-slate-700 hover:bg-slate-200 line-through opacity-70'
                    }`}
                    title={isEnabled ? `点击关闭 ${item.asset} 自动交易` : `点击允许 ${item.asset} 自动交易`}
                  >
                    {isEnabled ? <Check className="w-3 h-3 text-[#1b9c85]" /> : <XIcon className="w-3 h-3 text-slate-400" />}
                    <span>{isEnabled ? '允许买入' : '已停用'}</span>
                  </button>
                )}
              </div>

              {/* Price & 5M Baseline Information */}
              <div className="mt-4 pt-3 border-t border-slate-100 dark:border-slate-800 flex items-end justify-between">
                <div>
                  <span className="text-[10px] text-[#7d8da1] block font-semibold">
                    多所加权基准现货价
                  </span>
                  <div className="text-xl font-extrabold font-mono-num text-[#363949] dark:text-white tracking-tight">
                    {typeof priceData.price === 'number' && !isNaN(priceData.price)
                      ? `$${priceData.price.toLocaleString(undefined, {
                          minimumFractionDigits: item.asset === 'BTC' ? 1 : 2,
                          maximumFractionDigits: item.asset === 'BTC' ? 1 : 2,
                        })}`
                      : '--'}
                  </div>
                </div>

                {/* 5M Round Distance from Open */}
                <div className="text-right">
                  <span className="text-[10px] text-[#7d8da1] block font-semibold">
                    距 5M 开盘价偏离
                  </span>
                  {typeof priceData.distPct === 'number' && !isNaN(priceData.distPct) ? (
                    <span className={`inline-flex items-center gap-0.5 text-xs font-mono font-black ${
                      priceData.distPct >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'
                    }`}>
                      {priceData.distPct >= 0 ? (
                        <TrendingUp className="w-3.5 h-3.5" />
                      ) : (
                        <TrendingDown className="w-3.5 h-3.5" />
                      )}
                      {priceData.distPct >= 0 ? '+' : ''}
                      {priceData.distPct.toFixed(3)}%
                    </span>
                  ) : (
                    <span className="text-xs text-[#7d8da1] font-mono">0.00%</span>
                  )}
                </div>
              </div>

              {/* Bottom Strip: AI Model Prediction & Signal Badge */}
              <div className="mt-3 p-2 rounded-xl bg-slate-50 dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800/80 flex items-center justify-between text-xs font-mono">
                <div className="flex items-center gap-1.5">
                  <Zap className="w-3.5 h-3.5 text-amber-500" />
                  <span className="text-[#7d8da1] text-[11px]">AI 预测:</span>
                  <span className={`font-bold ${
                    isUpSignal ? 'text-[#1b9c85]' : isDownSignal ? 'text-[#ff0060]' : 'text-slate-500'
                  }`}>
                    {isUpSignal
                      ? `看涨 UP (${(pUp ?? 55).toFixed(0)}%)`
                      : isDownSignal
                      ? `看跌 DOWN (${(100 - (pUp ?? 45)).toFixed(0)}%)`
                      : '观望信号'}
                  </span>
                </div>

                {signal?.confidence && (
                  <span className={`px-2 py-0.5 rounded text-[10px] font-bold ${
                    signal.confidence === 'VERY_HIGH'
                      ? 'bg-purple-100 text-purple-700 dark:bg-purple-950 dark:text-purple-300'
                      : signal.confidence === 'HIGH'
                      ? 'bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-300'
                      : 'bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400'
                  }`}>
                    {signal.confidence}
                  </span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
