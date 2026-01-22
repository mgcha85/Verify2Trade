use axum::{
    extract::{Path, State},
    response::{sse::{Event, Sse}, IntoResponse},
    Json,
};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::{Arc, Mutex}};
use tokio::sync::broadcast;
use chrono::{DateTime, Utc};
use anyhow::Result;
use polars::prelude::*;
use tracing::{info, error};

use crate::{
    data::DataLoader,
    engine::{BacktestEngine, Trade},
    strategy::ma_touch::MATouchStrategy,
    ai::AIClient,
    settings::Settings,
};

#[derive(Clone)]
pub struct AppState {
    pub data_loader: Arc<DataLoader>,
    pub backtests: Arc<Mutex<HashMap<String, BacktestStatus>>>,
    pub progress_tx: broadcast::Sender<ProgressUpdate>,
    pub ai_client: Arc<AIClient>,
    pub settings: Arc<Settings>,
}

#[derive(Clone, Debug, Serialize)]
pub enum BacktestStatus {
    Running(f32),
    Completed(Vec<Trade>),
    Failed(String),
}

#[derive(Clone, Debug, Serialize)]
pub struct ProgressUpdate {
    pub id: String,
    pub progress: f32,
    pub status: String,
}

#[derive(Deserialize)]
pub struct RunBacktestRequest {
    pub symbol: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub initial_capital: f64,
    pub enable_ai_analysis: Option<bool>,
}

#[derive(Serialize)]
pub struct RunBacktestResponse {
    pub backtest_id: String,
}

pub async fn run_backtest(
    State(state): State<AppState>,
    Json(payload): Json<RunBacktestRequest>,
) -> impl IntoResponse {
    info!("Received backtest request for symbol: {}, start: {}, end: {}, capital: {}", 
        payload.symbol, payload.start_date, payload.end_date, payload.initial_capital);
    let backtest_id = uuid::Uuid::new_v4().to_string();
    let id_clone = backtest_id.clone();
    
    {
        let mut map = state.backtests.lock().unwrap();
        map.insert(backtest_id.clone(), BacktestStatus::Running(0.0));
    }

    let data_loader = state.data_loader.clone();
    let tx = state.progress_tx.clone();
    let backtest_map = state.backtests.clone();
    let _ai_client = state.ai_client.clone();

    tokio::spawn(async move {
        let _ = tx.send(ProgressUpdate {
            id: backtest_id.clone(),
            progress: 0.0,
            status: "Loading Data...".to_string(),
        });

        let result = tokio::task::spawn_blocking(move || -> Result<Vec<Trade>> {
            // 1. Load 1-minute candle data
            let df = data_loader.load_candles(&payload.symbol, payload.start_date, payload.end_date)?;
            let lf = df.lazy();
            
            // 2. Resample to 1-hour candles for strategy execution
            let lf_1h = crate::indicators::resample_to_timeframe(lf.clone(), "1h")?;
            
            // 3. Add real moving averages on 1-hour data
            let lf_1h_with_ma = crate::indicators::add_indicators(lf_1h)?;
            let df_1h = lf_1h_with_ma.collect()?;
            
            // 4. Also resample to 5-minute for charts
            let lf_5m = crate::indicators::resample_to_timeframe(lf, "5m")?;
            let lf_5m_with_ma = crate::indicators::add_indicators(lf_5m)?;
            let df_5m = lf_5m_with_ma.collect()?;
            
            // 5. Save resampled DataFrames for chart generation
            let data_dir = std::path::Path::new("/app/static/charts/data");
            let _ = std::fs::create_dir_all(data_dir);
            
            let df_1h_path = data_dir.join(format!("{}_1h.parquet", payload.symbol));
            let df_5m_path = data_dir.join(format!("{}_5m.parquet", payload.symbol));
            
            let mut file_1h = std::fs::File::create(&df_1h_path)?;
            ParquetWriter::new(&mut file_1h).finish(&mut df_1h.clone())?;
            
            let mut file_5m = std::fs::File::create(&df_5m_path)?;
            ParquetWriter::new(&mut file_5m).finish(&mut df_5m.clone())?;
            
            info!("Saved resampled data: {:?}, {:?}", df_1h_path, df_5m_path);
            
            let candles = candle_from_df(&df_1h, &payload.symbol)?;
            
            // 6. Run strategy on 1-hour candles with real MAs
            let mut engine = BacktestEngine::new(payload.initial_capital);
            let strategy = MATouchStrategy::new(&df_1h);
            
            Ok(engine.run(&candles, strategy))
        }).await.unwrap();

        match result {
            Ok(trades) => {
                if !trades.is_empty() {
                    info!("DEBUG: First trade Symbol: {}, Entry: {:?}", trades[0].symbol, trades[0].entry_time);
                    if trades[0].symbol == "Unknown" {
                       error!("CRITICAL: Symbol is still Unknown!");
                    }
                }
                info!("Backtest {} completed successfully with {} trades", backtest_id, trades.len());
                let mut map = backtest_map.lock().unwrap();
                map.insert(backtest_id.clone(), BacktestStatus::Completed(trades));
                let _ = tx.send(ProgressUpdate {
                    id: backtest_id,
                    progress: 1.0,
                    status: "Completed".to_string(),
                });
            }
            Err(e) => {
                error!("Backtest {} failed: {}", backtest_id, e);
                let mut map = backtest_map.lock().unwrap();
                map.insert(backtest_id.clone(), BacktestStatus::Failed(e.to_string()));
                let _ = tx.send(ProgressUpdate {
                    id: backtest_id,
                    progress: 1.0,
                    status: format!("Failed: {}", e),
                });
            }
        }
    });

    Json(RunBacktestResponse { backtest_id: id_clone })
}

pub async fn get_progress_sse(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    let mut rx = state.progress_tx.subscribe();
    
    let stream = stream::unfold(rx, move |mut rx| {
        let id_target = id.clone();
        async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        if msg.id == id_target {
                            let event = Event::default()
                                .json_data(&msg)
                                .unwrap(); 
                            return Some((Ok(event), rx));
                        }
                    }
                    Err(_) => return None,
                }
            }
        }
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

pub async fn get_result(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let map = state.backtests.lock().unwrap();
    if let Some(status) = map.get(&id) {
        Json(status.clone())
    } else {
        Json(BacktestStatus::Failed("Not Found".to_string()))
    }
}

pub async fn list_symbols() -> impl IntoResponse {
    Json(vec!["BTCUSDT", "ETHUSDT"])
}

// ... existing code ...

#[derive(Deserialize)]
pub struct GetChartRequest {
    pub symbol: String,
    pub timestamp: i64, // Entry time in seconds
}

use axum::response::Response;
use axum::http::header;

pub async fn get_chart_image(
    State(_state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<GetChartRequest>,
) -> impl IntoResponse {
    let filename = format!("{}_{}.png", params.symbol, params.timestamp);
    let file_path = std::path::Path::new("/app/static/charts").join(&filename);

    // 1. Check if chart PNG already exists
    if file_path.exists() {
        if let Ok(bytes) = std::fs::read(&file_path) {
            return Response::builder()
                .header(header::CONTENT_TYPE, "image/png")
                .body(axum::body::Body::from(bytes))
                .unwrap();
        }
    }

    // 2. Load saved resampled DataFrames from backtest (NO re-resampling!)
    let data_dir = std::path::Path::new("/app/static/charts/data");
    let df_1h_path = data_dir.join(format!("{}_1h.parquet", params.symbol));
    let df_5m_path = data_dir.join(format!("{}_5m.parquet", params.symbol));
    
    if !df_1h_path.exists() || !df_5m_path.exists() {
        error!("Backtest data files not found: {:?}, {:?}", df_1h_path, df_5m_path);
        return Response::builder()
            .status(404)
            .body(axum::body::Body::from("Backtest data not found. Run backtest first."))
            .unwrap();
    }
    
    // Load from Parquet files
    let hourly_df = match ParquetReader::new(std::fs::File::open(&df_1h_path).unwrap()).finish() {
        Ok(df) => df,
        Err(e) => {
            error!("Failed to read 1H parquet: {}", e);
            return Response::builder()
                .status(500)
                .body(axum::body::Body::from("Failed to read data"))
                .unwrap();
        }
    };
    
    let fivemin_df = match ParquetReader::new(std::fs::File::open(&df_5m_path).unwrap()).finish() {
        Ok(df) => df,
        Err(e) => {
            error!("Failed to read 5M parquet: {}", e);
            return Response::builder()
                .status(500)
                .body(axum::body::Body::from("Failed to read data"))
                .unwrap();
        }
    };
    
    // 3. Generate chart using the SAME data from backtest
    match crate::charting::generate_stacked_chart(&hourly_df, &fivemin_df, params.timestamp) {
        Ok(bytes) => {
            // Save to file
            let _ = std::fs::create_dir_all("/app/static/charts");
            if let Err(e) = std::fs::write(&file_path, &bytes) {
                error!("Failed to save chart to file: {}", e);
            }
            
            Response::builder()
                .header(header::CONTENT_TYPE, "image/png")
                .body(axum::body::Body::from(bytes))
                .unwrap()
        },
        Err(e) => {
            error!("Failed to generate chart: {}", e);
            Response::builder()
                .status(500)
                .body(axum::body::Body::from("Failed to generate chart"))
                .unwrap()
        }
    }
}

fn candle_from_df(df: &DataFrame, symbol: &str) -> Result<Vec<crate::data::Candle>> {
    use crate::data::Candle;
    
    let open_time = df.column("open_time")?.datetime()?.as_datetime_iter();
    let open = df.column("open")?.f64()?.into_no_null_iter();
    let high = df.column("high")?.f64()?.into_no_null_iter();
    let low = df.column("low")?.f64()?.into_no_null_iter();
    let close = df.column("close")?.f64()?.into_no_null_iter();
    let volume = df.column("volume")?.f64()?.into_no_null_iter();
    
    let mut candles = Vec::with_capacity(df.height());
    
    let times: Vec<_> = open_time.collect();
    let opens: Vec<_> = open.collect();
    let highs: Vec<_> = high.collect();
    let lows: Vec<_> = low.collect();
    let closes: Vec<_> = close.collect();
    let volumes: Vec<_> = volume.collect();
    
    for i in 0..df.height() {
         if let Some(t) = times[i] {
             let utc_time = DateTime::from_naive_utc_and_offset(t, Utc);
             
             candles.push(Candle {
                 symbol: symbol.to_string(), 
                 open_time: utc_time,
                 open: opens[i],
                 high: highs[i],
                 low: lows[i],
                 close: closes[i],
                 volume: volumes[i],
                 close_time: utc_time, 
             });
         }
    }
    
    Ok(candles)
}
