import React, { useState, useEffect } from 'react';
import { Terminal, Shield, RefreshCw } from 'lucide-react';
import { SystemEvent } from '../types';
import { api } from '../services/api';

export const EventLogViewer: React.FC = () => {
  const [events, setEvents] = useState<SystemEvent[]>([]);
  const [filterSeverity, setFilterSeverity] = useState<string>('ALL');
  const [loading, setLoading] = useState(false);

  const fetchEvents = async () => {
    setLoading(true);
    try {
      const data = await api.getEvents(100);
      setEvents(data);
    } catch (e) {
      // ignore
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchEvents();
    const timer = setInterval(fetchEvents, 3000);
    return () => clearInterval(timer);
  }, []);

  const filteredEvents = events.filter((e) => {
    if (filterSeverity === 'ALL') return true;
    return e.severity.toUpperCase() === filterSeverity.toUpperCase();
  });

  return (
    <div className="quant-card p-4 mb-4">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Terminal className="h-4 w-4 text-cyan-400" />
          <h2 className="text-xs font-bold text-slate-400 uppercase tracking-wider">
            System Security Audit Logs & Real-Time Event Stream
          </h2>
        </div>

        <div className="flex items-center gap-2">
          {/* Severity Filter */}
          <select
            value={filterSeverity}
            onChange={(e) => setFilterSeverity(e.target.value)}
            className="bg-slate-900 border border-slate-700 rounded text-xs px-2 py-1 text-slate-300 font-mono"
          >
            <option value="ALL">All Severities</option>
            <option value="INFO">INFO</option>
            <option value="WARN">WARN</option>
            <option value="ERROR">ERROR</option>
            <option value="CRITICAL">CRITICAL</option>
          </select>

          <button
            onClick={fetchEvents}
            disabled={loading}
            className="p-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 transition-all"
            title="Refresh events"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${loading ? 'animate-spin' : ''}`} />
          </button>
        </div>
      </div>

      <div className="bg-slate-950/80 p-3 rounded-lg border border-slate-800 font-mono text-xs max-h-96 overflow-y-auto space-y-1.5">
        {filteredEvents.length === 0 ? (
          <div className="text-center py-6 text-slate-600">No events recorded.</div>
        ) : (
          filteredEvents.map((e, i) => {
            const isErr = e.severity === 'ERROR' || e.severity === 'CRITICAL';
            const isWarn = e.severity === 'WARN';
            return (
              <div
                key={i}
                className="flex items-start gap-2.5 py-1 px-2 rounded hover:bg-slate-900/60 border-b border-slate-900"
              >
                <span className="text-[10px] text-slate-500 shrink-0">
                  {new Date(e.timestamp_ms).toLocaleTimeString()}
                </span>
                <span
                  className={`text-[10px] font-bold px-1.5 py-0.2 rounded shrink-0 ${
                    isErr
                      ? 'bg-rose-950 text-rose-400'
                      : isWarn
                      ? 'bg-amber-950 text-amber-400'
                      : 'bg-slate-800 text-slate-400'
                  }`}
                >
                  {e.severity}
                </span>
                <span className="text-cyan-400 font-semibold shrink-0">[{e.component}]</span>
                <span className="text-slate-300 flex-1 break-all">{e.message}</span>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
