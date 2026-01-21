use anyhow::Result;
use chrono::{DateTime, Utc};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub symbol: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub open_time: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub close_time: DateTime<Utc>,
}

pub struct DataLoader {
    base_path: PathBuf,
}

impl DataLoader {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    pub fn load_candles(
        &self,
        symbol: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<DataFrame> {
        let path = self.base_path.join(format!("symbol={}", symbol));

        if !path.exists() {
            return Err(anyhow::anyhow!("Symbol data not found: {}", symbol));
        }

        // Scan parquet
        let args = ScanArgsParquet::default();

        // Use 'timestamp' column which exists in parquet, then rename to 'open_time'
        // Filter and sort using original column name to avoid pushdown errors
        let lf = LazyFrame::scan_parquet(path.join("**/*.parquet"), args)?
            .filter(col("timestamp").gt_eq(lit(start_time.naive_utc())))
            .filter(col("timestamp").lt_eq(lit(end_time.naive_utc())))
            .sort(vec!["timestamp"], SortMultipleOptions::default())
            .with_column(col("timestamp").alias("open_time"));

        Ok(lf.collect()?)
    }
}
