<script lang="ts">
  export let progress: number = 0; // 0 to 1
  export let status: string = '';
</script>

<div class="w-full">
  <div class="flex justify-between mb-1">
    <span class="text-base font-medium text-blue-700 dark:text-white">Backtest Progress</span>
    <span class="text-sm font-medium text-blue-700 dark:text-white">{Math.round(progress * 100)}%</span>
  </div>
  <div class="w-full bg-gray-200 rounded-full h-2.5 dark:bg-gray-700">
    <div class="bg-blue-600 h-2.5 rounded-full" style="width: {progress * 100}%"></div>
  </div>
  <p class="text-sm text-gray-500 mt-1">{status}</p>
</div>
