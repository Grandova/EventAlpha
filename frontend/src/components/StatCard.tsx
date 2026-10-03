import React from 'react';

interface StatCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  progress?: number; // 0 to 100
  percentageText?: string; // e.g. "+81%", "-48%", "+21%"
  accentColor?: string; // e.g. "#1b9c85", "#ff0060", "#6c9bcf", "#ffbb55"
  icon?: React.ReactNode;
  trend?: {
    value: string;
    isPositive?: boolean;
    label?: string;
  };
}

export const StatCard: React.FC<StatCardProps> = ({
  title,
  value,
  subtitle,
  progress = 75,
  percentageText,
  accentColor = '#1b9c85',
  trend,
}) => {
  const radius = 28;
  const strokeWidth = 6;
  const circumference = 2 * Math.PI * radius;
  // clamped progress 0-100
  const normalizedProgress = Math.min(100, Math.max(0, progress));
  const strokeDashoffset = circumference - (normalizedProgress / 100) * circumference;

  const displayPercent = percentageText || `${normalizedProgress > 0 ? '+' : ''}${Math.round(normalizedProgress)}%`;

  return (
    <div className="asmr-card p-6 flex items-center justify-between transition-all duration-300 hover:-translate-y-1">
      {/* Left: Titles & Large Value */}
      <div className="space-y-1">
        <h4 className="text-sm font-semibold text-[#7d8da1] dark:text-slate-400">
          {title}
        </h4>
        <h2 className="text-2xl lg:text-3xl font-extrabold text-[#363949] dark:text-white font-mono-num tracking-tight">
          {value}
        </h2>
        {subtitle && (
          <p className="text-[11px] text-[#7d8da1] dark:text-slate-400 font-medium">
            {subtitle}
          </p>
        )}
        {trend && (
          <div className="pt-1 flex items-center gap-1.5 text-xs font-semibold">
            <span
              className={trend.isPositive !== false ? 'text-[#1b9c85]' : 'text-[#ff0060]'}
            >
              {trend.value}
            </span>
            {trend.label && (
              <span className="text-[10px] text-[#7d8da1] dark:text-slate-500 font-normal">
                {trend.label}
              </span>
            )}
          </div>
        )}
      </div>

      {/* Right: AsmrProg Iconic SVG Circular Progress Ring */}
      <div className="relative flex items-center justify-center shrink-0">
        <svg className="w-20 h-20 transform -rotate-90">
          {/* Background Track */}
          <circle
            cx="40"
            cy="40"
            r={radius}
            stroke="currentColor"
            strokeWidth={strokeWidth}
            fill="transparent"
            className="text-slate-100 dark:text-slate-800"
          />
          {/* Animated Progress Arc */}
          <circle
            cx="40"
            cy="40"
            r={radius}
            stroke={accentColor}
            strokeWidth={strokeWidth}
            strokeDasharray={circumference}
            strokeDashoffset={strokeDashoffset}
            strokeLinecap="round"
            fill="transparent"
            className="transition-all duration-1000 ease-out"
          />
        </svg>

        {/* Center Percentage Text */}
        <div
          className="absolute inset-0 flex items-center justify-center text-xs font-extrabold font-mono"
          style={{ color: accentColor }}
        >
          {displayPercent}
        </div>
      </div>
    </div>
  );
};
