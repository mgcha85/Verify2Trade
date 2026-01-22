# MA Touch Resistance Trading Strategy

## Overview

This is a **Short-only** trading strategy that enters positions when the price touches the 25-period moving average from below and gets rejected (forms a bearish candle), indicating resistance at the MA level.

## Strategy Algorithm

### Entry Conditions (Short Position)

The strategy enters a SHORT position when ALL of the following conditions are met on **1-hour candles**:

1. **Price was below MA25**: `candle.close < MA25` at some recent point
2. **Candle touches MA25**: `candle.high >= MA25` (price reached up to the MA)
3. **Rejection (Bearish Close)**: `candle.close < MA25` (price closes below the MA, confirming rejection)

**Implementation**:
```rust
// State tracking
if candle.close < ma {
    price_was_below_ma = true;  // Price is below MA
} else if candle.low > ma {
    price_was_below_ma = false;  // Price is clearly above MA
}

// Entry trigger
if price_was_below_ma && candle.high >= ma && candle.close < ma {
    // Open SHORT with 25% of account equity
    Signal::Open(Side::Short, equity * 0.25)
}
```

### Position Sizing

- **Initial Entry**: 25% of total account equity
- **Maximum**: 2 entries (initial + 1 pyramiding position)

### Pyramiding (Adding to Position)

**Condition**: When price rises 2% above the first entry price (averaging down for shorts)

```rust
if position.entries.len() < 2 && candle.close >= first_entry_price * 1.02 {
    // Add same quantity as first entry
    Signal::AddToPosition(first_entry_quantity * current_price)
}
```

### Exit Conditions

#### 1. Stop Loss (SL)
- **Initial SL**: Average entry price + 2%
- **Breakeven SL**: After partial profit is taken, SL moves to breakeven (average entry price)

```rust
if candle.close >= average_entry_price * 1.02 {
    Signal::Close("SL")
}

// After partial profit taken
if partial_profit_taken && candle.close >= average_entry_price {
    Signal::Close("SL_Breakeven")
}
```

#### 2. Take Profit (TP)

**Profit Calculation for Short**: When price decreases from entry
```
profit_pct = (average_entry_price - current_price) / average_entry_price
```

- **Partial TP (50%)**: Close half position at 1% profit
- **Full TP**: Close entire position at 3% profit

```rust
// Take 50% profit at 1%
if profit_pct >= 0.01 && !partial_profit_taken {
    partial_profit_taken = true;
    Signal::PartialClose(0.5, "TP_HALF")
}

// Full exit at 3%
if profit_pct >= 0.03 {
    Signal::Close("TP_MAX")
}
```

## Chart Visualization

### Chart Configuration

Each trade generates a stacked chart with two panels:

#### Top Panel: 5-Minute Candles (Light Blue Background)
- **Candles**: Most recent 200 5-minute candles before entry
- **Entry Point**: Black vertical line at the rightmost edge
- **Moving Averages**: Pre-calculated MA25, 50, 200, 400
  - MA25: <span style="color:red">**Red**</span>
  - MA50: <span style="color:green">**Green**</span>
  - MA200: <span style="color:blue">**Blue**</span>  
  - MA400: <span style="color:gray">**Gray**</span>

#### Bottom Panel: Daily Candles (Light Yellow Background)
- **Candles**: Most recent 200 daily candles before entry
- **Entry Point**: Black vertical line at the rightmost edge
- **Moving Averages**: Same color scheme as 5-minute chart

### Important Notes

- **Entry time is at the RIGHT EDGE**: The rightmost point of both charts represents the exact moment the SHORT position was opened
- **Only historical data shown**: Charts display ONLY data from before or at the entry time (no future data)
- **MA Pre-calculation**: Moving averages are calculated once during data loading using `rolling_mean`, not re-calculated during chart rendering for optimal performance

## Data Flow

1. **Data Loading** (`api.rs`):
   - Load 400 days of 1-minute candle data before entry time
   - Resample to 5-minute and daily timeframes
   - Calculate MA25, 50, 200, 400 using `rolling_mean` and store as DataFrame columns

2. **Chart Generation** (`charting.rs`):
   - Extract candle data and pre-calculated MA values from DataFrame
   - Filter to show only last 200 candles before entry time
   - Draw candlesticks and MA lines
   - Mark entry time with vertical black line at right edge

3. **Strategy Execution** (`ma_touch.rs`):
   - Iterate through 1-hour candles chronologically
   - Track price position relative to MA25
   - Detect MA touch + rejection pattern
   - Execute SHORT entry with 25% position size

## File Structure

```
backend/
├── src/
│   ├── strategy/
│   │   └── ma_touch.rs       # Strategy algorithm implementation
│   ├── api.rs                # Chart API endpoint, MA pre-calculation
│   └── charting.rs           # Chart rendering with pre-calculated MAs
```

## Configuration

Located in [`config.yaml`](file:///mnt/data/projects/Verify2Trade/config.yaml):
- Symbol to trade
- Initial capital
- Data path for historical candles

## Validation

The strategy requires at least 400 candles of historical data at chart generation time to ensure MA400 can be calculated accurately.

**Entry Validation on Charts**:
The rightmost candle (at the black vertical line) should show:
- High that touched or exceeded MA25 (red line)
- Close that is below MA25 (bearish rejection)
- This creates a wick/shadow above the candle body touching the red MA25 line
