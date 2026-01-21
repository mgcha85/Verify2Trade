<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let symbols: string[] = [];
  
  let selectedSymbol = 'BTCUSDT';
  let startDate = '2023-01-01';
  let endDate = '2023-12-31';
  let initialCapital = 10000;

  const dispatch = createEventDispatcher();

  function startBacktest() {
    dispatch('start', {
      symbol: selectedSymbol,
      start_date: new Date(startDate).toISOString(),
      end_date: new Date(endDate).toISOString(),
      initial_capital: initialCapital
    });
  }
</script>

<div class="card bg-base-100 shadow-xl p-4">
  <h2 class="card-title mb-4">Backtest Configuration</h2>
  
  <div class="form-control w-full max-w-xs mb-2">
    <label class="label" for="symbol-select">
      <span class="label-text">Symbol</span>
    </label>
    <select id="symbol-select" class="select select-bordered" bind:value={selectedSymbol}>
      {#each symbols as symbol}
        <option value={symbol}>{symbol}</option>
      {/each}
    </select>
  </div>

  <div class="form-control w-full max-w-xs mb-2">
    <label class="label" for="start-date">
      <span class="label-text">Start Date</span>
    </label>
    <input id="start-date" type="date" class="input input-bordered" bind:value={startDate} />
  </div>

  <div class="form-control w-full max-w-xs mb-2">
    <label class="label" for="end-date">
      <span class="label-text">End Date</span>
    </label>
    <input id="end-date" type="date" class="input input-bordered" bind:value={endDate} />
  </div>

  <div class="form-control w-full max-w-xs mb-4">
    <label class="label" for="capital">
      <span class="label-text">Initial Capital ($)</span>
    </label>
    <input id="capital" type="number" class="input input-bordered" bind:value={initialCapital} />
  </div>

  <div class="card-actions justify-end">
    <button class="btn btn-primary" on:click={startBacktest}>Run Backtest</button>
  </div>
</div>
