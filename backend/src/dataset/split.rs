use serde::{Deserialize, Serialize};

use crate::types::{Asset, Resolution};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRecord {
    pub market_id: String,
    pub asset: Asset,
    pub timestamp_ms: i64,
    pub features: Vec<f64>,
    pub target_up: f64, // 1.0 for UP, 0.0 for DOWN
    pub resolution: Resolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSummary {
    pub total_samples: usize,
    pub train_samples: usize,
    pub val_samples: usize,
    pub test_samples: usize,
    pub up_count: usize,
    pub down_count: usize,
    pub void_count: usize,
    pub up_ratio: f64,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub feature_dimension: usize,
    pub invalid_value_count: usize,
}

#[derive(Debug, Clone)]
pub struct WalkForwardSplit {
    pub train: Vec<DatasetRecord>,
    pub val: Vec<DatasetRecord>,
    pub test: Vec<DatasetRecord>,
    pub summary: DatasetSummary,
}

pub struct WalkForwardSplitter {
    pub train_ratio: f64,
    pub val_ratio: f64,
    pub test_ratio: f64,
}

impl Default for WalkForwardSplitter {
    fn default() -> Self {
        Self {
            train_ratio: 0.70,
            val_ratio: 0.15,
            test_ratio: 0.15,
        }
    }
}

impl WalkForwardSplitter {
    pub fn new(train_ratio: f64, val_ratio: f64, test_ratio: f64) -> Self {
        assert!(
            (train_ratio + val_ratio + test_ratio - 1.0).abs() < 1e-6,
            "Ratios must sum to 1.0"
        );
        Self {
            train_ratio,
            val_ratio,
            test_ratio,
        }
    }

    /// Split time-series dataset into chronological non-overlapping Train, Val, Test partitions
    /// Guaranteed: max(train_ts) < min(val_ts) and max(val_ts) < min(test_ts)
    /// Strictly NO random shuffling or future lookahead leakage!
    pub fn split(&self, mut records: Vec<DatasetRecord>) -> Result<WalkForwardSplit, String> {
        if records.is_empty() {
            return Err("Cannot split an empty dataset".to_string());
        }

        // 1. Sort strictly by timestamp ascending
        records.sort_by_key(|r| r.timestamp_ms);

        let total = records.len();
        let train_end = ((total as f64) * self.train_ratio).round() as usize;
        let val_end = ((total as f64) * (self.train_ratio + self.val_ratio)).round() as usize;

        let train = records[0..train_end].to_vec();
        let val = records[train_end..val_end].to_vec();
        let test = records[val_end..].to_vec();

        // 2. Strict non-overlapping time boundary check
        if !train.is_empty() && !val.is_empty() {
            let max_train = train.last().unwrap().timestamp_ms;
            let min_val = val.first().unwrap().timestamp_ms;
            if max_train > min_val {
                return Err(format!(
                    "Train and Val partitions overlap! max_train={} > min_val={}",
                    max_train, min_val
                ));
            }
        }

        if !val.is_empty() && !test.is_empty() {
            let max_val = val.last().unwrap().timestamp_ms;
            let min_test = test.first().unwrap().timestamp_ms;
            if max_val > min_test {
                return Err(format!(
                    "Val and Test partitions overlap! max_val={} > min_test={}",
                    max_val, min_test
                ));
            }
        }

        // 3. Compute statistics and verify data validity
        let mut up_count = 0;
        let mut down_count = 0;
        let mut void_count = 0;
        let mut invalid_count = 0;

        let feature_dim = records.first().map(|r| r.features.len()).unwrap_or(0);

        for r in &records {
            match r.resolution {
                Resolution::Up => up_count += 1,
                Resolution::Down => down_count += 1,
                Resolution::Void => void_count += 1,
            }

            for &val in &r.features {
                if val.is_nan() || val.is_infinite() {
                    invalid_count += 1;
                }
            }
        }

        let non_void = up_count + down_count;
        let up_ratio = if non_void > 0 {
            up_count as f64 / non_void as f64
        } else {
            0.50
        };

        let summary = DatasetSummary {
            total_samples: total,
            train_samples: train.len(),
            val_samples: val.len(),
            test_samples: test.len(),
            up_count,
            down_count,
            void_count,
            up_ratio,
            start_time_ms: records.first().unwrap().timestamp_ms,
            end_time_ms: records.last().unwrap().timestamp_ms,
            feature_dimension: feature_dim,
            invalid_value_count: invalid_count,
        };

        Ok(WalkForwardSplit {
            train,
            val,
            test,
            summary,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_walk_forward_strict_no_leakage() {
        let mut records = Vec::new();
        for i in 0..100 {
            records.push(DatasetRecord {
                market_id: format!("mkt-{}", i),
                asset: Asset::BTC,
                timestamp_ms: 1000 + (i * 10),
                features: vec![i as f64, 0.5],
                target_up: if i % 2 == 0 { 1.0 } else { 0.0 },
                resolution: if i % 2 == 0 { Resolution::Up } else { Resolution::Down },
            });
        }

        let splitter = WalkForwardSplitter::default();
        let split = splitter.split(records).unwrap();

        assert_eq!(split.train.len(), 70);
        assert_eq!(split.val.len(), 15);
        assert_eq!(split.test.len(), 15);

        let max_train_ts = split.train.last().unwrap().timestamp_ms;
        let min_val_ts = split.val.first().unwrap().timestamp_ms;
        assert!(max_train_ts <= min_val_ts, "Train must strictly precede Val");

        let max_val_ts = split.val.last().unwrap().timestamp_ms;
        let min_test_ts = split.test.first().unwrap().timestamp_ms;
        assert!(max_val_ts <= min_test_ts, "Val must strictly precede Test");

        assert_eq!(split.summary.total_samples, 100);
        assert_eq!(split.summary.invalid_value_count, 0);
        assert_eq!(split.summary.feature_dimension, 2);
    }
}
