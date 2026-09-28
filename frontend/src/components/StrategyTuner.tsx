import React, { useState, useEffect } from 'react';
import {
  Sliders,
  CheckCircle2,
  AlertCircle,
  Database,
  Cpu,
  RefreshCw,
  Save,
  Zap,
} from 'lucide-react';
import { StrategyConfig, ModelTrainResult, GenerateSyntheticResponse } from '../types';
import { api } from '../services/api';

interface StrategyTunerProps {
  onConfigUpdated?: (config: StrategyConfig) => void;
}

export const StrategyTuner: React.FC<StrategyTunerProps> = ({ onConfigUpdated }) => {
  const [config, setConfig] = useState<StrategyConfig | null>(null);
  const [loading, setLoading] = useState(false);
  const [saveStatus, setSaveStatus] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Synthetic Data State
  const [syntheticRounds, setSyntheticRounds] = useState(50);
  const [isGenerating, setIsGenerating] = useState(false);
  const [syntheticResult, setSyntheticResult] = useState<GenerateSyntheticResponse | null>(null);

  // Model Retraining State
  const [epochs, setEpochs] = useState(25);
  const [lr, setLr] = useState(0.05);
  const [isTraining, setIsTraining] = useState(false);
  const [trainResult, setTrainResult] = useState<ModelTrainResult | null>(null);

  useEffect(() => {
    fetchConfig();
  }, []);

  const fetchConfig = async () => {
    try {
      setLoading(true);
      const data = await api.getStrategyConfig();
      setConfig(data);
    } catch (err: any) {
      setErrorMessage(err.message || 'Failed to fetch strategy config');
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!config) return;

    try {
      setSaveStatus('saving');
      const updated = await api.updateStrategyConfig(config);
      setConfig(updated);
      setSaveStatus('saved');
      if (onConfigUpdated) onConfigUpdated(updated);
      setTimeout(() => setSaveStatus('idle'), 3000);
    } catch (err: any) {
      setSaveStatus('error');
      setErrorMessage(err.message || 'Failed to save config');
    }
  };

  const handleGenerateSynthetic = async () => {
    try {
      setIsGenerating(true);
      setSyntheticResult(null);
      const res = await api.generateSyntheticDataset(syntheticRounds);
      setSyntheticResult(res);
    } catch (err: any) {
      setErrorMessage(err.message || 'Failed to generate synthetic data');
    } finally {
      setIsGenerating(false);
    }
  };

  const handleTrainModel = async () => {
    try {
      setIsTraining(true);
      setTrainResult(null);
      const res = await api.trainModel({ epochs, lr, l2_reg: 0.001 });
      setTrainResult(res);
    } catch (err: any) {
      setErrorMessage(err.message || 'Model training failed');
    } finally {
      setIsTraining(false);
    }
  };

  if (loading && !config) {
    return (
      <div className="quant-card p-6 flex items-center justify-center gap-2 text-slate-400">
        <RefreshCw className="h-4 w-4 animate-spin text-cyan-400" />
        <span>Loading strategy parameters...</span>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* 1. Dynamic Strategy Parameter Hot-Reload Panel */}
      <div className="quant-card p-4">
        <div className="flex flex-wrap items-center justify-between gap-3 mb-4 pb-3 border-b border-slate-800">
          <div className="flex items-center gap-2">
            <div className="p-1.5 bg-amber-950 border border-amber-800/80 rounded">
              <Sliders className="h-4 w-4 text-amber-400" />
            </div>
            <div>
              <h3 className="text-sm font-bold text-white flex items-center gap-2">
                Strategy Parameter Hot-Reload
                <span className="text-[10px] text-amber-400 font-mono bg-amber-950/60 px-2 py-0.5 rounded border border-amber-800/60">
                  {config?.strategy_version || 'v1.0.0'}
                </span>
              </h3>
              <p className="text-xs text-slate-400">
                Adjust quantitative gate thresholds in real-time with zero system downtime
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            {saveStatus === 'saved' && (
              <span className="text-xs text-emerald-400 flex items-center gap-1 font-semibold">
                <CheckCircle2 className="h-3.5 w-3.5" />
                Parameters Hot-Reloaded!
              </span>
            )}
            {saveStatus === 'error' && (
              <span className="text-xs text-rose-400 flex items-center gap-1 font-semibold">
                <AlertCircle className="h-3.5 w-3.5" />
                Update Failed
              </span>
            )}
            <button
              onClick={handleSave}
              disabled={saveStatus === 'saving' || !config}
              className="px-3 py-1.5 bg-amber-600 hover:bg-amber-500 disabled:opacity-50 text-slate-950 font-bold text-xs rounded-lg transition-colors flex items-center gap-1.5 shadow-sm"
            >
              {saveStatus === 'saving' ? (
                <>
                  <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                  Applying...
                </>
              ) : (
                <>
                  <Save className="h-3.5 w-3.5" />
                  Apply Hot-Reload
                </>
              )}
            </button>
          </div>
        </div>

        {config && (
          <form onSubmit={handleSave} className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 text-xs">
            {/* Min Calibrated Probability */}
            <div className="bg-slate-900/60 border border-slate-800 p-3 rounded-lg space-y-1.5">
              <div className="flex justify-between items-center">
                <label className="font-semibold text-slate-300">Min Win Probability (P)</label>
                <span className="font-mono font-bold text-cyan-400">
                  {(config.min_probability * 100).toFixed(0)}%
                </span>
              </div>
              <input
                type="range"
                min="0.50"
                max="0.90"
                step="0.01"
                value={config.min_probability}
                onChange={(e) =>
                  setConfig({ ...config, min_probability: parseFloat(e.target.value) })
                }
                className="w-full accent-cyan-400 cursor-pointer"
              />
              <p className="text-[10px] text-slate-500">
                Signals rejected if predicted directional probability is below this threshold.
              </p>
            </div>

            {/* Min Net Edge */}
            <div className="bg-slate-900/60 border border-slate-800 p-3 rounded-lg space-y-1.5">
              <div className="flex justify-between items-center">
                <label className="font-semibold text-slate-300">Min Net Edge (Fee+Slip Deducted)</label>
                <span className="font-mono font-bold text-emerald-400">
                  +{(config.min_net_edge * 100).toFixed(1)}%
                </span>
              </div>
              <input
                type="range"
                min="0.01"
                max="0.15"
                step="0.005"
                value={config.min_net_edge}
                onChange={(e) =>
                  setConfig({ ...config, min_net_edge: parseFloat(e.target.value) })
                }
                className="w-full accent-emerald-400 cursor-pointer"
              />
              <p className="text-[10px] text-slate-500">
                Strict edge hurdle ensuring expected net positive profit expectancy.
              </p>
            </div>

            {/* Max Entry Price Ceiling */}
            <div className="bg-slate-900/60 border border-slate-800 p-3 rounded-lg space-y-1.5">
              <div className="flex justify-between items-center">
                <label className="font-semibold text-slate-300">Max Entry Price Ceiling</label>
                <span className="font-mono font-bold text-amber-400">
                  ${config.max_entry_price.toFixed(2)}
                </span>
              </div>
              <input
                type="range"
                min="0.60"
                max="0.95"
                step="0.01"
                value={config.max_entry_price}
                onChange={(e) =>
                  setConfig({ ...config, max_entry_price: parseFloat(e.target.value) })
                }
                className="w-full accent-amber-400 cursor-pointer"
              />
              <p className="text-[10px] text-slate-500">
                Prevents entering overbought contracts with unfavorable risk/reward skew.
              </p>
            </div>

            {/* Max Spread Hurdle */}
            <div className="bg-slate-900/60 border border-slate-800 p-3 rounded-lg space-y-1.5">
              <div className="flex justify-between items-center">
                <label className="font-semibold text-slate-300">Max Allowable Spread</label>
                <span className="font-mono font-bold text-rose-400">
                  {(config.max_spread * 100).toFixed(1)}¢
                </span>
              </div>
              <input
                type="range"
                min="0.01"
                max="0.10"
                step="0.005"
                value={config.max_spread}
                onChange={(e) =>
                  setConfig({ ...config, max_spread: parseFloat(e.target.value) })
                }
                className="w-full accent-rose-400 cursor-pointer"
              />
              <p className="text-[10px] text-slate-500">
                Rejects trades if Polymarket bid-ask spread is wider than this limit.
              </p>
            </div>

            {/* Min Liquidity Requirement */}
            <div className="bg-slate-900/60 border border-slate-800 p-3 rounded-lg space-y-1.5">
              <div className="flex justify-between items-center">
                <label className="font-semibold text-slate-300">Min Orderbook Liquidity</label>
                <span className="font-mono font-bold text-cyan-400">
                  ${config.min_liquidity.toFixed(0)} USDC
                </span>
              </div>
              <input
                type="number"
                min="50"
                max="50000"
                step="50"
                value={config.min_liquidity}
                onChange={(e) =>
                  setConfig({ ...config, min_liquidity: parseFloat(e.target.value) || 0 })
                }
                className="quant-input text-xs w-full py-1"
              />
              <p className="text-[10px] text-slate-500">
                Guarantees adequate orderbook depth to fill orders with low market impact.
              </p>
            </div>

            {/* Opportunity Score Threshold */}
            <div className="bg-slate-900/60 border border-slate-800 p-3 rounded-lg space-y-1.5">
              <div className="flex justify-between items-center">
                <label className="font-semibold text-slate-300">Min Opportunity Score (0-100)</label>
                <span className="font-mono font-bold text-purple-400">
                  {config.score_thresholds.skip_below.toFixed(0)} pts
                </span>
              </div>
              <input
                type="range"
                min="40"
                max="85"
                step="1"
                value={config.score_thresholds.skip_below}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    score_thresholds: {
                      ...config.score_thresholds,
                      skip_below: parseFloat(e.target.value),
                    },
                  })
                }
                className="w-full accent-purple-400 cursor-pointer"
              />
              <p className="text-[10px] text-slate-500">
                Minimum total composite score across Edge, Probability, OBI, & Momentum.
              </p>
            </div>
          </form>
        )}
      </div>

      {/* 2. Quantitative Testing & Offline ML Tools */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Synthetic Data Generator */}
        <div className="quant-card p-4">
          <div className="flex items-center gap-2 mb-3 pb-2 border-b border-slate-800">
            <div className="p-1 bg-cyan-950 border border-cyan-800/80 rounded">
              <Database className="h-3.5 w-3.5 text-cyan-400" />
            </div>
            <div>
              <h4 className="text-xs font-bold text-white">Realistic Scenario Generator</h4>
              <p className="text-[10px] text-slate-400">
                Generate synthetic resolved 5M rounds with 37-dim feature vectors for immediate backtesting
              </p>
            </div>
          </div>

          <div className="flex items-center gap-3 mb-3">
            <div className="flex-1">
              <label className="text-[10px] text-slate-400 block mb-1">Rounds to Generate:</label>
              <select
                value={syntheticRounds}
                onChange={(e) => setSyntheticRounds(parseInt(e.target.value, 10))}
                className="quant-input text-xs w-full py-1.5"
              >
                <option value={20}>20 Rounds (60 markets, ~120 samples)</option>
                <option value={50}>50 Rounds (150 markets, ~300 samples)</option>
                <option value={100}>100 Rounds (300 markets, ~600 samples)</option>
              </select>
            </div>

            <button
              onClick={handleGenerateSynthetic}
              disabled={isGenerating}
              className="mt-4 px-4 py-1.5 bg-cyan-600 hover:bg-cyan-500 disabled:opacity-50 text-white font-bold text-xs rounded-lg transition-colors flex items-center gap-1.5"
            >
              {isGenerating ? (
                <>
                  <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                  Generating...
                </>
              ) : (
                <>
                  <Zap className="h-3.5 w-3.5" />
                  Generate Data
                </>
              )}
            </button>
          </div>

          {syntheticResult && (
            <div className="p-2.5 bg-emerald-950/40 border border-emerald-800/60 rounded-lg text-xs text-emerald-300">
              <div className="font-bold flex items-center gap-1.5 mb-1">
                <CheckCircle2 className="h-3.5 w-3.5 text-emerald-400" />
                {syntheticResult.message}
              </div>
              <div className="text-[11px] text-slate-300 font-mono">
                Markets: {syntheticResult.markets_created} | Features: {syntheticResult.features_created}
              </div>
            </div>
          )}
        </div>

        {/* Offline SGD Model Retrainer */}
        <div className="quant-card p-4">
          <div className="flex items-center gap-2 mb-3 pb-2 border-b border-slate-800">
            <div className="p-1 bg-purple-950 border border-purple-800/80 rounded">
              <Cpu className="h-3.5 w-3.5 text-purple-400" />
            </div>
            <div>
              <h4 className="text-xs font-bold text-white">Offline SGD ML Model Retrainer</h4>
              <p className="text-[10px] text-slate-400">
                Train L2 regularized logistic regression on resolved walk-forward SQLite records
              </p>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-2 mb-3">
            <div>
              <label className="text-[10px] text-slate-400 block mb-1">Epochs:</label>
              <input
                type="number"
                min="5"
                max="100"
                value={epochs}
                onChange={(e) => setEpochs(parseInt(e.target.value, 10) || 10)}
                className="quant-input text-xs w-full py-1"
              />
            </div>
            <div>
              <label className="text-[10px] text-slate-400 block mb-1">Learning Rate (LR):</label>
              <input
                type="number"
                min="0.001"
                max="0.5"
                step="0.01"
                value={lr}
                onChange={(e) => setLr(parseFloat(e.target.value) || 0.05)}
                className="quant-input text-xs w-full py-1"
              />
            </div>
          </div>

          <button
            onClick={handleTrainModel}
            disabled={isTraining}
            className="w-full py-1.5 bg-purple-600 hover:bg-purple-500 disabled:opacity-50 text-white font-bold text-xs rounded-lg transition-colors flex items-center justify-center gap-1.5 mb-2"
          >
            {isTraining ? (
              <>
                <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                Optimizing Weights via SGD...
              </>
            ) : (
              <>
                <Cpu className="h-3.5 w-3.5" />
                Retrain Model on Historical DB
              </>
            )}
          </button>

          {trainResult && (
            <div className="p-2.5 bg-slate-900 border border-purple-800/60 rounded-lg text-xs space-y-1">
              <div className="font-bold text-purple-400 flex items-center justify-between">
                <span>Model: {trainResult.model_version}</span>
                <span className="text-emerald-400">{(trainResult.accuracy * 100).toFixed(1)}% Acc</span>
              </div>
              <div className="grid grid-cols-3 gap-1 font-mono text-[10px] text-slate-400">
                <div>Brier: {trainResult.brier_score.toFixed(4)}</div>
                <div>LogLoss: {trainResult.log_loss.toFixed(4)}</div>
                <div>ECE: {(trainResult.expected_calibration_error * 100).toFixed(1)}%</div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
