<script lang="ts">
  import type { Trade } from '../types';

  export let trades: Trade[] = [];
  
  // Calculate stats
  $: totalTrades = trades.length;
  $: winRate = totalTrades > 0 
    ? (trades.filter(t => t.profit_pct > 0).length / totalTrades) * 100 
    : 0;
  $: totalProfit = trades.reduce((acc, t) => acc + t.profit_abs, 0);
</script>

<div class="card bg-base-100 shadow-xl p-4 mt-4">
  <div class="flex justify-between items-center mb-4">
    <h2 class="card-title">Backtest Results</h2>
    <div class="stats shadow">
      <div class="stat">
        <div class="stat-title">Total Trades</div>
        <div class="stat-value">{totalTrades}</div>
      </div>
      <div class="stat">
        <div class="stat-title">Win Rate</div>
        <div class="stat-value">{winRate.toFixed(1)}%</div>
      </div>
      <div class="stat">
        <div class="stat-title">Total Profit</div>
        <div class="stat-value text-primary">${totalProfit.toFixed(2)}</div>
      </div>
    </div>
  </div>

  <div class="overflow-x-auto">
    <table class="table table-zebra w-full">
      <thead>
        <tr>
          <th>Symbol</th>
          <th>Side</th>
          <th>Entry Time</th>
          <th>Exit Time</th>
          <th>Entry Price</th>
          <th>Exit Price</th>
          <th>Profit ($)</th>
          <th>Profit (%)</th>
          <th>Reason</th>
        </tr>
      </thead>
      <tbody>
        {#each trades as trade}
          <tr>
            <td>{trade.symbol}</td>
            <td>
              <span class="badge {trade.side === 'Long' ? 'badge-success' : 'badge-error'}">
                {trade.side}
              </span>
            </td>
            <td>{new Date(trade.entry_time).toLocaleString()}</td>
            <td>{new Date(trade.exit_time).toLocaleString()}</td>
            <td>{trade.entry_price.toFixed(2)}</td>
            <td>{trade.exit_price.toFixed(2)}</td>
            <td class={trade.profit_abs >= 0 ? 'text-success' : 'text-error'}>
              {trade.profit_abs.toFixed(2)}
            </td>
            <td class={trade.profit_pct >= 0 ? 'text-success' : 'text-error'}>
              {trade.profit_pct.toFixed(2)}%
            </td>
            <td>{trade.exit_reason}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
