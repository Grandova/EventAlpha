use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationConfig {
    pub platt_a: f64,
    pub platt_b: f64,
    pub min_clamped_prob: f64,
    pub max_clamped_prob: f64,
    pub default_shrinkage_lambda: f64,
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            platt_a: 1.0,
            platt_b: 0.0,
            min_clamped_prob: 0.08,
            max_clamped_prob: 0.92,
            default_shrinkage_lambda: 0.88,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProbabilityCalibrator {
    config: CalibrationConfig,
}

impl ProbabilityCalibrator {
    pub fn new(config: CalibrationConfig) -> Self {
        Self { config }
    }

    pub fn default_calibrator() -> Self {
        Self::new(CalibrationConfig::default())
    }

    /// Calibrate a raw probability output from a machine learning model
    /// Applies Platt scaling, shrinkage towards prior (0.50), and boundary clamping
    pub fn calibrate(&self, raw_p_up: f64, time_decay_factor: f64, realized_vol: f64) -> (f64, f64) {
        // 1. Sanitize raw probability
        let p_clean = raw_p_up.clamp(1e-6, 1.0 - 1e-6);

        // 2. Convert to logit
        let raw_logit = (p_clean / (1.0 - p_clean)).ln();

        // 3. Platt Scaling: z' = A * z + B
        let calibrated_logit = self.config.platt_a * raw_logit + self.config.platt_b;
        let platt_p = 1.0 / (1.0 + (-calibrated_logit).exp());

        // 4. Overconfidence Shrinkage: Shrink towards 0.50 based on uncertainty
        // High volatility or early in the 5M round increases uncertainty -> shrink more towards 0.50
        let vol_penalty = (realized_vol * 100.0).min(0.15);
        let time_uncertainty = (1.0 - time_decay_factor) * 0.10;
        let effective_lambda = (self.config.default_shrinkage_lambda - vol_penalty - time_uncertainty).clamp(0.60, 0.95);

        let shrunk_p = effective_lambda * platt_p + (1.0 - effective_lambda) * 0.50;

        // 5. Clamping to eliminate extreme tail overconfidence
        let p_up = shrunk_p.clamp(self.config.min_clamped_prob, self.config.max_clamped_prob);
        let p_down = 1.0 - p_up;

        (p_up, p_down)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibrator_compresses_extreme_probabilities() {
        let cal = ProbabilityCalibrator::default_calibrator();

        // Raw 0.99 should be compressed to < 0.92
        let (p_up, p_down) = cal.calibrate(0.99, 0.8, 0.001);
        assert!(p_up < 0.92);
        assert!(p_up > 0.70);
        assert!((p_up + p_down - 1.0).abs() < 1e-6);

        // Raw 0.01 should be compressed to > 0.08
        let (p_low, _) = cal.calibrate(0.01, 0.8, 0.001);
        assert!(p_low > 0.08);
        assert!(p_low < 0.30);
    }

    #[test]
    fn test_high_volatility_causes_greater_shrinkage() {
        let cal = ProbabilityCalibrator::default_calibrator();

        let (p_low_vol, _) = cal.calibrate(0.80, 0.5, 0.0001);
        let (p_high_vol, _) = cal.calibrate(0.80, 0.5, 0.01);

        // High vol should be closer to 0.50 than low vol
        assert!(p_high_vol < p_low_vol);
        assert!(p_high_vol > 0.50);
    }
}
