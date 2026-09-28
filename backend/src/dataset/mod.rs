pub mod split;

use anyhow::{Context, Result};
use sqlx::Row;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::db::Database;
use crate::features::{FeatureSnapshot, FEATURE_NAMES};
use crate::types::{Asset, Resolution};
pub use split::{DatasetRecord, DatasetSummary, WalkForwardSplit, WalkForwardSplitter};

pub struct DatasetExporter;

impl DatasetExporter {
    /// Load resolved market feature vectors directly from SQLite
    pub async fn load_from_db(db: &Database) -> Result<Vec<DatasetRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                f.market_id,
                f.timestamp,
                f.feature_vector_json,
                m.asset,
                m.resolution
            FROM features f
            JOIN markets m ON f.market_id = m.id
            WHERE m.resolution IS NOT NULL
            ORDER BY f.timestamp ASC
            "#,
        )
        .fetch_all(db.pool())
        .await
        .context("Failed to query features joined with resolved markets")?;

        let mut records = Vec::with_capacity(rows.len());

        for row in rows {
            let market_id: String = row.get("market_id");
            let timestamp_ms: i64 = row.get("timestamp");
            let json_str: String = row.get("feature_vector_json");
            let asset_str: String = row.get("asset");
            let resolution_str: String = row.get("resolution");

            let asset = asset_str.parse::<Asset>().unwrap_or(Asset::BTC);
            let resolution = match resolution_str.to_uppercase().as_str() {
                "UP" => Resolution::Up,
                "DOWN" => Resolution::Down,
                _ => Resolution::Void,
            };

            let target_up = match resolution {
                Resolution::Up => 1.0,
                Resolution::Down => 0.0,
                Resolution::Void => 0.5,
            };

            // Attempt to parse FeatureSnapshot or raw vector
            let features: Vec<f64> = if let Ok(snap) = serde_json::from_str::<FeatureSnapshot>(&json_str) {
                snap.to_vector()
            } else if let Ok(vec) = serde_json::from_str::<Vec<f64>>(&json_str) {
                vec
            } else {
                continue;
            };

            records.push(DatasetRecord {
                market_id,
                asset,
                timestamp_ms,
                features,
                target_up,
                resolution,
            });
        }

        Ok(records)
    }

    /// Export records to CSV format with standard header
    pub fn export_to_csv(records: &[DatasetRecord], output_path: &Path) -> Result<()> {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(output_path)
            .with_context(|| format!("Failed to create CSV file at {:?}", output_path))?;
        let mut writer = BufWriter::new(file);

        // Header: metadata + 37 feature names
        let mut header = vec!["timestamp_ms", "market_id", "asset", "target_up", "resolution"];
        header.extend_from_slice(FEATURE_NAMES);
        writeln!(writer, "{}", header.join(","))?;

        for r in records {
            let mut row = vec![
                r.timestamp_ms.to_string(),
                r.market_id.clone(),
                r.asset.to_string(),
                r.target_up.to_string(),
                format!("{:?}", r.resolution),
            ];

            for f in &r.features {
                row.push(format!("{:.6}", f));
            }

            writeln!(writer, "{}", row.join(","))?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Export records to JSONL format
    pub fn export_to_jsonl(records: &[DatasetRecord], output_path: &Path) -> Result<()> {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(output_path)
            .with_context(|| format!("Failed to create JSONL file at {:?}", output_path))?;
        let mut writer = BufWriter::new(file);

        for r in records {
            let line = serde_json::to_string(r)?;
            writeln!(writer, "{}", line)?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Get summary statistics of dataset in database
    pub async fn get_dataset_summary(db: &Database) -> Result<DatasetSummary> {
        let records = Self::load_from_db(db).await?;
        if records.is_empty() {
            return Ok(DatasetSummary {
                total_samples: 0,
                train_samples: 0,
                val_samples: 0,
                test_samples: 0,
                up_count: 0,
                down_count: 0,
                void_count: 0,
                up_ratio: 0.50,
                start_time_ms: 0,
                end_time_ms: 0,
                feature_dimension: FEATURE_NAMES.len(),
                invalid_value_count: 0,
            });
        }

        let splitter = WalkForwardSplitter::default();
        let split = splitter.split(records).map_err(|e| anyhow::anyhow!(e))?;
        Ok(split.summary)
    }
}
