import React, { useState } from 'react';
import {
  Timer,
  ArrowUpRight,
  ArrowDownRight,
  Zap,
  CheckCircle2,
  AlertCircle,
  Loader2,
  BookmarkCheck,
  PauseCircle,
  ShieldCheck,
  ShieldAlert,
  Wallet,
  SlidersHorizontal,
  ExternalLink,
  TrendingUp,
  TrendingDown,
} from 'lucide-react';
import {
  MarketDisplayInfo,
  MarketBookSummary,
  ModelPrediction,
  Asset,
  TradingMode,
  PaperPosition,
  PolymarketAccountPublic,
  BankrollState,
  CompositePriceSnapshot,
} from '../types';
import { api } from '../services/api';

interface PolymarketRoundCardProps {
  market: MarketDisplayInfo | null;
  book: MarketBookSummary | null;
  prediction: ModelPrediction | null;
  tradingMode?: TradingMode;
  activeAsset?: Asset;
  activePositions?: PaperPosition[];
  isAutoTradingEnabled?: boolean;
  onTradeExecuted?: () => void;
  activeAccount?: PolymarketAccountPublic | null;
  liveBankroll?: BankrollState | null;
  paperBankroll?: BankrollState | null;
  onOpenSetBankroll?: (mode?: 'paper' | 'live') => void;
  onOpenAccountManager?: () => void;
  currentSpotPrice?: number;
  composite?: CompositePriceSnapshot | null;
}

export const PolymarketRoundCard: React.FC<PolymarketRoundCardProps> = ({
  market,
  book,
  prediction,
  tradingMode = 'paper',
  activeAsset = 'BTC',
  activePositions = [],
  isAutoTradingEnabled = true,
  onTradeExecuted,
  activeAccount,
  liveBankroll,
  paperBankroll,
  onOpenSetBankroll,
  onOpenAccountManager,
  currentSpotPrice,
  composite,
}) => {
  const isPaper = tradingMode === 'paper';
  const [stakeInput, setStakeInput] = useState<string>(() => {
    if (isPaper) {
      return localStorage.getItem('polyquant_paper_stake') || '5.0';
    } else {
      return localStorage.getItem('polyquant_live_stake') || '0.5';
    }
  });
  const [isExecuting, setIsExecuting] = useState<boolean>(false);
  const [execStatus, setExecStatus] = useState<{ success: boolean; msg: string } | null>(null);

  // Sync stake input from localStorage when trading mode toggles
  React.useEffect(() => {
    if (isPaper) {
      const saved = localStorage.getItem('polyquant_paper_stake') || '5.0';
      setStakeInput(saved);
    } else {
      const saved = localStorage.getItem('polyquant_live_stake') || '0.5';
      setStakeInput(saved);
    }
  }, [isPaper]);

  const updateStake = (val: string) => {
    setStakeInput(val);
    if (isPaper) {
      localStorage.setItem('polyquant_paper_stake', val);
    } else {
      localStorage.setItem('polyquant_live_stake', val);
    }
  };

  // Financial safety calculations
  const paperBalance = typeof paperBankroll?.active_bankroll === 'number' ? paperBankroll.active_bankroll : 10.0;
  const liveBalance = typeof activeAccount?.balance_usdc === 'number'
    ? activeAccount.balance_usdc
    : (typeof liveBankroll?.active_bankroll === 'number' ? liveBankroll.active_bankroll : 0.0);
  const minFloor = typeof liveBankroll?.minimum_bankroll === 'number'
    ? liveBankroll.minimum_bankroll
    : (typeof liveBankroll?.min_floor === 'number' ? liveBankroll.min_floor : 0.0);
  const maxUsableLive = Math.max(0, liveBalance - minFloor);

  const parsedStake = parseFloat(stakeInput);
  const isValidStake = !isNaN(parsedStake) && parsedStake > 0;

  // Handler to instantly reset live floor to 0 so the entire balance can be used
  const handleResetFloorToZero = async () => {
    try {
      setIsExecuting(true);
      await api.setBankrollFunds(
        {
          active_bankroll: liveBalance,
          minimum_bankroll: 0,
        },
        'live'
      );
      setExecStatus({
        success: true,
        msg: '✅ 已一键将实盘安全底线调整为 $0.00，全部链上资金现已可用于交易！',
      });
      if (onTradeExecuted) {
        onTradeExecuted();
      }
    } catch (err: any) {
      setExecStatus({ success: false, msg: `调整底线失败: ${err.message || '网络异常'}` });
    } finally {
      setIsExecuting(false);
    }
  };

  // Real-time pre-flight validation check
  let preflightError: string | null = null;
  let canOneClickResetFloor = false;
  if (!isPaper) {
    if (!activeAccount) {
      preflightError = '未检测到绑定的实盘账户，请先添加并激活账户';
    } else if (!isValidStake) {
      preflightError = '请输入大于 0 的有效下单金额';
    } else if (parsedStake > maxUsableLive) {
      if (parsedStake <= liveBalance && minFloor > 0) {
        preflightError = `下注金额 ($${parsedStake.toFixed(2)}) 超过可用额度 ($${maxUsableLive.toFixed(2)})！(余额 $${liveBalance.toFixed(2)} - 安全底线 $${minFloor.toFixed(2)})`;
        canOneClickResetFloor = true;
      } else {
        preflightError = `下注金额 ($${parsedStake.toFixed(2)}) 超过账户总余额 ($${liveBalance.toFixed(2)})！`;
      }
    }
  } else {
    if (!isValidStake) {
      preflightError = '请输入大于 0 的有效下单金额';
    } else if (parsedStake > paperBalance) {
      preflightError = `下注金额 ($${parsedStake.toFixed(2)}) 超过模拟盘可用本金 ($${paperBalance.toFixed(2)})`;
    }
  }

  // Check if there is an active OPEN position for this round / asset
  const currentPosition = activePositions.find(
    (p) => p.status === 'OPEN' && (p.market_id === market?.id || p.asset === activeAsset)
  );

  const handleManualOrder = async (side: 'UP' | 'DOWN') => {
    if (!market) {
      setExecStatus({ success: false, msg: '暂无当前市场信息' });
      return;
    }

    if (preflightError) {
      setExecStatus({ success: false, msg: preflightError });
      return;
    }

    const finalStake = parsedStake;

    if (!isPaper) {
      const confirmed = window.confirm(
        `⚡ 实盘真金下单确认：\n\n账户: ${activeAccount?.label || '默认'}\n标的: ${activeAsset} 5M\n方向: ${side === 'UP' ? '看涨 (UP)' : '看跌 (DOWN)'}\n下注金额: $${finalStake.toFixed(2)} USDC\n当前账户余额: $${liveBalance.toFixed(2)} USDC\n\n该操作将以真实资金向 Polymarket CLOB 发送真实订单，确认提交？`
      );
      if (!confirmed) return;
    }

    try {
      setIsExecuting(true);
      setExecStatus(null);
      const res = await api.executeManualTrade({
        market_id: market.id,
        asset: activeAsset,
        side,
        stake: finalStake,
        mode: tradingMode,
      });

      if (res.success) {
        setExecStatus({
          success: true,
          msg: res.message || `${isPaper ? '🎮 模拟' : '⚡ 实盘'}下单成功！(${side} $${finalStake.toFixed(2)})`,
        });
        if (onTradeExecuted) {
          onTradeExecuted();
        }
      } else {
        setExecStatus({ success: false, msg: res.message || '下单失败' });
      }
    } catch (err: any) {
      setExecStatus({ success: false, msg: err.message || '网络异常，下单失败' });
    } finally {
      setIsExecuting(false);
      setTimeout(() => {
        setExecStatus((prev) => (prev?.success ? null : prev));
      }, 7000);
    }
  };
  const remainingSecs = market?.remaining_seconds ?? 0;
  const progressPct = Math.max(0, Math.min(100, ((300 - remainingSecs) / 300) * 100));

  const mins = Math.floor(remainingSecs / 60);
  const secs = remainingSecs % 60;
  const timerStr = `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;

  const polymarketSlug = market?.slug || `${activeAsset.toLowerCase()}-updown-5m`;
  const officialPolymarketUrl = `https://polymarket.com/event/${polymarketSlug}`;

  // Price to Beat benchmark calculations
  const openPrice = market?.open_price ?? composite?.open_price;
  const polyPrice = market?.poly_current_price ?? composite?.poly_current_price;
  const spotPrice = composite?.composite_price ?? currentSpotPrice;
  const effectiveCurrentPrice = (polyPrice && polyPrice > 0) ? polyPrice : spotPrice;
  const priceDiff = (typeof openPrice === 'number' && typeof effectiveCurrentPrice === 'number' && openPrice > 0)
    ? effectiveCurrentPrice - openPrice
    : null;
  const priceDiffPct = (priceDiff !== null && openPrice && openPrice > 0)
    ? (priceDiff / openPrice) * 100
    : null;
  const isUpWinning = priceDiff !== null ? priceDiff >= 0 : null;

  // Basis between 4-exchange spot and Poly official
  const basis = (typeof spotPrice === 'number' && typeof polyPrice === 'number' && spotPrice > 0 && polyPrice > 0)
    ? spotPrice - polyPrice
    : null;

  const impliedUp = (book?.implied_prob_up ?? market?.up_price ?? 0.50) * 100;
  const impliedDown = (book?.implied_prob_down ?? market?.down_price ?? 0.50) * 100;

  const modelUp = (prediction?.calibrated_p_up ?? 0.50) * 100;
  const modelDown = (prediction?.calibrated_p_down ?? 0.50) * 100;

  const upBid = book?.up_book?.best_bid ?? market?.up_bid ?? market?.up_price ?? 0.50;
  const upAsk = book?.up_book?.best_ask ?? market?.up_ask ?? (upBid + 0.01);
  const downBid = book?.down_book?.best_bid ?? market?.down_bid ?? market?.down_price ?? 0.50;
  const downAsk = book?.down_book?.best_ask ?? market?.down_ask ?? (downBid + 0.01);

  const spreadUp = book?.up_book?.spread ?? (upAsk - upBid);
  const totalLiquidity = (book?.up_book?.total_bid_depth_usdc ?? 500) + (book?.down_book?.total_bid_depth_usdc ?? 500);

  return (
    <div className="asmr-card p-6 h-full flex flex-col justify-between">
      {/* Header: Market Question & 5M Countdown Timer */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-3">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-[10px] font-extrabold bg-[#6c9bcf]/15 text-[#6c9bcf] px-2.5 py-0.5 rounded-full uppercase tracking-wider font-mono">
              Polymarket 5M 官方期权
            </span>
            <span className="text-xs text-[#7d8da1] dark:text-slate-400 font-mono">
              ID: {market?.id ? market.id.slice(0, 16) : '等待市场中...'}
            </span>
            <a
              href={officialPolymarketUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[11px] font-bold bg-[#6c9bcf]/15 text-[#6c9bcf] hover:bg-[#6c9bcf]/25 hover:text-blue-500 transition-all cursor-pointer"
              title="在 Polymarket 官网打开本轮合约盘口"
            >
              <span>官网原盘 ↗</span>
              <ExternalLink className="h-3 w-3" />
            </a>
          </div>
          <h3 className="text-base font-extrabold text-[#363949] dark:text-white mt-1.5 tracking-tight flex items-center gap-2">
            {market?.question ?? `${activeAsset} 5 分钟涨跌交割合约`}
          </h3>
        </div>

        {/* 5-Minute Round Countdown */}
        <div className="bg-[#f6f6f9] dark:bg-[#181a1e] p-3 rounded-2xl min-w-[190px] border border-slate-100 dark:border-slate-800">
          <div className="flex items-center justify-between text-xs mb-1.5 font-semibold">
            <span className="text-[#7d8da1] dark:text-slate-400 flex items-center gap-1.5">
              <Timer className="h-3.5 w-3.5 text-[#6c9bcf]" />
              交割窗口:
            </span>
            <span
              className={`font-mono-num font-black text-sm ${
                remainingSecs <= 30
                  ? 'text-[#ff0060] animate-pulse'
                  : remainingSecs <= 60
                  ? 'text-[#f7d154]'
                  : 'text-[#1b9c85]'
              }`}
            >
              {timerStr}
            </span>
          </div>
          <div className="h-2 w-full bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
            <div
              className={`h-full transition-all duration-1000 ${
                remainingSecs <= 30
                  ? 'bg-[#ff0060]'
                  : remainingSecs <= 60
                  ? 'bg-[#f7d154]'
                  : 'bg-[#1b9c85]'
              }`}
              style={{ width: `${progressPct}%` }}
            />
          </div>
        </div>
      </div>

      {/* Official Benchmark Strip: Price to Beat vs Poly Official vs Multi-Exchange Spot */}
      <div className="mb-3.5 p-3 rounded-2xl bg-slate-50 dark:bg-[#181a1e] border border-slate-200/80 dark:border-slate-800 flex flex-wrap items-center justify-between gap-3 text-xs">
        <div className="flex flex-wrap items-center gap-4">
          <div className="flex items-center gap-1.5">
            <span className="text-[#7d8da1] dark:text-slate-400 font-medium">
              🎯 目标基准:
            </span>
            <span className="font-mono-num font-extrabold text-[#363949] dark:text-white text-sm">
              {typeof openPrice === 'number' && openPrice > 0 ? `$${openPrice.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}` : '获取基准中...'}
            </span>
          </div>

          <div className="flex items-center gap-1.5 border-l border-slate-200 dark:border-slate-700 pl-3">
            <span className="text-amber-500 font-medium flex items-center gap-1">
              ⚡ Poly官方现价:
            </span>
            <span className="font-mono-num font-extrabold text-sm text-amber-500 dark:text-amber-400">
              {typeof polyPrice === 'number' && polyPrice > 0
                ? `$${polyPrice.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
                : (typeof spotPrice === 'number' && spotPrice > 0 ? `$${spotPrice.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}` : '--')}
            </span>
          </div>

          <div className="flex items-center gap-1.5 border-l border-slate-200 dark:border-slate-700 pl-3">
            <span className="text-[#6c9bcf] font-medium">
              🌐 4所综合现货:
            </span>
            <span className="font-mono-num font-extrabold text-sm text-[#363949] dark:text-slate-200">
              {typeof spotPrice === 'number' && spotPrice > 0 ? `$${spotPrice.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}` : '--'}
            </span>
            {basis !== null && (
              <span className="text-[10px] font-mono font-bold px-1.5 py-0.2 rounded bg-blue-500/10 text-blue-600 dark:text-blue-400">
                基差 {basis >= 0 ? '+' : ''}${basis.toFixed(2)}
              </span>
            )}
          </div>
        </div>

        <div className="flex items-center gap-3 font-mono">
          {priceDiff !== null && priceDiffPct !== null && (
            <div className={`flex items-center gap-1 px-2.5 py-1 rounded-xl text-xs font-black ${
              isUpWinning
                ? 'bg-emerald-500/15 text-[#1b9c85]'
                : 'bg-rose-500/15 text-[#ff0060]'
            }`}>
              {isUpWinning ? <TrendingUp className="h-3.5 w-3.5" /> : <TrendingDown className="h-3.5 w-3.5" />}
              <span>{priceDiff >= 0 ? '+' : ''}${priceDiff.toFixed(2)} ({priceDiffPct >= 0 ? '+' : ''}{priceDiffPct.toFixed(3)}%)</span>
              <span className="text-[10px] px-1 py-0.2 bg-white/40 dark:bg-black/20 rounded ml-1 font-bold">
                {isUpWinning ? '看涨 (UP) 胜出' : '看跌 (DOWN) 胜出'}
              </span>
            </div>
          )}
        </div>
      </div>

      {/* Mode Status Strip: Live Cockpit vs Paper Sandbox */}
      {!isPaper ? (
        activeAccount ? (
          <div className="mb-4 p-2.5 px-3.5 rounded-2xl bg-rose-500/10 border border-rose-500/30 flex flex-wrap items-center justify-between gap-2 text-xs">
            <div className="flex items-center gap-2">
              <span className="relative flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[#ff0060] opacity-75"></span>
                <span className="relative inline-flex rounded-full h-2 w-2 bg-[#ff0060]"></span>
              </span>
              <span className="font-extrabold text-[#ff0060] flex items-center gap-1">
                <Zap className="h-3.5 w-3.5" /> 实盘 CLOB 撮合
              </span>
              <span className="text-[#7d8da1]">|</span>
              <span className="text-[#363949] dark:text-white font-medium">
                账户: <strong className="text-[#6c9bcf] font-bold">{activeAccount.label}</strong>
              </span>
            </div>
            <div className="flex items-center gap-3 font-mono">
              <span className="text-[#7d8da1] dark:text-slate-400">
                链上余额: <strong className="text-[#1b9c85] font-extrabold">${liveBalance.toFixed(2)}</strong> USDC
              </span>
              <span className="text-[#7d8da1] dark:text-slate-400">
                安全底线: <strong className="text-amber-500 font-bold">${minFloor.toFixed(2)}</strong>
              </span>
              <span className="text-[#363949] dark:text-white font-bold">
                可用额度: <strong className="text-[#1b9c85]">${maxUsableLive.toFixed(2)}</strong>
              </span>
              {onOpenSetBankroll && (
                <button
                  type="button"
                  onClick={() => onOpenSetBankroll('live')}
                  className="text-[11px] text-[#6c9bcf] hover:underline cursor-pointer flex items-center gap-0.5"
                  title="修改实盘安全底线与风控参数"
                >
                  <SlidersHorizontal className="h-3 w-3" /> 调整底线
                </button>
              )}
            </div>
          </div>
        ) : (
          <div className="mb-4 p-3 rounded-2xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-between text-xs">
            <div className="flex items-center gap-2">
              <AlertCircle className="h-4 w-4 text-amber-500 shrink-0" />
              <span className="font-bold text-amber-700 dark:text-amber-400">
                当前处于实盘交易模式，但尚未绑定或激活实盘账户，无法向 Polymarket CLOB 发送真实订单
              </span>
            </div>
            {onOpenAccountManager && (
              <button
                type="button"
                onClick={onOpenAccountManager}
                className="px-3 py-1 bg-amber-500 hover:bg-amber-600 text-white font-bold rounded-xl shadow-sm text-xs cursor-pointer whitespace-nowrap ml-2"
              >
                立即绑定账户
              </button>
            )}
          </div>
        )
      ) : (
        <div className="mb-4 p-2.5 px-3.5 rounded-2xl bg-emerald-500/10 border border-emerald-500/25 flex flex-wrap items-center justify-between gap-2 text-xs">
          <div className="flex items-center gap-2">
            <ShieldCheck className="h-4 w-4 text-[#1b9c85]" />
            <span className="font-extrabold text-[#1b9c85]">
              🎮 模拟演练沙盒
            </span>
            <span className="text-[#7d8da1] dark:text-slate-400">
              零真金风险 · 本地撮合验证模型预期
            </span>
          </div>
          <div className="flex items-center gap-3 font-mono">
            <span className="text-[#7d8da1] dark:text-slate-400">
              模拟可用资金: <strong className="text-[#1b9c85] font-extrabold">${paperBalance.toFixed(2)}</strong> USDC
            </span>
            {onOpenSetBankroll && (
              <button
                type="button"
                onClick={() => onOpenSetBankroll('paper')}
                className="text-[11px] text-[#6c9bcf] hover:underline cursor-pointer flex items-center gap-0.5"
                title="调整模拟盘本金预设"
              >
                <SlidersHorizontal className="h-3 w-3" /> 设置本金
              </button>
            )}
          </div>
        </div>
      )}

      {/* Main Orderbook Prices & Microstructure Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* UP Side Book */}
        <div className="p-4 rounded-2xl border border-[#1b9c85]/20 bg-[#1b9c85]/5">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-extrabold text-[#1b9c85] flex items-center gap-1 uppercase tracking-wider">
              <ArrowUpRight className="h-4 w-4" /> 看涨合约 (UP)
            </span>
            <span className="text-[11px] text-[#7d8da1] font-mono">
              盘口胜率: <strong className="text-[#1b9c85]">{(typeof impliedUp === 'number' && !isNaN(impliedUp) ? impliedUp : 50).toFixed(1)}%</strong>
            </span>
          </div>

          <div className="grid grid-cols-2 gap-2.5 mb-3">
            <div className="bg-white dark:bg-[#202528] p-2.5 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">买一价 (Bid)</span>
              <span className="text-base font-mono-num font-extrabold text-[#1b9c85]">
                ${(typeof upBid === 'number' && !isNaN(upBid) ? upBid : 0.5).toFixed(3)}
              </span>
            </div>
            <div className="bg-white dark:bg-[#202528] p-2.5 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">卖一价 (Ask)</span>
              <span className="text-base font-mono-num font-extrabold text-[#363949] dark:text-white">
                ${(typeof upAsk === 'number' && !isNaN(upAsk) ? upAsk : 0.51).toFixed(3)}
              </span>
            </div>
          </div>

          <div className="text-xs space-y-1 font-medium">
            <div className="flex justify-between text-[#7d8da1] dark:text-slate-400">
              <span>模型胜率:</span>
              <span className="font-mono-num font-bold text-[#363949] dark:text-white">{(typeof modelUp === 'number' && !isNaN(modelUp) ? modelUp : 50).toFixed(1)}%</span>
            </div>
            <div className="flex justify-between text-[#7d8da1] dark:text-slate-400">
              <span>净数学期望:</span>
              <span
                className={`font-mono-num font-bold ${
                  modelUp / 100 - upAsk >= 0.05 ? 'text-[#1b9c85]' : 'text-[#7d8da1]'
                }`}
              >
                {(((typeof modelUp === 'number' ? modelUp : 50) / 100 - (typeof upAsk === 'number' ? upAsk : 0.51)) * 100).toFixed(1)}%
              </span>
            </div>
          </div>
        </div>

        {/* DOWN Side Book */}
        <div className="p-4 rounded-2xl border border-[#ff0060]/20 bg-[#ff0060]/5">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-extrabold text-[#ff0060] flex items-center gap-1 uppercase tracking-wider">
              <ArrowDownRight className="h-4 w-4" /> 看跌合约 (DOWN)
            </span>
            <span className="text-[11px] text-[#7d8da1] font-mono">
              盘口胜率: <strong className="text-[#ff0060]">{(typeof impliedDown === 'number' && !isNaN(impliedDown) ? impliedDown : 50).toFixed(1)}%</strong>
            </span>
          </div>

          <div className="grid grid-cols-2 gap-2.5 mb-3">
            <div className="bg-white dark:bg-[#202528] p-2.5 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">买一价 (Bid)</span>
              <span className="text-base font-mono-num font-extrabold text-[#ff0060]">
                ${(typeof downBid === 'number' && !isNaN(downBid) ? downBid : 0.5).toFixed(3)}
              </span>
            </div>
            <div className="bg-white dark:bg-[#202528] p-2.5 rounded-xl shadow-sm border border-slate-100 dark:border-slate-800">
              <span className="text-[10px] text-[#7d8da1] block font-semibold">卖一价 (Ask)</span>
              <span className="text-base font-mono-num font-extrabold text-[#363949] dark:text-white">
                ${(typeof downAsk === 'number' && !isNaN(downAsk) ? downAsk : 0.51).toFixed(3)}
              </span>
            </div>
          </div>

          <div className="text-xs space-y-1 font-medium">
            <div className="flex justify-between text-[#7d8da1] dark:text-slate-400">
              <span>模型胜率:</span>
              <span className="font-mono-num font-bold text-[#363949] dark:text-white">{(typeof modelDown === 'number' && !isNaN(modelDown) ? modelDown : 50).toFixed(1)}%</span>
            </div>
            <div className="flex justify-between text-[#7d8da1] dark:text-slate-400">
              <span>净数学期望:</span>
              <span
                className={`font-mono-num font-bold ${
                  modelDown / 100 - downAsk >= 0.05 ? 'text-[#1b9c85]' : 'text-[#7d8da1]'
                }`}
              >
                {(((typeof modelDown === 'number' ? modelDown : 50) / 100 - (typeof downAsk === 'number' ? downAsk : 0.51)) * 100).toFixed(1)}%
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Manual Quick Trade Panel: Separated Paper vs Live Experience */}
      <div className={`mt-4 p-4 rounded-2xl border space-y-3 transition-colors ${
        isPaper
          ? 'bg-slate-50 dark:bg-[#181a1e] border-slate-200/70 dark:border-slate-800'
          : 'bg-rose-50/30 dark:bg-rose-950/10 border-rose-200/50 dark:border-rose-900/30'
      }`}>
        {/* Active Open Position Badge (If User Already Has a Position In This Round) */}
        {currentPosition ? (
          <div className="p-2.5 rounded-xl bg-emerald-500/10 border border-emerald-500/30 dark:bg-emerald-950/30 flex items-center justify-between text-xs">
            <div className="flex items-center gap-2">
              <BookmarkCheck className="h-4 w-4 text-[#1b9c85] shrink-0" />
              <div>
                <span className={`px-2 py-0.5 rounded-md font-extrabold text-[10px] mr-1.5 ${
                  currentPosition.side === 'UP'
                    ? 'bg-[#1b9c85] text-white'
                    : 'bg-[#ff0060] text-white'
                }`}>
                  本轮已开仓: {currentPosition.side === 'UP' ? '看涨 UP' : '看跌 DOWN'}
                </span>
                <span className="font-mono font-bold text-[#363949] dark:text-white">
                  ${currentPosition.stake.toFixed(2)} USDC ({currentPosition.shares.toFixed(2)} 股 @ ${currentPosition.entry_price.toFixed(3)})
                </span>
              </div>
            </div>
            <span className="text-[11px] text-[#1b9c85] font-bold font-mono">
              等待 5M 窗口交割
            </span>
          </div>
        ) : null}

        {/* Informational Banner if Global Auto-Trading is Paused */}
        {!isAutoTradingEnabled && (
          <div className="p-2 px-3 rounded-xl bg-amber-500/10 border border-amber-500/25 text-[11px] text-amber-700 dark:text-amber-300 flex items-center gap-1.5">
            <PauseCircle className="h-3.5 w-3.5 text-amber-500 shrink-0" />
            <span>{isPaper ? '模拟盘' : '实盘'}自动下单已暂停，您仍可通过下方按钮随时手动快速下注</span>
          </div>
        )}

        {/* Top Control Bar: Mode Title & Quick Stake Selectors */}
        <div className="flex flex-wrap items-center justify-between gap-2.5">
          <div className="flex items-center gap-2">
            <div className={`p-1.5 rounded-xl ${isPaper ? 'bg-emerald-500/15 text-[#1b9c85]' : 'bg-rose-500/15 text-[#ff0060]'}`}>
              <Zap className="h-4 w-4" />
            </div>
            <div>
              <div className="flex items-center gap-1.5">
                <span className="text-xs font-bold text-[#363949] dark:text-white">
                  {isPaper ? '🎮 模拟盘快速下单' : '⚡ 实盘 CLOB 极速下单'}
                </span>
                <span className="text-[10px] text-[#7d8da1] font-mono">
                  ({isPaper ? '无真金风险' : '真实链上资金'})
                </span>
              </div>
              <p className="text-[10px] text-[#7d8da1] font-mono">
                {isPaper
                  ? `模拟本金: $${paperBalance.toFixed(2)} USDC`
                  : (activeAccount
                      ? `可用额度: $${maxUsableLive.toFixed(2)} USDC (总余额 $${liveBalance.toFixed(2)} - 底线 $${minFloor.toFixed(2)})`
                      : '请先绑定并激活实盘账户')}
              </p>
            </div>
          </div>

          {/* Quick Stake Selector & Custom Input */}
          <div className="flex flex-wrap items-center gap-1.5">
            <div className="flex items-center gap-1">
              {(isPaper
                ? [1, 2, 5, 10, 20]
                : (maxUsableLive <= 2.0
                    ? [0.5, 1.0]
                    : maxUsableLive <= 5.0
                    ? [0.5, 1.0, 2.0, 5.0]
                    : [0.5, 1.0, 2.0, 5.0, 10.0])
              ).map((amt) => (
                <button
                  key={amt}
                  type="button"
                  onClick={() => updateStake(amt.toString())}
                  className={`px-2 py-1 text-[11px] font-mono font-bold rounded-lg border transition cursor-pointer ${
                    parseFloat(stakeInput) === amt
                      ? (isPaper ? 'bg-[#1b9c85] text-white border-[#1b9c85] shadow-sm' : 'bg-[#ff0060] text-white border-[#ff0060] shadow-sm')
                      : 'bg-white dark:bg-[#202528] text-[#7d8da1] border-slate-200 dark:border-slate-700 hover:text-[#363949]'
                  }`}
                >
                  ${amt}
                </button>
              ))}

              {/* In Live Mode: One-click "Max Usable" button */}
              {!isPaper && maxUsableLive > 0 && (
                <button
                  type="button"
                  onClick={() => updateStake(maxUsableLive.toFixed(2))}
                  className={`px-2 py-1 text-[11px] font-mono font-bold rounded-lg border transition cursor-pointer ${
                    parseFloat(stakeInput) === maxUsableLive
                      ? 'bg-amber-500 text-white border-amber-500 shadow-sm'
                      : 'bg-amber-50 dark:bg-amber-950/40 text-amber-600 dark:text-amber-400 border-amber-300 dark:border-amber-800 hover:bg-amber-100'
                  }`}
                  title="一键填入实盘最大安全可用额度"
                >
                  全部可用 (${maxUsableLive.toFixed(2)})
                </button>
              )}
            </div>

            {/* Custom Stake Input Box */}
            <div className="relative flex items-center">
              <span className="absolute left-2.5 text-[11px] font-mono font-semibold text-[#7d8da1]">$</span>
              <input
                type="number"
                step="any"
                min="0.1"
                placeholder="自定义"
                value={stakeInput}
                onChange={(e) => updateStake(e.target.value)}
                className={`w-20 pl-5 pr-2 py-1 text-[11px] font-mono font-bold rounded-lg border bg-white dark:bg-[#202528] text-[#363949] dark:text-white focus:outline-none ${
                  preflightError
                    ? 'border-rose-400 focus:border-rose-500'
                    : 'border-slate-200 dark:border-slate-700 focus:border-[#6c9bcf]'
                }`}
                title="输入自定义下注金额 (USDC)"
              />
            </div>
          </div>
        </div>

        {/* Real-time Preflight Validation or Clearance Alert */}
        {preflightError ? (
          <div className="p-2.5 px-3 rounded-xl bg-rose-500/10 border border-rose-500/30 text-xs text-[#ff0060] flex flex-wrap items-center justify-between gap-2 font-bold animate-fade-in">
            <div className="flex items-center gap-2">
              <AlertCircle className="h-4 w-4 shrink-0" />
              <span>{preflightError}</span>
            </div>
            {canOneClickResetFloor && (
              <button
                type="button"
                onClick={handleResetFloorToZero}
                disabled={isExecuting}
                className="px-2.5 py-1 bg-amber-500 hover:bg-amber-600 text-white rounded-lg text-xs font-extrabold shadow-sm transition flex items-center gap-1 cursor-pointer whitespace-nowrap ml-auto"
                title="立即将实盘风控底线设置为 0，释放全部钱包余额用于交易"
              >
                <Zap className="h-3 w-3" />
                <span>⚡ 一键将底线设为 $0 (释放可用 ${liveBalance.toFixed(2)})</span>
              </button>
            )}
          </div>
        ) : !isPaper && isValidStake ? (
          <div className="p-2 px-3 rounded-xl bg-slate-100 dark:bg-[#202528] border border-slate-200/80 dark:border-slate-800 text-[11px] text-[#7d8da1] font-mono flex items-center justify-between">
            <span>
              预计下单后剩余可用: <strong className="text-[#363949] dark:text-white font-bold">${(maxUsableLive - parsedStake).toFixed(2)} USDC</strong>
            </span>
            <span className="text-[#1b9c85] font-bold">✓ 满足安全余量要求 (安全底线: ${minFloor.toFixed(2)})</span>
          </div>
        ) : null}

        {/* Action Buttons: Buy UP / Buy DOWN (Strictly Differentiated between Paper and Live) */}
        {isPaper ? (
          <div className="grid grid-cols-2 gap-2.5">
            <button
              type="button"
              disabled={isExecuting || !market || !!preflightError}
              onClick={() => handleManualOrder('UP')}
              className="flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl bg-emerald-600 hover:bg-emerald-700 text-white font-extrabold text-xs shadow-sm transition disabled:opacity-40 cursor-pointer"
            >
              {isExecuting ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <ArrowUpRight className="h-4 w-4" />
              )}
              <span>🎮 模拟买入 看涨 UP (${isValidStake ? parsedStake.toFixed(2) : '5.00'})</span>
            </button>

            <button
              type="button"
              disabled={isExecuting || !market || !!preflightError}
              onClick={() => handleManualOrder('DOWN')}
              className="flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl bg-[#ff0060] hover:bg-[#e00055] text-white font-extrabold text-xs shadow-sm transition disabled:opacity-40 cursor-pointer"
            >
              {isExecuting ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <ArrowDownRight className="h-4 w-4" />
              )}
              <span>🎮 模拟买入 看跌 DOWN (${isValidStake ? parsedStake.toFixed(2) : '5.00'})</span>
            </button>
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-2.5">
            <button
              type="button"
              disabled={isExecuting || !market || !!preflightError || !activeAccount}
              onClick={() => handleManualOrder('UP')}
              className="flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-700 hover:to-teal-700 text-white font-extrabold text-xs shadow-md shadow-emerald-600/20 transition disabled:opacity-40 disabled:from-slate-400 disabled:to-slate-400 cursor-pointer"
              title={preflightError || '向 Polymarket CLOB 发送真实看涨买单'}
            >
              {isExecuting ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <Zap className="h-3.5 w-3.5 text-amber-300" />
              )}
              <span>⚡ 实盘买入 看涨 UP (${isValidStake ? parsedStake.toFixed(2) : '0.50'})</span>
            </button>

            <button
              type="button"
              disabled={isExecuting || !market || !!preflightError || !activeAccount}
              onClick={() => handleManualOrder('DOWN')}
              className="flex items-center justify-center gap-1.5 py-2.5 px-3 rounded-xl bg-gradient-to-r from-rose-600 to-[#ff0060] hover:from-rose-700 hover:to-[#e00055] text-white font-extrabold text-xs shadow-md shadow-[#ff0060]/20 transition disabled:opacity-40 disabled:from-slate-400 disabled:to-slate-400 cursor-pointer"
              title={preflightError || '向 Polymarket CLOB 发送真实看跌买单'}
            >
              {isExecuting ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <Zap className="h-3.5 w-3.5 text-amber-300" />
              )}
              <span>⚡ 实盘买入 看跌 DOWN (${isValidStake ? parsedStake.toFixed(2) : '0.50'})</span>
            </button>
          </div>
        )}

        {/* Feedback Alert */}
        {execStatus && (
          <div
            className={`p-2.5 rounded-xl text-xs flex items-center justify-between gap-2 font-medium animate-fade-in ${
              execStatus.success
                ? 'bg-emerald-50 dark:bg-emerald-950/40 text-[#1b9c85] border border-emerald-200 dark:border-emerald-800'
                : 'bg-rose-50 dark:bg-rose-950/40 text-[#ff0060] border border-rose-200 dark:border-rose-900'
            }`}
          >
            <div className="flex items-center gap-1.5">
              {execStatus.success ? (
                <CheckCircle2 className="h-4 w-4 shrink-0" />
              ) : (
                <AlertCircle className="h-4 w-4 shrink-0" />
              )}
              <span>{execStatus.msg}</span>
            </div>
            {!isPaper && !execStatus.success && (
              <span className="text-[10px] text-[#7d8da1] shrink-0 font-normal">
                (请在下方实盘成交流水中点击 [查看详情] 排查原因)
              </span>
            )}
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="mt-4 pt-3 border-t border-slate-100 dark:border-slate-800 flex items-center justify-between text-xs text-[#7d8da1] font-mono">
        <span>买卖价差: ${(typeof spreadUp === 'number' && !isNaN(spreadUp) ? spreadUp : 0.01).toFixed(3)}</span>
        <span>流动性深度: ${(typeof totalLiquidity === 'number' && !isNaN(totalLiquidity) ? totalLiquidity : 1000).toFixed(0)} USDC</span>
      </div>
    </div>
  );
};
