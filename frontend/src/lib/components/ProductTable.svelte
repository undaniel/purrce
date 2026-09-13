<script lang="ts">
  import { t } from '$lib/i18n';
  import type { Product, PriceHistory } from '$lib/types';
  import { formatPrice, timeAgo, titleCase, currencyLabel } from '$lib/utils';
  import { computePriceSignal } from '$lib/utils/signal';
  import Sparkline from './Sparkline.svelte';
  import BuySignalBadge from './BuySignalBadge.svelte';

  interface Props {
    products: Product[];
    priceHistories?: Record<string, PriceHistory[]>;
    onDelete?: (product: Product) => void;
    deletingId?: string | null;
    showCurrency?: boolean;
  }

  let { products, priceHistories = {}, onDelete, deletingId, showCurrency = false }: Props = $props();

  function getBestOffer(product: Product) {
    if (product.offers.length === 0) return null;
    return product.offers.reduce((best, offer) =>
      offer.current_price > 0 && (best.current_price === 0 || offer.current_price < best.current_price) ? offer : best
    );
  }

  function getPriceChange(product: Product): number | null {
    const bestOffer = getBestOffer(product);
    if (!bestOffer) return null;
    const history = priceHistories[bestOffer.id];
    if (!history || history.length < 2) return null;
    const sorted = [...history].sort((a, b) => new Date(b.observed_at).getTime() - new Date(a.observed_at).getTime());
    const current = sorted[0].price;
    const previous = sorted[1].price;
    if (previous === 0) return null;
    return ((current - previous) / previous) * 100;
  }

  function getSparklineData(product: Product): number[] {
    const bestOffer = getBestOffer(product);
    if (!bestOffer) return [];
    const history = priceHistories[bestOffer.id];
    if (!history || history.length < 2) return [];
    const sorted = [...history].sort((a, b) => new Date(a.observed_at).getTime() - new Date(b.observed_at).getTime());
    return sorted.map(h => h.price);
  }

  function getSignal(product: Product) {
    const bestOffer = getBestOffer(product);
    if (!bestOffer || bestOffer.current_price <= 0) return null;
    return computePriceSignal(priceHistories[bestOffer.id], bestOffer.current_price);
  }

  function hasMixedCurrency(product: Product): boolean {
    return new Set(product.offers.map(o => currencyLabel(o.currency))).size > 1;
  }

  const showTrend = $derived(products.some(product => {
    const bestOffer = getBestOffer(product);
    return bestOffer ? (priceHistories[bestOffer.id]?.length ?? 0) >= 2 : false;
  }));
</script>

<div class="card overflow-hidden">
  <div class="overflow-x-auto">
    <table class="w-full min-w-[860px]" style="table-layout: fixed;">
      <thead>
        <tr class="border-b border-[var(--border-subtle)]">
          <th class="table-head text-left px-4 py-3.5" style="width: {showTrend ? '34%' : '46%'};">{$t('table.product')}</th>
          <th class="table-head text-left px-4 py-3.5 hidden sm:table-cell" style="width: {showTrend ? '14%' : '16%'};">{$t('table.store')}</th>
          <th class="table-head text-right px-4 py-3.5" style="width: {showTrend ? '16%' : '22%'};">{$t('table.price')}</th>
          {#if showTrend}
            <th class="table-head text-right px-4 py-3.5 hidden md:table-cell" style="width: 11%;">{$t('table.change')}</th>
            <th class="table-head text-right px-4 py-3.5 hidden lg:table-cell" style="width: 12%;">{$t('table.trend')}</th>
          {/if}
          <th class="table-head text-right px-4 py-3.5" style="width: {showTrend ? '13%' : '16%'};">{$t('table.time')}</th>
        </tr>
      </thead>
      <tbody>
        {#each products as product (product.id)}
          {@const bestOffer = getBestOffer(product)}
          {@const change = getPriceChange(product)}
          {@const sparklineData = getSparklineData(product)}
          {@const isPending = product.name === 'Pending'}
          {@const signal = getSignal(product)}
          {@const mixedCurrency = hasMixedCurrency(product)}

          <tr class="table-row-hover border-b border-[var(--border-subtle)] last:border-b-0 group">
            <td class="px-4 py-3">
              <a href="/products/{product.id}" class="flex items-start gap-3 min-w-0">
                <div class="flex-shrink-0 w-9 h-9 rounded-[var(--radius-md)] overflow-hidden bg-[var(--bg-subtle)] border border-[var(--border)] flex items-center justify-center">
                  {#if product.image_url}
                    <img src={product.image_url} alt={product.name} class="w-full h-full object-contain p-0.5" loading="lazy" />
                  {:else}
                    <svg class="w-4 h-4 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
                    </svg>
                  {/if}
                </div>
                <div class="min-w-0">
                  <div class="flex items-start gap-2 min-w-0">
                    <p class="text-[14px] font-medium text-[var(--fg)] line-clamp-2 leading-snug group-hover:text-[var(--accent)] transition-colors duration-150">
                      {isPending ? $t('table.pending') : product.name}
                    </p>
                    {#if signal && signal.signal !== 'fair'}
                      <BuySignalBadge signal={signal.signal} label={signal.isHistoricalLow ? $t('table.historicalLow') : signal.label} reason={signal.reason} />
                    {/if}
                  </div>
                  {#if product.tags && product.tags.length > 0}
                    <div class="flex flex-wrap items-center gap-1.5 mt-1.5">
                      {#each product.tags.slice(0, 2) as tag}
                        <span class="inline-flex items-center gap-1 text-[11px] font-medium text-[var(--fg-secondary)] bg-[var(--bg-subtle)] border border-[var(--border)] rounded-full px-2 py-0.5">
                          <span class="w-1.5 h-1.5 rounded-full flex-shrink-0" style="background-color: {tag.color}"></span>
                          {tag.name}
                        </span>
                      {/each}
                      {#if product.tags.length > 2}
                        <span class="text-[11px] text-[var(--fg-muted)]" title={product.tags.slice(2).map(t => t.name).join(', ')}>+{product.tags.length - 2}</span>
                      {/if}
                    </div>
                  {/if}
                </div>
              </a>
            </td>

            <td class="px-4 py-3 hidden sm:table-cell">
              <span class="text-[13px] text-[var(--fg-muted)]">
                {bestOffer ? titleCase(bestOffer.store_name) || $t('table.storeFallback') : '—'}
              </span>
            </td>

            <td class="px-4 py-3 text-right">
              {#if bestOffer && bestOffer.current_price > 0}
                <span class="inline-flex items-center justify-end gap-1.5 whitespace-nowrap">
                  {#if mixedCurrency}
                    <span class="text-[var(--warning)]" title={$t('table.mixedCurrency')}>
                      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v4m0 4h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
                      </svg>
                    </span>
                  {/if}
                  <span class="text-[15px] price-cell">{formatPrice(bestOffer.current_price, bestOffer.currency)}</span>
                  {#if showCurrency}
                    <span class="text-[10px] font-medium text-[var(--fg-muted)] align-top">{currencyLabel(bestOffer.currency)}</span>
                  {/if}
                </span>
              {:else}
                <span class="text-[12px] text-[var(--fg-muted)]">—</span>
              {/if}
            </td>

            {#if showTrend}
              <td class="px-4 py-3 text-right hidden md:table-cell">
                {#if change !== null}
                  {#if change === 0}
                    <span class="badge badge-neutral">0.0%</span>
                  {:else}
                    <span class="badge {change < 0 ? 'badge-down' : 'badge-up'}">
                      {change < 0 ? '▼' : '▲'} {Math.abs(change).toFixed(1)}%
                    </span>
                  {/if}
                {:else}
                  <span class="text-[10px] text-[var(--fg-muted)] whitespace-nowrap" title={$t('table.notEnoughRecords')}>—</span>
                {/if}
              </td>

              <td class="px-4 py-3 text-right hidden lg:table-cell">
                {#if sparklineData.length >= 2}
                  <Sparkline data={sparklineData} width={56} height={20} />
                {:else}
                  <span class="text-[11px] text-[var(--fg-muted)]" title={$t('table.notEnoughRecords')}>—</span>
                {/if}
              </td>
            {/if}

            <td class="px-4 py-3 text-right">
              <div class="flex items-center justify-end gap-2">
                <span class="text-[12px] text-[var(--fg-muted)] whitespace-nowrap">{timeAgo(product.updated_at)}</span>
                {#if onDelete}
                  <button
                    onclick={(e) => { e.preventDefault(); onDelete(product); }}
                    disabled={deletingId === product.id}
                    class="p-1 rounded-[var(--radius-sm)] opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity text-[var(--fg-muted)] hover:bg-[var(--danger-subtle)] hover:text-[var(--danger)] disabled:opacity-50"
                    title={$t('table.delete')}
                  >
                    {#if deletingId === product.id}
                      <svg class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                      </svg>
                    {:else}
                      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                      </svg>
                    {/if}
                  </button>
                {/if}
              </div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
