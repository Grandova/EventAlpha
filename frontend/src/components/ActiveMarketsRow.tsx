import React from 'react';
import { Plus, Check, X as XIcon, Zap, Power } from 'lucide-react';
import { Asset, PriceSummary } from '../types';

interface ActiveMarketsRowProps {
  activeAsset: Asset;
  onSelectAsset: (asset: Asset) => void;
  spotPrices: PriceSummary[];
  remainingSeconds: number;
  enabledAssets?: Asset[];
  onToggleEnabledAsset?: (asset: Asset) => void;
}

export const ActiveMarketsRow: React.FC<ActiveMarketsRowProps> = ({
  activeAsset,
  onSelectAsset,
  spotPrices,
  remainingSeconds,
  enabledAssets = ['BTC', 'ETH', 'SOL'],
  onToggleEnabledAsset,
}) => {
  const assets: { asset: Asset; name: string; iconBg: string; symbol: string }[] = [
    { asset: 'BTC', name: '比特币 5M (BTC)', iconBg: 'bg-[#f7931a]', symbol: '₿' },
    { asset: 'ETH', name: '以太坊 5M (ETH)', iconBg: 'bg-[#627eea]', symbol: 'Ξ' },
    { asset: 'SOL', name: '索拉纳 5M (SOL)', iconBg: 'bg-[#14f195]', symbol: '◎' },
  ];

  const getPrice = (a: Asset) => {
    let p: any = null;
    if (Array.isArray(spotPrices)) {
      p = spotPrices.find((sp) => sp && sp.asset === a);
    } else if (spotPrices && typeof spotPrices === 'object') {
      const assetMap = (spotPrices as any)[a];
      if (assetMap) {
        if (typeof assetMap.price === 'number') {
          p = assetMap;
        } else if (typeof assetMap === 'object') {
          const firstExch = Object.values(assetMap)[0] as any;
          if (firstExch) p = firstExch;
        }
      }
    }
    const val = p ? (p.price ?? p.mid ?? p.last) : undefined;
    return typeof val === 'number' && !isNaN(val)
      ? `$${val.toLocaleString(undefined, { minimumFractionDigits: a === 'BTC' ? 1 : 2, maximumFractionDigits: a === 'BTC' ? 1 : 2 })}`
      : '--';
  };

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h3 className="text-base font-extrabold text-[#363949] dark:text-white">
            活跃 5 分钟盘面与标的交易开关
          </h3>
          <p className="text-xs text-[#7d8da1]">
            点击卡片切换主监控视角；点击右侧按钮可独立开启或禁用某一币种的自动买入
          </p>
        </div>

        {/* Global Trading Asset Filter Pills */}
        {onToggleEnabledAsset && (
          <div className="flex items-center gap-1.5 bg-white dark:bg-[#202528] p-1.5 rounded-xl border border-slate-200 dark:border-slate-800 shadow-sm">
            <span className="text-[11px] font-bold text-[#7d8da1] px-2 flex items-center gap-1">
              <Zap className="w-3 h-3 text-[#1b9c85]" /> 允许买入:
            </span>
            {assets.map((item) => {
              const isEnabled = enabledAssets.includes(item.asset);
              return (
                <button
                  key={item.asset}
                  type="button"
                  onClick={() => onToggleEnabledAsset(item.asset)}
                  className={`px-3 py-1 rounded-lg font-mono font-bold text-xs transition-all flex items-center gap-1.5 cursor-pointer ${
                    isEnabled
                      ? 'bg-[#1b9c85] text-white shadow-sm hover:bg-[#178572]'
                      : 'bg-slate-100 dark:bg-slate-800 text-[#7d8da1] hover:text-[#363949] dark:hover:text-white line-through opacity-70'
                  }`}
                  title={isEnabled ? `点击暂停 ${item.asset} 自动买入` : `点击开启 ${item.asset} 自动买入`}
                >
                  {isEnabled ? <Check className="w-3 h-3" /> : <XIcon className="w-3 h-3" />}
                  {item.asset}
                </button>
              );
            })}
          </div>
        )}
      </div>

      <div className="asmr-card p-6 flex flex-wrap items-center justify-around gap-6">
        {assets.map((item) => {
          const isSelected = activeAsset === item.asset;
          const isTradingEnabled = enabledAssets.includes(item.asset);
          const price = getPrice(item.asset);

          return (
            <div
              key={item.asset}
              onClick={() => onSelectAsset(item.asset)}
              className={`flex flex-col items-center cursor-pointer transition-all duration-300 group ${
                isSelected ? 'scale-105' : 'opacity-80 hover:opacity-100'
              }`}
            >
              {/* Circular Avatar */}
              <div
                className={`relative w-16 h-16 rounded-full flex items-center justify-center text-white text-2xl font-black shadow-lg transition-transform group-hover:scale-110 ${item.iconBg} ${
                  isSelected
                    ? 'ring-4 ring-[#6c9bcf] ring-offset-2 dark:ring-offset-[#202528]'
                    : ''
                }`}
              >
                <span>{item.symbol}</span>
                {isSelected && (
                  <span className="absolute -top-1 -right-1 w-4 h-4 bg-[#1b9c85] rounded-full border-2 border-white dark:border-[#202528] flex items-center justify-center">
                    <span className="w-1.5 h-1.5 bg-white rounded-full" />
                  </span>
                )}
              </div>

              <span className="mt-2 text-xs font-bold text-[#363949] dark:text-white">
                {item.name}
              </span>
              <span className="text-[11px] font-mono text-[#7d8da1] dark:text-slate-400 font-semibold">
                {price}
              </span>
              <span className="text-[10px] font-mono text-[#6c9bcf]">
                剩余 {remainingSeconds} 秒
              </span>

              {/* Individual Coin Auto-Trading Toggle Pill */}
              {onToggleEnabledAsset && (
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onToggleEnabledAsset(item.asset);
                  }}
                  className={`mt-2 px-2.5 py-0.5 rounded-full text-[10px] font-bold transition flex items-center gap-1 cursor-pointer ${
                    isTradingEnabled
                      ? 'bg-emerald-100 dark:bg-emerald-950/80 text-[#1b9c85] border border-emerald-300 dark:border-emerald-800 hover:bg-emerald-200'
                      : 'bg-slate-100 dark:bg-slate-800 text-[#7d8da1] border border-slate-300 dark:border-slate-700 hover:bg-slate-200'
                  }`}
                  title={isTradingEnabled ? `点击暂停 ${item.asset} 自动下单` : `点击允许 ${item.asset} 自动下单`}
                >
                  <span
                    className={`w-1.5 h-1.5 rounded-full ${
                      isTradingEnabled ? 'bg-[#1b9c85] animate-pulse' : 'bg-slate-400'
                    }`}
                  />
                  {isTradingEnabled ? '自动买入: 开启' : '自动买入: 暂停'}
                </button>
              )}
            </div>
          );
        })}

        {/* "+ More" button */}
        <div
          onClick={() => {
            const nextAsset: Asset = activeAsset === 'BTC' ? 'ETH' : activeAsset === 'ETH' ? 'SOL' : 'BTC';
            onSelectAsset(nextAsset);
          }}
          className="flex flex-col items-center cursor-pointer transition-all duration-300 opacity-70 hover:opacity-100 group"
        >
          <div className="w-16 h-16 rounded-full border-2 border-dashed border-[#6c9bcf] flex items-center justify-center text-[#6c9bcf] transition-all group-hover:bg-[#6c9bcf]/10 group-hover:border-solid">
            <Plus className="w-7 h-7" />
          </div>
          <span className="mt-2 text-xs font-bold text-[#6c9bcf]">切换视角</span>
          <span className="text-[10px] text-[#7d8da1] dark:text-slate-400">BTC / ETH / SOL</span>
        </div>
      </div>
    </div>
  );
};
