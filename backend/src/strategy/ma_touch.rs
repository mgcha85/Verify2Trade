use crate::data::Candle;
use crate::engine::{Position, Side, Signal, Strategy};
use polars::prelude::*;

/// MA25 Retest Strategy (Short only)
/// Entry condition:
/// 1. Price was ABOVE MA25, then breaks BELOW (breakdown)
/// 2. Price comes back up and touches MA25 from below (retest)
/// 3. Current candle: high touches MA25, but closes below MA25 (rejection)
pub struct MATouchStrategy {
    ma_25: Vec<f64>,

    // State tracking
    was_above_ma: bool,        // Was price ever above MA25?
    had_breakdown: bool,       // Did price break below MA25 after being above?
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
            was_above_ma: false,
            had_breakdown: false,
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

            // Step 1: Track if price was above MA25
            if candle.close > ma && candle.low > ma {
                // Price is clearly above MA25
                self.was_above_ma = true;
                self.had_breakdown = false;  // Reset breakdown
            }

            // Step 2: Detect breakdown (price breaks below MA25 after being above)
            if self.was_above_ma && candle.close < ma {
                self.had_breakdown = true;
                self.was_above_ma = false;  // Reset above state
            }

            // Step 3: Entry trigger - Retest rejection
            // After breakdown, price touches MA25 (high >= MA) but closes below (rejection)
            if self.had_breakdown && candle.high >= ma && candle.close < ma {
                // Log entry for debugging
                tracing::info!(
                    "RETEST ENTRY: time={:?}, high={:.2}, low={:.2}, close={:.2}, ma25={:.2}",
                    candle.open_time,
                    candle.high,
                    candle.low,
                    candle.close,
                    ma
                );
                // Open Short with 1/4 account size
                let amount = equity * 0.25;
                self.had_breakdown = false;  // Reset after entry
                return Signal::Open(Side::Short, amount);
            }

            // Reset if price goes clearly above MA25 again (invalidates breakdown)
            if candle.low > ma {
                self.had_breakdown = false;
            }
        } else if let Some(pos) = position {
            let _price_change_pct =
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
