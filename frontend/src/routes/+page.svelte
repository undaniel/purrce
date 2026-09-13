<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import type { Product, PriceHistory, ProductStats, Watch, Offer } from '$lib/types';
  import { computePriceSignal } from '$lib/utils/signal';
  import ProductTable from '$lib/components/ProductTable.svelte';
  import TableSkeleton from '$lib/components/TableSkeleton.svelte';
  import AddProductDialog from '$lib/components/AddProductDialog.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import ErrorState from '$lib/components/ErrorState.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import Sparkline from '$lib/components/Sparkline.svelte';
  import { timeAgo, formatPrice, formatDateTime, titleCase, currencyLabel } from '$lib/utils';
  import { t, translate, currentLocale } from '$lib/i18n';

  let products: Product[] = $state([]);
  let priceHistories: Record<string, PriceHistory[]> = $state({});
  let watches: Watch[] = $state([]);
  let initialLoading = $state(true);
  let error = $state('');
  let showAddDialog = $state(false);
  let deletingId = $state<string | null>(null);
  let showDeleteConfirm = $state(false);
  let productToDelete = $state<Product | null>(null);
  let currentPage = $state(1);
  let totalPages = $state(1);
  let refreshing = $state(false);
  const pageSize = 8;

  let stats: ProductStats = $state({ total: 0, down: 0, up: 0, unchanged: 0, at_min: 0, no_price: 0, values: [] });

  const atMinPct = $derived(stats.total > 0 ? (stats.at_min / stats.total) * 100 : 0);
  const noPricePct = $derived(stats.total > 0 ? (stats.no_price / stats.total) * 100 : 0);
  const restPct = $derived(Math.max(0, 100 - atMinPct - noPricePct));
  const restCount = $derived(Math.max(0, stats.total - stats.at_min - stats.no_price));

  const watchHealth = $derived.by(() => {
    const total = watches.length;
    const error = watches.filter(w => ['ERROR', 'BLOCKED', 'CAPTCHA_REQUIRED'].includes(w.status)).length;
    const active = watches.filter(w => w.enabled && ['ACTIVE', 'CHECKING'].includes(w.status)).length;
    const paused = watches.filter(w => !w.enabled).length;
    return { total, error, active, paused };
  });

  const lastUpdatedAt = $derived.by(() => {
    if (products.length === 0) return null;
    const dates = products.map(p => new Date(p.updated_at).getTime());
    return new Date(Math.max(...dates)).toISOString();
  });

  const hasMixedCurrencies = $derived(
    (stats.values?.length ?? 0) > 1 ||
    new Set(products.flatMap(p => p.offers.map(o => currencyLabel(o.currency)))).size > 1
  );

  function getBestOffer(product: Product) {
    if (product.offers.length === 0) return null;
    return product.offers.reduce((best, offer) =>
      offer.current_price > 0 && (best.current_price === 0 || offer.current_price < best.current_price) ? offer : best
    );
  }

  function getSparklineData(product: Product): number[] {
    const offer = getBestOffer(product);
    if (!offer) return [];
    const history = priceHistories[offer.id];
    if (!history || history.length < 2) return [];
    return [...history].sort((a, b) => new Date(a.observed_at).getTime() - new Date(b.observed_at).getTime()).map(h => h.price);
  }

  function historyCount(product: Product): number {
    const offer = getBestOffer(product);
    if (!offer) return 0;
    return priceHistories[offer.id]?.length ?? 0;
  }

  // Suficientes muestras para detectar mínimos y tendencias (mínimo 3 registros)
  const historyReadyCount = $derived(products.filter(p => historyCount(p) >= 3).length);
  const hasAnyHistory = $derived(historyReadyCount > 0);
  const hasTrendData = $derived(products.some(p => historyCount(p) >= 2));

  // Productos con señal de compra (mínimo histórico o cerca de él)
  const opportunities = $derived.by(() => {
    return products.filter(p => {
      const offer = getBestOffer(p);
      if (!offer || offer.current_price <= 0) return false;
      const signal = computePriceSignal(priceHistories[offer.id], offer.current_price);
      return signal?.signal === 'buy';
    });
  });

  // Ahorro potencial agrupado por moneda (nunca se suman monedas distintas)
  const opportunitySavings = $derived.by(() => {
    const totals = new Map<string, number>();
    for (const p of opportunities) {
      const offer = getBestOffer(p);
      if (!offer) continue;
      const signal = computePriceSignal(priceHistories[offer.id], offer.current_price);
      if (signal && signal.savingsVsAvg > 0) {
        const currency = currencyLabel(offer.currency);
        totals.set(currency, (totals.get(currency) ?? 0) + signal.savingsVsAvg);
      }
    }
    return [...totals.entries()].map(([currency, total]) => ({ currency, total }));
  });

  // Últimos cambios de precio detectados
  const recentChanges = $derived.by(() => {
    const changes: {
      product: Product;
      offer: NonNullable<ReturnType<typeof getBestOffer>>;
      from: number;
      to: number;
      pct: number;
      at: string;
    }[] = [];

    for (const product of products) {
      const offer = getBestOffer(product);
      if (!offer) continue;
      const history = priceHistories[offer.id];
      if (!history || history.length < 2) continue;
      const sorted = [...history].sort((a, b) => new Date(b.observed_at).getTime() - new Date(a.observed_at).getTime());
      const current = sorted[0];
      const previous = sorted[1];
      if (!previous || previous.price === 0 || current.price === previous.price) continue;
      changes.push({
        product,
        offer,
        from: previous.price,
        to: current.price,
        pct: ((current.price - previous.price) / previous.price) * 100,
        at: current.observed_at
      });
    }

    return changes.sort((a, b) => new Date(b.at).getTime() - new Date(a.at).getTime()).slice(0, 6);
  });

  onMount(async () => {
    await Promise.all([loadProducts(true), loadStats(), loadWatches()]);
  });

  async function loadStats() {
    try { stats = await api.products.stats(); } catch {}
  }

  async function loadWatches() {
    try { watches = (await api.watches.list(1, 100)).data ?? []; } catch {}
  }

  async function loadProducts(initial = false) {
    if (initial) initialLoading = true;
    error = '';
    try {
      const response = await api.products.list(currentPage, pageSize);
      products = response.data;
      totalPages = response.total_pages;
      await loadPriceHistories();
    } catch (e) {
      error = e instanceof Error ? e.message : translate(currentLocale(), 'dash.loadError');
    } finally {
      initialLoading = false;
    }
  }

  async function loadPriceHistories() {
    const bestOffers = products
      .map(p => getBestOffer(p))
      .filter((o): o is Offer => !!o);
    if (bestOffers.length === 0) {
      priceHistories = {};
      return;
    }
    try {
      const rows = await api.offers.historyBulk(bestOffers.map(o => o.id), 120);
      const grouped: Record<string, PriceHistory[]> = {};
      for (const row of rows) (grouped[row.offer_id] ??= []).push(row);
      priceHistories = grouped;
    } catch {
      priceHistories = {};
    }
  }

  async function handleProductAdded() {
    refreshing = true;
    try {
      await Promise.all([loadProducts(), loadStats()]);
    } finally {
      refreshing = false;
    }
  }

  async function refresh() {
    if (refreshing) return;
    refreshing = true;
    try {
      await Promise.all([loadProducts(), loadStats(), loadWatches()]);
    } finally {
      refreshing = false;
    }
  }

  async function handlePageChange(page: number) {
    currentPage = page;
    refreshing = true;
    try {
      await loadProducts();
    } finally {
      refreshing = false;
    }
  }

  function requestDelete(product: Product) {
    productToDelete = product;
    showDeleteConfirm = true;
  }

  async function confirmDelete() {
    if (!productToDelete) return;
    deletingId = productToDelete.id;
    showDeleteConfirm = false;
    try {
      await api.products.delete(productToDelete.id);
      products = products.filter(p => p.id !== productToDelete!.id);
      await loadStats();
    } catch {
      alert(translate(currentLocale(), 'dash.deleteError'));
    } finally {
      deletingId = null;
      productToDelete = null;
    }
  }

  function cancelDelete() {
    showDeleteConfirm = false;
    productToDelete = null;
  }
</script>

<svelte:head>
  <title>{$t('dash.pageTitle')}</title>
</svelte:head>

<div class="space-y-6 {refreshing ? 'refreshing' : ''}">
  <!-- Header -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div>
      <h1 class="page-title">Dashboard</h1>
      {#if !initialLoading && lastUpdatedAt}
        <p class="text-[13px] text-[var(--fg-muted)] mt-0.5" title={$t('dash.lastUpdateTitle', { when: formatDateTime(lastUpdatedAt) })}>
          {$t('dash.updatedAgo', { ago: timeAgo(lastUpdatedAt, false) })}
        </p>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      <button
        onclick={refresh}
        disabled={refreshing || initialLoading}
        class="btn btn-ghost"
        title={$t('dash.refresh')}
        aria-label={$t('dash.refresh')}
      >
        <svg class="w-4 h-4 {refreshing ? 'animate-spin' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
      </button>
      <button onclick={() => showAddDialog = true} class="btn btn-primary">
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.2" d="M12 4v16m8-8H4" />
        </svg>
        {$t('dash.addProduct')}
      </button>
    </div>
  </div>

  {#if !initialLoading && stats.total > 0}
    {#if !hasAnyHistory}
      <!-- Estado de preparación: aún sin suficiente historial -->
      <div class="card p-5 sm:p-6">
        <div class="flex flex-col sm:flex-row sm:items-start gap-4">
          <div class="w-11 h-11 rounded-full bg-[var(--accent-subtle)] flex items-center justify-center flex-shrink-0">
            <svg class="w-5 h-5 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
          <div class="flex-1 min-w-0">
            <p class="section-label">{$t('dash.preparingData')}</p>
            <h2 class="text-[16px] font-semibold mt-1">{$t('dash.firstPricesTitle')}</h2>
            <p class="text-[13px] text-[var(--fg-muted)] mt-1 max-w-xl">
              {$t('dash.firstPricesBody')}
            </p>
            <div class="flex flex-wrap items-center gap-2 mt-3">
              <span class="badge badge-neutral">{$t('dash.productsMonitored', { count: stats.total })}</span>
              {#if stats.no_price > 0}
                <span class="badge badge-neutral">{$t('dash.noPriceCount', { count: stats.no_price })}</span>
              {/if}
              {#if historyReadyCount > 0}
                <span class="badge badge-accent">{$t('dash.withHistory', { count: historyReadyCount })}</span>
              {/if}
            </div>
          </div>
        </div>
      </div>
    {:else}
      <!-- Hero: oportunidades de compra + composición -->
      <div class="card p-5 sm:p-6">
        <div class="flex flex-col lg:flex-row lg:items-end justify-between gap-6">
          <div class="min-w-0">
            <p class="section-label">{stats.at_min > 0 ? $t('dash.atMin') : $t('dash.noMins')}</p>
            {#if stats.at_min > 0}
              <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1 mt-2">
                <span class="text-[36px] sm:text-[42px] font-semibold price-number leading-none text-[var(--success)]">{stats.at_min}</span>
                <span class="text-[14px] text-[var(--fg-secondary)]">
                  {stats.at_min === 1 ? $t('dash.productAtLow') : $t('dash.productsAtLow')}
                </span>
              </div>
            {:else}
              <p class="text-[16px] font-semibold mt-2">{$t('dash.noneAtLow')}</p>
              <p class="text-[13px] text-[var(--fg-muted)] mt-1">{$t('dash.notifyLow')}</p>
            {/if}
            <p class="text-[13px] text-[var(--fg-muted)] mt-1.5">
              {$t('dash.productsMonitored', { count: stats.total })}{#if stats.no_price > 0} · {$t('dash.noPriceCount', { count: stats.no_price })}{/if}
            </p>

            <div class="flex flex-wrap items-center gap-2 mt-3">
              {#if stats.down > 0}
                <span class="badge badge-down">▼ {$t('dash.dropped', { count: stats.down })}</span>
              {/if}
              {#if stats.up > 0}
                <span class="badge badge-up">▲ {$t('dash.rose', { count: stats.up })}</span>
              {/if}
              {#if stats.down === 0 && stats.up === 0}
                <span class="badge badge-neutral">{$t('dash.stable')}</span>
              {/if}
              {#each opportunitySavings as saving (saving.currency)}
                <span class="badge badge-accent" title={$t('dash.savingHint')}>
                  {$t('dash.potentialSaving', { amount: formatPrice(saving.total, saving.currency) })}
                </span>
              {/each}
            </div>
          </div>

          <div class="w-full lg:w-80 flex-shrink-0">
            <div class="flex items-center justify-between mb-2">
              <span class="section-label">{$t('dash.status')}</span>
              <span class="num text-[12px] text-[var(--fg-muted)]">{stats.total}</span>
            </div>
            <div class="flex h-2.5 rounded-full overflow-hidden bg-[var(--bg-subtle)]">
              {#if atMinPct > 0}
                <div class="h-full bg-[var(--success)]" style="width: {atMinPct}%" title={`${stats.at_min} ${$t('dash.atLowShort')} (${Math.round(atMinPct)}%)`}></div>
              {/if}
              {#if restPct > 0}
                <div class="h-full bg-[var(--accent-muted)]" style="width: {restPct}%" title={`${restCount} ${$t('dash.trackingShort')} (${Math.round(restPct)}%)`}></div>
              {/if}
              {#if noPricePct > 0}
                <div class="h-full bg-[var(--border-strong)]" style="width: {noPricePct}%" title={`${stats.no_price} ${$t('dash.noPriceShort')} (${Math.round(noPricePct)}%)`}></div>
              {/if}
            </div>
            <div class="flex flex-wrap items-center gap-x-4 gap-y-1.5 mt-3 text-[11px] text-[var(--fg-muted)]">
              <span class="inline-flex items-center gap-1.5">
                <span class="w-2 h-2 rounded-full bg-[var(--success)]"></span>
                <span class="num text-[var(--fg-secondary)]">{stats.at_min}</span> {$t('dash.atLowShort')}
                <span class="num text-[var(--fg-muted)]">· {Math.round(atMinPct)}%</span>
              </span>
              <span class="inline-flex items-center gap-1.5">
                <span class="w-2 h-2 rounded-full bg-[var(--accent-muted)]"></span>
                <span class="num text-[var(--fg-secondary)]">{restCount}</span> {$t('dash.trackingShort')}
                <span class="num text-[var(--fg-muted)]">· {Math.round(restPct)}%</span>
              </span>
              {#if stats.no_price > 0}
                <span class="inline-flex items-center gap-1.5">
                  <span class="w-2 h-2 rounded-full bg-[var(--border-strong)]"></span>
                  <span class="num text-[var(--fg-secondary)]">{stats.no_price}</span> {$t('dash.noPriceShort')}
                  <span class="num text-[var(--fg-muted)]">· {Math.round(noPricePct)}%</span>
                </span>
              {/if}
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Salud del monitoreo -->
    {#if watchHealth.total > 0}
      <a href="/watches" class="card flex items-center justify-between gap-3 px-4 py-3 hover:border-[var(--border-strong)] transition-colors">
        <div class="flex items-center gap-2.5 min-w-0">
          <span class="w-2 h-2 rounded-full flex-shrink-0 {watchHealth.error > 0 ? 'bg-[var(--danger)]' : 'bg-[var(--success)]'}"></span>
          <span class="text-[13px] font-medium truncate">
            {watchHealth.error > 0
              ? $t(watchHealth.error === 1 ? 'dash.watchIssueOne' : 'dash.watchIssueMany', { count: watchHealth.error })
              : $t('dash.watchOk')}
          </span>
          <span class="text-[12px] text-[var(--fg-muted)] hidden sm:inline">
            {$t('dash.activeCount', { count: watchHealth.active })}{#if watchHealth.paused > 0} · {$t('dash.pausedCount', { count: watchHealth.paused })}{/if}
          </span>
        </div>
        <span class="text-[12px] text-[var(--accent)] font-medium flex-shrink-0">{$t('dash.viewWatches')}</span>
      </a>
    {/if}

    <!-- Cambios recientes + mínimos históricos -->
    {#if recentChanges.length > 0 || opportunities.length > 0}
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {#if recentChanges.length > 0}
          <div class="card overflow-hidden">
            <div class="px-4 py-3 border-b border-[var(--border)] flex items-center justify-between">
              <h2 class="section-title">{$t('dash.recentChanges')}</h2>
              <span class="text-[11px] text-[var(--fg-muted)]">{$t('dash.lastMovements')}</span>
            </div>
            <ul>
              {#each recentChanges as change (change.product.id)}
                <li>
                  <a href="/products/{change.product.id}" class="flex items-center gap-3 px-4 py-2.5 hover:bg-[var(--bg-subtle)] transition-colors border-b border-[var(--border-subtle)] last:border-b-0">
                    <div class="w-8 h-8 rounded-[var(--radius-sm)] bg-[var(--bg-subtle)] border border-[var(--border)] overflow-hidden flex items-center justify-center flex-shrink-0">
                      {#if change.product.image_url}
                        <img src={change.product.image_url} alt={change.product.name} class="w-full h-full object-contain p-0.5" loading="lazy" />
                      {:else}
                        <svg class="w-3.5 h-3.5 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
                        </svg>
                      {/if}
                    </div>
                    <div class="flex-1 min-w-0">
                      <p class="text-[13px] font-medium truncate">{change.product.name}</p>
                      <p class="text-[11px] text-[var(--fg-muted)] truncate">{titleCase(change.offer.store_name) || $t('dash.store')} · {timeAgo(change.at)}</p>
                    </div>
                    <div class="text-right flex-shrink-0">
                      <p class="num text-[13px] font-semibold">
                        {formatPrice(change.to, change.offer.currency)}{#if hasMixedCurrencies}<span class="text-[10px] font-medium text-[var(--fg-muted)] align-top ml-1">{currencyLabel(change.offer.currency)}</span>{/if}
                      </p>
                      <span class="badge {change.pct < 0 ? 'badge-down' : 'badge-up'} mt-0.5">
                        {change.pct < 0 ? '▼' : '▲'} {Math.abs(change.pct).toFixed(1)}%
                      </span>
                    </div>
                  </a>
                </li>
              {/each}
            </ul>
          </div>
        {/if}

        {#if opportunities.length > 0}
          <div class="card overflow-hidden">
            <div class="px-4 py-3 border-b border-[var(--border)] flex items-center justify-between gap-2">
              <h2 class="section-title">{$t('dash.opportunities')}</h2>
              <span class="badge badge-down">{opportunities.length}</span>
            </div>
            <ul>
              {#each opportunities as product (product.id)}
                {@const offer = getBestOffer(product)}
                {@const sparkData = getSparklineData(product)}
                {@const signal = offer ? computePriceSignal(priceHistories[offer.id], offer.current_price) : null}
                <li>
                  <a href="/products/{product.id}" class="flex items-center gap-3 px-4 py-2.5 hover:bg-[var(--bg-subtle)] transition-colors border-b border-[var(--border-subtle)] last:border-b-0">
                    <div class="w-8 h-8 rounded-[var(--radius-sm)] bg-[var(--bg-subtle)] border border-[var(--border)] overflow-hidden flex items-center justify-center flex-shrink-0">
                      {#if product.image_url}
                        <img src={product.image_url} alt={product.name} class="w-full h-full object-contain p-0.5" loading="lazy" />
                      {:else}
                        <svg class="w-3.5 h-3.5 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
                        </svg>
                      {/if}
                    </div>
                    <div class="flex-1 min-w-0">
                      <p class="text-[13px] font-medium truncate">{product.name}</p>
                      <p class="text-[11px] text-[var(--fg-muted)] truncate">{signal?.reason ?? $t('dash.goodPrice')}</p>
                    </div>
                    {#if sparkData.length >= 2}
                      <Sparkline data={sparkData} width={44} height={18} />
                    {/if}
                    {#if offer}
                      <span class="num text-[13px] font-semibold text-[var(--success)] flex-shrink-0">
                        {formatPrice(offer.current_price, offer.currency)}{#if hasMixedCurrencies}<span class="text-[10px] font-medium text-[var(--fg-muted)] align-top ml-1">{currencyLabel(offer.currency)}</span>{/if}
                      </span>
                    {/if}
                  </a>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>
    {/if}
  {/if}

  <!-- Tabla de productos -->
  {#if initialLoading}
    <TableSkeleton rows={6} />
  {:else if error}
    <ErrorState message={error} retry={() => loadProducts(true)} />
  {:else if products.length === 0}
    <EmptyState oncta={() => showAddDialog = true} ctaLabel={$t('dash.addFirstProduct')} showOnboarding={true} />
  {:else}
    <div>
      <p class="section-label mb-3">{$t('dash.allProducts')}</p>
      {#if !hasTrendData}
        <div class="flex items-start gap-2.5 px-3.5 py-3 mb-3 rounded-[var(--radius-md)] bg-[var(--bg-subtle)] border border-[var(--border)]">
          <svg class="w-4 h-4 text-[var(--fg-muted)] flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <p class="text-[12px] text-[var(--fg-muted)]">
            {$t('dash.historyNoticeBefore')}<span class="text-[var(--fg-secondary)] font-medium">{$t('dash.variation')}</span>{$t('dash.historyNoticeJoin')}<span class="text-[var(--fg-secondary)] font-medium">{$t('dash.trend')}</span>{$t('dash.historyNoticeAfter')}
          </p>
        </div>
      {/if}
      <ProductTable
        products={products}
        {priceHistories}
        onDelete={requestDelete}
        {deletingId}
        showCurrency={hasMixedCurrencies}
      />
      <Pagination
        {currentPage}
        {totalPages}
        loading={refreshing}
        onPageChange={handlePageChange}
      />
    </div>
  {/if}
</div>

<AddProductDialog
  open={showAddDialog}
  onclose={() => showAddDialog = false}
  onsuccess={handleProductAdded}
/>

<ConfirmDialog
  open={showDeleteConfirm}
  title={$t('dash.deleteTitle')}
  message={$t('dash.deleteMessage', { name: productToDelete?.name ?? '' })}
  confirmLabel={$t('dash.delete')}
  cancelLabel={$t('dash.cancel')}
  onconfirm={confirmDelete}
  oncancel={cancelDelete}
/>
