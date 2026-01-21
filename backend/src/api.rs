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

use crate::{
    data::DataLoader,
    engine::{BacktestEngine, Trade},
    strategy::ma_touch::MATouchStrategy,
    ai::AIClient,
};

#[derive(Clone)]
pub struct AppState {
    pub data_loader: Arc<DataLoader>,
    pub backtests: Arc<Mutex<HashMap<String, BacktestStatus>>>,
    pub progress_tx: broadcast::Sender<ProgressUpdate>,
    pub ai_client: Arc<AIClient>,
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
            let df = data_loader.load_candles(&payload.symbol, payload.start_date, payload.end_date)?;
            // Convert DataFrame to LazyFrame for indicators
            let lf = df.lazy();
            
            let lf_indicators = crate::indicators::add_indicators(lf)?;
            
            let df = lf_indicators.collect()?;
            let candles = candle_from_df(&df)?;
            
            let mut engine = BacktestEngine::new(payload.initial_capital);
            let strategy = MATouchStrategy::new(&df);
            
            Ok(engine.run(&candles, strategy))
        }).await.unwrap();

        match result {
            Ok(trades) => {
                let mut map = backtest_map.lock().unwrap();
                map.insert(backtest_id.clone(), BacktestStatus::Completed(trades));
                let _ = tx.send(ProgressUpdate {
                    id: backtest_id,
                    progress: 1.0,
                    status: "Completed".to_string(),
                });
            }
            Err(e) => {
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

fn candle_from_df(df: &DataFrame) -> Result<Vec<crate::data::Candle>> {
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
                 symbol: "Unknown".to_string(), 
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
