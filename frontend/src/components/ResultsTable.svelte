<script lang="ts">
  import type { Trade } from "../types";

  export let trades: Trade[] = [];

  // Calculate stats
  $: totalTrades = trades.length;
  $: winRate =
    totalTrades > 0
      ? (trades.filter((t) => t.profit_pct > 0).length / totalTrades) * 100
      : 0;
  $: totalProfit = trades.reduce((acc, t) => acc + t.profit_abs, 0);

  let selectedChartUrl: string | null = null;
  const API_BASE = "http://localhost:3100/api";

  function viewChart(trade: Trade) {
    const timestamp = Math.floor(new Date(trade.entry_time).getTime() / 1000);
    selectedChartUrl = `${API_BASE}/chart?symbol=${trade.symbol}&timestamp=${timestamp}`;
  }

  function closeChart() {
    selectedChartUrl = null;
  }
</script>

<div class="card bg-base-100 shadow-xl p-4 mt-4 relative">
  <!-- Modal for Chart -->
  {#if selectedChartUrl}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50"
      on:click={closeChart}
    >
      <div
        class="bg-white p-4 rounded-lg shadow-xl max-w-5xl w-full"
        on:click|stopPropagation
      >
        <div class="flex justify-between items-center mb-2">
          <h3 class="text-lg font-bold">Trade Chart</h3>
          <button class="btn btn-sm btn-circle" on:click={closeChart}>✕</button>
        </div>
        <div class="flex justify-center">
          <img
            src={selectedChartUrl}
            alt="Trade Chart"
            class="max-h-[80vh] object-contain border rounded"
          />
        </div>
      </div>
    </div>
  {/if}

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
          <th>Chart</th>
        </tr>
      </thead>
      <tbody>
        {#each trades as trade}
          <tr>
            <td>{trade.symbol}</td>
            <td>
              <span
                class="badge {trade.side === 'Long'
                  ? 'badge-success'
                  : 'badge-error'}"
              >
                {trade.side}
              </span>
            </td>
            <td>{new Date(trade.entry_time).toLocaleString()}</td>
            <td>{new Date(trade.exit_time).toLocaleString()}</td>
            <td>{trade.entry_price.toFixed(2)}</td>
            <td>{trade.exit_price.toFixed(2)}</td>
            <td class={trade.profit_abs >= 0 ? "text-success" : "text-error"}>
              {trade.profit_abs.toFixed(2)}
            </td>
            <td class={trade.profit_pct >= 0 ? "text-success" : "text-error"}>
              {trade.profit_pct.toFixed(2)}%
            </td>
            <td>{trade.exit_reason}</td>
            <td>
              <button
                class="btn btn-xs btn-primary"
                on:click={() => viewChart(trade)}>View</button
              >
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
