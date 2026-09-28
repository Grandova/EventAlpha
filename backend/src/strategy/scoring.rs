use serde::{Deserialize, Serialize};

use crate::config::ScoreThresholds;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScoreTier {
    Skip,
    Low,
    Medium,
    High,
    VeryHigh,
}

impl std::fmt::Display for ScoreTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScoreTier::Skip => write!(f, "SKIP"),
            ScoreTier::Low => write!(f, "LOW"),
            ScoreTier::Medium => write!(f, "MEDIUM"),
            ScoreTier::High => write!(f, "HIGH"),
            ScoreTier::VeryHigh => write!(f, "VERY_HIGH"),
        }
    }
}

/// Detailed point breakdown for auditability and transparency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub edge_points: f64,     // max 35
    pub prob_points: f64,     // max 25
    pub obi_points: f64,      // max 20
    pub momentum_points: f64, // max 15
    pub timing_points: f64,   // max 5
    pub total_score: f64,     // 0 - 100
    pub tier: ScoreTier,
}

pub struct SignalScorer;

impl SignalScorer {
    /// 1. Net edge points: [0, 35]
    ///
    /// 5% net edge gives 10 pts, scales linearly to 35 pts at 15%+ net edge
    pub fn compute_edge_points(net_edge: f64, min_net_edge: f64) -> f64 {
        if !net_edge.is_finite() || net_edge <= 0.0 {
            0.0
        } else if min_net_edge <= 0.0 {
            10.0
        } else if net_edge < min_net_edge {
            // Partial credit if positive but below hard filter
            (net_edge / min_net_edge).clamp(0.0, 1.0) * 10.0
        } else {
            let high_edge = 0.15;
            let ratio = if high_edge > min_net_edge {
                ((net_edge - min_net_edge) / (high_edge - min_net_edge)).clamp(0.0, 1.0)
            } else {
                1.0
            };
            10.0 + ratio * 25.0
        }
    }

    /// 2. Calibrated probability points: [0, 25]
    ///
    /// 70% gives 10 pts, scales linearly to 25 pts at 90%+
    pub fn compute_prob_points(calibrated_p: f64, min_prob: f64) -> f64 {
        if !calibrated_p.is_finite() || calibrated_p <= 0.50 {
            0.0
        } else if min_prob <= 0.50 {
            10.0
        } else if calibrated_p < min_prob {
            let ratio = ((calibrated_p - 0.50) / (min_prob - 0.50)).clamp(0.0, 1.0);
            ratio * 10.0
        } else {
            let max_target = 0.90;
            let ratio = if max_target > min_prob {
                ((calibrated_p - min_prob) / (max_target - min_prob)).clamp(0.0, 1.0)
            } else {
                1.0
            };
            10.0 + ratio * 15.0
        }
    }

    /// 3. Directed OrderBook Imbalance points: [0, 20]
    ///
    /// directed_obi in [-1.0, 1.0] (positive means depth favors prediction direction)
    pub fn compute_obi_points(directed_obi: f64) -> f64 {
        if !directed_obi.is_finite() {
            return 10.0;
        }
        let clamped = directed_obi.clamp(-1.0, 1.0);
        if clamped >= 0.5 {
            20.0
        } else if clamped >= 0.0 {
            10.0 + (clamped / 0.5) * 10.0
        } else {
            // [0.0, 10.0] for negative OBI
            (1.0 + clamped) * 10.0
        }
    }

    /// 4. Multi-Exchange trade momentum / CVD agreement: [0, 15]
    ///
    /// directed_momentum in [-1.0, 1.0] (positive means trade flow agrees with prediction)
    pub fn compute_momentum_points(directed_momentum: f64) -> f64 {
        if !directed_momentum.is_finite() {
            return 7.5;
        }
        let clamped = directed_momentum.clamp(-1.0, 1.0);
        if clamped >= 0.4 {
            15.0
        } else if clamped >= 0.0 {
            7.5 + (clamped / 0.4) * 7.5
        } else {
            (1.0 + clamped) * 7.5
        }
    }

    /// 5. Timing window sweet spot: [0, 5]
    ///
    /// Sweet spot between 60s and 210s remaining (market established, safe from last-minute chaotic flushes)
    pub fn compute_timing_points(remaining_seconds: i64) -> f64 {
        if (60..=210).contains(&remaining_seconds) {
            5.0
        } else if (30..60).contains(&remaining_seconds) || (211..=270).contains(&remaining_seconds) {
            3.0
        } else if (15..30).contains(&remaining_seconds) || (271..=285).contains(&remaining_seconds) {
            1.0
        } else {
            0.0
        }
    }

    /// Calculate total score in [0.0, 100.0] and return detailed breakdown
    #[allow(clippy::too_many_arguments)]
    pub fn compute_score(
        net_edge: f64,
        calibrated_p: f64,
        directed_obi: f64,
        directed_momentum: f64,
        remaining_seconds: i64,
        min_net_edge: f64,
        min_prob: f64,
        thresholds: &ScoreThresholds,
    ) -> ScoreBreakdown {
        let edge_points = Self::compute_edge_points(net_edge, min_net_edge);
        let prob_points = Self::compute_prob_points(calibrated_p, min_prob);
        let obi_points = Self::compute_obi_points(directed_obi);
        let momentum_points = Self::compute_momentum_points(directed_momentum);
        let timing_points = Self::compute_timing_points(remaining_seconds);

        let total = (edge_points + prob_points + obi_points + momentum_points + timing_points)
            .clamp(0.0, 100.0);

        let tier = if total < thresholds.skip_below {
            ScoreTier::Skip
        } else if total < thresholds.medium {
            ScoreTier::Low
        } else if total < thresholds.high {
            ScoreTier::Medium
        } else if total < thresholds.very_high {
            ScoreTier::High
        } else {
            ScoreTier::VeryHigh
        };

        ScoreBreakdown {
            edge_points,
            prob_points,
            obi_points,
            momentum_points,
            timing_points,
            total_score: total,
            tier,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_thresholds() -> ScoreThresholds {
        ScoreThresholds {
            skip_below: 60.0,
            low: 60.0,
            medium: 70.0,
            high: 80.0,
            very_high: 90.0,
        }
    }

    #[test]
    fn test_edge_points() {
        assert_eq!(SignalScorer::compute_edge_points(0.0, 0.05), 0.0);
        assert_eq!(SignalScorer::compute_edge_points(0.05, 0.05), 10.0);
        assert!((SignalScorer::compute_edge_points(0.15, 0.05) - 35.0).abs() < 1e-4);
        assert_eq!(SignalScorer::compute_edge_points(0.25, 0.05), 35.0);
    }

    #[test]
    fn test_prob_points() {
        assert_eq!(SignalScorer::compute_prob_points(0.50, 0.70), 0.0);
        assert_eq!(SignalScorer::compute_prob_points(0.70, 0.70), 10.0);
        assert!((SignalScorer::compute_prob_points(0.90, 0.70) - 25.0).abs() < 1e-4);
        assert_eq!(SignalScorer::compute_prob_points(0.95, 0.70), 25.0);
    }

    #[test]
    fn test_obi_and_momentum_points() {
        assert_eq!(SignalScorer::compute_obi_points(0.5), 20.0);
        assert_eq!(SignalScorer::compute_obi_points(0.0), 10.0);
        assert_eq!(SignalScorer::compute_obi_points(-1.0), 0.0);

        assert_eq!(SignalScorer::compute_momentum_points(0.4), 15.0);
        assert_eq!(SignalScorer::compute_momentum_points(0.0), 7.5);
        assert_eq!(SignalScorer::compute_momentum_points(-1.0), 0.0);
    }

    #[test]
    fn test_timing_points() {
        assert_eq!(SignalScorer::compute_timing_points(120), 5.0);
        assert_eq!(SignalScorer::compute_timing_points(45), 3.0);
        assert_eq!(SignalScorer::compute_timing_points(20), 1.0);
        assert_eq!(SignalScorer::compute_timing_points(5), 0.0);
    }

    #[test]
    fn test_full_score_tiers() {
        let thresholds = test_thresholds();

        // High quality opportunity
        let high = SignalScorer::compute_score(
            0.12, 0.85, 0.40, 0.35, 150, 0.05, 0.70, &thresholds,
        );
        assert!(high.total_score >= 80.0);
        assert!(matches!(high.tier, ScoreTier::High | ScoreTier::VeryHigh));

        // Sub-par opportunity (barely positive)
        let sub = SignalScorer::compute_score(
            0.02, 0.55, -0.30, -0.20, 10, 0.05, 0.70, &thresholds,
        );
        assert!(sub.total_score < 60.0);
        assert_eq!(sub.tier, ScoreTier::Skip);
    }
}
