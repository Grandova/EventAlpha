import React, { useState, useEffect } from 'react';
import { Sidebar } from './components/Sidebar';
import { Header } from './components/Header';
import { StatCard } from './components/StatCard';
import { ActiveMarketsRow } from './components/ActiveMarketsRow';
import { CurrencyTabBar } from './components/CurrencyTabBar';
import { AssetPriceChart } from './components/AssetPriceChart';
import { RightProfileWidget } from './components/RightProfileWidget';
import { PolymarketRoundCard } from './components/PolymarketRoundCard';
import { OpportunityCenter } from './components/OpportunityCenter';
import { BankrollRiskMonitor } from './components/BankrollRiskMonitor';
import { TradingBlotter } from './components/TradingBlotter';
import { FeatureMonitor } from './components/FeatureMonitor';
import { BacktestConsole } from './components/BacktestConsole';
import { ReplayConsole } from './components/ReplayConsole';
import { EventLogViewer } from './components/EventLogViewer';
import { OrderbookVisualizer } from './components/OrderbookVisualizer';
import { StrategyTuner } from './components/StrategyTuner';
import { SelfLearningPanel } from './components/SelfLearningPanel';
import { LiveTradingBlotter } from './components/LiveTradingBlotter';
import { AccountManagerModal } from './components/AccountManagerModal';
import { ErrorBoundary } from './components/ErrorBoundary';
import { LoginPage } from './components/LoginPage';
import { SetBankrollModal } from './components/SetBankrollModal';
import { UnlockProfitModal } from './components/UnlockProfitModal';
import { api, getStoredToken, clearStoredToken } from './services/api';
import { wsClient } from './services/ws';
import {
  Asset,
  HealthResponse,
  PriceSummary,
  CompositePriceSnapshot,
  MarketDisplayInfo,
  MarketBookSummary,
  FeatureSnapshot,
  ModelPrediction,
  PredictionSignal,
  BankrollState,
  RiskStatus,
  PaperPosition,
  PaperOrder,
  PaperResult,
  TradeStatistics,
  TradingMode,
  PolymarketAccountPublic,
} from './types';

export const App: React.FC = () => {
  const [activeAsset, setActiveAsset] = useState<Asset>('BTC');
  const [activeTab, setActiveTab] = useState<string>('dashboard');

  // Authentication State
  const [isAuthenticated, setIsAuthenticated] = useState<boolean>(false);
  const [currentUser, setCurrentUser] = useState<string>('admin');
  const [isCheckingAuth, setIsCheckingAuth] = useState<boolean>(true);

  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [spotPrices, setSpotPrices] = useState<PriceSummary[]>([]);
  const [composite, setComposite] = useState<CompositePriceSnapshot | null>(null);
  const [market, setMarket] = useState<MarketDisplayInfo | null>(null);
  const [book, setBook] = useState<MarketBookSummary | null>(null);
  const [features, setFeatures] = useState<FeatureSnapshot | null>(null);
  const [prediction, setPrediction] = useState<ModelPrediction | null>(null);
  const [signal, setSignal] = useState<PredictionSignal | null>(null);
  const [allComposite, setAllComposite] = useState<Record<string, CompositePriceSnapshot>>({});
  const [allSignals, setAllSignals] = useState<Record<string, PredictionSignal>>({});
  const [allPredictions, setAllPredictions] = useState<Record<string, ModelPrediction>>({});
  const [bankroll, setBankroll] = useState<BankrollState | null>(null);
  const [risk, setRisk] = useState<RiskStatus | null>(null);
  const [liveBankroll, setLiveBankroll] = useState<BankrollState | null>(null);
  const [liveRisk, setLiveRisk] = useState<RiskStatus | null>(null);
  const [bankrollModalMode, setBankrollModalMode] = useState<'paper' | 'live'>('paper');
  const [unlockModalMode, setUnlockModalMode] = useState<'paper' | 'live'>('paper');
  const [activePositions, setActivePositions] = useState<PaperPosition[]>([]);
  const [recentOrders, setRecentOrders] = useState<PaperOrder[]>([]);
  const [settledResults, setSettledResults] = useState<PaperResult[]>([]);
  const [statistics, setStatistics] = useState<TradeStatistics | null>(null);

  // Real Trading & Polymarket Account Management State
  const [tradingMode, setTradingMode] = useState<TradingMode>('paper');
  const [activeAccount, setActiveAccount] = useState<PolymarketAccountPublic | null>(null);
  const [isAccountModalOpen, setIsAccountModalOpen] = useState(false);
  const [isBankrollModalOpen, setIsBankrollModalOpen] = useState(false);
  const [enabledAssets, setEnabledAssets] = useState<Asset[]>(['BTC', 'ETH', 'SOL']);
  const [isPaperAutoTradingEnabled, setIsPaperAutoTradingEnabled] = useState<boolean>(true);
  const [isLiveAutoTradingEnabled, setIsLiveAutoTradingEnabled] = useState<boolean>(false);
  const isAutoTradingEnabled = tradingMode === 'live' ? isLiveAutoTradingEnabled : isPaperAutoTradingEnabled;
  const [isUnlockModalOpen, setIsUnlockModalOpen] = useState<boolean>(false);

  // AsmrProg Light / Dark Theme State (Light by default, matching screenshot)
  const [isDarkMode, setIsDarkMode] = useState<boolean>(() => {
    return localStorage.getItem('polyquant_theme') === 'dark';
  });

  useEffect(() => {
    if (isDarkMode) {
      document.documentElement.classList.add('dark');
      localStorage.setItem('polyquant_theme', 'dark');
    } else {
      document.documentElement.classList.remove('dark');
      localStorage.setItem('polyquant_theme', 'light');
    }
  }, [isDarkMode]);

  // Auth verification check on startup
  useEffect(() => {
    let isMounted = true;
    const checkAuth = async () => {
      try {
        const status = await api.getAuthStatus();
        if (!isMounted) return;
        if (!status.auth_enabled) {
          setIsAuthenticated(true);
          setCurrentUser('admin');
        } else if (status.authenticated) {
          setIsAuthenticated(true);
          setCurrentUser(status.username || 'admin');
        } else {
          setIsAuthenticated(false);
        }
      } catch (err) {
        if (!isMounted) return;
        const token = getStoredToken();
        if (!token) {
          setIsAuthenticated(false);
        }
      } finally {
        if (isMounted) {
          setIsCheckingAuth(false);
        }
      }
    };

    checkAuth();

    const handleAuthRequired = () => {
      clearStoredToken();
      setIsAuthenticated(false);
    };

    window.addEventListener('polyquant_auth_required', handleAuthRequired);
    return () => {
      isMounted = false;
      window.removeEventListener('polyquant_auth_required', handleAuthRequired);
    };
  }, []);

  const handleLogout = async () => {
    try {
      await api.logout();
    } catch (err) {
      // ignore
    }
    clearStoredToken();
    setIsAuthenticated(false);
  };

  // Sync mode and accounts from backend on load
  const syncTradingMode = async () => {
    try {
      const modeRes = await api.getTradingMode();
      setTradingMode(modeRes.mode);
      if (modeRes.active_account) {
        setActiveAccount(modeRes.active_account);
      } else {
        const accounts = await api.getAccounts();
        const active = accounts.find((a) => a.is_active) || accounts[0] || null;
        setActiveAccount(active);
      }
    } catch (err) {
      // ignore
    }
  };

  useEffect(() => {
    if (isAuthenticated) {
      syncTradingMode();
      api.getEnabledAssets().then((res) => {
        if (res.success && Array.isArray(res.assets) && res.assets.length > 0) {
          setEnabledAssets(res.assets);
        }
      }).catch(() => {});
      api.getAutoTrading().then((res) => {
        if (res) {
          if (typeof res.paper_enabled === 'boolean') {
            setIsPaperAutoTradingEnabled(res.paper_enabled);
          }
          if (typeof res.live_enabled === 'boolean') {
            setIsLiveAutoTradingEnabled(res.live_enabled);
          }
        }
      }).catch(() => {});
    }
  }, [isAuthenticated]);

  const handleTogglePaperAutoTrading = async (enabled: boolean) => {
    try {
      const res = await api.setAutoTrading({ paper_enabled: enabled });
      setIsPaperAutoTradingEnabled(res.paper_enabled);
    } catch (err: any) {
      alert(`更新模拟盘自动交易状态失败: ${err.message || '网络异常'}`);
    }
  };

  const handleToggleLiveAutoTrading = async (enabled: boolean) => {
    if (enabled) {
      if (!activeAccount) {
        alert('开启实盘自动交易前，请先绑定并激活 Polymarket 账户！');
        setIsAccountModalOpen(true);
        return;
      }
      const confirmed = window.confirm(
        `⚠️ 实盘自动交易启用确认：\n\n您即将开启【实盘 CLOB 自动交易】！\n系统在接收到高胜率 AI 策略信号时，将自动使用账户 "${activeAccount.label}" 的真实 USDC 下单撮合。\n\n确认开启？`
      );
      if (!confirmed) return;
    }
    try {
      const res = await api.setAutoTrading({ live_enabled: enabled });
      setIsLiveAutoTradingEnabled(res.live_enabled);
    } catch (err: any) {
      alert(`更新实盘自动交易状态失败: ${err.message || '网络异常'}`);
    }
  };

  const handleToggleAutoTrading = async (enabled: boolean) => {
    if (tradingMode === 'live') {
      await handleToggleLiveAutoTrading(enabled);
    } else {
      await handleTogglePaperAutoTrading(enabled);
    }
  };

  const handleToggleEnabledAsset = async (asset: Asset) => {
    const current = new Set(enabledAssets);
    if (current.has(asset)) {
      if (current.size === 1) {
        alert('请至少保留一个币种开启交易，或在风控设置中暂停交易！');
        return;
      }
      current.delete(asset);
    } else {
      current.add(asset);
    }
    const nextList = Array.from(current);
    setEnabledAssets(nextList);
    try {
      await api.setEnabledAssets(nextList);
    } catch (err: any) {
      console.error('Failed to update enabled trading assets:', err);
    }
  };

  const handleToggleTradingMode = async (newMode: TradingMode) => {
    if (newMode === tradingMode) return;

    if (newMode === 'live') {
      let currentActive = activeAccount;
      if (!currentActive) {
        try {
          const accounts = await api.getAccounts();
          const active = accounts.find((a) => a.is_active) || accounts[0] || null;
          if (active) {
            currentActive = active;
            setActiveAccount(active);
          }
        } catch (_) {}
      }

      if (!currentActive) {
        alert('实盘交易要求必须先绑定并激活至少一个 Polymarket 账户！已为您打开账户管理面板。');
        setIsAccountModalOpen(true);
        return;
      }

      if (currentActive.balance_usdc <= 0) {
        const proceed = window.confirm(
          `⚠️ 提示：当前实盘账户 "${currentActive.label}" 的 Polygon USDC 余额为 $0.00。\n\n如您已在 Polymarket 充值，请确认您绑定的钱包是否为个人主页网址里的 Proxy 代理钱包。\n是否仍要切换至实盘模式？`
        );
        if (!proceed) return;
      } else {
        const proceed = window.confirm(
          `⚠️ 实盘风险警示：即将切换至 Polymarket CLOB 实盘撮合模式！\n\n当前活跃账户：${currentActive.label}\nPolygon 钱包：${currentActive.wallet_address}\n当前可用余额：$${currentActive.balance_usdc.toFixed(2)} USDC\n\n系统将以 Mode B 资金硬顶（最大 $10.00 USDC）向真实订单簿下达限价/市价单。确认继续？`
        );
        if (!proceed) return;
      }

      try {
        await api.setTradingMode('live');
        setTradingMode('live');
      } catch (err: any) {
        alert(`切换实盘模式失败: ${err.message || '请确保已激活账户且系统安全核验通过'}`);
      }
    } else {
      try {
        await api.setTradingMode('paper');
        setTradingMode('paper');
      } catch (err: any) {
        alert(`切换模拟盘失败: ${err.message}`);
      }
    }
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
      const res = await api.emergencyHalt();
      setTradingMode('paper');
      alert(`紧急熔断已触发：${res.message}`);
    } catch (err: any) {
      alert(`熔断执行失败: ${err.message}`);
    }
  };

  const handleTabSelect = (tab: string) => {
    if (tab === 'accounts') {
      setIsAccountModalOpen(true);
    } else {
      setActiveTab(tab);
    }
  };

  // 1. High frequency polling for live prices & round state (1000ms)
  useEffect(() => {
    if (!isAuthenticated) return;
    let isMounted = true;
    const pollFast = async () => {
      try {
        const [prices, comp, markets, bk, feat, pred, sig] = await Promise.allSettled([
          api.getCollectorPrices(),
          api.getCompositePrice(activeAsset),
          api.getPolymarketMarkets(),
          api.getPolymarketBook(activeAsset),
          api.getFeaturesLatest(activeAsset),
          api.getPrediction(activeAsset),
          api.getLatestSignal(activeAsset),
        ]);

        if (!isMounted) return;

        if (prices.status === 'fulfilled') {
          const val = prices.value;
          if (Array.isArray(val)) {
            setSpotPrices(val);
          } else if (val && typeof val === 'object') {
            const list: PriceSummary[] = [];
            for (const exchMap of Object.values(val)) {
              if (exchMap && typeof exchMap === 'object') {
                if ('price' in exchMap) {
                  list.push(exchMap as any);
                } else {
                  for (const item of Object.values(exchMap as any)) {
                    if (item && typeof item === 'object') {
                      list.push(item as any);
                    }
                  }
                }
              }
            }
            setSpotPrices(list);
          }
        }
        if (comp.status === 'fulfilled') setComposite(comp.value);
        if (markets.status === 'fulfilled') {
          const mList = Array.isArray(markets.value) ? markets.value : [];
          const match = mList.find((m) => m && m.asset === activeAsset) || mList[0] || null;
          setMarket(match);
        }
        if (bk.status === 'fulfilled') setBook(bk.value);
        if (feat.status === 'fulfilled') setFeatures(feat.value);
        if (pred.status === 'fulfilled') setPrediction(pred.value);
        if (sig.status === 'fulfilled') setSignal(sig.value);
      } catch (err) {
        // network error / backend restarting
      }
    };

    pollFast();
    const timer = setInterval(pollFast, 1000);
    return () => {
      isMounted = false;
      clearInterval(timer);
    };
  }, [activeAsset, isAuthenticated]);

  // 2. Medium frequency polling for bankroll, risk, positions & blotter (2000ms)
  useEffect(() => {
    if (!isAuthenticated) return;
    let isMounted = true;
    const pollMedium = async () => {
      try {
        const [hlth, br, rsk, lbr, lrsk, pos, ord, res, stat, allComp, allSig, allPred] = await Promise.allSettled([
          api.getHealth(),
          api.getBankroll('paper'),
          api.getRiskStatus('paper'),
          api.getBankroll('live'),
          api.getRiskStatus('live'),
          api.getActivePositions(),
          api.getPaperOrders(20),
          api.getPaperResults(50),
          api.getPaperStatistics(),
          api.getAllCompositePrices(),
          api.getAllLatestSignals(),
          api.getAllPredictions(),
        ]);

        if (!isMounted) return;

        if (hlth.status === 'fulfilled') setHealth(hlth.value);
        if (br.status === 'fulfilled') setBankroll(br.value);
        if (rsk.status === 'fulfilled') setRisk(rsk.value);
        if (lbr.status === 'fulfilled') setLiveBankroll(lbr.value);
        if (lrsk.status === 'fulfilled') setLiveRisk(lrsk.value);
        if (pos.status === 'fulfilled') setActivePositions(Array.isArray(pos.value) ? pos.value : []);
        if (ord.status === 'fulfilled') setRecentOrders(Array.isArray(ord.value) ? ord.value : []);
        if (res.status === 'fulfilled') setSettledResults(Array.isArray(res.value) ? res.value : []);
        if (stat.status === 'fulfilled') setStatistics(stat.value);
        if (allComp.status === 'fulfilled') setAllComposite(allComp.value || {});
        if (allSig.status === 'fulfilled') setAllSignals(allSig.value || {});
        if (allPred.status === 'fulfilled') setAllPredictions(allPred.value || {});
      } catch (err) {
        // network error
      }
    };

    pollMedium();
    const timer = setInterval(pollMedium, 2000);
    return () => {
      isMounted = false;
      clearInterval(timer);
    };
  }, [isAuthenticated]);

  // 3. Real-time sub-100ms WebSocket Multiplexed Stream
  useEffect(() => {
    const unsubscribe = wsClient.subscribe((msg) => {
      switch (msg.type) {
        case 'ticker':
          if (msg.data.asset === activeAsset) {
            setComposite((prev) =>
              prev
                ? { ...prev, composite_price: msg.data.composite_price }
                : null
            );
            if (msg.data.spot_prices && msg.data.spot_prices.length > 0) {
              setSpotPrices(msg.data.spot_prices);
            }
          }
          break;

        case 'signal':
          if (msg.data.signal.asset === activeAsset) {
            setSignal(msg.data.signal);
          }
          break;

        case 'bankroll':
          setBankroll((prev) =>
            prev
              ? {
                  ...prev,
                  active_bankroll: msg.data.active,
                  locked_profit: msg.data.locked,
                  total_equity: msg.data.total,
                }
              : null
          );
          break;

        case 'resolution':
          api.getActivePositions().then(setActivePositions).catch(() => {});
          api.getPaperResults(50).then(setSettledResults).catch(() => {});
          api.getPaperStatistics().then(setStatistics).catch(() => {});
          api.getBankroll().then(setBankroll).catch(() => {});
          syncTradingMode();
          break;

        case 'heartbeat':
          setHealth((prev) =>
            prev
              ? {
                  ...prev,
                  uptime_secs: msg.data.uptime_secs,
                  timestamp_ms: msg.data.timestamp_ms,
                }
              : null
          );
          break;
      }
    });

    return () => {
      unsubscribe();
    };
  }, [activeAsset]);

  const roundRemaining = market?.remaining_seconds ?? 150;

  // 1. Loading screen while verifying token & session
  if (isCheckingAuth) {
    return (
      <div className="min-h-screen bg-[#f6f6f9] dark:bg-[#181a1e] flex flex-col items-center justify-center p-4">
        <div className="relative flex items-center justify-center w-20 h-20 rounded-full border-4 border-[#ff0060] p-1 bg-white dark:bg-[#202528] shadow-lg animate-pulse-ring mb-4">
          <div className="w-full h-full rounded-full border-2 border-[#ff0060] flex items-center justify-center font-black text-[#ff0060] text-xl">
            AP
          </div>
        </div>
        <p className="text-xs font-mono font-bold text-[#7d8da1] dark:text-slate-400 tracking-wider animate-pulse">
          EventAlpha · 系统安全验证中...
        </p>
      </div>
    );
  }

  // 2. Unauthenticated: Render AsmrProg Login Screen
  if (!isAuthenticated) {
    return (
      <LoginPage
        onLoginSuccess={(user) => {
          setIsAuthenticated(true);
          setCurrentUser(user);
        }}
        isDarkMode={isDarkMode}
        onToggleTheme={() => setIsDarkMode(!isDarkMode)}
      />
    );
  }

  return (
    <div className="min-h-screen bg-[#f6f6f9] dark:bg-[#181a1e] text-[#363949] dark:text-[#edeffd] flex flex-col lg:flex-row antialiased transition-colors duration-300">
      {/* Left Column: AsmrProg Floating Sidebar */}
      <Sidebar
        activeTab={activeTab}
        onSelectTab={handleTabSelect}
        activePositionsCount={activePositions.length}
        isLive={wsClient.getStatus()}
        tradingMode={tradingMode}
        onLogout={handleLogout}
      />

      {/* Center Column: Main Workspace */}
      <div className="flex-1 flex flex-col min-w-0 p-4 lg:p-6 xl:p-8 overflow-y-auto">
        {/* Top Header with Analytics Title, Theme Switcher & User Avatar */}
        <Header
          health={health}
          activeAsset={activeAsset}
          onSelectAsset={(a) => setActiveAsset(a)}
          compositePrice={composite?.composite_price}
          spotPrices={spotPrices}
          isWsConnected={wsClient.getStatus()}
          tradingMode={tradingMode}
          onToggleTradingMode={handleToggleTradingMode}
          isAutoTradingEnabled={isAutoTradingEnabled}
          onToggleAutoTrading={handleToggleAutoTrading}
          isPaperAutoTradingEnabled={isPaperAutoTradingEnabled}
          onTogglePaperAutoTrading={handleTogglePaperAutoTrading}
          isLiveAutoTradingEnabled={isLiveAutoTradingEnabled}
          onToggleLiveAutoTrading={handleToggleLiveAutoTrading}
          activeAccount={activeAccount}
          onOpenAccountManager={() => setIsAccountModalOpen(true)}
          onEmergencyHalt={handleEmergencyHalt}
          isDarkMode={isDarkMode}
          onToggleTheme={() => setIsDarkMode(!isDarkMode)}
          currentUser={currentUser}
          onLogout={handleLogout}
        />

        {/* Dashboard Main View */}
        <main className="space-y-6">
          <ErrorBoundary fallbackTitle="模块安全隔离异常">
            {activeTab === 'dashboard' && (
              <div className="space-y-6">
                {/* 1. AsmrProg Iconic Top Stat Cards Row with Circular Progress Rings! */}
                <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-6">
                  <StatCard
                    title={tradingMode === 'live' ? '⚡ 实盘活跃资金 (Bankroll)' : '🎮 模拟活跃资金 (Bankroll)'}
                    value={
                      tradingMode === 'live'
                        ? `$${(typeof activeAccount?.balance_usdc === 'number' && !isNaN(activeAccount.balance_usdc) ? activeAccount.balance_usdc : (liveBankroll?.active_bankroll ?? 0.0)).toFixed(2)}`
                        : `$${(typeof bankroll?.active_bankroll === 'number' && !isNaN(bankroll.active_bankroll) ? bankroll.active_bankroll : 10.0).toFixed(2)}`
                    }
                    subtitle={
                      tradingMode === 'live'
                        ? (activeAccount ? `${activeAccount.label} · 链上真实 USDC` : '未绑定实盘账户')
                        : `模式 B 动态硬顶: $${(typeof bankroll?.bankroll_cap === 'number' && !isNaN(bankroll.bankroll_cap) ? bankroll.bankroll_cap : 10.0).toFixed(2)}`
                    }
                    progress={
                      tradingMode === 'live'
                        ? Math.min(100, (((activeAccount?.balance_usdc ?? liveBankroll?.active_bankroll ?? 0.0)) / (liveBankroll?.bankroll_cap || 10.0)) * 100)
                        : ((bankroll?.active_bankroll ?? 10.0) / (bankroll?.bankroll_cap ?? 10.0)) * 100
                    }
                    percentageText={tradingMode === 'live' ? '实盘' : '模拟'}
                    accentColor={tradingMode === 'live' ? '#ff0060' : '#1b9c85'}
                    onClick={() => {
                      setBankrollModalMode(tradingMode);
                      setIsBankrollModalOpen(true);
                    }}
                    actionLabel={tradingMode === 'live' ? '⚙️ 实盘风控设置' : '⚙️ 设置本金'}
                  />

                  <StatCard
                    title={tradingMode === 'live' ? '⚡ 实盘锁定利润金库' : '🎮 模拟锁定利润金库'}
                    value={
                      tradingMode === 'live'
                        ? `+$${(typeof liveBankroll?.locked_profit === 'number' && !isNaN(liveBankroll.locked_profit) ? liveBankroll.locked_profit : 0.0).toFixed(2)}`
                        : `+$${(typeof bankroll?.locked_profit === 'number' && !isNaN(bankroll.locked_profit) ? bankroll.locked_profit : 0.0).toFixed(2)}`
                    }
                    subtitle={tradingMode === 'live' ? '实盘利润 100% 绝对隔离' : '模拟利润 100% 绝对隔离'}
                    progress={
                      tradingMode === 'live'
                        ? Math.min(100, Math.max(10, ((liveBankroll?.locked_profit ?? 0.0) / (liveBankroll?.bankroll_cap || 10.0)) * 100))
                        : Math.min(100, Math.max(10, ((bankroll?.locked_profit ?? 0.0) / 10.0) * 100))
                    }
                    percentageText={
                      tradingMode === 'live'
                        ? (liveBankroll?.locked_profit ? `+$${liveBankroll.locked_profit.toFixed(1)}` : '0.0')
                        : (bankroll?.locked_profit ? `+$${bankroll.locked_profit.toFixed(1)}` : '-48%')
                    }
                    accentColor="#ff0060"
                    onClick={() => {
                      const curLocked = tradingMode === 'live' ? (liveBankroll?.locked_profit ?? 0) : (bankroll?.locked_profit ?? 0);
                      if (curLocked > 0) {
                        setUnlockModalMode(tradingMode);
                        setIsUnlockModalOpen(true);
                      }
                    }}
                    actionLabel={
                      (tradingMode === 'live' ? (liveBankroll?.locked_profit ?? 0) : (bankroll?.locked_profit ?? 0)) > 0
                        ? '🔓 提取利润'
                        : undefined
                    }
                  />

                  <StatCard
                    title="模型综合预测胜率"
                    value={`${(typeof statistics?.win_rate === 'number' && !isNaN(statistics.win_rate) ? statistics.win_rate : 0.0).toFixed(1)}%`}
                    subtitle={`${statistics?.winning_trades ?? 0} 胜 / ${statistics?.losing_trades ?? 0} 负`}
                    progress={statistics?.win_rate ?? 68}
                    percentageText="+21%"
                    accentColor="#6c9bcf"
                  />
                </div>

                {/* 2. Active Markets & 5M Rounds (Matches AsmrProg "New Users" row!) */}
                {/* 2. Independent Multi-Currency Trading Desk Navigation */}
                <CurrencyTabBar
                  activeAsset={activeAsset}
                  onSelectAsset={setActiveAsset}
                  spotPrices={spotPrices}
                  allComposite={allComposite}
                  allSignals={allSignals}
                  allPredictions={allPredictions}
                  remainingSeconds={roundRemaining}
                  enabledAssets={enabledAssets}
                  onToggleEnabledAsset={handleToggleEnabledAsset}
                />

                {/* 3. Dedicated Asset Trading Desk: Left = Price Chart & Indicators & Predictions; Right = Polymarket 5M Round */}
                <div className="grid grid-cols-1 xl:grid-cols-12 gap-6">
                  {/* Left: Real-time Price Chart, Technical Indicators, and AI Model Predictions */}
                  <div className="xl:col-span-7">
                    <AssetPriceChart
                      asset={activeAsset}
                      composite={composite}
                      features={features}
                      prediction={prediction}
                      signal={signal}
                      market={market}
                    />
                  </div>

                  {/* Right: Dedicated Polymarket 5M Round & Manual Order Cockpit */}
                  <div className="xl:col-span-5">
                    <PolymarketRoundCard
                      market={market}
                      book={book}
                      prediction={prediction}
                      tradingMode={tradingMode}
                      activeAsset={activeAsset}
                      activePositions={activePositions}
                      isAutoTradingEnabled={isAutoTradingEnabled}
                      activeAccount={activeAccount}
                      liveBankroll={liveBankroll}
                      paperBankroll={bankroll}
                      currentSpotPrice={composite?.composite_price}
                      onOpenSetBankroll={(poolMode) => {
                        setBankrollModalMode(poolMode || tradingMode);
                        setIsBankrollModalOpen(true);
                      }}
                      onOpenAccountManager={() => setIsAccountModalOpen(true)}
                      onTradeExecuted={() => {
                        api.getActivePositions().then(setActivePositions).catch(() => {});
                        api.getPaperOrders(20).then(setRecentOrders).catch(() => {});
                        api.getBankroll('paper').then(setBankroll).catch(() => {});
                        api.getBankroll('live').then(setLiveBankroll).catch(() => {});
                        if (activeAccount) {
                          api.getAccounts().then((accs) => {
                            const found = accs.find((a) => a.id === activeAccount.id);
                            if (found) setActiveAccount(found);
                          }).catch(() => {});
                        }
                      }}
                    />
                  </div>
                </div>

                {/* 4. AI Opportunity Center */}
                <OpportunityCenter
                  signal={signal}
                  prediction={prediction}
                  market={market}
                  book={book}
                  onRefreshSignal={() => {
                    api.getLatestSignal(activeAsset).then(setSignal).catch(() => {});
                  }}
                />

                {/* 4. Recent Orders Table (Matches AsmrProg "Recent Orders" table!) */}
                {tradingMode === 'live' ? (
                  <LiveTradingBlotter
                    activeAccount={activeAccount}
                    onOpenAccountManager={() => setIsAccountModalOpen(true)}
                  />
                ) : (
                  <TradingBlotter
                    activePositions={activePositions}
                    recentOrders={recentOrders}
                    settledResults={settledResults}
                    statistics={statistics}
                  />
                )}

                {/* 5. Orderbook Depth Ladder */}
                <OrderbookVisualizer
                  asset={activeAsset}
                  book={book}
                  onRefresh={() => api.getPolymarketBook(activeAsset).then(setBook)}
                />

                {/* 6. Bankroll & Risk Monitor */}
                <BankrollRiskMonitor
                  bankroll={bankroll}
                  risk={risk}
                  liveBankroll={liveBankroll}
                  liveRisk={liveRisk}
                  tradingMode={tradingMode}
                  activeAccount={activeAccount}
                  onOpenSetBankroll={(poolMode) => {
                    setBankrollModalMode(poolMode || tradingMode);
                    setIsBankrollModalOpen(true);
                  }}
                  onBankrollUpdated={(newB, poolMode) => {
                    if (poolMode === 'live') {
                      setLiveBankroll(newB);
                    } else {
                      setBankroll(newB);
                    }
                  }}
                  onRefreshLiveBalance={async () => {
                    const accounts = await api.getAccounts();
                    const active = accounts.find((a) => a.is_active) || accounts[0] || null;
                    if (active) {
                      setActiveAccount(active);
                    }
                    const refreshed = await api.getBankroll('live');
                    setLiveBankroll(refreshed);
                  }}
                />
              </div>
            )}

            {/* Continuous Self-Learning Panel */}
            {activeTab === 'self-learning' && (
              <SelfLearningPanel
                activeAsset={activeAsset}
                onSelectAsset={(a) => setActiveAsset(a)}
              />
            )}

            {/* Live Trading Orders Blotter */}
            {activeTab === 'live-orders' && (
              <LiveTradingBlotter
                activeAccount={activeAccount}
                onOpenAccountManager={() => setIsAccountModalOpen(true)}
              />
            )}

            {activeTab === 'microstructure' && (
              <div className="space-y-6">
                <OrderbookVisualizer
                  asset={activeAsset}
                  book={book}
                  onRefresh={() => api.getPolymarketBook(activeAsset).then(setBook)}
                />
                <FeatureMonitor asset={activeAsset} features={features} />
              </div>
            )}

            {activeTab === 'features' && (
              <FeatureMonitor asset={activeAsset} features={features} />
            )}

            {activeTab === 'tuning' && (
              <StrategyTuner />
            )}

            {activeTab === 'backtest' && (
              <BacktestConsole defaultAsset={activeAsset} />
            )}

            {activeTab === 'replay' && (
              <ReplayConsole defaultAsset={activeAsset} />
            )}

            {activeTab === 'events' && (
              <EventLogViewer />
            )}
          </ErrorBoundary>
        </main>
      </div>

      {/* Right Column: AsmrProg Profile & Reminders Widget Panel (Matches screenshot!) */}
      {activeTab === 'dashboard' && (
        <div className="p-4 lg:p-6 lg:pl-0">
          <RightProfileWidget
            activeAccount={activeAccount}
            tradingMode={tradingMode}
            paperBankroll={bankroll}
            liveBankroll={liveBankroll}
            onOpenAccountManager={() => setIsAccountModalOpen(true)}
            onEmergencyHalt={handleEmergencyHalt}
          />
        </div>
      )}

      {/* Account Manager Modal */}
      <AccountManagerModal
        isOpen={isAccountModalOpen}
        onClose={() => {
          setIsAccountModalOpen(false);
          syncTradingMode();
        }}
        onAccountActivated={(acc) => {
          setActiveAccount(acc);
          syncTradingMode();
        }}
      />

      {/* Set Bankroll Modal */}
      <SetBankrollModal
        isOpen={isBankrollModalOpen}
        onClose={() => setIsBankrollModalOpen(false)}
        mode={bankrollModalMode}
        currentBankroll={bankroll}
        currentRisk={risk}
        liveBankroll={liveBankroll}
        liveRisk={liveRisk}
        activeAccount={activeAccount}
        onSuccess={(newBr, modalMode) => {
          if (modalMode === 'live') {
            setLiveBankroll(newBr);
            api.getRiskStatus('live').then(setLiveRisk).catch(() => {});
          } else {
            setBankroll(newBr);
            api.getRiskStatus('paper').then(setRisk).catch(() => {});
          }
        }}
      />

      {/* Unlock / Withdraw Profit Modal */}
      <UnlockProfitModal
        isOpen={isUnlockModalOpen}
        onClose={() => setIsUnlockModalOpen(false)}
        mode={unlockModalMode}
        currentBankroll={unlockModalMode === 'live' ? liveBankroll : bankroll}
        onSuccess={(newBr) => {
          if (unlockModalMode === 'live') {
            setLiveBankroll(newBr);
          } else {
            setBankroll(newBr);
          }
        }}
      />
    </div>
  );
};

export default App;
