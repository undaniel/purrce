<script lang="ts">
  import type { PriceHistory } from '$lib/types';
  import { formatPrice, formatDateTime, calculateDelta, getChangeColor, getChangeIcon } from '$lib/utils';
  import { t } from '$lib/i18n';

  interface Props {
    history: PriceHistory[];
    currency?: string;
  }

  let { history, currency = 'CLP' }: Props = $props();

  const sortedHistory = $derived(
    [...history].sort((a, b) => new Date(b.observed_at).getTime() - new Date(a.observed_at).getTime())
  );

  const priceChange = $derived.by(() => {
    if (sortedHistory.length < 2) return null;
    const current = sortedHistory[0].price;
    const previous = sortedHistory[1].price;
    if (previous === 0) return null;
    return ((current - previous) / previous) * 100;
  });
</script>

<div class="card p-4 sm:p-5">
  <div class="flex items-center justify-between mb-4">
    <h3 class="title-sm">{$t('history.title')}</h3>
    {#if priceChange !== null && priceChange !== 0}
      <span class="badge {priceChange < 0 ? 'badge-down' : 'badge-up'}">
        {priceChange > 0 ? '↑' : '↓'} {Math.abs(priceChange).toFixed(1)}%
      </span>
    {/if}
  </div>
  {#if sortedHistory.length === 0}
    <p class="text-[13px] text-[var(--fg-muted)] py-6 text-center">{$t('history.empty')}</p>
  {:else}
    <div>
      {#each sortedHistory as entry, i}
        {@const previousEntry = i < sortedHistory.length - 1 ? sortedHistory[i + 1] : null}
        {@const delta = previousEntry ? calculateDelta(entry.price, previousEntry.price) : null}

        <div class="flex items-center justify-between py-2.5 {i < sortedHistory.length - 1 ? 'border-b border-[var(--border-subtle)]' : ''}">
          <span class="text-[12px] text-[var(--fg-muted)]">{formatDateTime(entry.observed_at)}</span>
          <div class="flex items-center gap-3">
            {#if delta && delta.absolute !== 0}
              <span class="num text-[11px] font-medium {getChangeColor(delta.percent)}">
                {getChangeIcon(delta.percent)} {formatPrice(Math.abs(delta.absolute), currency)}
              </span>
            {/if}
            <span class="num text-[13px] font-semibold">{formatPrice(entry.price, currency)}</span>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
