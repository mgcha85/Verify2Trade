use crate::data::Candle;
use crate::engine::{Position, Side, Signal, Strategy};
use polars::prelude::*;

pub struct MATouchStrategy {
    ma_25: Vec<f64>,

    // State
    price_was_below_ma: bool,
    partial_profit_taken: bool,
}

impl MATouchStrategy {
    pub fn new(df: &DataFrame) -> Self {
        let ma_25 = df
            .column("ma_25")
            .expect("ma_25 column missing")
            .f64()
            .expect("ma_25 is not f64")
            .into_no_null_iter()
            .collect();

        Self {
            ma_25,
            price_was_below_ma: false,
            partial_profit_taken: false,
        }
    }
}

impl Strategy for MATouchStrategy {
    fn update(
        &mut self,
        candle: &Candle,
        index: usize,
        position: Option<&Position>,
        equity: f64,
    ) -> Signal {
        if index >= self.ma_25.len() || index < 400 {
            return Signal::None;
        }

        let ma = self.ma_25[index];

        if position.is_none() {
            self.partial_profit_taken = false;

            // Check if price is/was below MA
            if candle.close < ma {
                self.price_was_below_ma = true;
            } else if candle.low > ma {
                self.price_was_below_ma = false;
            }

            // Entry trigger: price was below MA, touched MA (High >= MA), and rejected (Close < MA)
            if self.price_was_below_ma && candle.high >= ma && candle.close < ma {
                // Log entry for debugging
                tracing::info!(
                    "ENTRY TRIGGERED: time={:?}, high={:.2}, low={:.2}, close={:.2}, ma25={:.2}, high>=ma:{}, close<ma:{}",
                    candle.open_time,
                    candle.high,
                    candle.low,
                    candle.close,
                    ma,
                    candle.high >= ma,
                    candle.close < ma
                );
                // Open Short with 1/4 account size
                let amount = equity * 0.25;
                return Signal::Open(Side::Short, amount);
            }
        } else if let Some(pos) = position {
            let price_change_pct =
                (candle.close - pos.average_entry_price) / pos.average_entry_price;

            // Pyramiding: Add if price rises 2% above first entry (averaging down for short)
            // Limit to max 2 entries (initial + 1 add)
            if pos.entries.len() < 2 && candle.close >= pos.entries[0].0 * 1.02 {
                let add_amount = pos.entries[0].1 * candle.close;
                return Signal::AddToPosition(add_amount);
            }

            // Stop Loss: Avg Entry + 2%
            if candle.close >= pos.average_entry_price * 1.02 {
                return Signal::Close("SL".to_string());
            }

            // Take Profit
            // Profit for short is when price decreases (negative change vs entry)
            // profit_pct variable above is (Current - Entry) / Entry. For short, -0.01 means 1% profit.
            // Let's use correct profit calc for clarity
            let profit_pct = (pos.average_entry_price - candle.close) / pos.average_entry_price;

            if profit_pct >= 0.03 {
                return Signal::Close("TP_MAX".to_string());
            }

            if profit_pct >= 0.01 && !self.partial_profit_taken {
                self.partial_profit_taken = true;
                return Signal::PartialClose(0.5, "TP_HALF".to_string());
            }

            // If partial profit taken, SL is moved to Breakeven (Avg Entry)
            if self.partial_profit_taken {
                if candle.close >= pos.average_entry_price {
                    return Signal::Close("SL_Breakeven".to_string());
                }
            }
        }

        Signal::None
    }
}
