<script lang="ts">
  export let imageUrl: string;
  export let title: string;
</script>

<div class="card bg-base-100 shadow-xl p-4 mt-4">
  <h2 class="card-title mb-4">{title}</h2>
  <figure>
    <img src={imageUrl} alt={title} class="w-full h-auto rounded-lg" />
  </figure>
</div>
