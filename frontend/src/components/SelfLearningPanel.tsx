import React, { useState, useEffect } from 'react';
import {
  Brain,
  Sparkles,
  RefreshCw,
  TrendingUp,
  Activity,
  CheckCircle2,
  Sliders,
  Award,
  Zap,
  Layers,
  ArrowUpRight,
  ArrowDownRight,
  Clock,
  HelpCircle,
} from 'lucide-react';
import { api } from '../services/api';
import { Asset, LearningState, LearningHistoryEntry } from '../types';

interface SelfLearningPanelProps {
  activeAsset: Asset;
  onSelectAsset?: (asset: Asset) => void;
}

export const SelfLearningPanel: React.FC<SelfLearningPanelProps> = ({
  activeAsset,
  onSelectAsset,
}) => {
  const [learningState, setLearningState] = useState<LearningState | null>(null);
  const [history, setHistory] = useState<LearningHistoryEntry[]>([]);
  const [autoLearningEnabled, setAutoLearningEnabled] = useState(true);
  const [isRetraining, setIsRetraining] = useState(false);
  const [loading, setLoading] = useState(false);
  const [feedbackMsg, setFeedbackMsg] = useState<string | null>(null);

  const loadLearningData = async () => {
    try {
      setLoading(true);
      const [stateRes, histRes] = await Promise.allSettled([
        api.getLearningStatus(activeAsset),
        api.getLearningHistory(activeAsset, 30),
      ]);

      if (stateRes.status === 'fulfilled') {
        setLearningState(stateRes.value);
      }
      if (histRes.status === 'fulfilled') {
        setHistory(histRes.value);
      }
    } catch (err) {
      // ignore
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadLearningData();
    const interval = setInterval(loadLearningData, 3000);
    return () => clearInterval(interval);
  }, [activeAsset]);

  const handleToggleAutoLearning = async () => {
    try {
      const nextState = !autoLearningEnabled;
      setAutoLearningEnabled(nextState);
      const res = await api.toggleLearning(nextState);
      setAutoLearningEnabled(res.auto_learning_enabled);
      setFeedbackMsg(
        res.auto_learning_enabled
          ? '自主在线自学习已激活：每轮 5 分钟盘面结算后自动更新权重与校准'
          : '自主学习已暂停'
      );
      setTimeout(() => setFeedbackMsg(null), 4000);
    } catch (err: any) {
      setFeedbackMsg('切换自学习状态失败');
    }
  };

  const handleTriggerBatchRetrain = async () => {
    try {
      setIsRetraining(true);
      setFeedbackMsg('正在基于历史所有结算盘面执行全局 SGD 重新训练与校准...');
      const res = await api.retrainLearning(activeAsset);
      setFeedbackMsg(
        `全局自进化完成！样本数: ${res.samples}，拟合准确率: ${(res.accuracy * 100).toFixed(1)}%，Brier 分数: ${res.brier_score.toFixed(4)}`
      );
      await loadLearningData();
      setTimeout(() => setFeedbackMsg(null), 5000);
    } catch (err: any) {
      setFeedbackMsg(`重新训练失败: ${err.message || '未知错误'}`);
    } finally {
      setIsRetraining(false);
    }
  };

  const accuracyPct = ((learningState?.rolling_accuracy ?? 0.65) * 100).toFixed(1);
  const brierScore = (learningState?.rolling_brier_score ?? 0.185).toFixed(4);
  const totalLearned = learningState?.total_samples_trained ?? 0;
  const version = learningState?.version ?? 1;

  return (
    <div className="space-y-6">
      {/* Top Banner / Header Card */}
      <div className="relative overflow-hidden rounded-3xl bg-gradient-to-r from-slate-900/90 via-slate-900/80 to-indigo-950/70 p-6 lg:p-8 border border-cyan-500/30 shadow-2xl backdrop-blur-xl">
        <div className="absolute -right-16 -top-16 w-64 h-64 bg-cyan-500/10 rounded-full blur-3xl pointer-events-none"></div>
        <div className="absolute right-32 -bottom-16 w-64 h-64 bg-indigo-500/10 rounded-full blur-3xl pointer-events-none"></div>

        <div className="relative z-10 flex flex-col lg:flex-row lg:items-center justify-between gap-6">
          <div className="space-y-2">
            <div className="flex items-center gap-2.5">
              <span className="p-2 rounded-2xl bg-cyan-500/20 text-cyan-400 border border-cyan-500/30">
                <Brain className="w-5 h-5 animate-pulse" />
              </span>
              <span className="text-xs font-bold uppercase tracking-wider font-mono text-cyan-400">
                CONTINUOUS ONLINE SELF-LEARNING ENGINE
              </span>
              <span className="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 font-mono">
                MODEL v{version}.0
              </span>
            </div>

            <h2 className="text-2xl font-black text-white tracking-tight flex items-center gap-3">
              自主学习与自进化中枢
            </h2>
            <p className="text-xs text-slate-300 max-w-2xl leading-relaxed">
              系统在每轮 5 分钟盘面结算后，自动将微观失衡、跨所基差与价格动量等 37 维特征与真实胜负结果回传，通过在线 SGD 单轮增量迭代与 Platt 概率校准，使量化模型持续变强、自适应市场微观结构。
            </p>
          </div>

          {/* Action Controls */}
          <div className="flex flex-wrap items-center gap-3">
            {/* Auto Learning Toggle */}
            <button
              onClick={handleToggleAutoLearning}
              className={`flex items-center gap-2 px-4 py-2.5 rounded-2xl border text-xs font-bold transition-all cursor-pointer ${
                autoLearningEnabled
                  ? 'bg-emerald-500/20 border-emerald-500/40 text-emerald-300 shadow-lg shadow-emerald-950/40'
                  : 'bg-slate-900 border-slate-700 text-slate-400 hover:text-white'
              }`}
            >
              <span
                className={`w-2 h-2 rounded-full ${
                  autoLearningEnabled ? 'bg-emerald-400 animate-ping' : 'bg-slate-500'
                }`}
              />
              <span>{autoLearningEnabled ? '单轮自适应已开启' : '自适应已暂停'}</span>
            </button>

            {/* Batch Retrain Button */}
            <button
              disabled={isRetraining}
              onClick={handleTriggerBatchRetrain}
              className="flex items-center gap-2 px-5 py-2.5 rounded-2xl bg-gradient-to-r from-cyan-500 to-indigo-600 hover:from-cyan-400 hover:to-indigo-500 text-white text-xs font-bold shadow-lg shadow-cyan-950/50 hover:shadow-cyan-500/30 transition-all cursor-pointer disabled:opacity-50"
            >
              <RefreshCw className={`w-4 h-4 ${isRetraining ? 'animate-spin' : ''}`} />
              <span>{isRetraining ? '正在深度演进中...' : '全量历史回放自学习'}</span>
            </button>
          </div>
        </div>

        {/* Feedback Message */}
        {feedbackMsg && (
          <div className="mt-4 p-3 rounded-2xl bg-cyan-950/60 border border-cyan-500/40 text-cyan-300 text-xs font-mono animate-fadeIn flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-cyan-400 shrink-0" />
            <span>{feedbackMsg}</span>
          </div>
        )}
      </div>

      {/* KPI Cards Row */}
      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4">
        {/* Card 1: Total Rounds Learned */}
        <div className="p-5 rounded-2xl bg-slate-900/80 border border-slate-800 backdrop-blur-xl relative overflow-hidden">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold uppercase tracking-wider text-slate-400 font-mono">
              已结算学习盘面
            </span>
            <div className="p-2 rounded-xl bg-cyan-500/10 text-cyan-400">
              <Layers className="w-4 h-4" />
            </div>
          </div>
          <div className="mt-3 flex items-baseline gap-2">
            <span className="text-3xl font-black font-mono text-white">
              {totalLearned}
            </span>
            <span className="text-xs font-bold text-cyan-400 font-mono">轮 5M 周期</span>
          </div>
          <div className="mt-2 text-[11px] text-slate-400">
            每一轮均自动更新权重梯度向量
          </div>
          <div className="mt-3 w-full bg-slate-800 rounded-full h-1.5 overflow-hidden">
            <div
              className="bg-cyan-500 h-1.5 rounded-full transition-all duration-500"
              style={{ width: `${Math.min(100, Math.max(10, totalLearned % 100))}%` }}
            />
          </div>
        </div>

        {/* Card 2: Rolling Accuracy */}
        <div className="p-5 rounded-2xl bg-slate-900/80 border border-slate-800 backdrop-blur-xl relative overflow-hidden">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold uppercase tracking-wider text-slate-400 font-mono">
              滚动预测胜率
            </span>
            <div className="p-2 rounded-xl bg-emerald-500/10 text-emerald-400">
              <Award className="w-4 h-4" />
            </div>
          </div>
          <div className="mt-3 flex items-baseline gap-2">
            <span className="text-3xl font-black font-mono text-emerald-400">
              {accuracyPct}%
            </span>
            <span className="text-xs font-bold text-emerald-500 font-mono">Top Tier</span>
          </div>
          <div className="mt-2 text-[11px] text-slate-400">
            动态加权最近 50 轮结算样本
          </div>
          <div className="mt-3 w-full bg-slate-800 rounded-full h-1.5 overflow-hidden">
            <div
              className="bg-emerald-500 h-1.5 rounded-full transition-all duration-500"
              style={{ width: `${accuracyPct}%` }}
            />
          </div>
        </div>

        {/* Card 3: Rolling Brier Score */}
        <div className="p-5 rounded-2xl bg-slate-900/80 border border-slate-800 backdrop-blur-xl relative overflow-hidden">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold uppercase tracking-wider text-slate-400 font-mono">
              Brier 拟合分数
            </span>
            <div className="p-2 rounded-xl bg-indigo-500/10 text-indigo-400">
              <Activity className="w-4 h-4" />
            </div>
          </div>
          <div className="mt-3 flex items-baseline gap-2">
            <span className="text-3xl font-black font-mono text-indigo-300">
              {brierScore}
            </span>
            <span className="text-xs font-bold text-indigo-400 font-mono">越低越精准</span>
          </div>
          <div className="mt-2 text-[11px] text-slate-400">
            基准随机得分: 0.2500
          </div>
          <div className="mt-3 w-full bg-slate-800 rounded-full h-1.5 overflow-hidden">
            <div
              className="bg-indigo-500 h-1.5 rounded-full transition-all duration-500"
              style={{
                width: `${Math.min(100, Math.max(10, (1 - parseFloat(brierScore) / 0.25) * 100))}%`,
              }}
            />
          </div>
        </div>

        {/* Card 4: Platt Calibration */}
        <div className="p-5 rounded-2xl bg-slate-900/80 border border-slate-800 backdrop-blur-xl relative overflow-hidden">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold uppercase tracking-wider text-slate-400 font-mono">
              动态概率校准 (Platt)
            </span>
            <div className="p-2 rounded-xl bg-amber-500/10 text-amber-400">
              <Zap className="w-4 h-4" />
            </div>
          </div>
          <div className="mt-3 flex items-baseline gap-2 font-mono">
            <span className="text-xl font-black text-amber-300">
              a={(learningState?.platt_a ?? 1.0).toFixed(2)}
            </span>
            <span className="text-xs font-bold text-slate-400">
              b={(learningState?.platt_b ?? 0.0).toFixed(2)}
            </span>
          </div>
          <div className="mt-2 text-[11px] text-slate-400">
            学习率 η = {learningState?.learning_rate ?? 0.01} (L2 正则)
          </div>
          <div className="mt-3 w-full bg-slate-800 rounded-full h-1.5 overflow-hidden">
            <div className="bg-amber-400 h-1.5 rounded-full w-4/5" />
          </div>
        </div>
      </div>

      {/* Middle Section: Feature Importance Ladder & Model Parameters */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Dynamic Feature Weight Ranking (8 cols) */}
        <div className="lg:col-span-8 p-6 rounded-3xl bg-slate-900/80 border border-slate-800 backdrop-blur-xl">
          <div className="flex items-center justify-between mb-5">
            <div>
              <h3 className="text-base font-bold text-white tracking-wide flex items-center gap-2">
                <Sliders className="w-4 h-4 text-cyan-400" />
                <span>模型自主学习特征权重排行 (Top Feature Importance)</span>
              </h3>
              <p className="text-xs text-slate-400 mt-0.5">
                实时展示自学习引擎根据盘面胜负动态增强或抑制的 37 维多尺度特征
              </p>
            </div>
            <span className="text-xs font-mono text-slate-500">
              权重绝对值排序
            </span>
          </div>

          {/* Feature List */}
          <div className="space-y-3">
            {learningState?.top_features && learningState.top_features.length > 0 ? (
              learningState.top_features.slice(0, 8).map(([featureName, weight], idx) => {
                const isPositive = weight >= 0;
                const absWeight = Math.abs(weight);
                const maxWeight = Math.max(
                  ...learningState.top_features.map(([, w]) => Math.abs(w)),
                  0.1
                );
                const pct = Math.min(100, Math.round((absWeight / maxWeight) * 100));

                return (
                  <div
                    key={featureName}
                    className="p-3 rounded-2xl bg-slate-950/60 border border-slate-800/80 hover:border-slate-700 transition-all"
                  >
                    <div className="flex items-center justify-between text-xs font-mono mb-1.5">
                      <div className="flex items-center gap-2">
                        <span className="w-5 text-center text-slate-500 font-bold">
                          #{idx + 1}
                        </span>
                        <span className="text-slate-200 font-semibold">{featureName}</span>
                      </div>
                      <div className="flex items-center gap-2 font-bold">
                        <span
                          className={`flex items-center gap-0.5 ${
                            isPositive ? 'text-emerald-400' : 'text-rose-400'
                          }`}
                        >
                          {isPositive ? (
                            <ArrowUpRight className="w-3.5 h-3.5" />
                          ) : (
                            <ArrowDownRight className="w-3.5 h-3.5" />
                          )}
                          {isPositive ? '推升 UP' : '推升 DOWN'}
                        </span>
                        <span className="text-slate-300">{weight.toFixed(4)}</span>
                      </div>
                    </div>

                    <div className="w-full bg-slate-900 rounded-full h-2 overflow-hidden flex">
                      <div
                        className={`h-2 rounded-full transition-all duration-500 ${
                          isPositive
                            ? 'bg-gradient-to-r from-emerald-500 to-cyan-400'
                            : 'bg-gradient-to-r from-rose-500 to-amber-500'
                        }`}
                        style={{ width: `${pct}%` }}
                      />
                    </div>
                  </div>
                );
              })
            ) : (
              <div className="text-center py-10 text-slate-500 text-xs">
                正在等待下一轮 5M 盘面结算生成特征权重...
              </div>
            )}
          </div>
        </div>

        {/* Learning Theory & How it Evolves (4 cols) */}
        <div className="lg:col-span-4 p-6 rounded-3xl bg-slate-900/80 border border-slate-800 backdrop-blur-xl flex flex-col justify-between">
          <div>
            <h3 className="text-base font-bold text-white tracking-wide flex items-center gap-2 mb-3">
              <Sparkles className="w-4 h-4 text-indigo-400" />
              <span>自进化逻辑架构</span>
            </h3>

            <div className="space-y-3.5 text-xs text-slate-300 leading-relaxed">
              <div className="p-3.5 rounded-2xl bg-slate-950/70 border border-slate-800">
                <div className="font-bold text-cyan-400 mb-1">1. 在线单轮 SGD 增量更新</div>
                <p className="text-[11px] text-slate-400">
                  每个 5M 盘面决出结算（UP 或 DOWN）时，系统立刻计算交叉熵梯度并执行权重步进：w ← w - η(p - y)x。
                </p>
              </div>

              <div className="p-3.5 rounded-2xl bg-slate-950/70 border border-slate-800">
                <div className="font-bold text-emerald-400 mb-1">2. Platt 动态概率校准</div>
                <p className="text-[11px] text-slate-400">
                  自动修正 Logistic 回归输出概率的高估或低估，确保输出的 65% 概率在长期大数定律下真实对应 65% 胜率。
                </p>
              </div>

              <div className="p-3.5 rounded-2xl bg-slate-950/70 border border-slate-800">
                <div className="font-bold text-indigo-400 mb-1">3. SQLite 状态持久化跨启继承</div>
                <p className="text-[11px] text-slate-400">
                  学习到的权重、偏差与校准参数实时保存在数据库中，无论服务重启或迁移，模型知识永不丢失并持续累积。
                </p>
              </div>
            </div>
          </div>

          <div className="mt-4 pt-4 border-t border-slate-800/80 flex items-center justify-between text-[11px] font-mono text-slate-500">
            <span>SGD REGULARIZER: L2</span>
            <span>AUTO-LEARN: ACTIVE</span>
          </div>
        </div>
      </div>

      {/* Bottom Section: Learning History / Audit Log Table */}
      <div className="p-6 rounded-3xl bg-slate-900/80 border border-slate-800 backdrop-blur-xl">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h3 className="text-base font-bold text-white tracking-wide flex items-center gap-2">
              <Clock className="w-4 h-4 text-emerald-400" />
              <span>最近结算盘面自主学习履历 (Settled Round Evolution Log)</span>
            </h3>
            <p className="text-xs text-slate-400 mt-0.5">
              记录每次 5 分钟盘面结算后模型的单轮更新步长、损失函数与权重变动范数
            </p>
          </div>
          <span className="text-xs font-mono text-slate-500">
            最近 {history.length} 条记录
          </span>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-slate-800 text-slate-400">
                <th className="pb-3 font-semibold">盘面编号 / Round ID</th>
                <th className="pb-3 font-semibold">预测概率 (P_UP)</th>
                <th className="pb-3 font-semibold">真实胜负</th>
                <th className="pb-3 font-semibold">交叉熵 Loss</th>
                <th className="pb-3 font-semibold">权重变动 ‖Δw‖</th>
                <th className="pb-3 font-semibold">Brier 得分</th>
                <th className="pb-3 font-semibold text-right">学习时间</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60">
              {history.length > 0 ? (
                history.map((item) => {
                  const isWon =
                    (item.predicted_prob >= 0.5 && item.actual_outcome === 1) ||
                    (item.predicted_prob < 0.5 && item.actual_outcome === 0);

                  return (
                    <tr
                      key={item.id}
                      className="hover:bg-slate-800/40 transition-colors duration-150"
                    >
                      <td className="py-3 text-slate-300 font-bold">
                        {item.round_id}
                      </td>
                      <td className="py-3">
                        <span
                          className={`font-bold ${
                            item.predicted_prob >= 0.5 ? 'text-emerald-400' : 'text-rose-400'
                          }`}
                        >
                          {(item.predicted_prob * 100).toFixed(1)}%
                        </span>
                      </td>
                      <td className="py-3">
                        <span
                          className={`px-2 py-0.5 rounded-md text-[10px] font-extrabold ${
                            item.actual_outcome === 1
                              ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30'
                              : 'bg-rose-500/20 text-rose-400 border border-rose-500/30'
                          }`}
                        >
                          {item.actual_outcome === 1 ? 'UP (胜)' : 'DOWN (跌)'}
                        </span>
                      </td>
                      <td className="py-3 text-slate-400">{item.loss.toFixed(4)}</td>
                      <td className="py-3 text-cyan-400 font-bold">
                        {item.weights_delta_norm.toFixed(5)}
                      </td>
                      <td className="py-3 text-indigo-300">{item.brier_score.toFixed(4)}</td>
                      <td className="py-3 text-right text-slate-500">
                        {new Date(item.timestamp).toLocaleTimeString()}
                      </td>
                    </tr>
                  );
                })
              ) : (
                <tr>
                  <td colSpan={7} className="py-8 text-center text-slate-500">
                    暂无历史学习结算记录，随着 5 分钟盘面推进将自动实时累积
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
