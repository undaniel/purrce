<script lang="ts">
  import { page } from '$app/state';
  import { onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import { lastEvent } from '$lib/stores/events';
  import type { Product, PriceHistory, Comparison, Watch, Tag, Alert, ProductList } from '$lib/types';
  import { computePriceSignal } from '$lib/utils/signal';
  import PriceChart from '$lib/components/PriceChart.svelte';
  import PriceHistoryComponent from '$lib/components/PriceHistory.svelte';
  import BuySignalBadge from '$lib/components/BuySignalBadge.svelte';
  import PriceRangeBar from '$lib/components/PriceRangeBar.svelte';
  import LoadingState from '$lib/components/LoadingState.svelte';
  import ErrorState from '$lib/components/ErrorState.svelte';
  import TagInput from '$lib/components/TagInput.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import { formatPrice, getStatusColor, getStatusLabel, timeAgo, formatDate } from '$lib/utils';
  import { t, translate, currentLocale } from '$lib/i18n';

  let product: Product | null = $state(null);
  let comparison: Comparison | null = $state(null);
  let offerHistories: Record<string, PriceHistory[]> = $state({});
  let watches: Watch[] = $state([]);
  let loading = $state(true);
  let error = $state('');
  let selectedOffer = $state<string | null>(null);
  let showAddOffer = $state(false);
  let newOfferUrl = $state('');
  let addingOffer = $state(false);
  let checking = $state(false);
  let editingTags = $state(false);
  let savingTags = $state(false);
  let productTags: Tag[] = $state([]);
  let showDeleteConfirm = $state(false);
  let deleting = $state(false);
  let showAddToList = $state(false);
  let allLists: ProductList[] = $state([]);
  let addingToList = $state<string | null>(null);
  let offerAlerts: Alert[] = $state([]);

  async function openAddToList() {
    try { allLists = await api.lists.list(); } catch {}
    showAddToList = true;
  }

  async function addToList(listId: string) {
    if (!product) return;
    addingToList = listId;
    try {
      await api.lists.addItem(listId, product.id);
      showAddToList = false;
    } catch (e) {
      console.error(e);
    } finally {
      addingToList = null;
    }
  }

  $effect(() => {
    const id = page.params.id;
    if (id) loadProduct(id);
  });

  async function loadProduct(id: string) {
    loading = true;
    error = '';
    try {
      product = await api.products.get(id);
      comparison = await api.products.comparison(id);
      watches = (await api.watches.list(1, 200)).data ?? [];
      productTags = product.tags || [];
      if (product.offers?.length > 0) {
        selectedOffer = product.offers[0].id;
      }
      await loadAllHistories();
      if (selectedOffer) loadAlerts(selectedOffer);
    } catch (e) {
      error = e instanceof Error ? e.message : translate(currentLocale(), 'product.loadError');
    } finally {
      loading = false;
    }
  }

  async function loadAllHistories() {
    if (!product) return;
    const entries: Record<string, PriceHistory[]> = {};
    await Promise.all(product.offers.map(async (offer) => {
      try {
        const result = await api.offers.history(offer.id, 1, 200);
        entries[offer.id] = result.data ?? (result as any);
      } catch {
        entries[offer.id] = [];
      }
    }));
    offerHistories = entries;
  }

  async function loadAlerts(offerId: string) {
    try {
      const res = await api.alerts.list(1, 200);
      offerAlerts = res.data.filter(a => a.offer_id === offerId && a.triggered_at);
    } catch {}
  }

  // SSE: recargar si el precio de una de nuestras ofertas cambió
  const unsubscribeEvents = lastEvent.subscribe(ev => {
    if (!ev || !product) return;
    if (ev.type === 'price_updated') {
      const isOurs = product.offers?.some((o: any) => o.id === ev.offer_id);
      if (isOurs) {
        api.products.get(product.id).then(p => { product = p; }).catch(() => {});
        loadAllHistories();
      }
    }
  });

  onDestroy(unsubscribeEvents);

  async function handleCheck(offerId: string) {
    checking = true;
    try {
      const watch = watches.find(w => w.offer_id === offerId);
      if (watch) {
        await api.watches.check(watch.id);
        await loadAllHistories();
        if (product) {
          product = await api.products.get(product.id);
          comparison = await api.products.comparison(product.id);
        }
      } else {
        console.error('No watch found for offer:', offerId);
      }
    } catch (e) {
      console.error('Error checking:', e);
    } finally {
      checking = false;
    }
  }

  async function handleAddOffer() {
    if (!newOfferUrl.trim() || !product) return;
    addingOffer = true;
    try {
      await api.products.addOffer(product.id, newOfferUrl, true, 3600);
      newOfferUrl = '';
      showAddOffer = false;
      await loadProduct(product.id);
    } catch (e) {
      console.error('Error adding offer:', e);
    } finally {
      addingOffer = false;
    }
  }

  async function handleTagsChange(tags: Tag[]) {
    if (!product) return;
    savingTags = true;
    try {
      await api.products.setTags(product.id, tags.map(t => t.name));
      productTags = tags;
    } catch (e) {
      console.error('Error updating tags:', e);
    } finally {
      savingTags = false;
    }
  }

  async function handleDelete() {
    if (!product) return;
    deleting = true;
    try {
      await api.products.delete(product.id);
      goto('/products');
    } catch (e) {
      console.error('Error deleting product:', e);
      alert(translate(currentLocale(), 'dash.deleteError'));
      deleting = false;
    }
  }

  const history = $derived(selectedOffer ? (offerHistories[selectedOffer] ?? []) : []);

  const currentOffer = $derived(product?.offers?.find(o => o.id === selectedOffer));
  const currentWatch = $derived(watches.find(w => w.offer_id === selectedOffer));
  const hasPrice = $derived(!!currentOffer && currentOffer.current_price > 0);

  const signal = $derived(
    currentOffer && currentOffer.current_price > 0
      ? computePriceSignal(history, currentOffer.current_price)
      : null
  );

  // Objetivo de precio inline
  let targetPrice = $state('');
  let savingTarget = $state(false);
  let targetSaved = $state(false);

  const targetThreshold = $derived.by(() => {
    const value = parseFloat(targetPrice.replace(/\./g, '').replace(',', '.'));
    return !isNaN(value) && value > 0 ? value : null;
  });

  const targetDeltaPct = $derived.by(() => {
    if (targetThreshold === null || !currentOffer || currentOffer.current_price <= 0) return null;
    return ((targetThreshold - currentOffer.current_price) / currentOffer.current_price) * 100;
  });

  async function saveTarget() {
    if (!selectedOffer || targetThreshold === null) return;
    savingTarget = true;
    targetSaved = false;
    try {
      await api.alerts.create(selectedOffer, 'BELOW_PRICE', targetThreshold);
      targetSaved = true;
      targetPrice = '';
      setTimeout(() => targetSaved = false, 2500);
    } catch (e) {
      console.error('Error creating target alert:', e);
    } finally {
      savingTarget = false;
    }
  }

  const priceChange = $derived.by(() => {
    if (history.length < 2) return null;
    const sorted = [...history].sort((a, b) => new Date(b.observed_at).getTime() - new Date(a.observed_at).getTime());
    const current = sorted[0].price;
    const previous = sorted[1].price;
    if (previous === 0) return null;
    return ((current - previous) / previous) * 100;
  });

  const priceStats = $derived.by(() => {
    if (history.length === 0) return null;
    const valid = history.filter(h => h.price > 0);
    if (valid.length === 0) return null;
    const prices = valid.map(h => h.price);
    const minPrice = Math.min(...prices);
    const maxPrice = Math.max(...prices);
    const minEntry = valid.find(h => h.price === minPrice)!;
    const maxEntry = valid.find(h => h.price === maxPrice)!;
    return {
      min: minPrice,
      max: maxPrice,
      avg: prices.reduce((a, b) => a + b, 0) / prices.length,
      minDate: minEntry.observed_at,
      maxDate: maxEntry.observed_at,
    };
  });

  const chartSeries = $derived(
    (product?.offers ?? [])
      .filter(o => (offerHistories[o.id]?.length ?? 0) > 0)
      .map(o => ({
        name: o.store_name || translate(currentLocale(), 'table.storeFallback'),
        data: offerHistories[o.id],
        highlighted: o.id === selectedOffer
      }))
  );

  const chartEvents = $derived(
    offerAlerts
      .filter(a => a.triggered_at)
      .map(a => ({
        date: a.triggered_at!,
        label: '▼',
        price: a.threshold_price ?? 0
      }))
  );
</script>

<svelte:head>
  <title>{product?.name || $t('product.fallbackName')} · Purrce</title>
</svelte:head>

{#if loading}
  <LoadingState />
{:else if error}
  <ErrorState message={error} retry={() => page.params.id && loadProduct(page.params.id)} />
{:else if product}
  <div class="space-y-5">
    <!-- Barra superior -->
    <div class="flex items-center justify-between gap-3">
      <a href="/products" class="inline-flex items-center gap-1.5 text-[13px] text-[var(--fg-muted)] hover:text-[var(--fg)] transition-colors font-medium">
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
        </svg>
        {$t('nav.products')}
      </a>
      <div class="flex items-center gap-2">
        <button onclick={openAddToList} class="btn btn-secondary">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01" />
          </svg>
          {$t('product.addToList')}
        </button>
        <button
          onclick={() => showDeleteConfirm = true}
          class="btn btn-ghost text-[var(--danger)] hover:bg-[var(--danger-subtle)] hover:text-[var(--danger)]"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
          {$t('table.delete')}
        </button>
      </div>
    </div>

    <!-- Cabecera del producto -->
    <div class="card p-5 sm:p-6">
      <div class="flex flex-col sm:flex-row gap-5">
        <div class="flex-shrink-0">
          <div class="w-20 h-20 sm:w-24 sm:h-24 rounded-[var(--radius-lg)] bg-[var(--bg-subtle)] border border-[var(--border)] flex items-center justify-center overflow-hidden">
            {#if product.image_url}
              <img src={product.image_url} alt={product.name} class="w-full h-full object-contain p-2" />
            {:else}
              <svg class="w-8 h-8 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
              </svg>
            {/if}
          </div>
        </div>

        <div class="flex-1 min-w-0">
          <h1 class="page-title text-[19px] sm:text-[21px]">{product.name}</h1>

          <!-- Tags -->
          <div class="mt-2">
            {#if editingTags}
              <div class="mb-2">
                <TagInput tags={productTags} onTagsChange={handleTagsChange} />
              </div>
              <button
                onclick={() => editingTags = false}
                disabled={savingTags}
                class="text-[12px] text-[var(--accent)] hover:text-[var(--accent-hover)] font-medium disabled:opacity-50 transition-colors"
              >
                {savingTags ? $t('product.saving') : $t('product.done')}
              </button>
            {:else}
              <div class="flex flex-wrap items-center gap-2">
                {#each productTags as tag}
                  <span class="inline-flex items-center gap-1 text-[11px] font-medium text-[var(--fg-secondary)]">
                    <span class="w-1.5 h-1.5 rounded-full" style="background-color: {tag.color}"></span>
                    {tag.name}
                  </span>
                {/each}
                <button
                  onclick={() => editingTags = true}
                  class="text-[11px] text-[var(--fg-muted)] hover:text-[var(--accent)] font-medium transition-colors"
                >
                  {productTags.length > 0 ? $t('product.edit') : $t('product.addTags')}
                </button>
              </div>
            {/if}
          </div>

          {#if currentOffer}
            <div class="flex items-center gap-2 mt-2.5">
              <span class="text-[11px] px-2 py-0.5 rounded-[var(--radius-sm)] bg-[var(--bg-subtle)] text-[var(--fg-secondary)] font-medium">
                {currentOffer.store_name || $t('table.storeFallback')}
              </span>
              <a
                href={currentOffer.url}
                target="_blank"
                rel="noopener noreferrer"
                class="inline-flex items-center gap-1 text-[11px] text-[var(--accent)] hover:text-[var(--accent-hover)] font-medium transition-colors"
              >
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                </svg>
                {$t('product.viewInStore')}
              </a>
            </div>
          {/if}

          {#if hasPrice}
            <div class="mt-3 flex flex-wrap items-baseline gap-2.5">
              <p class="text-[28px] sm:text-[32px] price-number">{formatPrice(currentOffer.current_price, currentOffer.currency)}</p>
              {#if priceChange !== null && priceChange !== 0}
                <span class="badge {priceChange < 0 ? 'badge-down' : 'badge-up'}">
                  {priceChange > 0 ? '↑' : '↓'} {Math.abs(priceChange).toFixed(1)}%
                </span>
              {/if}
              {#if signal}
                <BuySignalBadge signal={signal.signal} size="md" reason={signal.reason} />
              {/if}
            </div>
            {#if signal}
              <p class="text-[12px] text-[var(--fg-muted)] mt-1.5">
                {signal.reason}{#if signal.savingsVsAvg > 0} · <span class="text-[var(--success)] font-medium">{$t('product.savingsText', { amount: formatPrice(signal.savingsVsAvg, currentOffer.currency) })}</span>{/if}
              </p>
            {/if}
          {/if}

          {#if currentWatch}
            <div class="mt-2.5 flex items-center gap-2">
              <span class="inline-flex items-center gap-1.5 text-[11px] font-medium {getStatusColor(currentWatch.status)}">
                <span class="w-1.5 h-1.5 rounded-full bg-current"></span>
                {getStatusLabel(currentWatch.status)}
              </span>
              {#if currentWatch.last_check_at}
                <span class="text-[11px] text-[var(--fg-muted)]">· {$t('product.lastCheck')} {timeAgo(currentWatch.last_check_at)}</span>
              {/if}
            </div>
          {/if}
        </div>

        <div class="flex-shrink-0">
          <button
            onclick={() => selectedOffer && handleCheck(selectedOffer)}
            disabled={checking || !currentWatch}
            class="btn btn-primary w-full sm:w-auto"
          >
            {#if checking}
              <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              {$t('product.checking')}
            {:else}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              {$t('product.check')}
            {/if}
          </button>
        </div>
      </div>
    </div>

    <!-- Posición en el rango + estadísticas -->
    {#if priceStats && signal}
      <div class="card p-4 sm:p-5">
        <div class="flex flex-wrap items-center justify-between gap-3 mb-4">
          <div>
            <h3 class="section-title">{$t('product.rangePosition')}</h3>
            <p class="text-[11px] text-[var(--fg-muted)] mt-0.5">{$t('product.records', { count: signal.samples })} · {signal.reason}</p>
          </div>
          <BuySignalBadge signal={signal.signal} reason={signal.reason} />
        </div>

        <PriceRangeBar {signal} currency={currentOffer?.currency || 'CLP'} />

        <div class="grid grid-cols-3 gap-3 mt-5 pt-4 border-t border-[var(--border)]">
          <div>
            <p class="section-label">{$t('product.minimum')}</p>
            <p class="num text-[14px] font-semibold text-[var(--success)] mt-1">{formatPrice(priceStats.min, currentOffer?.currency || 'CLP')}</p>
            <p class="text-[11px] text-[var(--fg-muted)] mt-0.5">{formatDate(priceStats.minDate)}</p>
          </div>
          <div>
            <p class="section-label">{$t('product.average')}</p>
            <p class="num text-[14px] font-semibold mt-1">{formatPrice(priceStats.avg, currentOffer?.currency || 'CLP')}</p>
            <p class="text-[11px] text-[var(--fg-muted)] mt-0.5">{$t('product.historical')}</p>
          </div>
          <div>
            <p class="section-label">{$t('product.maximum')}</p>
            <p class="num text-[14px] font-semibold text-[var(--danger)] mt-1">{formatPrice(priceStats.max, currentOffer?.currency || 'CLP')}</p>
            <p class="text-[11px] text-[var(--fg-muted)] mt-0.5">{formatDate(priceStats.maxDate)}</p>
          </div>
        </div>
      </div>
    {/if}

    <!-- Objetivo de precio inline -->
    {#if hasPrice && currentOffer}
      <div class="card p-4 sm:p-5">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h3 class="section-title">{$t('product.alertWhenBelow')}</h3>
            <p class="text-[11px] text-[var(--fg-muted)] mt-0.5">{$t('product.createAlertHint')}</p>
          </div>
          <form onsubmit={(e) => { e.preventDefault(); saveTarget(); }} class="flex items-center gap-2">
            <input
              type="text"
              bind:value={targetPrice}
              placeholder={String(Math.round(currentOffer.current_price * 0.9))}
              class="input num w-32"
              inputmode="numeric"
            />
            <button type="submit" disabled={savingTarget || targetThreshold === null} class="btn btn-secondary">
              {savingTarget ? $t('product.saving') : targetSaved ? $t('product.ready') : $t('product.createAlert')}
            </button>
          </form>
        </div>
        {#if targetDeltaPct !== null}
          <p class="text-[11px] mt-2 {targetDeltaPct < 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]'}">
            {targetDeltaPct < 0 ? '↓' : '↑'} {$t('product.vsCurrentPrice', { percent: Math.abs(targetDeltaPct).toFixed(1) })}
          </p>
        {/if}
      </div>
    {/if}

    <!-- Selector de tienda -->
    {#if product.offers.length > 0}
      <div class="flex flex-wrap gap-2">
        {#each product.offers as offer}
          <button
            onclick={() => { selectedOffer = offer.id; loadAlerts(offer.id); }}
            class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-md)] text-[12px] font-medium border transition-colors
              {selectedOffer === offer.id
                ? 'bg-[var(--accent-subtle)] border-[var(--accent)]/40 text-[var(--accent)]'
                : 'bg-[var(--card)] border-[var(--border)] text-[var(--fg-secondary)] hover:border-[var(--border-strong)] hover:text-[var(--fg)]'}"
          >
            <span>{offer.store_name || $t('table.storeFallback')}</span>
            {#if offer.url}
              <a
                href={offer.url}
                target="_blank"
                rel="noopener noreferrer"
                onclick={(e) => e.stopPropagation()}
                class="opacity-60 hover:opacity-100 transition-opacity"
              >
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                </svg>
              </a>
            {/if}
          </button>
        {/each}
        <button
          onclick={() => showAddOffer = true}
          class="inline-flex items-center gap-1 px-3 py-1.5 rounded-[var(--radius-md)] text-[12px] font-medium border border-dashed border-[var(--border)] text-[var(--fg-muted)] hover:text-[var(--fg)] hover:border-[var(--border-strong)] transition-colors"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
          </svg>
          {$t('product.addStore')}
        </button>
      </div>
    {/if}

    {#if showAddOffer}
      <div class="card p-4">
        <h3 class="section-title mb-3">{$t('product.addOfferTitle')}</h3>
        <form onsubmit={(e) => { e.preventDefault(); handleAddOffer(); }} class="flex flex-col sm:flex-row gap-2">
          <input
            type="url"
            bind:value={newOfferUrl}
            placeholder="https://otra-tienda.cl/mismo-producto"
            class="input flex-1"
            required
          />
          <div class="flex gap-2">
            <button type="submit" disabled={addingOffer || !newOfferUrl.trim()} class="btn btn-primary flex-1 sm:flex-none">
              {addingOffer ? $t('product.adding') : $t('product.add')}
            </button>
            <button type="button" onclick={() => showAddOffer = false} class="btn btn-ghost">{$t('dash.cancel')}</button>
          </div>
        </form>
      </div>
    {/if}

    {#if comparison && comparison.offers.length > 1}
      <div class="card p-4">
        <h2 class="title-sm mb-3">{$t('product.storeComparison')}</h2>
        <div class="space-y-1.5">
          {#each comparison.offers as offer, i}
            <div class="flex items-center justify-between p-3 rounded-[var(--radius-md)] {i === 0 ? 'bg-[var(--success-subtle)]' : 'bg-[var(--bg-subtle)]'}">
              <div class="flex items-center gap-2.5 min-w-0">
                {#if i === 0}
                  <span class="badge badge-down flex-shrink-0">{$t('product.bestPrice')}</span>
                {/if}
                <span class="text-[13px] font-medium truncate">{offer.store_name || $t('table.storeFallback')}</span>
                <a
                  href={offer.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  class="text-[var(--fg-muted)] hover:text-[var(--accent)] transition-colors flex-shrink-0"
                >
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                  </svg>
                </a>
              </div>
              <div class="flex items-center gap-2.5 flex-shrink-0">
                <span class="num text-[14px] font-semibold {i === 0 ? 'text-[var(--success)]' : ''}">{formatPrice(offer.current_price, offer.currency)}</span>
                {#if i > 0 && comparison.min_price > 0}
                  {@const diff = offer.current_price - comparison.min_price}
                  {@const diffPct = ((diff / comparison.min_price) * 100).toFixed(0)}
                  <span class="badge badge-up">+{formatPrice(diff, offer.currency)} ({diffPct}%)</span>
                {/if}
              </div>
            </div>
          {/each}
        </div>
        {#if comparison.savings > 0}
          <div class="mt-3 p-3 rounded-[var(--radius-md)] bg-[var(--success-subtle)]">
            <p class="text-[13px] text-[var(--success)] font-medium">
              {$t('product.maxSavings')} <span class="num font-semibold">{formatPrice(comparison.savings, comparison.offers[0]?.currency || 'CLP')}</span>
            </p>
          </div>
        {/if}
      </div>
    {/if}

    {#if currentOffer}
      <PriceChart series={chartSeries} currency={currentOffer.currency} events={chartEvents} />
      <PriceHistoryComponent {history} currency={currentOffer.currency} />
    {/if}
  </div>
{/if}

{#if showAddToList && product}
  <div class="fixed inset-0 m3-scrim flex items-center justify-center z-50 p-4" onclick={() => showAddToList = false}>
    <div class="m3-dialog w-full max-w-sm p-5 flex flex-col gap-4 animate-in" onclick={(e) => e.stopPropagation()}>
      <div>
        <h2 class="title-md">{$t('product.addToList')}</h2>
        <p class="text-[13px] text-[var(--fg-muted)] mt-0.5 truncate">{product.name}</p>
      </div>
      {#if allLists.length === 0}
        <div class="text-center py-4">
          <p class="text-[13px] text-[var(--fg-muted)] mb-3">{$t('product.noLists')}</p>
          <a href="/lists" class="text-[13px] text-[var(--accent)] hover:underline">{$t('product.createList')} →</a>
        </div>
      {:else}
        <div class="space-y-0.5 max-h-60 overflow-y-auto">
          {#each allLists as list}
            <button
              onclick={() => addToList(list.id)}
              disabled={addingToList === list.id}
              class="w-full flex items-center gap-3 px-3 py-2.5 rounded-[var(--radius-md)] text-left hover:bg-[var(--bg-subtle)] transition-colors disabled:opacity-50"
            >
              <svg class="w-4 h-4 text-[var(--fg-muted)] flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
              </svg>
              <span class="text-[13px] font-medium flex-1 truncate">{list.name}</span>
              {#if addingToList === list.id}
                <span class="text-[11px] text-[var(--fg-muted)]">{$t('product.adding')}</span>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
      <div class="flex justify-end border-t border-[var(--border)] pt-3">
        <button onclick={() => showAddToList = false} class="btn btn-ghost">{$t('product.close')}</button>
      </div>
    </div>
  </div>
{/if}

<ConfirmDialog
  open={showDeleteConfirm}
  title={$t('dash.deleteTitle')}
  message={$t('dash.deleteMessage', { name: product?.name ?? '' })}
  confirmLabel={$t('dash.delete')}
  cancelLabel={$t('dash.cancel')}
  onconfirm={handleDelete}
  oncancel={() => showDeleteConfirm = false}
/>
