<script lang="ts">
  import BacktestConfig from './components/BacktestConfig.svelte';
  import ProgressBar from './components/ProgressBar.svelte';
  import ResultsTable from './components/ResultsTable.svelte';
  import type { Trade } from './types';

  let symbols: string[] = ['BTCUSDT']; // Should fetch from API
  let backtestId: string | null = null;
  let progress: number = 0;
  let statusMsg: string = '';
  let trades: Trade[] = [];
  let isRunning = false;

  const API_BASE = 'http://localhost:3100/api';

  async function handleStart(event: CustomEvent) {
    const params = event.detail;
    
    try {
      isRunning = true;
      trades = [];
      progress = 0;
      statusMsg = 'Starting...';

      const res = await fetch(`${API_BASE}/backtest/run`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(params)
      });
      
      const data = await res.json();
      backtestId = data.backtest_id;
      
      if (backtestId) {
        subscribeToProgress(backtestId);
      }
    } catch (e) {
      console.error(e);
      statusMsg = 'Failed to start backtest';
      isRunning = false;
    }
  }

  function subscribeToProgress(id: string) {
    const eventSource = new EventSource(`${API_BASE}/backtest/progress/${id}`);
    
    eventSource.onmessage = (event) => {
      const data = JSON.parse(event.data);
      progress = data.progress;
      statusMsg = data.status;
      
      if (data.status === 'Completed' || data.status.startsWith('Failed')) {
        eventSource.close();
        isRunning = false;
        if (data.status === 'Completed') {
          fetchResults(id);
        }
      }
    };
    
    eventSource.onerror = () => {
        eventSource.close();
        isRunning = false;
    }
  }

  async function fetchResults(id: string) {
    try {
      const res = await fetch(`${API_BASE}/backtest/result/${id}`);
      const data = await res.json();
      if (data.Completed) {
        trades = data.Completed;
      }
    } catch (e) {
      console.error(e);
    }
  }
</script>

<main class="container mx-auto p-4">
  <h1 class="text-3xl font-bold mb-8 text-center">Verify2Trade Backtester</h1>
  
  <div class="grid grid-cols-1 gap-6">
    <BacktestConfig {symbols} on:start={handleStart} />
    
    {#if isRunning || progress > 0}
      <div class="card bg-base-100 shadow-xl p-4">
        <ProgressBar {progress} status={statusMsg} />
      </div>
    {/if}

    {#if trades.length > 0}
      <ResultsTable {trades} />
    {/if}
  </div>
</main>
