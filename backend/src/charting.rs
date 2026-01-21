// Existing imports...
use std::io::Cursor;
use image::{ImageOutputFormat, Rgb, RgbImage};

pub fn generate_stacked_chart(
    df_hourly: &DataFrame,
    df_daily: &DataFrame,
    entry_time: i64, // Unix timestamp in seconds
) -> Result<Vec<u8>> {
    // 1. Prepare Data
    let hourly_candles = df_to_candles(df_hourly)?;
    let daily_candles = df_to_candles(df_daily)?;

    // 2. Setup Drawing Area (Buffer)
    let width = 1200;
    let height = 800;
    let mut buffer = vec![0; (width * height * 3) as usize];
    
    {
        let root = BitMapBackend::with_buffer(&mut buffer, (width, height)).into_drawing_area();
        root.fill(&WHITE)?;

        let (top, bottom) = root.split_vertically(400);

        // 3. Draw Hourly Chart (Top)
        draw_chart(&top, &hourly_candles, "Hourly Chart", entry_time)?;

        // 4. Draw Daily Chart (Bottom)
        draw_chart(&bottom, &daily_candles, "Daily Chart", entry_time)?;
        
        root.present()?;
    }

    // 5. Encode to PNG
    // BitMapBackend writes raw RGB bytes. We need to encode using image crate.
    let img = RgbImage::from_raw(width, height, buffer).ok_or(anyhow::anyhow!("Failed to create image buffer"))?;
    let mut bytes: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut bytes);
    img.write_to(&mut cursor, ImageOutputFormat::Png)?;

    Ok(bytes)
}

fn df_to_candles(df: &DataFrame) -> Result<Vec<(i64, f64, f64, f64, f64)>> {
    let open_time = df.column("open_time")?.datetime()?.as_datetime_iter();
    let open = df.column("open")?.f64()?.into_no_null_iter();
    let high = df.column("high")?.f64()?.into_no_null_iter();
    let low = df.column("low")?.f64()?.into_no_null_iter();
    let close = df.column("close")?.f64()?.into_no_null_iter();

    let iter = open_time.zip(open).zip(high).zip(low).zip(close);
    
    let mut candles = Vec::new();
    for ((((opt, o), h), l), c) in iter {
         if let Some(dt) = opt {
             // Convert to timestamp seconds
             let ts = dt.timestamp();
             candles.push((ts, o, h, l, c));
         }
    }
    Ok(candles)
}

fn draw_chart<DB: DrawingBackend>(
    root: &DrawingArea<DB, plotters::coord::Shift>,
    candles: &[(i64, f64, f64, f64, f64)],
    title: &str,
    entry_time: i64,
) -> Result<()> where DB::ErrorType: 'static {
    if candles.is_empty() {
        return Ok(());
    }

    let min_ts = candles.first().unwrap().0;
    let max_ts = candles.last().unwrap().0;
    let min_low = candles.iter().map(|c| c.3).fold(f64::INFINITY, f64::min);
    let max_high = candles.iter().map(|c| c.2).fold(f64::NEG_INFINITY, f64::max);

    let mut chart = ChartBuilder::on(root)
        .caption(title, ("sans-serif", 20))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(min_ts..max_ts, min_low..max_high)?;

    chart.configure_mesh().draw()?;

    chart.draw_series(candles.iter().map(|&(x, o, h, l, c)| {
        CandleStick::new(x, o, h, l, c, GREEN.filled(), RED.filled(), 5)
    }))?;

    // Mark Entry Time
    // We can draw a vertical line
    chart.draw_series(LineSeries::new(
        vec![(entry_time, min_low), (entry_time, max_high)],
        &BLACK.stroke_width(2),
    ))?;

    Ok(())
}

