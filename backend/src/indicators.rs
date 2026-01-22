use anyhow::Result;
use polars::prelude::*;

/// Add moving average indicators using real rolling mean calculation
pub fn add_indicators(lf: LazyFrame) -> Result<LazyFrame> {
    // Calculate real moving averages using rolling_mean
    let lf = lf
        .with_columns(vec![
            col("close").rolling_mean(RollingOptionsFixedWindow {
                window_size: 25,
                min_periods: 1,
                ..Default::default()
            }).alias("ma_25"),
            col("close").rolling_mean(RollingOptionsFixedWindow {
                window_size: 50,
                min_periods: 1,
                ..Default::default()
            }).alias("ma_50"),
            col("close").rolling_mean(RollingOptionsFixedWindow {
                window_size: 100,
                min_periods: 1,
                ..Default::default()
            }).alias("ma_100"),
            col("close").rolling_mean(RollingOptionsFixedWindow {
                window_size: 200,
                min_periods: 1,
                ..Default::default()
            }).alias("ma_200"),
            col("close").rolling_mean(RollingOptionsFixedWindow {
                window_size: 400,
                min_periods: 1,
                ..Default::default()
            }).alias("ma_400"),
        ]);

    Ok(lf)
}

/// Resample 1-minute candles to specified timeframe (e.g., "1h" for 1-hour, "5m" for 5-minute)
pub fn resample_to_timeframe(lf: LazyFrame, timeframe: &str) -> Result<LazyFrame> {
    let resampled = lf
        .group_by_dynamic(
            col("open_time"),
            vec![],
            DynamicGroupOptions {
                every: Duration::parse(timeframe),
                period: Duration::parse(timeframe),
                offset: Duration::parse("0s"),
                ..Default::default()
            }
        )
        .agg(vec![
            col("open").first(),
            col("high").max(),
            col("low").min(),
            col("close").last(),
            col("volume").sum(),
        ]);

    Ok(resampled)
}
