import React from 'react';
import { Plus } from 'lucide-react';
import { Asset, PriceSummary } from '../types';

interface ActiveMarketsRowProps {
  activeAsset: Asset;
  onSelectAsset: (asset: Asset) => void;
  spotPrices: PriceSummary[];
  remainingSeconds: number;
}

export const ActiveMarketsRow: React.FC<ActiveMarketsRowProps> = ({
  activeAsset,
  onSelectAsset,
  spotPrices,
  remainingSeconds,
}) => {
  const assets: { asset: Asset; name: string; iconBg: string; symbol: string }[] = [
    { asset: 'BTC', name: '比特币 5M (BTC)', iconBg: 'bg-[#f7931a]', symbol: '₿' },
    { asset: 'ETH', name: '以太坊 5M (ETH)', iconBg: 'bg-[#627eea]', symbol: 'Ξ' },
    { asset: 'SOL', name: '索拉纳 5M (SOL)', iconBg: 'bg-[#14f195]', symbol: '◎' },
  ];

  const getPrice = (a: Asset) => {
    const p = spotPrices.find((sp) => sp.asset === a);
    const val = p ? (p.price ?? (p as any).mid ?? (p as any).last) : undefined;
    return typeof val === 'number' && !isNaN(val)
      ? `$${val.toLocaleString(undefined, { minimumFractionDigits: a === 'BTC' ? 1 : 2, maximumFractionDigits: a === 'BTC' ? 1 : 2 })}`
      : '--';
  };

  return (
    <div className="space-y-3">
      <h3 className="text-base font-extrabold text-[#363949] dark:text-white">
        活跃 5 分钟盘面
      </h3>

      <div className="asmr-card p-6 flex flex-wrap items-center justify-around gap-6">
        {assets.map((item) => {
          const isSelected = activeAsset === item.asset;
          const price = getPrice(item.asset);

          return (
            <div
              key={item.asset}
              onClick={() => onSelectAsset(item.asset)}
              className={`flex flex-col items-center cursor-pointer transition-all duration-300 group ${
                isSelected ? 'scale-105' : 'opacity-70 hover:opacity-100'
              }`}
            >
              {/* Circular Avatar (Matches screenshot user avatars!) */}
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
            </div>
          );
        })}

        {/* "+ More" button (Matches exact screenshot "+ More New User" item!) */}
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
          <span className="mt-2 text-xs font-bold text-[#6c9bcf]">切换币种</span>
          <span className="text-[10px] text-[#7d8da1] dark:text-slate-400">BTC / ETH / SOL</span>
        </div>
      </div>
    </div>
  );
};
