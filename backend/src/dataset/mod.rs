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

    /// Generate synthetic realistic resolved market rounds and feature snapshots for rapid testing and training
    pub async fn generate_synthetic_rounds(db: &Database, rounds: usize) -> Result<(usize, usize)> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let assets = [Asset::BTC, Asset::ETH, Asset::SOL];
        let mut total_markets = 0;
        let mut total_features = 0;

        for r in 0..rounds {
            let offset_ms = (rounds - r) as i64 * 300_000;
            let round_start = now_ms - offset_ms;
            let round_end = round_start + 300_000;

            for &asset in &assets {
                let base_price = match asset {
                    Asset::BTC => 65000.0,
                    Asset::ETH => 3400.0,
                    Asset::SOL => 145.0,
                };

                let seed = ((r * 17 + asset as usize * 31) % 100) as f64 / 100.0;
                let is_up = seed > 0.48;
                let price_change_pct = if is_up {
                    0.001 + seed * 0.004
                } else {
                    -0.001 - (1.0 - seed) * 0.004
                };

                let open_price = base_price * (1.0 + ((r % 20) as f64 - 10.0) * 0.001);
                let close_price = open_price * (1.0 + price_change_pct);
                let resolution_str = if is_up { "UP" } else { "DOWN" };
                let market_id = format!("{}-{}-syn", asset, round_start / 1000);
                let condition_id = format!("cond_{}", market_id);

                let _ = sqlx::query(
                    r#"
                    INSERT OR REPLACE INTO markets (
                        id, condition_id, asset, start_time, end_time, open_price, final_price, resolution, status, created_at, updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'resolved', ?, ?)
                    "#,
                )
                .bind(&market_id)
                .bind(&condition_id)
                .bind(asset.to_string())
                .bind(round_start)
                .bind(round_end)
                .bind(open_price)
                .bind(close_price)
                .bind(resolution_str)
                .bind(round_start)
                .bind(round_end)
                .execute(db.pool())
                .await?;

                total_markets += 1;

                for elapsed in [60, 180] {
                    let snap_time = round_start + elapsed * 1000;
                    let remaining_seconds = 300 - elapsed;
                    let progress = elapsed as f64 / 300.0;
                    let curr_price = open_price + (close_price - open_price) * progress;
                    let dist_pct = (curr_price - open_price) / open_price;
                    let sign = if is_up { 1.0 } else { -1.0 };

                    let snap = FeatureSnapshot {
                        asset,
                        market_id: market_id.clone(),
                        timestamp_ms: snap_time,
                        composite_price: curr_price,
                        return_1s: sign * 0.0001 * seed,
                        return_3s: sign * 0.0003 * seed,
                        return_5s: sign * 0.0005 * seed,
                        return_10s: sign * 0.0008 * seed,
                        return_30s: sign * 0.0015 * seed,
                        return_60s: sign * 0.0020 * seed,
                        realized_vol_5s: 0.0005,
                        realized_vol_10s: 0.0008,
                        realized_vol_30s: 0.0015,
                        realized_vol_60s: 0.0022,
                        velocity_5s: sign * 0.5 * seed,
                        velocity_15s: sign * 0.3 * seed,
                        acceleration_5s_15s: sign * 0.1,
                        distance_from_open: curr_price - open_price,
                        distance_percent: dist_pct,
                        distance_to_vol_ratio: dist_pct / 0.0022,
                        remaining_seconds: remaining_seconds as f64,
                        elapsed_seconds: elapsed as f64,
                        time_decay_factor: remaining_seconds as f64 / 300.0,
                        spread_binance_okx: 0.5,
                        spread_binance_bybit: 0.8,
                        spread_binance_coinbase: 1.0,
                        cvd_5s: sign * 15.0 * seed,
                        cvd_15s: sign * 35.0 * seed,
                        cvd_30s: sign * 70.0 * seed,
                        cvd_60s: sign * 120.0 * seed,
                        trade_imbalance_5s: sign * 0.3 * seed,
                        trade_imbalance_15s: sign * 0.25 * seed,
                        trade_imbalance_30s: sign * 0.2 * seed,
                        trade_imbalance_60s: sign * 0.15 * seed,
                        poly_obi_top5: sign * 0.35 * seed,
                        poly_obi_top10: sign * 0.25 * seed,
                        poly_obi_top20: sign * 0.18 * seed,
                        poly_spread: 0.02,
                        poly_total_liquidity: 5000.0,
                        poly_implied_prob: (0.50 + sign * 0.15 * seed).clamp(0.05, 0.95),
                    };

                    let json_vec = snap.to_json();
                    let _ = sqlx::query(
                        r#"
                        INSERT INTO features (market_id, timestamp, feature_name, feature_vector_json, created_at)
                        VALUES (?, ?, 'snapshot_v1', ?, ?)
                        "#,
                    )
                    .bind(&market_id)
                    .bind(snap_time)
                    .bind(&json_vec)
                    .bind(snap_time)
                    .execute(db.pool())
                    .await?;

                    total_features += 1;
                }
            }
        }

        Ok((total_markets, total_features))
    }
}
