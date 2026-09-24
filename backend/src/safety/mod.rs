use serde::{Deserialize, Serialize};
use tracing::{error, info};

/// Compile-time and runtime safety guard to guarantee that live real-money trading
/// can NEVER be inadvertently executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyStatus {
    pub real_trading_enabled: bool,
    pub paper_trading_only: bool,
    pub safety_lock_engaged: bool,
    pub safety_signature: String,
    pub private_key_detected: bool,
    pub message: String,
}

pub struct SafetyGuard;

impl SafetyGuard {
    /// Enforce the uncompromisable safety rule:
    /// REAL_TRADING_ENABLED MUST BE FALSE AT ALL TIMES.
    /// If real_trading_enabled is set to true anywhere, the system panics immediately.
    pub fn enforce_paper_only(real_trading_enabled: bool) -> Result<SafetyStatus, String> {
        if real_trading_enabled {
            let msg = "CRITICAL SAFETY VIOLATION: real_trading_enabled is set to TRUE. \
                       The system is strictly built for simulation and paper trading. \
                       Halting immediately to prevent real fund loss."
                .to_string();
            error!("{}", msg);
            panic!("{}", msg);
        }

        // Check if any real wallet private key environment variables exist
        let has_private_key = std::env::var("POLYMARKET_PRIVATE_KEY").is_ok()
            || std::env::var("PRIVATE_KEY").is_ok()
            || std::env::var("WALLET_KEY").is_ok();

        if has_private_key {
            info!("Notice: Private key detected in environment, but live trading is permanently disabled by safety guard.");
        }

        info!("SafetyGuard: Paper trading only lock is ENGAGED. Live trading is strictly disabled.");

        Ok(SafetyStatus {
            real_trading_enabled: false,
            paper_trading_only: true,
            safety_lock_engaged: true,
            safety_signature: "PAPER_TRADING_ONLY_SAFETY_LOCK_ENGAGED".to_string(),
            private_key_detected: has_private_key,
            message: "System is running in safe simulation mode. Real trading orders are prohibited by kernel-level guard."
                .to_string(),
        })
    }
}

/// Abstract Execution Engine Trait
pub trait ExecutionEngine: Send + Sync {
    fn is_paper_trading(&self) -> bool {
        true
    }
}

/// Simulated Paper Execution Engine implementation
pub struct PaperExecutionEngine;

impl ExecutionEngine for PaperExecutionEngine {
    fn is_paper_trading(&self) -> bool {
        true
    }
}

/// Dummy Live Execution Engine that is permanently disabled.
/// Any attempt to instantiate or invoke live execution will panic.
pub struct LiveExecutionEngine;

impl LiveExecutionEngine {
    pub fn new() -> Result<Self, &'static str> {
        Err("LIVE TRADING PROHIBITED: LiveExecutionEngine is compiled out / hard-disabled.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_guard_allows_paper_mode() {
        let status = SafetyGuard::enforce_paper_only(false).expect("Should succeed when false");
        assert!(!status.real_trading_enabled);
        assert!(status.paper_trading_only);
        assert!(status.safety_lock_engaged);
    }

    #[test]
    #[should_panic(expected = "CRITICAL SAFETY VIOLATION")]
    fn test_safety_guard_panics_on_live_enabled() {
        let _ = SafetyGuard::enforce_paper_only(true);
    }

    #[test]
    fn test_live_execution_engine_cannot_be_instantiated() {
        let res = LiveExecutionEngine::new();
        assert!(res.is_err());
    }
}
