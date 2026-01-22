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
    // 1. Prepare Data - filter to show only data BEFORE entry_time, limit to 200 candles
    let hourly_data = df_to_chart_data(df_hourly, entry_time)?;
    let daily_data = df_to_chart_data(df_daily, entry_time)?;

    // 2. Setup Drawing Area (Buffer)
    let width = 1200;
    let height = 800;
    let mut buffer = vec![0; (width * height * 3) as usize];
    
    {
        let root = BitMapBackend::with_buffer(&mut buffer, (width, height)).into_drawing_area();
        root.fill(&WHITE)?;

        let (top, bottom) = root.split_vertically(400);

        // 3. Draw 1-hour Chart (Top) with light blue background and "1H" label
        draw_chart(&top, &hourly_data, entry_time, &RGBColor(240, 248, 255), ChartType::OneHour)?;

        // 4. Draw 5-minute Chart (Bottom) with light yellow background and "5M" label
        draw_chart(&bottom, &daily_data, entry_time, &RGBColor(255, 250, 240), ChartType::FiveMinute)?;
        
        root.present()?;
    }

    // 5. Encode to PNG
    let img = RgbImage::from_raw(width, height, buffer).ok_or(anyhow::anyhow!("Failed to create image buffer"))?;
    let mut bytes: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut bytes);
    img.write_to(&mut cursor, ImageFormat::Png)?;

    Ok(bytes)
}

#[derive(Debug)]
enum ChartType {
    OneHour,
    FiveMinute,
}

#[derive(Debug)]
struct ChartData {
    candles: Vec<(i64, f64, f64, f64, f64)>, // timestamp, open, high, low, close
    ma25: Vec<(i64, f64)>,
    ma50: Vec<(i64, f64)>,
    ma200: Vec<(i64, f64)>,
    ma400: Vec<(i64, f64)>,
}

fn df_to_chart_data(df: &DataFrame, entry_time: i64) -> anyhow::Result<ChartData> {
    let open_time = df.column("open_time")?.datetime()?.as_datetime_iter();
    let open = df.column("open")?.f64()?.into_no_null_iter();
    let high = df.column("high")?.f64()?.into_no_null_iter();
    let low = df.column("low")?.f64()?.into_no_null_iter();
    let close = df.column("close")?.f64()?.into_no_null_iter();
    
    // Extract MA columns
    let ma25_col = df.column("ma_25")?.f64()?;
    let ma50_col = df.column("ma_50")?.f64()?;
    let ma200_col = df.column("ma_200")?.f64()?;
    let ma400_col = df.column("ma_400")?. f64()?;

    let iter = open_time.zip(open).zip(high).zip(low).zip(close);
    
    let mut all_candles = Vec::new();
    let mut all_ma25 = Vec::new();
    let mut all_ma50 = Vec::new();
    let mut all_ma200 = Vec::new();
    let mut all_ma400 = Vec::new();
    
    for (idx, ((((opt, o), h), l), c)) in iter.enumerate() {
         if let Some(dt) = opt {
             let ts = dt.and_utc().timestamp();
             // Only include data BEFORE or AT entry time
             if ts <= entry_time {
                 all_candles.push((ts, o, h, l, c));
                 
                 // Add MA values if not null
                 if let Some(v) = ma25_col.get(idx) {
                     all_ma25.push((ts, v));
                 }
                 if let Some(v) = ma50_col.get(idx) {
                     all_ma50.push((ts, v));
                 }
                 if let Some(v) = ma200_col.get(idx) {
                     all_ma200.push((ts, v));
                 }
                 if let Some(v) = ma400_col.get(idx) {
                     all_ma400.push((ts, v));
                 }
             }
         }
    }
    
    // Take only the last 200 candles
    let candles = if all_candles.len() > 200 {
        all_candles.split_off(all_candles.len() - 200)
    } else {
        all_candles
    };
    
    // Filter MA values to match candle timestamps
    let min_ts = candles.first().map(|c| c.0).unwrap_or(0);
    let ma25: Vec<_> = all_ma25.into_iter().filter(|(ts, _)| *ts >= min_ts).collect();
    let ma50: Vec<_> = all_ma50.into_iter().filter(|(ts, _)| *ts >= min_ts).collect();
    let ma200: Vec<_> = all_ma200.into_iter().filter(|(ts, _)| *ts >= min_ts).collect();
    let ma400: Vec<_> = all_ma400.into_iter().filter(|(ts, _)| *ts >= min_ts).collect();
    
    // Debug: Log the last candle (entry candle) and its MA25
    if let Some(last_candle) = candles.last() {
        let last_ma25 = ma25.last().map(|(_, v)| *v).unwrap_or(0.0);
        tracing::info!(
            "CHART ENTRY CANDLE: ts={}, high={:.2}, close={:.2}, ma25={:.2}, high>=ma25:{}, close<ma25:{}",
            last_candle.0,
            last_candle.2, // high
            last_candle.4, // close
            last_ma25,
            last_candle.2 >= last_ma25,
            last_candle.4 < last_ma25
        );
    }
    
    Ok(ChartData {
        candles,
        ma25,
        ma50,
        ma200,
        ma400,
    })
}

fn draw_chart<DB: DrawingBackend>(
    root: &DrawingArea<DB, plotters::coord::Shift>,
    data: &ChartData,
    entry_time: i64,
    bg_color: &RGBColor,
    chart_type: ChartType,
) -> anyhow::Result<()> where DB::ErrorType: 'static {
    if data.candles.is_empty() {
        return Ok(());
    }

    // Fill background with distinct color
    root.fill(bg_color)?;

    // Draw label box in top-left corner
    draw_label_indicator(root, &chart_type)?;

    let min_ts = data.candles.first().unwrap().0;
    let max_ts = entry_time; // Entry time at the right edge
    let min_low = data.candles.iter().map(|c| c.3).fold(f64::INFINITY, f64::min);
    let max_high = data.candles.iter().map(|c| c.2).fold(f64::NEG_INFINITY, f64::max);

    // Expand vertical range slightly for MAs
    let range_padding = (max_high - min_low) * 0.05;
    let chart_min = min_low - range_padding;
    let chart_max = max_high + range_padding;

    let mut chart = ChartBuilder::on(root)
        .margin(10)
        .build_cartesian_2d(min_ts..max_ts, chart_min..chart_max)?;

    // Draw candlesticks
    chart.draw_series(data.candles.iter().map(|&(x, o, h, l, c)| {
        CandleStick::new(x, o, h, l, c, GREEN.filled(), RED.filled(), 3)
    }))?;

    // Draw pre-calculated MA lines
    if !data.ma25.is_empty() {
        chart.draw_series(LineSeries::new(data.ma25.clone(), RED.stroke_width(2)))?;
    }
    if !data.ma50.is_empty() {
        chart.draw_series(LineSeries::new(data.ma50.clone(), GREEN.stroke_width(2)))?;
    }
    if !data.ma200.is_empty() {
        chart.draw_series(LineSeries::new(data.ma200.clone(), BLUE.stroke_width(2)))?;
    }
    if !data.ma400.is_empty() {
        chart.draw_series(LineSeries::new(data.ma400.clone(), RGBColor(128, 128, 128).stroke_width(2)))?;
    }

    Ok(())
}

fn draw_label_indicator<DB: DrawingBackend>(
    root: &DrawingArea<DB, plotters::coord::Shift>,
    chart_type: &ChartType,
) -> anyhow::Result<()> where DB::ErrorType: 'static {
    // Draw a label box in the top-left corner
    let label_box = Rectangle::new([(15, 15), (85, 45)], WHITE.filled());
    root.draw(&label_box)?;
    
    // Draw border
    let label_border = Rectangle::new([(15, 15), (85, 45)], BLACK.stroke_width(2));
    root.draw(&label_border)?;
    
    // Draw simple pattern to indicate chart type
    match chart_type {
        ChartType::OneHour => {
            // Draw "1H" using simple rectangles
            // Draw "1"
            root.draw(&Rectangle::new([(25, 20), (30, 40)], BLACK.filled()))?;
            // Draw "H"
            root.draw(&Rectangle::new([(40, 20), (44, 40)], BLACK.filled()))?;
            root.draw(&Rectangle::new([(56, 20), (60, 40)], BLACK.filled()))?;
            root.draw(&Rectangle::new([(44, 28), (56, 32)], BLACK.filled()))?;
        },
        ChartType::FiveMinute => {
            // Draw "5M" using simple rectangles
            // Draw "5"
            root.draw(&Rectangle::new([(22, 20), (34, 24)], BLACK.filled()))?;
            root.draw(&Rectangle::new([(22, 20), (26, 30)], BLACK.filled()))?;
            root.draw(&Rectangle::new([(22, 28), (34, 32)], BLACK.filled()))?;
            root.draw(&Rectangle::new([(30, 30), (34, 40)], BLACK.filled()))?;
            root.draw(&Rectangle::new([(22, 36), (34, 40)], BLACK.filled()))?;
            // Draw "M"
            root.draw(&Rectangle::new([(44, 20), (48, 40)], BLACK.filled()))?;
            root.draw(&Rectangle::new([(64, 20), (68, 40)], BLACK.filled()))?;
            root.draw(&Rectangle::new([(48, 20), (52, 28)], BLACK.filled()))?;
            root.draw(&Rectangle::new([(60, 20), (64, 28)], BLACK.filled()))?;
        },
    }
    
    Ok(())
}
