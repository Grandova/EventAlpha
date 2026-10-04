import React, { useState, useEffect, useMemo } from 'react';
import {
  TrendingUp,
  TrendingDown,
  Activity,
  Zap,
  BarChart,
  Layers,
  Sliders,
  Timer,
  RefreshCw,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import {
  Asset,
  CompositePriceSnapshot,
  FeatureSnapshot,
  ModelPrediction,
  PredictionSignal,
  MarketDisplayInfo,
  PriceHistoryPoint,
} from '../types';
import { api } from '../services/api';

interface AssetPriceChartProps {
  asset: Asset;
  composite: CompositePriceSnapshot | null;
  features: FeatureSnapshot | null;
  prediction: ModelPrediction | null;
  signal: PredictionSignal | null;
  market: MarketDisplayInfo | null;
  remainingSeconds?: number;
}

type IndicatorTab = 'momentum' | 'volatility' | 'cvd' | 'spreads';

export const AssetPriceChart: React.FC<AssetPriceChartProps> = ({
  asset,
  composite,
  features,
  prediction,
  signal,
  market,
  remainingSeconds,
}) => {
  const [history, setHistory] = useState<PriceHistoryPoint[]>([]);
  const [isLoadingHistory, setIsLoadingHistory] = useState<boolean>(true);
  const [activeIndicatorTab, setActiveIndicatorTab] = useState<IndicatorTab>('momentum');
  const [isIndicatorsExpanded, setIsIndicatorsExpanded] = useState<boolean>(false);
  const [showMA7, setShowMA7] = useState<boolean>(true);
  const [showEMA25, setShowEMA25] = useState<boolean>(true);
  const [showOpenBaseline, setShowOpenBaseline] = useState<boolean>(true);
  const [hoveredPoint, setHoveredPoint] = useState<{
    x: number;
    y: number;
    price: number;
    timestamp: number;
    ma7?: number;
    ema25?: number;
  } | null>(null);

  // 1. Initial history fetch upon asset switch
  useEffect(() => {
    let isMounted = true;
    setIsLoadingHistory(true);

    api
      .getPolymarketPriceHistory(asset)
      .then((data) => {
        if (!isMounted) return;
        if (Array.isArray(data) && data.length > 0) {
          setHistory(data);
        } else {
          return api.getCompositePriceHistory(asset, 300).then((compData) => {
            if (!isMounted) return;
            if (Array.isArray(compData) && compData.length > 0) {
              setHistory(compData);
            }
          });
        }
      })
      .catch(() => {
        if (!isMounted) return;
        api
          .getCompositePriceHistory(asset, 300)
          .then((compData) => {
            if (!isMounted) return;
            if (Array.isArray(compData) && compData.length > 0) {
              setHistory(compData);
            }
          })
          .catch(() => {});
      })
      .finally(() => {
        if (isMounted) setIsLoadingHistory(false);
      });

    return () => {
      isMounted = false;
    };
  }, [asset]);

  // 2. Real-time tick injection (prioritizing Polymarket official live price)
  useEffect(() => {
    const rawPrice = composite?.poly_current_price ?? composite?.composite_price;
    if (typeof rawPrice !== 'number' || isNaN(rawPrice) || rawPrice <= 0) {
      return;
    }

    const price = rawPrice;
    const now = composite?.timestamp_ms || Date.now();

    setHistory((prev) => {
      if (prev.length === 0) {
        return [{ timestamp_ms: now, price }];
      }
      const last = prev[prev.length - 1];
      if (now - last.timestamp_ms < 300) {
        return prev;
      }
      const updated = [...prev, { timestamp_ms: now, price }];
      if (updated.length > 300) {
        return updated.slice(updated.length - 300);
      }
      return updated;
    });
  }, [composite?.poly_current_price, composite?.composite_price, composite?.timestamp_ms]);

  // 3. Technical Indicator Calculations: MA(7) & EMA(25)
  const chartData = useMemo(() => {
    if (history.length === 0) return [];

    const ma7Window = 7;
    const ema25Window = 25;
    const emaMultiplier = 2 / (ema25Window + 1);

    let prevEma: number | null = null;

    return history.map((pt, idx, arr) => {
      let ma7: number | undefined = undefined;
      if (idx >= ma7Window - 1) {
        const slice = arr.slice(idx - ma7Window + 1, idx + 1);
        const sum = slice.reduce((acc, p) => acc + p.price, 0);
        ma7 = sum / ma7Window;
      }

      let ema25: number | undefined = undefined;
      if (idx === 0) {
        prevEma = pt.price;
        ema25 = pt.price;
      } else {
        prevEma = (pt.price - (prevEma ?? pt.price)) * emaMultiplier + (prevEma ?? pt.price);
        ema25 = prevEma;
      }

      return {
        ...pt,
        ma7,
        ema25,
      };
    });
  }, [history]);

  // 4. Coordinates & Scales for SVG Rendering
  const width = 680;
  const height = 240;
  const padding = { top: 22, right: 85, bottom: 25, left: 15 };
  const chartW = width - padding.left - padding.right;
  const chartH = height - padding.top - padding.bottom;

  // 1. Polymarket official price (Chainlink 60s TWAP)
  const polyPrice = composite?.poly_current_price ?? market?.poly_current_price;
  const currentPolyPrice = (polyPrice && polyPrice > 0)
    ? polyPrice
    : (history.length > 0 ? history[history.length - 1].price : (composite?.composite_price ?? 0));

  // 2. Multi-exchange composite spot price (Binance, Coinbase, OKX, Bybit combined)
  const compositeSpotPrice = composite?.composite_price ?? 0;

  // Basis / spread between multi-exchange composite spot and Poly official price
  const basis = (compositeSpotPrice > 0 && currentPolyPrice > 0)
    ? compositeSpotPrice - currentPolyPrice
    : 0;

  const openPrice = composite?.open_price ?? market?.open_price;
  const deltaPrice = openPrice && openPrice > 0 ? currentPolyPrice - openPrice : 0;
  const deltaPct = openPrice && openPrice > 0 ? (deltaPrice / openPrice) * 100 : (composite?.distance_percent ?? 0);
  const isUp = deltaPrice >= 0;

  const { minPrice, maxPrice, pointsSvg, areaSvg, ma7Svg, ema25Svg, openY, lastPoint } = useMemo(() => {
    if (chartData.length === 0) {
      return { minPrice: 0, maxPrice: 1, pointsSvg: '', areaSvg: '', ma7Svg: '', ema25Svg: '', openY: chartH / 2, lastPoint: null };
    }

    let min = Infinity;
    let max = -Infinity;

    chartData.forEach((d) => {
      if (d.price < min) min = d.price;
      if (d.price > max) max = d.price;
    });

    if (openPrice && openPrice > 0) {
      if (openPrice < min) min = openPrice;
      if (openPrice > max) max = openPrice;
    }

    const range = max - min || 1.0;
    const yMin = min - range * 0.10;
    const yMax = max + range * 0.10;
    const yRange = yMax - yMin;

    const getX = (idx: number) => padding.left + (idx / Math.max(1, chartData.length - 1)) * chartW;
    const getY = (val: number) => padding.top + chartH - ((val - yMin) / yRange) * chartH;

    const points = chartData.map((d, i) => `${getX(i).toFixed(1)},${getY(d.price).toFixed(1)}`).join(' ');

    // Area path for gradient fill
    const firstX = getX(0).toFixed(1);
    const lastX = getX(chartData.length - 1).toFixed(1);
    const bottomY = (padding.top + chartH).toFixed(1);
    const area = `${points} ${lastX},${bottomY} ${firstX},${bottomY}`;

    const ma7Pts = chartData
      .map((d, i) => (d.ma7 !== undefined ? `${getX(i).toFixed(1)},${getY(d.ma7).toFixed(1)}` : null))
      .filter(Boolean)
      .join(' ');

    const ema25Pts = chartData
      .map((d, i) => (d.ema25 !== undefined ? `${getX(i).toFixed(1)},${getY(d.ema25).toFixed(1)}` : null))
      .filter(Boolean)
      .join(' ');

    const openYPos = openPrice && openPrice > 0 ? getY(openPrice) : chartH / 2;

    const last = chartData[chartData.length - 1];
    const lastPt = {
      x: getX(chartData.length - 1),
      y: getY(last.price),
      price: last.price,
    };

    return {
      minPrice: yMin,
      maxPrice: yMax,
      pointsSvg: points,
      areaSvg: area,
      ma7Svg: ma7Pts,
      ema25Svg: ema25Pts,
      openY: openYPos,
      lastPoint: lastPt,
    };
  }, [chartData, openPrice, chartW, chartH]);

  // Round window subtitle formatting (e.g. "10月 4, 下午 1:15-下午 1:20 ET")
  const roundWindowStr = useMemo(() => {
    const startMs = market?.start_time_ms;
    const endMs = market?.end_time_ms;
    if (!startMs || !endMs) return '当前 5 分钟轮次';

    const start = new Date(startMs);
    const end = new Date(endMs);
    const month = start.getMonth() + 1;
    const day = start.getDate();

    const formatHourMin = (d: Date) => {
      const h = d.getHours();
      const m = d.getMinutes().toString().padStart(2, '0');
      const period = h >= 12 ? '下午' : '上午';
      const h12 = h > 12 ? h - 12 : h === 0 ? 12 : h;
      return `${period} ${h12}:${m}`;
    };

    return `${month}月 ${day}日, ${formatHourMin(start)}-${formatHourMin(end)} ET ⓘ`;
  }, [market?.start_time_ms, market?.end_time_ms]);

  // Countdown timer string (e.g. "01:04")
  const roundSecs = remainingSeconds ?? market?.remaining_seconds ?? 0;
  const mins = Math.floor(roundSecs / 60);
  const secs = roundSecs % 60;
  const minStr = mins.toString().padStart(2, '0');
  const secStr = secs.toString().padStart(2, '0');

  // Asset Badge Configuration
  const assetConfig = {
    BTC: {
      symbol: '₿',
      name: 'BTC 5分钟上涨或下跌',
      iconBg: 'bg-[#f7931a]',
      textColor: 'text-[#f7931a]',
    },
    ETH: {
      symbol: 'Ξ',
      name: 'ETH 5分钟上涨或下跌',
      iconBg: 'bg-[#627eea]',
      textColor: 'text-[#627eea]',
    },
    SOL: {
      symbol: '◎',
      name: 'SOL 5分钟上涨或下跌',
      iconBg: 'bg-[#14f195]',
      textColor: 'text-[#14f195]',
    },
  }[asset] || {
    symbol: '⚡',
    name: `${asset} 5分钟上涨或下跌`,
    iconBg: 'bg-[#6c9bcf]',
    textColor: 'text-[#6c9bcf]',
  };

  return (
    <div className="asmr-card p-5 space-y-4">
      {/* 1. Official Polymarket Style Top Header */}
      <div className="flex items-start justify-between gap-4 border-b border-slate-100 dark:border-slate-800/80 pb-3">
        <div className="flex items-center gap-3">
          {/* Asset Icon (e.g. orange ₿ square) */}
          <div className={`w-11 h-11 rounded-2xl ${assetConfig.iconBg} text-white flex items-center justify-center font-black text-2xl shadow-md shrink-0`}>
            {assetConfig.symbol}
          </div>
          <div>
            <h2 className="text-lg lg:text-xl font-black text-[#363949] dark:text-white tracking-tight flex items-center gap-2">
              <span>{assetConfig.name}</span>
            </h2>
            <p className="text-xs font-mono text-[#7d8da1] mt-0.5">
              {roundWindowStr}
            </p>
          </div>
        </div>

        {/* Polymarket Big Countdown in Top Right */}
        <div className="flex items-center gap-2 text-right">
          <div className="flex items-baseline gap-1 font-mono-num">
            <div className="text-center">
              <span className={`text-2xl lg:text-3xl font-black ${roundSecs <= 30 ? 'text-[#ff0060] animate-pulse' : 'text-[#ff0060]'}`}>
                {minStr}
              </span>
              <span className="block text-[10px] text-[#7d8da1] font-sans -mt-1">分钟</span>
            </div>
            <span className="text-2xl font-black text-[#ff0060] -mt-3">:</span>
            <div className="text-center">
              <span className={`text-2xl lg:text-3xl font-black ${roundSecs <= 30 ? 'text-[#ff0060] animate-pulse' : 'text-[#ff0060]'}`}>
                {secStr}
              </span>
              <span className="block text-[10px] text-[#7d8da1] font-sans -mt-1">秒</span>
            </div>
          </div>
          <div className="hidden sm:flex flex-col items-center justify-center pl-2 border-l border-slate-200 dark:border-slate-800">
            <span className="text-[10px] font-black tracking-wider text-[#7d8da1] uppercase">Polymarket</span>
            <span className="w-2 h-2 rounded-full bg-emerald-500 animate-ping mt-0.5" />
          </div>
        </div>
      </div>

      {/* 2. Official Polymarket Price Header: 目标价格 (Strike) & 当前价格 (Poly官方) & 综合现货 (4所结合) */}
      <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-6 sm:gap-8">
          {/* Target / Strike Price */}
          <div>
            <div className="flex items-center gap-1.5">
              <span className="text-[11px] font-bold text-[#7d8da1] block font-mono">
                目标价格
              </span>
              <span className="text-[9px] px-1.5 py-0.2 rounded bg-slate-100 dark:bg-slate-800 text-slate-500 font-mono font-medium">
                Strike
              </span>
            </div>
            <span className="text-2xl lg:text-3xl font-extrabold font-mono-num text-[#363949] dark:text-slate-100 tracking-tight">
              ${openPrice && openPrice > 0
                ? openPrice.toLocaleString(undefined, {
                    minimumFractionDigits: 2,
                    maximumFractionDigits: 2,
                  })
                : '--'}
            </span>
          </div>

          {/* Current Live Price & Delta (Based on Polymarket official price) */}
          <div>
            <div className="flex items-center gap-2">
              <span className="text-[11px] font-bold text-amber-500 block font-mono flex items-center gap-1">
                <span>当前价格</span>
                <span className="text-[9px] px-1.5 py-0.2 rounded-full bg-amber-500/15 text-amber-600 dark:text-amber-400 font-bold uppercase">
                  Poly 官方
                </span>
              </span>
              {/* Dollar Delta Pill */}
              <span
                className={`inline-flex items-center gap-0.5 text-xs font-black font-mono-num px-1.5 py-0.5 rounded-md ${
                  isUp
                    ? 'text-[#1b9c85] bg-emerald-50 dark:bg-emerald-950/40'
                    : 'text-[#ff0060] bg-rose-50 dark:bg-rose-950/40'
                }`}
              >
                {isUp ? '▲' : '▼'} ${Math.abs(deltaPrice).toFixed(2)}
                <span className="text-[10px] opacity-80 font-normal">
                  ({isUp ? '+' : ''}{deltaPct.toFixed(2)}%)
                </span>
              </span>
            </div>
            <span className={`text-2xl lg:text-3xl font-black font-mono-num tracking-tight ${
              isUp ? 'text-amber-500 dark:text-amber-400' : 'text-[#ff0060]'
            }`}>
              ${currentPolyPrice > 0
                ? currentPolyPrice.toLocaleString(undefined, {
                    minimumFractionDigits: 2,
                    maximumFractionDigits: 2,
                  })
                : '--'}
            </span>
          </div>

          {/* Multi-Exchange Combined Spot Price (旁边再显示几个交易所结合起来的价格) */}
          <div className="border-l border-slate-200 dark:border-slate-800 pl-4 sm:pl-6">
            <div className="flex items-center gap-2">
              <span className="text-[11px] font-bold text-[#6c9bcf] block font-mono flex items-center gap-1">
                <span>综合现货</span>
                <span className="text-[9px] px-1.5 py-0.2 rounded-full bg-[#6c9bcf]/15 text-[#6c9bcf] font-bold">
                  4所结合
                </span>
              </span>
              {compositeSpotPrice > 0 && currentPolyPrice > 0 && (
                <span
                  className="text-[10px] font-mono font-bold px-1.5 py-0.5 rounded bg-blue-500/10 text-blue-600 dark:text-blue-400"
                  title="多所综合现货与 Polymarket 官方计价的实时基差"
                >
                  基差 {basis >= 0 ? '+' : ''}${basis.toFixed(2)}
                </span>
              )}
            </div>
            <div className="flex items-baseline gap-2">
              <span className="text-2xl lg:text-3xl font-extrabold font-mono-num text-[#363949] dark:text-slate-200 tracking-tight">
                ${compositeSpotPrice > 0
                  ? compositeSpotPrice.toLocaleString(undefined, {
                      minimumFractionDigits: 2,
                      maximumFractionDigits: 2,
                    })
                  : '--'}
              </span>
            </div>
            {/* Breakdown of the 4 exchanges */}
            <div className="flex items-center gap-2 mt-0.5 text-[10px] font-mono text-[#7d8da1]">
              <span title="Binance (40% 权重)">
                BN: {composite?.price_binance ? `$${composite.price_binance.toFixed(1)}` : '--'}
              </span>
              <span>·</span>
              <span title="Coinbase (25% 权重)">
                CB: {composite?.price_coinbase ? `$${composite.price_coinbase.toFixed(1)}` : '--'}
              </span>
              <span>·</span>
              <span title="OKX (25% 权重)">
                OKX: {composite?.price_okx ? `$${composite.price_okx.toFixed(1)}` : '--'}
              </span>
              <span>·</span>
              <span title="Bybit (10% 权重)">
                BY: {composite?.price_bybit ? `$${composite.price_bybit.toFixed(1)}` : '--'}
              </span>
            </div>
          </div>
        </div>

        {/* Technical Overlay Toggles: MA7, EMA25, Target Baseline */}
        <div className="flex flex-wrap items-center gap-1.5">
          <button
            type="button"
            onClick={() => setShowMA7(!showMA7)}
            className={`px-2.5 py-1 rounded-xl text-xs font-mono font-bold transition-all flex items-center gap-1.5 cursor-pointer ${
              showMA7
                ? 'bg-[#6c9bcf]/20 text-[#6c9bcf] border border-[#6c9bcf]/40'
                : 'bg-slate-100 dark:bg-slate-800 text-slate-400 opacity-60'
            }`}
          >
            <span className="w-2 h-2 rounded-full bg-[#6c9bcf]" />
            <span>MA(7)</span>
          </button>

          <button
            type="button"
            onClick={() => setShowEMA25(!showEMA25)}
            className={`px-2.5 py-1 rounded-xl text-xs font-mono font-bold transition-all flex items-center gap-1.5 cursor-pointer ${
              showEMA25
                ? 'bg-purple-500/20 text-purple-600 dark:text-purple-400 border border-purple-500/40'
                : 'bg-slate-100 dark:bg-slate-800 text-slate-400 opacity-60'
            }`}
          >
            <span className="w-2 h-2 rounded-full bg-purple-500" />
            <span>EMA(25)</span>
          </button>

          <button
            type="button"
            onClick={() => setShowOpenBaseline(!showOpenBaseline)}
            className={`px-2.5 py-1 rounded-xl text-xs font-mono font-bold transition-all flex items-center gap-1.5 cursor-pointer ${
              showOpenBaseline
                ? 'bg-amber-500/20 text-amber-600 dark:text-amber-400 border border-amber-500/40'
                : 'bg-slate-100 dark:bg-slate-800 text-slate-400 opacity-60'
            }`}
          >
            <span className="w-2.5 h-0.5 bg-amber-500" />
            <span>目标线</span>
          </button>
        </div>
      </div>

      {/* 3. Compact Clean SVG Price Waveform Chart */}
      <div className="relative w-full rounded-2xl bg-gradient-to-b from-slate-50/70 to-slate-100/40 dark:from-[#181a1e] dark:to-[#131518] border border-slate-200/80 dark:border-slate-800/90 overflow-hidden shadow-inner p-1">
        {isLoadingHistory && (
          <div className="absolute inset-0 bg-white/60 dark:bg-[#181a1e]/60 flex items-center justify-center z-10 text-xs font-mono text-[#6c9bcf] font-bold">
            <RefreshCw className="w-4 h-4 animate-spin mr-2" />
            同步 {asset} 历史行情...
          </div>
        )}

        <svg
          viewBox={`0 0 ${width} ${height}`}
          className="w-full h-52 sm:h-60 select-none"
          onMouseLeave={() => setHoveredPoint(null)}
          onMouseMove={(e) => {
            const rect = e.currentTarget.getBoundingClientRect();
            const mouseX = e.clientX - rect.left;
            const relX = (mouseX / rect.width) * width;

            if (relX >= padding.left && relX <= width - padding.right && chartData.length > 0) {
              const ratio = (relX - padding.left) / chartW;
              const idx = Math.min(chartData.length - 1, Math.max(0, Math.round(ratio * (chartData.length - 1))));
              const item = chartData[idx];
              const curY = padding.top + chartH - ((item.price - minPrice) / (maxPrice - minPrice || 1)) * chartH;
              setHoveredPoint({
                x: relX,
                y: curY,
                price: item.price,
                timestamp: item.timestamp_ms,
                ma7: item.ma7,
                ema25: item.ema25,
              });
            }
          }}
        >
          <defs>
            <linearGradient id="upGradient" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#1b9c85" stopOpacity="0.25" />
              <stop offset="100%" stopColor="#1b9c85" stopOpacity="0.0" />
            </linearGradient>
            <linearGradient id="downGradient" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#ff0060" stopOpacity="0.22" />
              <stop offset="100%" stopColor="#ff0060" stopOpacity="0.0" />
            </linearGradient>
          </defs>

          {/* Grid lines */}
          {[0.25, 0.5, 0.75].map((pct, i) => {
            const y = padding.top + chartH * pct;
            return (
              <line
                key={i}
                x1={padding.left}
                y1={y}
                x2={width - padding.right}
                y2={y}
                stroke="currentColor"
                className="text-slate-200/60 dark:text-slate-800/80"
                strokeDasharray="3 3"
                strokeWidth="1"
              />
            );
          })}

          {/* Target Price Horizontal Dashed Line with Axis Label Badge */}
          {showOpenBaseline && openPrice && openPrice > 0 && (
            <g>
              <line
                x1={padding.left}
                y1={openY}
                x2={width - padding.right}
                y2={openY}
                stroke="#f59e0b"
                strokeDasharray="4 3"
                strokeWidth="1.5"
                opacity="0.85"
              />
              {/* Target pill label on right axis */}
              <rect
                x={width - padding.right + 4}
                y={openY - 9}
                width={76}
                height={18}
                rx={5}
                fill="#f59e0b"
                opacity="0.9"
              />
              <text
                x={width - padding.right + 42}
                y={openY + 3.5}
                textAnchor="middle"
                fill="#ffffff"
                fontSize="9"
                fontFamily="monospace"
                fontWeight="900"
              >
                Target ︾
              </text>
            </g>
          )}

          {/* Area fill gradient beneath curve */}
          {areaSvg && (
            <polygon
              points={areaSvg}
              fill={isUp ? 'url(#upGradient)' : 'url(#downGradient)'}
            />
          )}

          {/* Price trajectory line */}
          {pointsSvg && (
            <polyline
              points={pointsSvg}
              fill="none"
              stroke={isUp ? '#f59e0b' : '#ff0060'}
              strokeWidth="2.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          )}

          {/* MA(7) Overlay Line */}
          {showMA7 && ma7Svg && (
            <polyline
              points={ma7Svg}
              fill="none"
              stroke="#6c9bcf"
              strokeWidth="1.5"
              strokeLinecap="round"
              opacity="0.8"
            />
          )}

          {/* EMA(25) Overlay Line */}
          {showEMA25 && ema25Svg && (
            <polyline
              points={ema25Svg}
              fill="none"
              stroke="#a855f7"
              strokeWidth="1.5"
              strokeLinecap="round"
              opacity="0.8"
            />
          )}

          {/* Pulsating live dot on latest price point */}
          {lastPoint && (
            <g>
              <circle
                cx={lastPoint.x}
                cy={lastPoint.y}
                r="4"
                fill={isUp ? '#f59e0b' : '#ff0060'}
                stroke="#ffffff"
                strokeWidth="1.5"
              />
              <circle
                cx={lastPoint.x}
                cy={lastPoint.y}
                r="7"
                fill="none"
                stroke={isUp ? '#f59e0b' : '#ff0060'}
                strokeWidth="1"
                opacity="0.75"
              />
            </g>
          )}

          {/* Right Y-Axis Price Labels */}
          <text
            x={width - padding.right + 6}
            y={padding.top + 8}
            fill="currentColor"
            className="text-slate-400 font-mono"
            fontSize="9"
          >
            ${maxPrice.toFixed(asset === 'BTC' ? 0 : 2)}
          </text>
          <text
            x={width - padding.right + 6}
            y={padding.top + chartH}
            fill="currentColor"
            className="text-slate-400 font-mono"
            fontSize="9"
          >
            ${minPrice.toFixed(asset === 'BTC' ? 0 : 2)}
          </text>

          {/* Crosshair on hover */}
          {hoveredPoint && (
            <g>
              <line
                x1={hoveredPoint.x}
                y1={padding.top}
                x2={hoveredPoint.x}
                y2={padding.top + chartH}
                stroke="#6c9bcf"
                strokeDasharray="2 2"
                strokeWidth="1"
              />
              <circle cx={hoveredPoint.x} cy={hoveredPoint.y} r="4" fill="#6c9bcf" stroke="#fff" strokeWidth="2" />
            </g>
          )}
        </svg>

        {/* Hover Information Tooltip Overlay */}
        {hoveredPoint && (
          <div className="absolute top-2 left-3 bg-white/95 dark:bg-[#202528]/95 backdrop-blur-md p-2 rounded-xl border border-slate-200 dark:border-slate-700 shadow-md font-mono text-[11px] space-y-0.5 pointer-events-none z-20">
            <div className="text-[#7d8da1] text-[10px]">
              {new Date(hoveredPoint.timestamp).toLocaleTimeString()}
            </div>
            <div className="font-extrabold text-[#363949] dark:text-white">
              价格: ${hoveredPoint.price.toFixed(asset === 'BTC' ? 1 : 2)}
            </div>
            {hoveredPoint.ma7 !== undefined && (
              <div className="text-[#6c9bcf] font-bold">
                MA(7): ${hoveredPoint.ma7.toFixed(asset === 'BTC' ? 1 : 2)}
              </div>
            )}
            {hoveredPoint.ema25 !== undefined && (
              <div className="text-purple-500 font-bold">
                EMA(25): ${hoveredPoint.ema25.toFixed(asset === 'BTC' ? 1 : 2)}
              </div>
            )}
          </div>
        )}

        {/* Bottom Source Badge Watermark */}
        <div className="absolute bottom-1.5 left-3 text-[10px] font-mono text-[#7d8da1] opacity-75">
          来源: Polymarket.com · Chainlink/Coinbase USD
        </div>
      </div>

      {/* 4. Compact Collapsible Quantitative Technical Indicators */}
      <div className="border border-slate-200/70 dark:border-slate-800 rounded-2xl bg-slate-50/50 dark:bg-[#181a1e]/60 overflow-hidden transition-all">
        {/* Header Ribbon / Tabs */}
        <div className="p-2.5 px-3.5 flex flex-wrap items-center justify-between gap-2 border-b border-slate-100 dark:border-slate-800/80">
          <div className="flex items-center gap-1.5">
            <Sliders className="w-3.5 h-3.5 text-[#6c9bcf]" />
            <span className="text-xs font-extrabold text-[#363949] dark:text-white">
              量化技术指标
            </span>
          </div>

          <div className="flex items-center gap-1">
            {(
              [
                { id: 'momentum', label: '动量收益率', icon: Zap },
                { id: 'volatility', label: '波动率', icon: Activity },
                { id: 'cvd', label: 'CVD 订单流', icon: BarChart },
                { id: 'spreads', label: '跨所价差', icon: Layers },
              ] as { id: IndicatorTab; label: string; icon: any }[]
            ).map((tab) => {
              const Icon = tab.icon;
              const isActive = activeIndicatorTab === tab.id;
              return (
                <button
                  key={tab.id}
                  type="button"
                  onClick={() => {
                    setActiveIndicatorTab(tab.id);
                    setIsIndicatorsExpanded(true);
                  }}
                  className={`px-2.5 py-1 rounded-lg text-[11px] font-bold font-mono transition-all flex items-center gap-1 cursor-pointer ${
                    isActive
                      ? 'bg-white dark:bg-[#202528] text-[#6c9bcf] shadow-sm'
                      : 'text-[#7d8da1] hover:text-[#363949] dark:hover:text-white'
                  }`}
                >
                  <Icon className="w-3 h-3" />
                  <span>{tab.label}</span>
                </button>
              );
            })}

            {/* Expand / Collapse Button */}
            <button
              type="button"
              onClick={() => setIsIndicatorsExpanded(!isIndicatorsExpanded)}
              className="p-1 px-2 text-[#7d8da1] hover:text-[#363949] dark:hover:text-white rounded-lg hover:bg-slate-200/50 text-[11px] flex items-center gap-0.5 cursor-pointer font-mono"
            >
              <span>{isIndicatorsExpanded ? '收起' : '展开详情'}</span>
              {isIndicatorsExpanded ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
            </button>
          </div>
        </div>

        {/* Collapsed Single-Row Summary Ribbon */}
        {!isIndicatorsExpanded && (
          <div className="p-2 px-3.5 grid grid-cols-2 sm:grid-cols-4 gap-2 font-mono text-[11px]">
            <div>
              <span className="text-[#7d8da1] text-[10px] block">60s 收益率:</span>
              <strong className={(composite?.return_60s ?? 0) >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}>
                {(composite?.return_60s ?? 0) >= 0 ? '+' : ''}{((composite?.return_60s ?? 0) * 100).toFixed(3)}%
              </strong>
            </div>
            <div>
              <span className="text-[#7d8da1] text-[10px] block">60s 波动率:</span>
              <strong className="text-purple-600 dark:text-purple-400">
                {((composite?.realized_vol_60s ?? 0) * 100).toFixed(3)}%
              </strong>
            </div>
            <div>
              <span className="text-[#7d8da1] text-[10px] block">CVD 净流 (5s):</span>
              <strong className={(features?.cvd_5s ?? 0) >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}>
                {(features?.cvd_5s ?? 0) >= 0 ? '+' : ''}{(features?.cvd_5s ?? 12.5).toFixed(1)}
              </strong>
            </div>
            <div>
              <span className="text-[#7d8da1] text-[10px] block">跨所基差 (BN-CB):</span>
              <strong className="text-[#6c9bcf]">
                {typeof composite?.spread_binance_coinbase === 'number' ? `$${composite.spread_binance_coinbase.toFixed(2)}` : '--'}
              </strong>
            </div>
          </div>
        )}

        {/* Expanded Detailed Grid */}
        {isIndicatorsExpanded && (
          <div className="p-3 border-t border-slate-100 dark:border-slate-800">
            {activeIndicatorTab === 'momentum' && (
              <div className="grid grid-cols-3 sm:grid-cols-6 gap-2">
                {[
                  { label: '1s 动量', val: composite?.return_1s ?? features?.return_1s },
                  { label: '3s 动量', val: composite?.return_3s ?? features?.return_3s },
                  { label: '5s 动量', val: composite?.return_5s ?? features?.return_5s },
                  { label: '10s 动量', val: composite?.return_10s ?? features?.return_10s },
                  { label: '30s 动量', val: composite?.return_30s ?? features?.return_30s },
                  { label: '60s 动量', val: composite?.return_60s ?? features?.return_60s },
                ].map((m, i) => {
                  const v = m.val ?? 0;
                  return (
                    <div key={i} className="p-2 rounded-xl bg-white dark:bg-[#202528] border border-slate-100 dark:border-slate-800 font-mono text-center">
                      <span className="text-[10px] text-[#7d8da1] block">{m.label}</span>
                      <span className={`text-xs font-black ${v >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}`}>
                        {v >= 0 ? '+' : ''}{(v * 100).toFixed(3)}%
                      </span>
                    </div>
                  );
                })}
              </div>
            )}

            {activeIndicatorTab === 'volatility' && (
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                {[
                  { label: '波动率 (5s)', val: composite?.realized_vol_5s ?? features?.realized_vol_5s },
                  { label: '波动率 (10s)', val: composite?.realized_vol_10s ?? features?.realized_vol_10s },
                  { label: '波动率 (30s)', val: composite?.realized_vol_30s ?? features?.realized_vol_30s },
                  { label: '波动率 (60s)', val: composite?.realized_vol_60s ?? features?.realized_vol_60s },
                ].map((m, i) => {
                  const v = m.val ?? 0;
                  return (
                    <div key={i} className="p-2 rounded-xl bg-white dark:bg-[#202528] border border-slate-100 dark:border-slate-800 font-mono text-center">
                      <span className="text-[10px] text-[#7d8da1] block">{m.label}</span>
                      <span className="text-xs font-black text-purple-600 dark:text-purple-400">
                        {(v * 100).toFixed(3)}%
                      </span>
                    </div>
                  );
                })}
              </div>
            )}

            {activeIndicatorTab === 'cvd' && (
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                {[
                  { label: 'CVD 净额 (5s)', val: features?.cvd_5s ?? 12.5 },
                  { label: 'CVD 净额 (15s)', val: features?.cvd_15s ?? -5.2 },
                  { label: '盘口失衡 OBI Top5', val: features?.poly_obi_top5 ?? 0.18, isRatio: true },
                  { label: '盘口失衡 OBI Top20', val: features?.poly_obi_top20 ?? 0.12, isRatio: true },
                ].map((m, i) => {
                  const v = m.val ?? 0;
                  return (
                    <div key={i} className="p-2 rounded-xl bg-white dark:bg-[#202528] border border-slate-100 dark:border-slate-800 font-mono text-center">
                      <span className="text-[10px] text-[#7d8da1] block">{m.label}</span>
                      <span className={`text-xs font-black ${v >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}`}>
                        {v >= 0 ? '+' : ''}{m.isRatio ? `${(v * 100).toFixed(1)}%` : v.toFixed(2)}
                      </span>
                    </div>
                  );
                })}
              </div>
            )}

            {activeIndicatorTab === 'spreads' && (
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
                {[
                  { label: 'Binance - OKX 价差', val: composite?.spread_binance_okx },
                  { label: 'Binance - Bybit 价差', val: composite?.spread_binance_bybit },
                  { label: 'Binance - Coinbase 价差', val: composite?.spread_binance_coinbase },
                ].map((m, i) => (
                  <div key={i} className="p-2 rounded-xl bg-white dark:bg-[#202528] border border-slate-100 dark:border-slate-800 font-mono text-center">
                    <span className="text-[10px] text-[#7d8da1] block">{m.label}</span>
                    <span className="text-xs font-black text-[#6c9bcf]">
                      {typeof m.val === 'number' ? `$${m.val.toFixed(2)}` : '--'}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
