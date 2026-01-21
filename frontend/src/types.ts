export interface Trade {
  symbol: string;
  side: 'Long' | 'Short';
  entry_price: number;
  exit_price: number;
  quantity: number;
  profit_pct: number;
  profit_abs: number;
  entry_time: string;
  exit_time: string;
  exit_reason: string;
}

export interface BacktestStatus {
  Running?: number;
  Completed?: Trade[];
  Failed?: string;
}

export interface ProgressUpdate {
    id: string;
    progress: number;
    status: string;
}
