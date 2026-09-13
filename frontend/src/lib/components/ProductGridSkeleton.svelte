<script lang="ts">
  import Skeleton from './Skeleton.svelte';

  interface Props {
    cards?: number;
  }

  let { cards = 6 }: Props = $props();
</script>

<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
  {#each Array(cards) as _, i}
    <div class="card flex items-center gap-3.5 p-3.5">
      <Skeleton width="48px" height="48px" rounded="var(--radius-md)" />
      <div class="flex-1 min-w-0 space-y-2">
        <Skeleton width={`${70 + (i % 3) * 8}%`} height="13px" />
        <Skeleton width="55%" height="10px" />
      </div>
      <div class="flex-shrink-0 space-y-2 flex flex-col items-end">
        <Skeleton width="72px" height="16px" rounded="var(--radius-sm)" />
        <Skeleton width="40px" height="10px" rounded="var(--radius-sm)" />
      </div>
    </div>
  {/each}
</div>
