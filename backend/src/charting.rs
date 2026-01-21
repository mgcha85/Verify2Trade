use anyhow::Result;
use plotters::prelude::*;
use plotters::backend::BitMapBackend;
use polars::prelude::*;
use std::io::Cursor;
use image::{ImageFormat, RgbImage};

pub fn generate_stacked_chart(
    df_hourly: &DataFrame,
    df_daily: &DataFrame,
    entry_time: i64, // Unix timestamp in seconds
) -> anyhow::Result<Vec<u8>> {
    // 1. Prepare Data - filter to show only data BEFORE entry_time
    let hourly_candles = df_to_candles_before(df_hourly, entry_time)?;
    let daily_candles = df_to_candles_before(df_daily, entry_time)?;

    // 2. Setup Drawing Area (Buffer)
    let width = 1200;
    let height = 800;
    let mut buffer = vec![0; (width * height * 3) as usize];
    
    {
        let root = BitMapBackend::with_buffer(&mut buffer, (width, height)).into_drawing_area();
        root.fill(&WHITE)?;

        let (top, bottom) = root.split_vertically(400);

        // 3. Draw Hourly Chart (Top) - 5분봉
        draw_chart(&top, &hourly_candles, "5분봉", entry_time)?;

        // 4. Draw Daily Chart (Bottom) - 일봉
        draw_chart(&bottom, &daily_candles, "일봉", entry_time)?;
        
        root.present()?;
    }

    // 5. Encode to PNG
    let img = RgbImage::from_raw(width, height, buffer).ok_or(anyhow::anyhow!("Failed to create image buffer"))?;
    let mut bytes: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut bytes);
    img.write_to(&mut cursor, ImageFormat::Png)?;

    Ok(bytes)
}

fn df_to_candles_before(df: &DataFrame, entry_time: i64) -> anyhow::Result<Vec<(i64, f64, f64, f64, f64)>> {
    let open_time = df.column("open_time")?.datetime()?.as_datetime_iter();
    let open = df.column("open")?.f64()?.into_no_null_iter();
    let high = df.column("high")?.f64()?.into_no_null_iter();
    let low = df.column("low")?.f64()?.into_no_null_iter();
    let close = df.column("close")?.f64()?.into_no_null_iter();

    let iter = open_time.zip(open).zip(high).zip(low).zip(close);
    
    let mut candles = Vec::new();
    for ((((opt, o), h), l), c) in iter {
         if let Some(dt) = opt {
             let ts = dt.and_utc().timestamp();
             // Only include candles BEFORE or AT entry time
             if ts <= entry_time {
                 candles.push((ts, o, h, l, c));
             }
         }
    }
    Ok(candles)
}

fn calculate_ma(candles: &[(i64, f64, f64, f64, f64)], period: usize) -> Vec<(i64, f64)> {
    let mut ma_values = Vec::new();
    
    if candles.len() < period {
        return ma_values;
    }
    
    for i in period..=candles.len() {
        let start_idx = i.saturating_sub(period);
        let sum: f64 = candles[start_idx..i].iter().map(|c| c.4).sum();
        let ma = sum / period as f64;
        ma_values.push((candles[i - 1].0, ma));
    }
    
    ma_values
}

fn draw_chart<DB: DrawingBackend>(
    root: &DrawingArea<DB, plotters::coord::Shift>,
    candles: &[(i64, f64, f64, f64, f64)],
    title: &str,
    entry_time: i64,
) -> anyhow::Result<()> where DB::ErrorType: 'static {
    if candles.is_empty() {
        return Ok(());
    }

    let min_ts = candles.first().unwrap().0;
    let max_ts = entry_time; // Entry time at the right edge
    let min_low = candles.iter().map(|c| c.3).fold(f64::INFINITY, f64::min);
    let max_high = candles.iter().map(|c| c.2).fold(f64::NEG_INFINITY, f64::max);

    // Expand vertical range slightly for MAs
    let range_padding = (max_high - min_low) * 0.05;
    let chart_min = min_low - range_padding;
    let chart_max = max_high + range_padding;

    let mut chart = ChartBuilder::on(root)
        .margin(10)
        .build_cartesian_2d(min_ts..max_ts, chart_min..chart_max)?;

    // Draw candlesticks
    chart.draw_series(candles.iter().map(|&(x, o, h, l, c)| {
        CandleStick::new(x, o, h, l, c, GREEN.filled(), RED.filled(), 3)
    }))?;

    // Calculate and draw moving averages: 25, 50, 200, 400
    // Colors: RED, GREEN, BLUE, gray
    let ma25 = calculate_ma(candles, 25);
    let ma50 = calculate_ma(candles, 50);
    let ma200 = calculate_ma(candles, 200);
    let ma400 = calculate_ma(candles, 400);

    // Draw MA lines
    if !ma25.is_empty() {
        chart.draw_series(LineSeries::new(ma25, RED.stroke_width(2)))?;
    }
    if !ma50.is_empty() {
        chart.draw_series(LineSeries::new(ma50, GREEN.stroke_width(2)))?;
    }
    if !ma200.is_empty() {
        chart.draw_series(LineSeries::new(ma200, BLUE.stroke_width(2)))?;
    }
    if !ma400.is_empty() {
        chart.draw_series(LineSeries::new(ma400, RGBColor(128, 128, 128).stroke_width(2)))?;
    }

    // Mark Entry Time with vertical line at the right edge
    chart.draw_series(LineSeries::new(
        vec![(entry_time, chart_min), (entry_time, chart_max)],
        BLACK.stroke_width(3),
    ))?;

    // Draw title text manually (without font dependencies)
    // We'll draw it as a simple text annotation
    let title_pos = (min_ts + (max_ts - min_ts) / 20, chart_max - range_padding * 0.5);
    root.draw_text(
        title,
        &TextStyle::from(("sans-serif", 20).into_font()).color(&BLACK),
        (40, 20),
    )?;

    Ok(())
}

