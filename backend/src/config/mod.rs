use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

use crate::safety::SafetyGuard;
use crate::types::BankrollMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub mode: String,
    pub safety: SafetyConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub bankroll: BankrollConfig,
    pub position: PositionConfig,
    pub strategy: StrategyConfig,
    pub risk: RiskConfig,
    pub execution: ExecutionConfig,
    pub freshness: FreshnessConfig,
    pub assets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub real_trading_enabled: bool,
    pub safety_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankrollConfig {
    pub initial: f64,
    pub cap: f64,
    pub minimum: f64,
    pub mode: BankrollMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionConfig {
    pub mode: String, // "fixed" or "percentage"
    pub stake: f64,
    pub stake_percent: f64,
    pub max_stake: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreThresholds {
    pub skip_below: f64,
    pub low: f64,
    pub medium: f64,
    pub high: f64,
    pub very_high: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub strategy_version: String,
    pub min_probability: f64,
    pub min_net_edge: f64,
    pub max_entry_price: f64,
    pub max_spread: f64,
    pub min_liquidity: f64,
    pub min_time_remaining_sec: u32,
    pub max_time_remaining_sec: u32,
    pub score_thresholds: ScoreThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub daily_loss_limit: f64,
    pub max_drawdown: f64,
    pub max_consecutive_losses: u32,
    pub cooldown_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub latency_ms: u64,
    pub orderbook_depth_fill: bool,
    pub fee_rate: f64,
    pub slippage_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessConfig {
    pub stale_timeout_ms: u64,
    pub max_clock_skew_ms: u64,
}

impl AppConfig {
    /// Load configuration from a YAML file, apply ENV overrides, and enforce safety guards
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file at {:?}", path.as_ref()))?;

        let mut config: AppConfig = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse YAML configuration")?;

        // Apply environment variable overrides if present
        if let Ok(env_real_trading) = std::env::var("REAL_TRADING_ENABLED") {
            if let Ok(parsed) = env_real_trading.parse::<bool>() {
                config.safety.real_trading_enabled = parsed;
            }
        }

        if let Ok(host) = std::env::var("HOST") {
            config.server.host = host;
        }

        if let Ok(port) = std::env::var("PORT") {
            if let Ok(p) = port.parse::<u16>() {
                config.server.port = p;
            }
        }

        if let Ok(db_url) = std::env::var("DATABASE_URL") {
            config.database.url = db_url;
        }

        // Validate config integrity & enforce safety rules
        config.validate()?;

        info!("Configuration loaded and validated successfully from {:?}", path.as_ref());
        Ok(config)
    }

    /// Load default config from standard search paths
    pub fn load_default() -> Result<Self> {
        // Priority 1: APP_CONFIG_PATH
        if let Ok(custom_path) = std::env::var("APP_CONFIG_PATH") {
            if Path::new(&custom_path).exists() {
                return Self::load_from_path(&custom_path);
            }
        }

        // Priority 2: config/config.yaml
        let candidates = [
            "config/config.yaml",
            "../config/config.yaml",
            "../../config/config.yaml",
        ];

        for candidate in candidates {
            if Path::new(candidate).exists() {
                return Self::load_from_path(candidate);
            }
        }

        bail!("Could not find config.yaml in search paths. Please specify APP_CONFIG_PATH.")
    }

    /// Validation logic
    pub fn validate(&self) -> Result<()> {
        // 1. Mandatory Safety Guard Assertion
        SafetyGuard::enforce_paper_only(self.safety.real_trading_enabled)
            .map_err(|e| anyhow::anyhow!(e))?;

        // 2. Bankroll checks
        if self.bankroll.initial <= 0.0 {
            bail!("bankroll.initial must be greater than 0");
        }
        if self.bankroll.cap < self.bankroll.initial {
            bail!("bankroll.cap must be >= bankroll.initial");
        }
        if self.bankroll.minimum <= 0.0 || self.bankroll.minimum > self.bankroll.initial {
            bail!("bankroll.minimum must be positive and <= bankroll.initial");
        }

        // 3. Position checks
        if self.position.stake <= 0.0 {
            bail!("position.stake must be greater than 0");
        }
        if self.position.max_stake < self.position.stake {
            bail!("position.max_stake must be >= position.stake");
        }

        // 4. Strategy checks
        if self.strategy.min_probability <= 0.5 || self.strategy.min_probability >= 1.0 {
            bail!("strategy.min_probability must be in range (0.5, 1.0)");
        }
        if self.strategy.min_net_edge < 0.0 {
            bail!("strategy.min_net_edge must be non-negative");
        }
        if self.strategy.max_entry_price <= 0.0 || self.strategy.max_entry_price >= 1.0 {
            bail!("strategy.max_entry_price must be in range (0.0, 1.0)");
        }

        // 5. Freshness checks
        if self.freshness.stale_timeout_ms == 0 {
            bail!("freshness.stale_timeout_ms must be positive");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn sample_yaml(real_trading: bool) -> String {
        format!(
            r#"
mode: "paper"
safety:
  real_trading_enabled: {}
  safety_signature: "PAPER_TRADING_ONLY_SAFETY_LOCK_ENGAGED"
server:
  host: "127.0.0.1"
  port: 8080
  cors_allowed_origins: ["http://localhost:5173"]
database:
  url: "sqlite://:memory:"
  max_connections: 5
  min_connections: 1
  acquire_timeout_secs: 5
bankroll:
  initial: 10.0
  cap: 10.0
  minimum: 2.0
  mode: "capital_recovery"
position:
  mode: "fixed"
  stake: 1.0
  stake_percent: 0.10
  max_stake: 1.0
strategy:
  strategy_version: "v1.0.0"
  min_probability: 0.70
  min_net_edge: 0.05
  max_entry_price: 0.85
  max_spread: 0.04
  min_liquidity: 300.0
  min_time_remaining_sec: 15
  max_time_remaining_sec: 285
  score_thresholds:
    skip_below: 60.0
    low: 60.0
    medium: 70.0
    high: 80.0
    very_high: 90.0
risk:
  daily_loss_limit: 2.0
  max_drawdown: 0.20
  max_consecutive_losses: 5
  cooldown_minutes: 30
execution:
  latency_ms: 250
  orderbook_depth_fill: true
  fee_rate: 0.012
  slippage_rate: 0.005
freshness:
  stale_timeout_ms: 2000
  max_clock_skew_ms: 1000
assets: ["BTC", "ETH", "SOL"]
"#,
            real_trading
        )
    }

    #[test]
    fn test_valid_config_loads_successfully() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(sample_yaml(false).as_bytes()).unwrap();

        let config = AppConfig::load_from_path(file.path()).expect("Valid config should load");
        assert_eq!(config.mode, "paper");
        assert!(!config.safety.real_trading_enabled);
        assert_eq!(config.bankroll.initial, 10.0);
        assert_eq!(config.assets.len(), 3);
    }

    #[test]
    #[should_panic(expected = "CRITICAL SAFETY VIOLATION")]
    fn test_config_panics_if_real_trading_enabled() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(sample_yaml(true).as_bytes()).unwrap();

        let _ = AppConfig::load_from_path(file.path());
    }
}
