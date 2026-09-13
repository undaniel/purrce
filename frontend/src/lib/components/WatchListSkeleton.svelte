<script lang="ts">
  import Skeleton from './Skeleton.svelte';

  interface Props {
    rows?: number;
  }

  let { rows = 6 }: Props = $props();
</script>

<div class="space-y-2">
  {#each Array(rows) as _, i}
    <div class="card p-3.5">
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-3 min-w-0 flex-1">
          <Skeleton width="8px" height="8px" rounded="9999px" />
          <div class="min-w-0 flex-1 space-y-2">
            <Skeleton width={`${45 + (i % 4) * 10}%`} height="13px" />
            <Skeleton width="35%" height="10px" />
          </div>
        </div>
        <div class="flex items-center gap-2 flex-shrink-0">
          <Skeleton width="64px" height="24px" rounded="var(--radius-sm)" />
          <Skeleton width="72px" height="24px" rounded="var(--radius-md)" />
          <Skeleton width="28px" height="28px" rounded="var(--radius-md)" />
        </div>
      </div>
    </div>
  {/each}
</div>
