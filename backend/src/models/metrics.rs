use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEvaluationMetrics {
    pub sample_count: usize,
    pub brier_score: f64,
    pub log_loss: f64,
    pub accuracy: f64,
    pub expected_calibration_error: f64,
}

pub struct MetricsCalculator;

impl MetricsCalculator {
    /// Calculate Brier Score, Log Loss, Accuracy, and ECE
    /// `predictions` are P(Up) in [0.0, 1.0]
    /// `labels` are 1.0 (Up) or 0.0 (Down)
    pub fn evaluate(predictions: &[f64], labels: &[f64]) -> Option<ModelEvaluationMetrics> {
        if predictions.is_empty() || predictions.len() != labels.len() {
            return None;
        }

        let n = predictions.len() as f64;
        let mut brier_sum = 0.0;
        let mut log_loss_sum = 0.0;
        let mut correct_count = 0;

        for (&p, &y) in predictions.iter().zip(labels.iter()) {
            let p_clamped = p.clamp(1e-7, 1.0 - 1e-7);

            // Brier score: (p - y)^2
            brier_sum += (p - y).powi(2);

            // Log loss: -[y * ln(p) + (1-y) * ln(1-p)]
            log_loss_sum += -(y * p_clamped.ln() + (1.0 - y) * (1.0 - p_clamped).ln());

            // Accuracy
            let predicted_up = p >= 0.50;
            let actual_up = y >= 0.50;
            if predicted_up == actual_up {
                correct_count += 1;
            }
        }

        let brier_score = brier_sum / n;
        let log_loss = log_loss_sum / n;
        let accuracy = correct_count as f64 / n;

        // Expected Calibration Error (ECE) with 10 bins
        let ece = Self::calc_ece(predictions, labels, 10);

        Some(ModelEvaluationMetrics {
            sample_count: predictions.len(),
            brier_score,
            log_loss,
            accuracy,
            expected_calibration_error: ece,
        })
    }

    fn calc_ece(predictions: &[f64], labels: &[f64], num_bins: usize) -> f64 {
        let n = predictions.len() as f64;
        let bin_width = 1.0 / num_bins as f64;
        let mut total_ece = 0.0;

        for bin_idx in 0..num_bins {
            let bin_lower = bin_idx as f64 * bin_width;
            let bin_upper = bin_lower + bin_width;

            let mut bin_p_sum = 0.0;
            let mut bin_y_sum = 0.0;
            let mut bin_count = 0;

            for (&p, &y) in predictions.iter().zip(labels.iter()) {
                if (p >= bin_lower && p < bin_upper) || (bin_idx == num_bins - 1 && p >= bin_lower && p <= 1.0) {
                    bin_p_sum += p;
                    bin_y_sum += y;
                    bin_count += 1;
                }
            }

            if bin_count > 0 {
                let bin_conf = bin_p_sum / bin_count as f64;
                let bin_acc = bin_y_sum / bin_count as f64;
                let bin_weight = bin_count as f64 / n;
                total_ece += bin_weight * (bin_acc - bin_conf).abs();
            }
        }

        total_ece
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_model_metrics() {
        let preds = vec![1.0, 0.0, 1.0, 0.0];
        let labels = vec![1.0, 0.0, 1.0, 0.0];

        let m = MetricsCalculator::evaluate(&preds, &labels).unwrap();
        assert_eq!(m.sample_count, 4);
        assert!((m.brier_score - 0.0).abs() < 1e-6);
        assert!((m.accuracy - 1.0).abs() < 1e-6);
        assert!(m.log_loss < 1e-3);
    }

    #[test]
    fn test_uninformative_baseline_metrics() {
        let preds = vec![0.5, 0.5, 0.5, 0.5];
        let labels = vec![1.0, 0.0, 1.0, 0.0];

        let m = MetricsCalculator::evaluate(&preds, &labels).unwrap();
        // (0.5 - 1.0)^2 = 0.25
        assert!((m.brier_score - 0.25).abs() < 1e-4);
        // Log loss for 0.5 is -ln(0.5) = 0.6931
        assert!((m.log_loss - 0.6931).abs() < 1e-3);
    }
}
