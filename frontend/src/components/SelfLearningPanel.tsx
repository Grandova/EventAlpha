import React, { useState, useEffect } from 'react';
import {
  Brain,
  Sparkles,
  RefreshCw,
  Sliders,
  Award,
  Zap,
  Layers,
  ArrowUpRight,
  ArrowDownRight,
  Clock,
  CheckCircle2,
} from 'lucide-react';
import { api } from '../services/api';
import { Asset, LearningState, LearningHistoryEntry } from '../types';

interface SelfLearningPanelProps {
  activeAsset: Asset;
  onSelectAsset?: (asset: Asset) => void;
}

export const SelfLearningPanel: React.FC<SelfLearningPanelProps> = ({
  activeAsset,
}) => {
  const [learningState, setLearningState] = useState<LearningState | null>(null);
  const [history, setHistory] = useState<LearningHistoryEntry[]>([]);
  const [autoLearningEnabled, setAutoLearningEnabled] = useState(true);
  const [isRetraining, setIsRetraining] = useState(false);
  const [feedbackMsg, setFeedbackMsg] = useState<string | null>(null);

  const loadLearningData = async () => {
    try {
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
          ? '自主在线自学习已开启：每轮 5 分钟盘面结算后自动更新梯度与校准'
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
  const version = learningState?.version ?? '1.0';

  return (
    <div className="space-y-6">
      {/* Top Banner / Header Card */}
      <div className="asmr-card p-6 lg:p-8 flex flex-col lg:flex-row lg:items-center justify-between gap-6">
        <div className="space-y-2">
          <div className="flex items-center gap-2.5">
            <span className="p-2 rounded-2xl bg-[#6c9bcf]/15 text-[#6c9bcf] border border-[#6c9bcf]/30">
              <Brain className="w-5 h-5 animate-pulse" />
            </span>
            <span className="text-xs font-extrabold uppercase tracking-wider font-mono text-[#6c9bcf]">
              CONTINUOUS ONLINE SELF-LEARNING
            </span>
            <span className="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-[#1b9c85]/15 text-[#1b9c85] font-mono">
              MODEL {version}
            </span>
          </div>

          <h2 className="text-2xl font-black text-[#363949] dark:text-white tracking-tight">
            自主在线学习与自进化中枢
          </h2>
          <p className="text-xs text-[#7d8da1] dark:text-slate-400 max-w-2xl leading-relaxed">
            系统在每轮 5 分钟盘面结算后，自动将微观失衡、跨所基差与价格动量等 37 维特征与真实胜负结果回传，通过在线 SGD 单轮增量迭代与 Platt 概率校准，使量化模型持续变强、自适应市场微观结构。
          </p>
        </div>

        {/* Action Controls */}
        <div className="flex flex-wrap items-center gap-3">
          <button
            onClick={handleToggleAutoLearning}
            className={`flex items-center gap-2 px-4 py-2.5 rounded-2xl border text-xs font-bold transition-all cursor-pointer ${
              autoLearningEnabled
                ? 'bg-[#1b9c85]/10 border-[#1b9c85]/30 text-[#1b9c85] shadow-sm'
                : 'bg-slate-100 dark:bg-slate-800 border-slate-200 dark:border-slate-700 text-[#7d8da1]'
            }`}
          >
            <span
              className={`w-2 h-2 rounded-full ${
                autoLearningEnabled ? 'bg-[#1b9c85] animate-ping' : 'bg-slate-400'
              }`}
            />
            <span>{autoLearningEnabled ? '单轮自适应已开启' : '自适应已暂停'}</span>
          </button>

          <button
            disabled={isRetraining}
            onClick={handleTriggerBatchRetrain}
            className="flex items-center gap-2 px-5 py-2.5 rounded-2xl bg-[#6c9bcf] text-white text-xs font-bold shadow-md shadow-[#6c9bcf]/20 hover:brightness-105 transition-all cursor-pointer disabled:opacity-50"
          >
            <RefreshCw className={`w-4 h-4 ${isRetraining ? 'animate-spin' : ''}`} />
            <span>{isRetraining ? '正在深度演进中...' : '全量历史回放自学习'}</span>
          </button>
        </div>
      </div>

      {feedbackMsg && (
        <div className="p-3.5 rounded-2xl bg-[#6c9bcf]/10 border border-[#6c9bcf]/30 text-[#6c9bcf] text-xs font-mono flex items-center gap-2">
          <CheckCircle2 className="w-4 h-4 shrink-0" />
          <span>{feedbackMsg}</span>
        </div>
      )}

      {/* KPI Cards Row (Matches AsmrProg cards) */}
      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-6">
        <div className="asmr-card p-6">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold uppercase tracking-wider text-[#7d8da1] font-mono">
              已结算学习盘面
            </span>
            <div className="p-2 rounded-xl bg-[#6c9bcf]/15 text-[#6c9bcf]">
              <Layers className="w-4 h-4" />
            </div>
          </div>
          <div className="mt-3 flex items-baseline gap-2">
            <span className="text-3xl font-black font-mono text-[#363949] dark:text-white">
              {totalLearned}
            </span>
            <span className="text-xs font-bold text-[#6c9bcf] font-mono">轮 5M 周期</span>
          </div>
          <p className="mt-2 text-[11px] text-[#7d8da1]">每轮自动更新梯度步进</p>
        </div>

        <div className="asmr-card p-6">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold uppercase tracking-wider text-[#7d8da1] font-mono">
              滚动预测胜率
            </span>
            <div className="p-2 rounded-xl bg-[#1b9c85]/15 text-[#1b9c85]">
              <Award className="w-4 h-4" />
            </div>
          </div>
          <div className="mt-3 flex items-baseline gap-2">
            <span className="text-3xl font-black font-mono text-[#1b9c85]">
              {accuracyPct}%
            </span>
            <span className="text-xs font-bold text-[#1b9c85] font-mono">Top Tier</span>
          </div>
          <p className="mt-2 text-[11px] text-[#7d8da1]">动态加权最近结算样本</p>
        </div>

        <div className="asmr-card p-6">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold uppercase tracking-wider text-[#7d8da1] font-mono">
              Brier 拟合分数
            </span>
            <div className="p-2 rounded-xl bg-[#6c9bcf]/15 text-[#6c9bcf]">
              <Sparkles className="w-4 h-4" />
            </div>
          </div>
          <div className="mt-3 flex items-baseline gap-2">
            <span className="text-3xl font-black font-mono text-[#363949] dark:text-white">
              {brierScore}
            </span>
            <span className="text-xs font-bold text-[#6c9bcf] font-mono">越低越精准</span>
          </div>
          <p className="mt-2 text-[11px] text-[#7d8da1]">基准随机得分: 0.2500</p>
        </div>

        <div className="asmr-card p-6">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold uppercase tracking-wider text-[#7d8da1] font-mono">
              动态概率校准 (Platt)
            </span>
            <div className="p-2 rounded-xl bg-[#f7d154]/20 text-amber-500">
              <Zap className="w-4 h-4" />
            </div>
          </div>
          <div className="mt-3 flex items-baseline gap-2 font-mono">
            <span className="text-xl font-black text-amber-600 dark:text-amber-400">
              a={(learningState?.platt_a ?? 1.0).toFixed(2)}
            </span>
            <span className="text-xs font-bold text-[#7d8da1]">
              b={(learningState?.platt_b ?? 0.0).toFixed(2)}
            </span>
          </div>
          <p className="mt-2 text-[11px] text-[#7d8da1]">学习率 η = {learningState?.learning_rate ?? 0.015}</p>
        </div>
      </div>

      {/* Middle Section: Feature Importance Ladder & Model Parameters */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        <div className="lg:col-span-8 asmr-card p-6 lg:p-8">
          <div className="flex items-center justify-between mb-5">
            <div>
              <h3 className="text-base font-extrabold text-[#363949] dark:text-white tracking-wide flex items-center gap-2">
                <Sliders className="w-4 h-4 text-[#6c9bcf]" />
                <span>特征重要性权重排行 (Feature Importance Ladder)</span>
              </h3>
              <p className="text-xs text-[#7d8da1] dark:text-slate-400 mt-0.5">
                实时展示自学习引擎根据胜负动态增强或抑制的 37 维特征
              </p>
            </div>
            <span className="text-xs font-mono text-[#7d8da1]">绝对值排序</span>
          </div>

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
                    className="p-3.5 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800 transition-all"
                  >
                    <div className="flex items-center justify-between text-xs font-mono mb-2">
                      <div className="flex items-center gap-2">
                        <span className="w-5 text-center text-[#7d8da1] font-bold">
                          #{idx + 1}
                        </span>
                        <span className="text-[#363949] dark:text-white font-semibold">{featureName}</span>
                      </div>
                      <div className="flex items-center gap-2 font-bold">
                        <span
                          className={`flex items-center gap-0.5 ${
                            isPositive ? 'text-[#1b9c85]' : 'text-[#ff0060]'
                          }`}
                        >
                          {isPositive ? (
                            <ArrowUpRight className="w-3.5 h-3.5" />
                          ) : (
                            <ArrowDownRight className="w-3.5 h-3.5" />
                          )}
                          {isPositive ? '推升 UP' : '推升 DOWN'}
                        </span>
                        <span className="text-[#363949] dark:text-slate-300">{weight.toFixed(4)}</span>
                      </div>
                    </div>

                    <div className="w-full bg-slate-200 dark:bg-slate-700 rounded-full h-2 overflow-hidden flex">
                      <div
                        className={`h-2 rounded-full transition-all duration-500 ${
                          isPositive ? 'bg-[#1b9c85]' : 'bg-[#ff0060]'
                        }`}
                        style={{ width: `${pct}%` }}
                      />
                    </div>
                  </div>
                );
              })
            ) : (
              <div className="text-center py-10 text-[#7d8da1] text-xs">
                正在等待下一轮 5M 盘面结算生成特征权重...
              </div>
            )}
          </div>
        </div>

        <div className="lg:col-span-4 asmr-card p-6 lg:p-8 flex flex-col justify-between">
          <div>
            <h3 className="text-base font-extrabold text-[#363949] dark:text-white tracking-wide flex items-center gap-2 mb-4">
              <Sparkles className="w-4 h-4 text-[#6c9bcf]" />
              <span>自进化逻辑机制</span>
            </h3>

            <div className="space-y-4 text-xs text-[#363949] dark:text-slate-300 leading-relaxed">
              <div className="p-4 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800">
                <div className="font-extrabold text-[#1b9c85] mb-1">1. 在线单轮 SGD 增量更新</div>
                <p className="text-[11px] text-[#7d8da1] dark:text-slate-400">
                  每个 5M 盘面决出结算（UP 或 DOWN）时，系统立刻计算交叉熵梯度并执行权重步进：w ← w - η(p - y)x。
                </p>
              </div>

              <div className="p-4 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800">
                <div className="font-extrabold text-[#6c9bcf] mb-1">2. Platt 动态概率校准</div>
                <p className="text-[11px] text-[#7d8da1] dark:text-slate-400">
                  自动修正 Logistic 输出概率的高估或低估，确保输出的 65% 置信度在长期大数定律下真实对应 65% 胜率。
                </p>
              </div>

              <div className="p-4 rounded-2xl bg-[#f6f6f9] dark:bg-[#181a1e] border border-slate-100 dark:border-slate-800">
                <div className="font-extrabold text-[#ff0060] mb-1">3. SQLite 状态持久化跨启继承</div>
                <p className="text-[11px] text-[#7d8da1] dark:text-slate-400">
                  学习到的权重与校准参数实时保存在数据库中，无论服务重启或迁移，模型知识永不丢失并持续累积。
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Bottom Section: Learning History Table */}
      <div className="asmr-card p-6 lg:p-8">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h3 className="text-base font-extrabold text-[#363949] dark:text-white tracking-wide flex items-center gap-2">
              <Clock className="w-4 h-4 text-[#1b9c85]" />
              <span>最近结算盘面自主学习履历 (Evolution Log)</span>
            </h3>
            <p className="text-xs text-[#7d8da1] dark:text-slate-400 mt-0.5">
              记录每次 5 分钟盘面结算后模型的单轮更新步长、损失函数与权重变动范数
            </p>
          </div>
          <span className="text-xs font-mono text-[#7d8da1]">
            最近 {history.length} 条记录
          </span>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-slate-100 dark:border-slate-800 text-[#7d8da1]">
                <th className="pb-3 font-semibold">Round ID</th>
                <th className="pb-3 font-semibold">预测概率 (P_UP)</th>
                <th className="pb-3 font-semibold">真实胜负</th>
                <th className="pb-3 font-semibold">交叉熵 Loss</th>
                <th className="pb-3 font-semibold">权重变动 ‖Δw‖</th>
                <th className="pb-3 font-semibold">Brier 得分</th>
                <th className="pb-3 font-semibold text-right">学习时间</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800/60">
              {history.length > 0 ? (
                history.map((item) => (
                  <tr
                    key={item.id}
                    className="hover:bg-slate-50 dark:hover:bg-slate-800/40 transition-colors"
                  >
                    <td className="py-3 text-[#363949] dark:text-white font-bold">
                      {item.round_id}
                    </td>
                    <td className="py-3">
                      <span
                        className={`font-bold ${
                          item.predicted_prob >= 0.5 ? 'text-[#1b9c85]' : 'text-[#ff0060]'
                        }`}
                      >
                        {(item.predicted_prob * 100).toFixed(1)}%
                      </span>
                    </td>
                    <td className="py-3">
                      <span
                        className={`px-2 py-0.5 rounded-md text-[10px] font-extrabold ${
                          item.actual_outcome === 1
                            ? 'bg-[#1b9c85]/15 text-[#1b9c85]'
                            : 'bg-[#ff0060]/15 text-[#ff0060]'
                        }`}
                      >
                        {item.actual_outcome === 1 ? 'UP (胜)' : 'DOWN (跌)'}
                      </span>
                    </td>
                    <td className="py-3 text-[#7d8da1]">{item.loss.toFixed(4)}</td>
                    <td className="py-3 text-[#6c9bcf] font-bold">
                      {item.weights_delta_norm.toFixed(5)}
                    </td>
                    <td className="py-3 text-[#363949] dark:text-slate-300">{item.brier_score.toFixed(4)}</td>
                    <td className="py-3 text-right text-[#7d8da1]">
                      {new Date(item.timestamp).toLocaleTimeString()}
                    </td>
                  </tr>
                ))
              ) : (
                <tr>
                  <td colSpan={7} className="py-8 text-center text-[#7d8da1]">
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
