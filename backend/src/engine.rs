use crate::data::Candle;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Side {
    Long,
    Short,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub side: Side,
    pub entry_price: f64,
    pub quantity: f64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub entry_time: DateTime<Utc>,
    pub average_entry_price: f64,
    pub total_quantity: f64,
    #[serde(skip)] // Skip serializing entries history to simplify
    pub entries: Vec<(f64, f64, DateTime<Utc>)>,
}

impl Position {
    pub fn new(symbol: String, side: Side, price: f64, qty: f64, time: DateTime<Utc>) -> Self {
        Self {
            symbol,
            side,
            entry_price: price,
            quantity: qty,
            entry_time: time,
            average_entry_price: price,
            total_quantity: qty,
            entries: vec![(price, qty, time)],
        }
    }

    pub fn add(&mut self, price: f64, qty: f64, time: DateTime<Utc>) {
        let total_cost = self.average_entry_price * self.total_quantity;
        let new_cost = price * qty;
        self.total_quantity += qty;
        self.average_entry_price = (total_cost + new_cost) / self.total_quantity;
        self.entries.push((price, qty, time));
    }

    pub fn reduce(&mut self, qty: f64) {
        self.total_quantity -= qty;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub symbol: String,
    pub side: Side,
    pub entry_price: f64,
    pub exit_price: f64,
    pub quantity: f64,
    pub profit_pct: f64,
    pub profit_abs: f64,
    pub entry_time: DateTime<Utc>,
    pub exit_time: DateTime<Utc>,
    pub exit_reason: String,
}

#[derive(Debug, Clone)]
pub enum Signal {
    None,
    Open(Side, f64),
    Close(String),
    PartialClose(f64, String),
    AddToPosition(f64),
}

pub trait Strategy {
    fn update(
        &mut self,
        candle: &Candle,
        index: usize,
        current_position: Option<&Position>,
        equity: f64,
    ) -> Signal;
}

pub struct BacktestEngine {
    initial_capital: f64,
    equity: f64,
    position: Option<Position>,
    trades: Vec<Trade>,
}

impl BacktestEngine {
    pub fn new(initial_capital: f64) -> Self {
        Self {
            initial_capital,
            equity: initial_capital,
            position: None,
            trades: Vec::new(),
        }
    }

    pub fn run<S: Strategy>(&mut self, candles: &[Candle], mut strategy: S) -> Vec<Trade> {
        for (index, candle) in candles.iter().enumerate() {
            let signal = strategy.update(candle, index, self.position.as_ref(), self.equity);

            match signal {
                Signal::Open(side, amount) => {
                    if self.position.is_none() {
                        let qty = amount / candle.close;
                        self.position = Some(Position::new(
                            candle.symbol.clone(),
                            side,
                            candle.close,
                            qty,
                            candle.open_time,
                        ));
                    }
                }
                Signal::AddToPosition(amount) => {
                    if let Some(pos) = &mut self.position {
                        let qty = amount / candle.close;
                        pos.add(candle.close, qty, candle.open_time);
                    }
                }
                Signal::Close(reason) => {
                    if let Some(pos) = self.position.take() {
                        self.close_position(pos, candle, reason);
                    }
                }
                Signal::PartialClose(fraction, reason) => {
                    if let Some(pos) = &mut self.position {
                        let qty_to_close = pos.total_quantity * fraction;
                        let exit_price = candle.close;

                        let pnl_pct = match pos.side {
                            Side::Long => {
                                (exit_price - pos.average_entry_price) / pos.average_entry_price
                            }
                            Side::Short => {
                                (pos.average_entry_price - exit_price) / pos.average_entry_price
                            }
                        };
                        let pnl_abs = pnl_pct * pos.average_entry_price * qty_to_close;

                        self.equity += pnl_abs;

                        self.trades.push(Trade {
                            symbol: pos.symbol.clone(),
                            side: pos.side,
                            entry_price: pos.average_entry_price,
                            exit_price,
                            quantity: qty_to_close,
                            profit_pct: pnl_pct * 100.0,
                            profit_abs: pnl_abs,
                            entry_time: pos.entry_time,
                            exit_time: candle.open_time,
                            exit_reason: reason,
                        });

                        pos.reduce(qty_to_close);

                        if pos.total_quantity <= 0.00000001 {
                            self.position = None;
                        }
                    }
                }
                Signal::None => {}
            }
        }
        self.trades.clone()
    }

    fn close_position(&mut self, pos: Position, candle: &Candle, reason: String) {
        let exit_price = candle.close;
        let pnl_pct = match pos.side {
            Side::Long => (exit_price - pos.average_entry_price) / pos.average_entry_price,
            Side::Short => (pos.average_entry_price - exit_price) / pos.average_entry_price,
        };
        let pnl_abs = pnl_pct * pos.average_entry_price * pos.total_quantity;

        self.equity += pnl_abs;

        self.trades.push(Trade {
            symbol: pos.symbol,
            side: pos.side,
            entry_price: pos.average_entry_price,
            exit_price,
            quantity: pos.total_quantity,
            profit_pct: pnl_pct * 100.0,
            profit_abs: pnl_abs,
            entry_time: pos.entry_time,
            exit_time: candle.open_time,
            exit_reason: reason,
        });
    }
}
