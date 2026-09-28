import React, { useState, useEffect } from 'react';
import { Play, Pause, SkipForward, RotateCcw, FastForward, Film, AlertCircle } from 'lucide-react';
import { Asset, ReplayConfig, ReplayFrame, ReplayStateResponse, ReplayStatus } from '../types';
import { api } from '../services/api';

interface ReplayConsoleProps {
  defaultAsset: Asset;
}

export const ReplayConsole: React.FC<ReplayConsoleProps> = ({ defaultAsset }) => {
  const [asset, setAsset] = useState<Asset>(defaultAsset);
  const [speed, setSpeed] = useState<number>(5.0);
  const [state, setState] = useState<ReplayStateResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Poll status when playing
  useEffect(() => {
    let timer: any;
    const fetchStatus = async () => {
      try {
        const s = await api.getReplayStatus();
        setState(s);
      } catch (e) {
        // ignore network error
      }
    };

    fetchStatus();
    if (state?.status === 'playing') {
      timer = setInterval(fetchStatus, 300);
    }
    return () => clearInterval(timer);
  }, [state?.status]);

  const handleStart = async () => {
    setLoading(true);
    setError(null);
    try {
      const cfg: ReplayConfig = {
        asset,
        speed_multiplier: speed,
      };
      const res = await api.startReplay(cfg);
      setState(res);
    } catch (e: any) {
      setError(e?.message || 'Failed to start replay');
    } finally {
      setLoading(false);
    }
  };

  const handlePause = async () => {
    const res = await api.pauseReplay();
    setState(res);
  };

  const handleResume = async () => {
    const res = await api.resumeReplay();
    setState(res);
  };

  const handleStep = async () => {
    const frame = await api.stepReplay();
    if (frame) {
      const s = await api.getReplayStatus();
      setState(s);
    }
  };

  const handleSeek = async (idx: number) => {
    const frame = await api.seekReplay({ target_frame_index: idx });
    if (frame) {
      const s = await api.getReplayStatus();
      setState(s);
    }
  };

  const handleSpeedChange = async (newSpeed: number) => {
    setSpeed(newSpeed);
    await api.setReplaySpeed(newSpeed);
  };

  const handleStop = async () => {
    await api.stopReplay();
    const s = await api.getReplayStatus();
    setState(s);
  };

  const currentFrame = state?.current_frame;
  const isPlaying = state?.status === 'playing';
  const isPaused = state?.status === 'paused';
  const totalFrames = state?.total_frames ?? 0;
  const currentIdx = state?.current_frame_index ?? 0;

  return (
    <div className="quant-card p-4 mb-4">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Film className="h-4 w-4 text-cyan-400" />
          <h2 className="text-xs font-bold text-slate-400 uppercase tracking-wider">
            Historical Replay & Visual Diagnostics Console
          </h2>
        </div>
        <div className="flex items-center gap-2 font-mono text-xs">
          <span className="text-slate-500">Status:</span>
          <span
            className={`font-bold px-2 py-0.5 rounded uppercase ${
              isPlaying
                ? 'bg-emerald-950 text-emerald-400 border border-emerald-800'
                : isPaused
                ? 'bg-amber-950 text-amber-400 border border-amber-800'
                : 'bg-slate-900 text-slate-400'
            }`}
          >
            {state?.status ?? 'idle'}
          </span>
        </div>
      </div>

      {/* Replay Controls Toolbar */}
      <div className="bg-slate-900/60 p-4 rounded-xl border border-slate-800 mb-4">
        <div className="flex flex-wrap items-center justify-between gap-4">
          {/* Action Buttons */}
          <div className="flex items-center gap-2">
            {!isPlaying ? (
              <button
                onClick={isPaused ? handleResume : handleStart}
                disabled={loading}
                className="flex items-center gap-1.5 px-4 py-2 rounded-lg bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-bold text-xs shadow-lg shadow-cyan-500/20 transition-all"
              >
                <Play className="h-3.5 w-3.5 fill-slate-950" />
                {isPaused ? 'Resume Playback' : 'Start Replay'}
              </button>
            ) : (
              <button
                onClick={handlePause}
                className="flex items-center gap-1.5 px-4 py-2 rounded-lg bg-amber-500 hover:bg-amber-400 text-slate-950 font-bold text-xs transition-all"
              >
                <Pause className="h-3.5 w-3.5 fill-slate-950" />
                Pause
              </button>
            )}

            <button
              onClick={handleStep}
              disabled={isPlaying}
              className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-white font-semibold text-xs border border-slate-700 disabled:opacity-40"
              title="Step forward by exactly 1 frame"
            >
              <SkipForward className="h-3.5 w-3.5" />
              Step (单步逐帧)
            </button>

            <button
              onClick={handleStop}
              className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-rose-400 font-semibold text-xs border border-slate-700"
            >
              <RotateCcw className="h-3.5 w-3.5" />
              Reset
            </button>
          </div>

          {/* Speed Selector */}
          <div className="flex items-center gap-2 text-xs">
            <span className="text-slate-400 font-semibold flex items-center gap-1">
              <FastForward className="h-3 w-3 text-cyan-400" /> Speed:
            </span>
            {[1, 2, 5, 10, 20, 50].map((s) => (
              <button
                key={s}
                onClick={() => handleSpeedChange(s)}
                className={`px-2 py-1 rounded font-mono font-bold text-xs ${
                  speed === s ? 'bg-cyan-500 text-slate-950' : 'bg-slate-800 text-slate-300 hover:bg-slate-700'
                }`}
              >
                {s}x
              </button>
            ))}
          </div>
        </div>

        {/* Seek Bar */}
        {totalFrames > 0 && (
          <div className="mt-4 pt-3 border-t border-slate-800">
            <div className="flex justify-between text-xs text-slate-400 font-mono mb-1.5">
              <span>Frame: {currentIdx} / {totalFrames}</span>
              <span>
                Timestamp: {currentFrame ? new Date(currentFrame.timestamp_ms).toLocaleTimeString() : '--:--:--'}
              </span>
            </div>
            <input
              type="range"
              min={0}
              max={totalFrames - 1}
              value={currentIdx}
              onChange={(e) => handleSeek(parseInt(e.target.value))}
              className="w-full h-2 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-cyan-400"
            />
          </div>
        )}
      </div>

      {error && (
        <div className="p-3 mb-4 rounded bg-rose-950/40 border border-rose-800/80 text-rose-400 text-xs font-mono">
          {error}
        </div>
      )}

      {/* Frame Diagnostic Inspector */}
      {currentFrame ? (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {/* 1. Historical Prices */}
          <div className="bg-slate-900/80 p-3.5 rounded-lg border border-slate-800">
            <h3 className="text-xs font-bold text-slate-400 uppercase tracking-wide mb-2">
              Frame Market State
            </h3>
            <div className="space-y-2 text-xs">
              <div className="flex justify-between">
                <span className="text-slate-500">Asset:</span>
                <span className="font-bold text-white">{currentFrame.asset}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Composite Price:</span>
                <span className="font-mono-num font-bold text-cyan-400">
                  ${currentFrame.composite_price.toFixed(2)}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Poly Up Bid / Ask:</span>
                <span className="font-mono-num font-semibold text-emerald-400">
                  ${currentFrame.poly_up_bid.toFixed(3)} / ${currentFrame.poly_up_ask.toFixed(3)}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Poly Down Bid / Ask:</span>
                <span className="font-mono-num font-semibold text-rose-400">
                  ${currentFrame.poly_down_bid.toFixed(3)} / ${currentFrame.poly_down_ask.toFixed(3)}
                </span>
              </div>
            </div>
          </div>

          {/* 2. Model Prediction */}
          <div className="bg-slate-900/80 p-3.5 rounded-lg border border-slate-800">
            <h3 className="text-xs font-bold text-slate-400 uppercase tracking-wide mb-2">
              Prediction Snapshot
            </h3>
            {currentFrame.prediction ? (
              <div className="space-y-2 text-xs">
                <div className="flex justify-between">
                  <span className="text-slate-500">P(Up) Calibrated:</span>
                  <span className="font-mono-num font-bold text-emerald-400">
                    {(currentFrame.prediction.calibrated_p_up * 100).toFixed(1)}%
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-slate-500">P(Down) Calibrated:</span>
                  <span className="font-mono-num font-bold text-rose-400">
                    {(currentFrame.prediction.calibrated_p_down * 100).toFixed(1)}%
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-slate-500">Model Confidence:</span>
                  <span className="font-mono font-bold text-white uppercase">
                    {currentFrame.prediction.confidence}
                  </span>
                </div>
              </div>
            ) : (
              <span className="text-slate-500 text-xs">No prediction recorded</span>
            )}
          </div>

          {/* 3. Strategy Decision */}
          <div className="bg-slate-900/80 p-3.5 rounded-lg border border-slate-800">
            <h3 className="text-xs font-bold text-slate-400 uppercase tracking-wide mb-2">
              Strategy Signal
            </h3>
            {currentFrame.signal ? (
              <div className="space-y-2 text-xs">
                <div className="flex justify-between items-center">
                  <span className="text-slate-500">Action:</span>
                  <span
                    className={`font-black font-mono px-2 py-0.5 rounded text-[11px] ${
                      currentFrame.signal.action === 'BUY_UP'
                        ? 'bg-emerald-500 text-slate-950'
                        : currentFrame.signal.action === 'BUY_DOWN'
                        ? 'bg-rose-500 text-white'
                        : 'bg-slate-800 text-slate-400'
                    }`}
                  >
                    {currentFrame.signal.action}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-slate-500">Signal Score:</span>
                  <span className="font-mono-num font-bold text-white">
                    {currentFrame.signal.signal_score.toFixed(1)} / 100
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-slate-500">Net Edge:</span>
                  <span className="font-mono-num font-bold text-cyan-400">
                    {(currentFrame.signal.net_edge * 100).toFixed(2)}%
                  </span>
                </div>
              </div>
            ) : (
              <span className="text-slate-500 text-xs">No signal recorded</span>
            )}
          </div>
        </div>
      ) : (
        <div className="text-center py-8 text-xs text-slate-500 font-mono">
          Click "Start Replay" or "Step" to load and inspect historical frames.
        </div>
      )}
    </div>
  );
};
