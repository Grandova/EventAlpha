import React from 'react';

interface StatCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  icon: React.ReactNode;
  iconBgColor?: string;
  iconColor?: string;
  trend?: {
    value: string;
    isPositive?: boolean;
    label?: string;
  };
  progress?: number; // 0 to 100 for SVG circular ring
  accentColor?: string;
}

export const StatCard: React.FC<StatCardProps> = ({
  title,
  value,
  subtitle,
  icon,
  iconBgColor = 'bg-cyan-500/10',
  iconColor = 'text-cyan-400',
  trend,
  progress,
  accentColor = '#38bdf8',
}) => {
  const radius = 24;
  const circumference = 2 * Math.PI * radius;
  const strokeDashoffset = progress !== undefined ? circumference - (progress / 100) * circumference : 0;

  return (
    <div className="relative overflow-hidden rounded-2xl bg-slate-900/80 border border-slate-800/80 p-5 shadow-xl backdrop-blur-xl transition-all duration-300 hover:border-slate-700/80 hover:shadow-cyan-500/5 group">
      {/* Subtle top gradient accent */}
      <div
        className="absolute top-0 left-0 right-0 h-1 opacity-60 transition-opacity duration-300 group-hover:opacity-100"
        style={{
          background: `linear-gradient(90deg, ${accentColor}, transparent)`,
        }}
      />

      <div className="flex items-start justify-between">
        <div>
          <p className="text-xs font-medium tracking-wider text-slate-400 uppercase">{title}</p>
          <h3 className="mt-2 text-2xl font-bold tracking-tight text-white font-mono-num">{value}</h3>
          {subtitle && <p className="mt-1 text-xs text-slate-500">{subtitle}</p>}

          {trend && (
            <div className="mt-3 flex items-center gap-1.5 text-xs">
              <span
                className={`inline-flex items-center px-2 py-0.5 rounded-full font-bold text-[11px] ${
                  trend.isPositive !== false
                    ? 'bg-emerald-500/15 text-emerald-400 border border-emerald-500/20'
                    : 'bg-rose-500/15 text-rose-400 border border-rose-500/20'
                }`}
              >
                {trend.isPositive !== false ? '↑' : '↓'} {trend.value}
              </span>
              {trend.label && <span className="text-slate-500 text-[11px]">{trend.label}</span>}
            </div>
          )}
        </div>

        {/* Right side: Icon or Circular Progress */}
        <div className="flex items-center gap-2">
          {progress !== undefined ? (
            <div className="relative flex items-center justify-center">
              <svg className="w-14 h-14 transform -rotate-90">
                <circle
                  cx="28"
                  cy="28"
                  r={radius}
                  stroke="currentColor"
                  strokeWidth="4"
                  fill="transparent"
                  className="text-slate-800/60"
                />
                <circle
                  cx="28"
                  cy="28"
                  r={radius}
                  stroke={accentColor}
                  strokeWidth="4"
                  strokeDasharray={circumference}
                  strokeDashoffset={strokeDashoffset}
                  strokeLinecap="round"
                  fill="transparent"
                  className="transition-all duration-700 ease-out"
                />
              </svg>
              <div className="absolute inset-0 flex items-center justify-center text-[11px] font-bold font-mono text-slate-300">
                {Math.round(progress)}%
              </div>
            </div>
          ) : (
            <div className={`p-3 rounded-xl ${iconBgColor} ${iconColor} shadow-inner transition-transform duration-300 group-hover:scale-110`}>
              {icon}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
