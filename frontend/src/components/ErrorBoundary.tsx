import React, { Component, ErrorInfo, ReactNode } from 'react';
import { AlertTriangle, RefreshCw } from 'lucide-react';

interface Props {
  children: ReactNode;
  fallbackTitle?: string;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('[ErrorBoundary] Uncaught rendering exception:', error, errorInfo);
  }

  private handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  public render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-[220px] p-6 rounded-xl bg-slate-900/90 border border-rose-500/40 text-slate-200 flex flex-col items-center justify-center space-y-4 shadow-xl">
          <div className="flex items-center space-x-3 text-rose-400">
            <AlertTriangle className="w-8 h-8 animate-pulse" />
            <h3 className="text-lg font-bold">
              {this.props.fallbackTitle || 'Component Rendering Exception'}
            </h3>
          </div>
          <p className="text-xs text-slate-400 max-w-md text-center font-mono bg-slate-950/60 p-3 rounded border border-slate-800">
            {this.state.error?.message || 'An unknown render error occurred.'}
          </p>
          <div className="flex items-center space-x-3">
            <button
              onClick={this.handleReset}
              className="inline-flex items-center px-4 py-2 text-xs font-semibold text-white bg-rose-600 hover:bg-rose-500 rounded-lg transition-colors cursor-pointer shadow-md shadow-rose-900/40"
            >
              <RefreshCw className="w-3.5 h-3.5 mr-1.5" />
              Try Recovering
            </button>
            <button
              onClick={() => window.location.reload()}
              className="inline-flex items-center px-4 py-2 text-xs font-semibold text-slate-300 bg-slate-800 hover:bg-slate-700 rounded-lg transition-colors cursor-pointer"
            >
              Full Page Reload
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
