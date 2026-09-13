<script lang="ts">
  import type { Product, PriceHistory } from '$lib/types';
  import { formatPrice, timeAgo } from '$lib/utils';
  import { computePriceSignal } from '$lib/utils/signal';
  import { t } from '$lib/i18n';
  import BuySignalBadge from './BuySignalBadge.svelte';

  interface Props { product: Product; history?: PriceHistory[]; }
  let { product, history }: Props = $props();

  const bestOffer = $derived(
    product.offers.length > 0
      ? product.offers.reduce((best, offer) =>
          offer.current_price > 0 && (best.current_price === 0 || offer.current_price < best.current_price) ? offer : best)
      : null
  );

  const hasPrice = $derived(bestOffer && bestOffer.current_price > 0);
  const isPending = $derived(product.name === 'Pending');
  const signal = $derived(
    bestOffer && bestOffer.current_price > 0
      ? computePriceSignal(history, bestOffer.current_price)
      : null
  );
</script>

<a
  href="/products/{product.id}"
  class="card-interactive group flex items-center gap-3.5 p-3.5"
>
  <div class="flex-shrink-0 w-12 h-12 rounded-[var(--radius-md)] bg-[var(--bg-subtle)] border border-[var(--border)] overflow-hidden flex items-center justify-center">
    {#if product.image_url}
      <img src={product.image_url} alt={product.name} class="w-full h-full object-contain p-1" loading="lazy" />
    {:else}
      <svg class="w-5 h-5 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
      </svg>
    {/if}
  </div>

  <div class="flex-1 min-w-0">
    {#if product.tags && product.tags.length > 0}
      <div class="flex flex-wrap gap-1.5 mb-1">
        {#each product.tags.slice(0, 2) as tag}
          <span class="inline-flex items-center gap-1 text-[10px] font-medium text-[var(--fg-muted)]">
            <span class="w-1.5 h-1.5 rounded-full" style="background-color: {tag.color}"></span>
            {tag.name}
          </span>
        {/each}
      </div>
    {/if}

    <h3 class="font-medium text-[14px] text-[var(--fg)] truncate leading-snug group-hover:text-[var(--accent)] transition-colors duration-150">
      {isPending ? $t('table.pending') : product.name}
    </h3>

    <div class="flex items-center gap-1.5 mt-1 min-w-0">
      <p class="text-[12px] text-[var(--fg-muted)] truncate">
        {bestOffer ? (bestOffer.store_name || $t('table.storeFallback')) : $t('table.noOffer')} · {timeAgo(product.updated_at)}
      </p>
      {#if signal && signal.signal !== 'fair'}
        <BuySignalBadge signal={signal.signal} size="xs" reason={signal.reason} />
      {/if}
    </div>
  </div>

  <div class="flex-shrink-0 text-right">
    {#if hasPrice}
      <div class="text-[18px] price-number leading-none">
        {formatPrice(bestOffer.current_price, bestOffer.currency)}
      </div>
      {#if signal && signal.savingsVsAvg > 0}
        <div class="num text-[11px] text-[var(--success)] mt-1">-{formatPrice(signal.savingsVsAvg, bestOffer.currency)}</div>
      {/if}
    {:else if isPending}
      <span class="text-[12px] text-[var(--fg-muted)]">{$t('table.pendingShort')}</span>
    {:else}
      <span class="text-[12px] text-[var(--fg-muted)]">{$t('table.noPrice')}</span>
    {/if}
  </div>
</a>
