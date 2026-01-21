use anyhow::Result;
use plotters::prelude::*;
use polars::prelude::*;
use std::path::Path;

pub fn generate_candle_chart(df: &DataFrame, output_path: &Path, title: &str) -> Result<()> {
    // Expect columns: open_time, open, high, low, close

    let root = BitMapBackend::new(output_path, (1024, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let open_time = df
        .column("open_time")?
        .datetime()?
        .as_datetime_iter()
        .collect::<Vec<_>>();
    let open = df
        .column("open")?
        .f64()?
        .into_no_null_iter()
        .collect::<Vec<_>>();
    let high = df
        .column("high")?
        .f64()?
        .into_no_null_iter()
        .collect::<Vec<_>>();
    let low = df
        .column("low")?
        .f64()?
        .into_no_null_iter()
        .collect::<Vec<_>>();
    let close = df
        .column("close")?
        .f64()?
        .into_no_null_iter()
        .collect::<Vec<_>>();

    let len = open.len();
    if len == 0 {
        return Ok(());
    }

    let candles: Vec<(usize, f64, f64, f64, f64)> = (0..len)
        .map(|i| (i, open[i], high[i], low[i], close[i]))
        .collect();

    let min_low = low.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_high = high.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 30))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0..len, min_low..max_high)?;

    chart.configure_mesh().draw()?;

    chart.draw_series(candles.iter().map(|&(x, o, h, l, c)| {
        CandleStick::new(x, o, h, l, c, GREEN.filled(), RED.filled(), 10)
    }))?;

    if let Ok(ma) = df.column("ma_25") {
        if let Ok(ma_f64) = ma.f64() {
            let line_data: Vec<(usize, f64)> = ma_f64
                .into_iter()
                .enumerate()
                .filter_map(|(i, v)| v.map(|val| (i, val)))
                .collect();

            chart.draw_series(LineSeries::new(line_data, &BLUE))?;
        }
    }

    root.present()?;
    Ok(())
}
