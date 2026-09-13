<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import type { Alert, Product } from '$lib/types';
  import { formatPrice } from '$lib/utils';
  import LoadingState from '$lib/components/LoadingState.svelte';
  import ErrorState from '$lib/components/ErrorState.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import { t, translate, currentLocale } from '$lib/i18n';

  let alerts: Alert[] = $state([]);
  let initialLoading = $state(true);
  let refreshing = $state(false);
  let error = $state('');
  let togglingId = $state<string | null>(null);
  let creatingAlert = $state<string | null>(null);
  let showAddAlert = $state(false);
  let selectedOfferId = $state<string>('');
  let alertThreshold = $state<string>('');
  let alertMode = $state<'fixed' | 'percent'>('fixed');
  let alertPercent = $state<string>('');
  let currentPage = $state(1);
  let totalPages = $state(1);
  let totalAlerts = $state(0);
  const pageSize = 10;
  let offerPriceMap = $state<Record<string, { price: number; currency: string; name: string }>>({});

  let stats = $derived.by(() => {
    const total = totalAlerts;
    const active = alerts.filter(a => a.enabled).length;
    const triggered = alerts.filter(a => a.triggered_at).length;
    return { total, active, triggered };
  });

  onMount(async () => {
    await Promise.all([loadAlerts(true), loadOfferPrices()]);
  });

  async function loadOfferPrices() {
    try {
      const products: Product[] = await api.products.listAll();
      const map: Record<string, { price: number; currency: string; name: string }> = {};
      for (const p of products) {
        for (const o of p.offers) {
          if (o.current_price > 0) {
            map[o.id] = { price: o.current_price, currency: o.currency, name: p.name };
          }
        }
      }
      offerPriceMap = map;
    } catch {}
  }

  async function loadAlerts(initial = false) {
    if (initial) initialLoading = true;
    else refreshing = true;
    error = '';
    try {
      const response = await api.alerts.list(currentPage, pageSize);
      alerts = response.data;
      totalPages = response.total_pages;
      totalAlerts = response.total;
    } catch (e) {
      error = e instanceof Error ? e.message : translate(currentLocale(), 'alerts.loadError');
    } finally {
      initialLoading = false;
      refreshing = false;
    }
  }

  async function toggleAlert(alert: Alert) {
    togglingId = alert.id;
    try {
      await api.alerts.update(alert.id, { enabled: !alert.enabled });
      alerts = alerts.map(a => a.id === alert.id ? { ...a, enabled: !a.enabled } : a);
    } catch (e) {
      console.error('Error toggling alert:', e);
    } finally {
      togglingId = null;
    }
  }

  function handlePageChange(page: number) {
    currentPage = page;
    loadAlerts();
  }

  async function createAlert() {
    if (!selectedOfferId) return;
    creatingAlert = selectedOfferId;
    try {
      let threshold: number | null = null;
      if (alertMode === 'percent') {
        const pct = parseFloat(alertPercent);
        if (!isNaN(pct) && pct > 0 && selectedOfferPrice) {
          threshold = selectedOfferPrice.price * (1 - pct / 100);
        }
      } else {
        threshold = alertThreshold ? parseFloat(alertThreshold) : null;
      }
      await api.alerts.create(selectedOfferId, 'BELOW_PRICE', threshold);
      await loadAlerts();
      showAddAlert = false;
      selectedOfferId = '';
      alertThreshold = '';
      alertPercent = '';
      alertMode = 'fixed';
    } catch (e) {
      console.error('Error creating alert:', e);
    } finally {
      creatingAlert = null;
    }
  }

  const selectedOfferPrice = $derived(selectedOfferId ? offerPriceMap[selectedOfferId] : null);

  const computedPercentPrice = $derived.by(() => {
    const pct = parseFloat(alertPercent);
    if (!selectedOfferPrice || isNaN(pct) || pct <= 0) return null;
    return selectedOfferPrice.price * (1 - pct / 100);
  });

  const thresholdDelta = $derived.by(() => {
    const threshold = parseFloat(alertThreshold);
    if (!selectedOfferPrice || isNaN(threshold) || threshold <= 0) return null;
    const pct = ((threshold - selectedOfferPrice.price) / selectedOfferPrice.price) * 100;
    return pct;
  });

  function getUniqueOffers() {
    const offerMap = new Map<string, { id: string; name: string; store: string }>();
    for (const alert of alerts) {
      if (!offerMap.has(alert.offer_id)) {
        offerMap.set(alert.offer_id, {
          id: alert.offer_id,
          name: alert.product_name || translate(currentLocale(), 'table.product'),
          store: alert.store_name || translate(currentLocale(), 'table.storeFallback')
        });
      }
    }
    return Array.from(offerMap.values());
  }

  function getAlertTypeLabel(type: string): string {
    switch (type) {
      case 'PRICE_DECREASE': return translate(currentLocale(), 'alerts.type.priceDecrease');
      case 'BELOW_PRICE': return translate(currentLocale(), 'alerts.type.belowPrice');
      case 'HISTORICAL_LOW': return translate(currentLocale(), 'alerts.type.historicalLow');
      case 'BACK_IN_STOCK': return translate(currentLocale(), 'alerts.type.backInStock');
      case 'PRICE_INCREASE': return translate(currentLocale(), 'alerts.type.priceIncrease');
      default: return type;
    }
  }

  function getAlertIcon(type: string): string {
    switch (type) {
      case 'PRICE_DECREASE': return 'M13 17h8m0 0V9m0 8l-8-8-4 4-6-6';
      case 'HISTORICAL_LOW': return 'M13 7h8m0 0v8m0-8l-8 8-4-4-6 6';
      case 'BELOW_PRICE': return 'M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z';
      case 'BACK_IN_STOCK': return 'M5 13l4 4L19 7';
      default: return 'M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9';
    }
  }

  function getAlertColor(type: string): string {
    switch (type) {
      case 'PRICE_DECREASE': return 'text-[var(--success)] bg-[var(--success-subtle)]';
      case 'HISTORICAL_LOW': return 'text-[var(--accent)] bg-[var(--accent-subtle)]';
      case 'BELOW_PRICE': return 'text-[var(--warning)] bg-[var(--warning-subtle)]';
      case 'BACK_IN_STOCK': return 'text-[var(--info)] bg-[var(--info-subtle)]';
      default: return 'text-[var(--fg-muted)] bg-[var(--bg-subtle)]';
    }
  }
</script>

<svelte:head>
  <title>{$t('alerts.pageTitle')}</title>
</svelte:head>

<div class="space-y-5">
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div>
      <h1 class="page-title">{$t('alerts.title')}</h1>
      <p class="text-[13px] text-[var(--fg-muted)] mt-0.5">{$t('alerts.summary', { total: stats.total, active: stats.active, triggered: stats.triggered })}</p>
    </div>
    <button onclick={() => showAddAlert = true} class="btn btn-primary">
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.2" d="M12 4v16m8-8H4" />
      </svg>
      {$t('alerts.new')}
    </button>
  </div>

  {#if initialLoading}
    <LoadingState />
  {:else if error}
    <ErrorState message={error} retry={() => loadAlerts(true)} />
  {:else if alerts.length === 0}
    <div class="card p-8 text-center">
      <div class="w-12 h-12 mx-auto mb-4 rounded-full bg-[var(--accent-subtle)] flex items-center justify-center">
        <svg class="w-6 h-6 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
        </svg>
      </div>
      <h3 class="font-semibold mb-1">{$t('alerts.empty.title')}</h3>
      <p class="text-[13px] text-[var(--fg-muted)] mb-4">
        {$t('alerts.empty.message1')}<br/>
        {$t('alerts.empty.message2')}
      </p>
      <a href="/products" class="btn btn-primary inline-flex">{$t('dash.addProduct')}</a>
    </div>
  {:else}
    <div class="space-y-3 {refreshing ? 'refreshing' : ''}">
      {#each alerts as alert, i (alert.id)}
        {@const current = offerPriceMap[alert.offer_id]}
        <div class="card overflow-hidden animate-in" style="animation-delay: {Math.min(i, 8) * 30}ms">
          <div class="p-4 border-b border-[var(--border)]">
            <div class="flex items-center gap-3">
              {#if alert.product_image}
                <img
                  src={alert.product_image}
                  alt={alert.product_name || $t('table.product')}
                  class="w-10 h-10 object-contain rounded-[var(--radius-md)] bg-[var(--bg-subtle)] p-0.5"
                />
              {:else}
                <div class="w-10 h-10 bg-[var(--bg-subtle)] rounded-[var(--radius-md)] flex items-center justify-center">
                  <svg class="w-5 h-5 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
                  </svg>
                </div>
              {/if}

              <div class="flex-1 min-w-0">
                <h3 class="font-medium text-[13px] truncate">{alert.product_name || $t('alerts.productNoName')}</h3>
                <div class="flex items-center gap-2 mt-0.5">
                  {#if alert.store_name}
                    <span class="text-[11px] px-1.5 py-0.5 rounded-[var(--radius-sm)] bg-[var(--bg-subtle)] text-[var(--fg-muted)] font-medium">
                      {alert.store_name}
                    </span>
                  {/if}
                  {#if alert.offer_url}
                    <a href={alert.offer_url} target="_blank" rel="noopener noreferrer" class="text-[var(--accent)] hover:text-[var(--accent-hover)] text-[11px] font-medium transition-colors">
                      {$t('alerts.viewInStore')}
                    </a>
                  {/if}
                </div>
              </div>
            </div>
          </div>

          <div class="p-3 sm:p-4 flex items-center justify-between gap-3">
            <div class="flex items-center gap-3 min-w-0">
              <div class="p-2 rounded-[var(--radius-md)] {getAlertColor(alert.alert_type)}">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d={getAlertIcon(alert.alert_type)} />
                </svg>
              </div>
              <div class="min-w-0">
                <p class="font-medium text-[13px]">{getAlertTypeLabel(alert.alert_type)}</p>
                {#if alert.threshold_price}
                  <p class="num text-[11px] text-[var(--fg-muted)]">
                    {$t('alerts.target')} {formatPrice(alert.threshold_price, current?.currency || 'CLP')}
                    {#if current && current.price > 0}
                      {#if current.price <= alert.threshold_price}
                        <span class="text-[var(--success)] font-medium">· {$t('alerts.reached')}</span>
                      {:else}
                        {@const diff = current.price - alert.threshold_price}
                        {@const pct = (diff / current.price) * 100}
                        <span class="text-[var(--fg-muted)]">· {$t('alerts.missing', { amount: formatPrice(diff, current.currency), percent: pct.toFixed(0) })}</span>
                      {/if}
                    {/if}
                  </p>
                {/if}
                {#if current && current.price > 0}
                  <p class="num text-[11px] text-[var(--fg-secondary)] mt-0.5">{$t('alerts.current')} {formatPrice(current.price, current.currency)}</p>
                {/if}
                {#if alert.triggered_at}
                  <p class="text-[11px] text-[var(--success)] font-medium">{$t('alerts.triggered')}</p>
                {/if}
              </div>
            </div>

            <button
              onclick={() => toggleAlert(alert)}
              disabled={togglingId === alert.id}
              class="px-3 py-1.5 rounded-[var(--radius-md)] text-[12px] font-medium transition-colors disabled:opacity-50 flex-shrink-0
                {alert.enabled
                  ? 'bg-[var(--success-subtle)] text-[var(--success)] hover:bg-[var(--success-muted)]'
                  : 'bg-[var(--bg-subtle)] text-[var(--fg-muted)] hover:text-[var(--fg)]'}"
            >
              {#if togglingId === alert.id}
                <svg class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
              {:else}
                {alert.enabled ? $t('alerts.active') : $t('alerts.paused')}
              {/if}
            </button>
          </div>
        </div>
      {/each}
    </div>
    <Pagination {currentPage} {totalPages} loading={refreshing} onPageChange={handlePageChange} />
  {/if}
</div>

{#if showAddAlert}
  <div class="fixed inset-0 m3-scrim flex items-center justify-center z-50 p-4" onclick={() => showAddAlert = false}>
    <div class="m3-dialog w-full max-w-md p-5 animate-in" onclick={(e) => e.stopPropagation()}>
      <h2 class="title-md mb-4">{$t('alerts.new')}</h2>

      <form onsubmit={(e) => { e.preventDefault(); createAlert(); }} class="space-y-4">
        <div>
          <label for="alert-offer" class="block text-[12px] font-medium text-[var(--fg-secondary)] mb-1.5">{$t('table.product')}</label>
          <select id="alert-offer" bind:value={selectedOfferId} class="input" required>
            <option value="">{$t('alerts.selectProduct')}</option>
            {#each getUniqueOffers() as offer}
              <option value={offer.id}>{offer.name} - {offer.store}</option>
            {/each}
          </select>
          {#if selectedOfferPrice}
            <p class="text-[11px] text-[var(--fg-muted)] mt-1.5">
              {$t('alerts.currentPrice')} <span class="num font-semibold text-[var(--fg)]">{formatPrice(selectedOfferPrice.price, selectedOfferPrice.currency)}</span>
            </p>
          {/if}
        </div>

        <div>
          <div class="flex items-center gap-2 mb-2">
            <span class="text-[12px] font-medium text-[var(--fg-secondary)]">{$t('alerts.condition')}</span>
            <div class="flex rounded-[var(--radius-md)] border border-[var(--border)] overflow-hidden text-[12px]">
              <button type="button" onclick={() => alertMode = 'fixed'} class="px-3 py-1.5 font-medium transition-colors {alertMode === 'fixed' ? 'bg-[var(--accent)] text-[var(--accent-fg)]' : 'text-[var(--fg-muted)] hover:bg-[var(--bg-subtle)]'}">{$t('alerts.modeFixed')}</button>
              <button type="button" onclick={() => alertMode = 'percent'} class="px-3 py-1.5 font-medium transition-colors {alertMode === 'percent' ? 'bg-[var(--accent)] text-[var(--accent-fg)]' : 'text-[var(--fg-muted)] hover:bg-[var(--bg-subtle)]'}">{$t('alerts.modePercent')}</button>
            </div>
          </div>

          {#if alertMode === 'fixed'}
            <input
              id="alert-threshold"
              type="number"
              bind:value={alertThreshold}
              placeholder={selectedOfferPrice ? String(Math.round(selectedOfferPrice.price * 0.9)) : $t('alerts.thresholdPlaceholder')}
              class="input num"
            />
            <div class="flex items-center justify-between mt-1">
              <p class="text-[11px] text-[var(--fg-muted)]">{$t('alerts.fixedHint')}</p>
              {#if thresholdDelta !== null}
                <span class="num text-[11px] font-medium {thresholdDelta < 0 ? 'text-[var(--success)]' : 'text-[var(--danger)]'}">
                  {thresholdDelta < 0 ? '↓' : '↑'}{Math.abs(thresholdDelta).toFixed(1)}%
                </span>
              {/if}
            </div>
          {:else}
            <div class="flex items-center gap-2">
              <input
                type="number"
                bind:value={alertPercent}
                placeholder="10"
                min="0.1"
                max="99"
                step="0.5"
                class="input num flex-1"
              />
              <span class="text-[13px] font-medium text-[var(--fg-muted)]">{$t('alerts.modePercent')}</span>
            </div>
            {#if computedPercentPrice !== null}
              <p class="text-[11px] text-[var(--fg-muted)] mt-1">
                {$t('alerts.targetPrice')} <span class="num font-semibold text-[var(--success)]">{formatPrice(computedPercentPrice, selectedOfferPrice?.currency || 'CLP')}</span>
              </p>
            {:else}
              <p class="text-[11px] text-[var(--fg-muted)] mt-1">{$t('alerts.percentHint')}</p>
            {/if}
          {/if}
        </div>

        <div class="flex gap-2 justify-end">
          <button type="button" onclick={() => showAddAlert = false} class="btn btn-ghost">{$t('dash.cancel')}</button>
          <button type="submit" disabled={!selectedOfferId || creatingAlert !== null} class="btn btn-primary">
            {creatingAlert ? $t('alerts.creating') : $t('alerts.create')}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
