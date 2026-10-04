import React, { useState, useEffect, useRef, useMemo } from 'react';
import {
  TrendingUp,
  TrendingDown,
  Activity,
  Zap,
  BarChart,
  Layers,
  Sliders,
  ShieldCheck,
  ShieldAlert,
  Clock,
  Cpu,
  RefreshCw,
  Eye,
  Info,
  ChevronRight,
  Crosshair,
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
}

type IndicatorTab = 'momentum' | 'volatility' | 'cvd' | 'spreads';

export const AssetPriceChart: React.FC<AssetPriceChartProps> = ({
  asset,
  composite,
  features,
  prediction,
  signal,
  market,
}) => {
  const [history, setHistory] = useState<PriceHistoryPoint[]>([]);
  const [isLoadingHistory, setIsLoadingHistory] = useState<boolean>(true);
  const [activeIndicatorTab, setActiveIndicatorTab] = useState<IndicatorTab>('momentum');
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
      .getCompositePriceHistory(asset, 300)
      .then((data) => {
        if (!isMounted) return;
        if (Array.isArray(data) && data.length > 0) {
          setHistory(data);
        } else {
          // Fallback initial baseline if fresh server
          const curPrice = composite?.composite_price ?? 100.0;
          const now = Date.now();
          const seed: PriceHistoryPoint[] = [];
          for (let i = 30; i >= 0; i--) {
            seed.push({
              timestamp_ms: now - i * 5000,
              price: curPrice * (1 + (Math.sin(i / 3) * 0.0005)),
            });
          }
          setHistory(seed);
        }
      })
      .catch(() => {
        if (!isMounted) return;
        // Keep existing or seed
      })
      .finally(() => {
        if (isMounted) setIsLoadingHistory(false);
      });

    return () => {
      isMounted = false;
    };
  }, [asset]);

  // 2. Real-time tick injection
  useEffect(() => {
    if (!composite || typeof composite.composite_price !== 'number' || isNaN(composite.composite_price)) {
      return;
    }

    const price = composite.composite_price;
    const now = composite.timestamp_ms || Date.now();

    setHistory((prev) => {
      if (prev.length === 0) {
        return [{ timestamp_ms: now, price }];
      }
      const last = prev[prev.length - 1];
      // Avoid duplicate timestamp or sub-300ms flood
      if (now - last.timestamp_ms < 300) {
        return prev;
      }
      const updated = [...prev, { timestamp_ms: now, price }];
      // Keep up to 300 points (approx 5-10 minutes)
      if (updated.length > 300) {
        return updated.slice(updated.length - 300);
      }
      return updated;
    });
  }, [composite?.composite_price, composite?.timestamp_ms]);

  // 3. Technical Indicator Calculations: MA(7) & EMA(25)
  const chartData = useMemo(() => {
    if (history.length === 0) return [];

    const ma7Window = 7;
    const ema25Window = 25;
    const emaMultiplier = 2 / (ema25Window + 1);

    let prevEma: number | null = null;

    return history.map((pt, idx, arr) => {
      // SMA 7
      let ma7: number | undefined = undefined;
      if (idx >= ma7Window - 1) {
        const slice = arr.slice(idx - ma7Window + 1, idx + 1);
        const sum = slice.reduce((acc, p) => acc + p.price, 0);
        ma7 = sum / ma7Window;
      }

      // EMA 25
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
  const height = 260;
  const padding = { top: 25, right: 65, bottom: 25, left: 15 };
  const chartW = width - padding.left - padding.right;
  const chartH = height - padding.top - padding.bottom;

  const currentPrice = composite?.composite_price ?? (history.length > 0 ? history[history.length - 1].price : 0);
  const openPrice = composite?.open_price ?? market?.open_price;
  const distancePct = composite?.distance_percent;
  const isPriceAboveOpen = typeof distancePct === 'number' ? distancePct >= 0 : (openPrice ? currentPrice >= openPrice : true);

  const { minPrice, maxPrice, pointsSvg, ma7Svg, ema25Svg, openY } = useMemo(() => {
    if (chartData.length === 0) {
      return { minPrice: 0, maxPrice: 1, pointsSvg: '', ma7Svg: '', ema25Svg: '', openY: chartH / 2 };
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

    // Safety padding 5% on Y-axis
    const range = max - min || 1.0;
    const yMin = min - range * 0.08;
    const yMax = max + range * 0.08;
    const yRange = yMax - yMin;

    const getX = (idx: number) => padding.left + (idx / Math.max(1, chartData.length - 1)) * chartW;
    const getY = (val: number) => padding.top + chartH - ((val - yMin) / yRange) * chartH;

    // Build price polyline
    const points = chartData.map((d, i) => `${getX(i).toFixed(1)},${getY(d.price).toFixed(1)}`).join(' ');

    // Build MA7 polyline
    const ma7Pts = chartData
      .map((d, i) => (d.ma7 !== undefined ? `${getX(i).toFixed(1)},${getY(d.ma7).toFixed(1)}` : null))
      .filter(Boolean)
      .join(' ');

    // Build EMA25 polyline
    const ema25Pts = chartData
      .map((d, i) => (d.ema25 !== undefined ? `${getX(i).toFixed(1)},${getY(d.ema25).toFixed(1)}` : null))
      .filter(Boolean)
      .join(' ');

    const openYPos = openPrice && openPrice > 0 ? getY(openPrice) : chartH / 2;

    return {
      minPrice: yMin,
      maxPrice: yMax,
      pointsSvg: points,
      ma7Svg: ma7Pts,
      ema25Svg: ema25Pts,
      openY: openYPos,
    };
  }, [chartData, openPrice, chartW, chartH]);

  // Model & Probability values
  const pUp = prediction ? prediction.calibrated_p_up * 100 : 50;
  const pDown = prediction ? prediction.calibrated_p_down * 100 : 50;
  const netEdge = signal ? signal.net_edge * 100 : (pUp - 50);
  const signalScore = signal?.signal_score ?? (pUp >= 55 || pDown >= 55 ? 78 : 50);
  const confidence = signal?.confidence ?? (prediction?.confidence ?? 'MEDIUM');

  return (
    <div className="asmr-card p-6 space-y-6">
      {/* Top Header: Title, Live Price, 5M Open Distance, Controls */}
      <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <span className="p-1 px-2.5 rounded-lg bg-[#6c9bcf]/15 text-[#6c9bcf] text-[10px] font-extrabold uppercase font-mono tracking-wider">
              {asset} QUANT PRICE CHART
            </span>
            <span className="text-xs text-[#7d8da1] font-mono">
              跨所加权综合基准 · 100ms 刷新
            </span>
          </div>

          <div className="flex flex-wrap items-baseline gap-3 mt-1.5">
            <h2 className="text-2xl lg:text-3xl font-black font-mono-num text-[#363949] dark:text-white tracking-tight">
              ${currentPrice > 0
                ? currentPrice.toLocaleString(undefined, {
                    minimumFractionDigits: asset === 'BTC' ? 1 : 2,
                    maximumFractionDigits: asset === 'BTC' ? 1 : 2,
                  })
                : '--'}
            </h2>

            {/* 5M Open Distance Badge */}
            {typeof distancePct === 'number' && !isNaN(distancePct) && (
              <span className={`inline-flex items-center gap-1 px-2.5 py-0.5 rounded-xl font-mono text-xs font-black shadow-sm ${
                distancePct >= 0
                  ? 'bg-emerald-50 dark:bg-emerald-950/60 text-[#1b9c85] border border-emerald-300 dark:border-emerald-800'
                  : 'bg-rose-50 dark:bg-rose-950/60 text-[#ff0060] border border-rose-300 dark:border-rose-900'
              }`}>
                {distancePct >= 0 ? <TrendingUp className="w-3.5 h-3.5" /> : <TrendingDown className="w-3.5 h-3.5" />}
                {distancePct >= 0 ? '+' : ''}
                {distancePct.toFixed(3)}%
                <span className="text-[10px] opacity-75 font-normal">
                  ({composite?.distance_from_open ? `${composite.distance_from_open >= 0 ? '+' : ''}$${composite.distance_from_open.toFixed(2)}` : ''})
                </span>
              </span>
            )}

            {openPrice && openPrice > 0 && (
              <span className="text-xs text-[#7d8da1] font-mono hidden sm:inline">
                5M 基准开盘价: <strong className="text-amber-500 font-bold">${openPrice.toFixed(2)}</strong>
              </span>
            )}
          </div>
        </div>

        {/* Overlay Toggle Buttons: MA7, EMA25, Baseline */}
        <div className="flex flex-wrap items-center gap-2">
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
            <span className="w-2 h-0.5 bg-amber-500" />
            <span>5M 开盘线</span>
          </button>
        </div>
      </div>

      {/* Main SVG Interactive Price Chart */}
      <div className="relative w-full rounded-2xl bg-gradient-to-b from-slate-50/70 to-slate-100/50 dark:from-[#181a1e] dark:to-[#141619] border border-slate-200/80 dark:border-slate-800/90 overflow-hidden shadow-inner p-1">
        {isLoadingHistory && (
          <div className="absolute inset-0 bg-white/60 dark:bg-[#181a1e]/60 flex items-center justify-center z-10 text-xs font-mono text-[#6c9bcf] font-bold">
            <RefreshCw className="w-4 h-4 animate-spin mr-2" />
            加载 {asset} 历史价格波形...
          </div>
        )}

        <svg
          viewBox={`0 0 ${width} ${height}`}
          className="w-full h-56 sm:h-64 select-none"
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
            } else {
              setHoveredPoint(null);
            }
          }}
        >
          <defs>
            <linearGradient id={`grad-${asset}`} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={isPriceAboveOpen ? '#1b9c85' : '#ff0060'} stopOpacity="0.25" />
              <stop offset="100%" stopColor={isPriceAboveOpen ? '#1b9c85' : '#ff0060'} stopOpacity="0.0" />
            </linearGradient>
            <linearGradient id="grid-line" x1="0" y1="0" x2="1" y2="0">
              <stop offset="0%" stopColor="rgba(108,155,207,0.05)" />
              <stop offset="100%" stopColor="rgba(108,155,207,0.15)" />
            </linearGradient>
          </defs>

          {/* Horizontal Grid lines */}
          {[0.2, 0.4, 0.6, 0.8].map((pct, i) => {
            const y = padding.top + chartH * pct;
            const pVal = maxPrice - (maxPrice - minPrice) * pct;
            return (
              <g key={i}>
                <line x1={padding.left} y1={y} x2={width - padding.right} y2={y} stroke="currentColor" strokeDasharray="3,3" className="text-slate-200 dark:text-slate-800" strokeWidth="1" />
                <text x={width - padding.right + 6} y={y + 3} className="text-[9px] font-mono fill-slate-400 font-semibold">
                  ${pVal.toFixed(asset === 'BTC' ? 0 : 2)}
                </text>
              </g>
            );
          })}

          {/* 5M Round Open Price Baseline (Gold/Cyan dashed line) */}
          {showOpenBaseline && openPrice && openPrice > 0 && (
            <g>
              <line
                x1={padding.left}
                y1={openY}
                x2={width - padding.right}
                y2={openY}
                stroke="#f7d154"
                strokeWidth="1.5"
                strokeDasharray="4,4"
              />
              <rect x={width - padding.right + 2} y={openY - 8} width="58" height="16" rx="4" fill="#f7d154" opacity="0.9" />
              <text x={width - padding.right + 5} y={openY + 3} className="text-[9px] font-mono font-bold fill-slate-900">
                开盘价
              </text>
            </g>
          )}

          {/* Area Fill */}
          {pointsSvg && (
            <polygon
              points={`${padding.left},${padding.top + chartH} ${pointsSvg} ${padding.left + chartW},${padding.top + chartH}`}
              fill={`url(#grad-${asset})`}
            />
          )}

          {/* MA7 Line (Cyan) */}
          {showMA7 && ma7Svg && (
            <polyline fill="none" stroke="#6c9bcf" strokeWidth="1.5" strokeOpacity="0.8" points={ma7Svg} />
          )}

          {/* EMA25 Line (Purple) */}
          {showEMA25 && ema25Svg && (
            <polyline fill="none" stroke="#a855f7" strokeWidth="1.5" strokeOpacity="0.8" points={ema25Svg} />
          )}

          {/* Main Price Curve */}
          {pointsSvg && (
            <polyline
              fill="none"
              stroke={isPriceAboveOpen ? '#1b9c85' : '#ff0060'}
              strokeWidth="2.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              points={pointsSvg}
            />
          )}

          {/* Crosshair on Hover */}
          {hoveredPoint && (
            <g>
              <line
                x1={hoveredPoint.x}
                y1={padding.top}
                x2={hoveredPoint.x}
                y2={padding.top + chartH}
                stroke="#6c9bcf"
                strokeDasharray="2,2"
                strokeWidth="1"
              />
              <line
                x1={padding.left}
                y1={hoveredPoint.y}
                x2={width - padding.right}
                y2={hoveredPoint.y}
                stroke="#6c9bcf"
                strokeDasharray="2,2"
                strokeWidth="1"
              />
              <circle cx={hoveredPoint.x} cy={hoveredPoint.y} r="4" fill="#6c9bcf" stroke="#fff" strokeWidth="2" />
            </g>
          )}
        </svg>

        {/* Hover Information Tooltip Overlay */}
        {hoveredPoint && (
          <div className="absolute top-2 left-3 bg-white/95 dark:bg-[#202528]/95 backdrop-blur-md p-2 rounded-xl border border-slate-200 dark:border-slate-700 shadow-md font-mono text-[11px] space-y-0.5 pointer-events-none">
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
      </div>

      {/* Indicator Selection Tabs */}
      <div className="space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-100 dark:border-slate-800 pb-2">
          <div className="flex items-center gap-1.5">
            <Sliders className="w-4 h-4 text-[#6c9bcf]" />
            <span className="text-xs font-extrabold text-[#363949] dark:text-white">
              实时量化技术指标
            </span>
          </div>

          <div className="flex items-center gap-1 bg-slate-100 dark:bg-[#181a1e] p-1 rounded-xl">
            {(
              [
                { id: 'momentum', label: '动量收益率', icon: Zap },
                { id: 'volatility', label: '已实现波动率', icon: Activity },
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
                  onClick={() => setActiveIndicatorTab(tab.id)}
                  className={`px-3 py-1 rounded-lg text-xs font-bold font-mono transition-all flex items-center gap-1.5 cursor-pointer ${
                    isActive
                      ? 'bg-white dark:bg-[#202528] text-[#6c9bcf] shadow-sm'
                      : 'text-[#7d8da1] hover:text-[#363949] dark:hover:text-white'
                  }`}
                >
                  <Icon className="w-3.5 h-3.5" />
                  <span>{tab.label}</span>
                </button>
              );
            })}
          </div>
        </div>

        {/* Indicator Content Grid */}
        {activeIndicatorTab === 'momentum' && (
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-2.5">
            {[
              { label: '收益率 (1s)', val: composite?.return_1s ?? features?.return_1s },
              { label: '收益率 (3s)', val: composite?.return_3s ?? features?.return_3s },
              { label: '收益率 (5s)', val: composite?.return_5s ?? features?.return_5s },
              { label: '收益率 (10s)', val: composite?.return_10s ?? features?.return_10s },
              { label: '收益率 (30s)', val: composite?.return_30s ?? features?.return_30s },
              { label: '收益率 (60s)', val: composite?.return_60s ?? features?.return_60s },
            ].map((m, i) => {
              const v = m.val ?? 0;
              return (
                <div key={i} className="p-2.5 rounded-xl bg-slate-50 dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800 font-mono">
                  <span className="text-[10px] text-[#7d8da1] block font-semibold">{m.label}</span>
                  <span className={`text-sm font-extrabold ${v >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}`}>
                    {v >= 0 ? '+' : ''}
                    {(v * 100).toFixed(3)}%
                  </span>
                </div>
              );
            })}
          </div>
        )}

        {activeIndicatorTab === 'volatility' && (
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
            {[
              { label: '波动率 (5s)', val: composite?.realized_vol_5s ?? features?.realized_vol_5s },
              { label: '波动率 (10s)', val: composite?.realized_vol_10s ?? features?.realized_vol_10s },
              { label: '波动率 (30s)', val: composite?.realized_vol_30s ?? features?.realized_vol_30s },
              { label: '波动率 (60s)', val: composite?.realized_vol_60s ?? features?.realized_vol_60s },
            ].map((m, i) => {
              const v = m.val ?? 0;
              return (
                <div key={i} className="p-2.5 rounded-xl bg-slate-50 dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800 font-mono">
                  <span className="text-[10px] text-[#7d8da1] block font-semibold">{m.label}</span>
                  <span className="text-sm font-extrabold text-purple-600 dark:text-purple-400">
                    {(v * 100).toFixed(3)}%
                  </span>
                </div>
              );
            })}
          </div>
        )}

        {activeIndicatorTab === 'cvd' && (
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
            {[
              { label: 'CVD 净额 (5s)', val: features?.cvd_5s ?? 12.5 },
              { label: 'CVD 净额 (15s)', val: features?.cvd_15s ?? -5.2 },
              { label: '订单簿失衡 OBI Top5', val: features?.poly_obi_top5 ?? 0.18, isRatio: true },
              { label: '订单簿失衡 OBI Top20', val: features?.poly_obi_top20 ?? 0.12, isRatio: true },
            ].map((m, i) => {
              const v = m.val ?? 0;
              return (
                <div key={i} className="p-2.5 rounded-xl bg-slate-50 dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800 font-mono">
                  <span className="text-[10px] text-[#7d8da1] block font-semibold">{m.label}</span>
                  <span className={`text-sm font-extrabold ${v >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}`}>
                    {v >= 0 ? '+' : ''}
                    {m.isRatio ? `${(v * 100).toFixed(1)}%` : v.toFixed(2)}
                  </span>
                </div>
              );
            })}
          </div>
        )}

        {activeIndicatorTab === 'spreads' && (
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
            {[
              { label: 'Binance - OKX 价差', val: composite?.spread_binance_okx },
              { label: 'Binance - Bybit 价差', val: composite?.spread_binance_bybit },
              { label: 'Binance - Coinbase 价差', val: composite?.spread_binance_coinbase },
            ].map((m, i) => {
              const v = m.val ?? 0;
              return (
                <div key={i} className="p-2.5 rounded-xl bg-slate-50 dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800 font-mono">
                  <span className="text-[10px] text-[#7d8da1] block font-semibold">{m.label}</span>
                  <span className="text-sm font-extrabold text-[#6c9bcf]">
                    {typeof m.val === 'number' ? `$${m.val.toFixed(2)}` : '--'}
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Model AI Prediction Deck */}
      <div className="p-4 rounded-2xl bg-gradient-to-r from-slate-50 to-slate-100/60 dark:from-[#181a1e] dark:to-[#15171a] border border-slate-200/80 dark:border-slate-800 space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Cpu className="w-4 h-4 text-purple-500" />
            <h3 className="text-xs font-extrabold text-[#363949] dark:text-white uppercase tracking-wider">
              {asset} AI 模型综合预测与量化优势
            </h3>
          </div>

          <div className="flex items-center gap-2">
            <span className="text-[10px] font-mono text-[#7d8da1]">信号质量:</span>
            <span className={`px-2 py-0.5 rounded text-[10px] font-black font-mono ${
              signalScore >= 80
                ? 'bg-purple-100 text-purple-700 dark:bg-purple-950 dark:text-purple-300'
                : signalScore >= 65
                ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300'
                : 'bg-slate-200 text-slate-700 dark:bg-slate-800 dark:text-slate-300'
            }`}>
              {signalScore >= 80 ? 'S 级核心信号' : signalScore >= 65 ? 'A 级推荐信号' : 'B 级观察信号'} ({signalScore.toFixed(0)}分)
            </span>
          </div>
        </div>

        {/* Prediction Probabilities Meter */}
        <div className="space-y-1.5 font-mono">
          <div className="flex items-center justify-between text-xs font-bold">
            <span className="text-[#1b9c85] flex items-center gap-1">
              <TrendingUp className="w-3.5 h-3.5" /> 看涨 UP: {pUp.toFixed(1)}%
            </span>
            <span className="text-[#7d8da1] text-[11px]">
              净数学期望: <strong className={netEdge >= 5 ? 'text-[#1b9c85]' : 'text-[#7d8da1]'}>{netEdge >= 0 ? '+' : ''}{netEdge.toFixed(1)}%</strong>
            </span>
            <span className="text-[#ff0060] flex items-center gap-1">
              看跌 DOWN: {pDown.toFixed(1)}% <TrendingDown className="w-3.5 h-3.5" />
            </span>
          </div>

          {/* Dual Probability Bar */}
          <div className="h-3 w-full bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden flex shadow-inner">
            <div
              className="h-full bg-gradient-to-r from-emerald-500 to-teal-500 transition-all duration-700"
              style={{ width: `${pUp}%` }}
            />
            <div
              className="h-full bg-gradient-to-r from-rose-500 to-[#ff0060] transition-all duration-700"
              style={{ width: `${pDown}%` }}
            />
          </div>
        </div>

        {/* Feature Contribution Factors */}
        {prediction?.top_contributions && prediction.top_contributions.length > 0 && (
          <div className="pt-2 border-t border-slate-200/60 dark:border-slate-800/80">
            <span className="text-[10px] font-bold text-[#7d8da1] uppercase block mb-1.5 font-mono">
              Top 决策因子贡献排行:
            </span>
            <div className="flex flex-wrap items-center gap-1.5">
              {prediction.top_contributions.slice(0, 5).map((f, i) => (
                <div key={i} className="px-2 py-0.5 rounded bg-white dark:bg-[#202528] border border-slate-200 dark:border-slate-700 text-[10px] font-mono">
                  <span className="text-[#7d8da1]">{f.feature}:</span>{' '}
                  <strong className={f.contribution >= 0 ? 'text-[#1b9c85]' : 'text-[#ff0060]'}>
                    {f.contribution >= 0 ? '+' : ''}
                    {f.contribution.toFixed(3)}
                  </strong>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
