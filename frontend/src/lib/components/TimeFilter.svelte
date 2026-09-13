<script lang="ts">
  export type TimeRange = '7d' | '30d' | '3m' | '6m' | '1y' | 'all';

  import { t } from '$lib/i18n';

  interface Props {
    selected: TimeRange;
    onchange: (range: TimeRange) => void;
  }

  let { selected, onchange }: Props = $props();

  const ranges: { value: TimeRange; label: string }[] = $derived([
    { value: '7d' as TimeRange, label: '7d' },
    { value: '30d' as TimeRange, label: '30d' },
    { value: '3m' as TimeRange, label: '3m' },
    { value: '6m' as TimeRange, label: '6m' },
    { value: '1y' as TimeRange, label: $t('timefilter.1y') },
    { value: 'all' as TimeRange, label: $t('timefilter.all') },
  ]);
</script>

<div class="flex items-center gap-0.5 p-0.5 bg-[var(--surface-container)] rounded-full">
  {#each ranges as range}
    <button
      onclick={() => onchange(range.value)}
      class="num px-2.5 py-1 text-[11px] font-semibold rounded-full transition-colors duration-200
        {selected === range.value
          ? 'bg-[var(--accent-subtle)] text-[var(--accent)]'
          : 'text-[var(--fg-muted)] hover:text-[var(--fg-secondary)]'}"
    >
      {range.label}
    </button>
  {/each}
</div>
