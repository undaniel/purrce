<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import type { Alert } from '$lib/types';
  import { formatPrice, timeAgo } from '$lib/utils';
  import LoadingState from '$lib/components/LoadingState.svelte';
  import ErrorState from '$lib/components/ErrorState.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import { t, translate, currentLocale } from '$lib/i18n';

  let alerts: Alert[] = $state([]);
  let initialLoading = $state(true);
  let refreshing = $state(false);
  let error = $state('');
  let currentPage = $state(1);
  let totalPages = $state(1);
  const pageSize = 20;
  let dismissed = $state<Set<string>>(new Set());

  const DISMISS_KEY = 'purrce_dismissed_alerts';

  const triggered = $derived(alerts.filter(a => a.triggered_at));
  const visibleAlerts = $derived(triggered.filter(a => !dismissed.has(a.id)));

  onMount(async () => {
    try {
      const raw = localStorage.getItem(DISMISS_KEY);
      if (raw) dismissed = new Set(JSON.parse(raw));
    } catch {}
    await loadAlerts(true);
  });

  function dismiss(id: string) {
    dismissed = new Set(dismissed).add(id);
    try { localStorage.setItem(DISMISS_KEY, JSON.stringify([...dismissed])); } catch {}
  }

  async function loadAlerts(initial = false) {
    if (initial) initialLoading = true;
    else refreshing = true;
    error = '';
    try {
      const res = await api.alerts.list(currentPage, pageSize);
      alerts = res.data;
      totalPages = res.total_pages;
    } catch (e) {
      error = e instanceof Error ? e.message : translate(currentLocale(), 'notifications.loadError');
    } finally {
      initialLoading = false;
      refreshing = false;
    }
  }

  function handlePageChange(page: number) {
    currentPage = page;
    loadAlerts();
  }

  function getAlertTypeLabel(type: string): string {
    switch (type) {
      case 'PRICE_DECREASE': return translate(currentLocale(), 'notifications.type.priceDecrease');
      case 'BELOW_PRICE': return translate(currentLocale(), 'notifications.type.belowPrice');
      case 'HISTORICAL_LOW': return translate(currentLocale(), 'notifications.type.historicalLow');
      case 'BACK_IN_STOCK': return translate(currentLocale(), 'notifications.type.backInStock');
      case 'PRICE_INCREASE': return translate(currentLocale(), 'notifications.type.priceIncrease');
      default: return type;
    }
  }

  function getAlertHeadline(type: string): string {
    switch (type) {
      case 'HISTORICAL_LOW': return translate(currentLocale(), 'notifications.headline.historicalLow');
      case 'PRICE_DECREASE': return translate(currentLocale(), 'notifications.headline.priceDecrease');
      case 'BELOW_PRICE': return translate(currentLocale(), 'notifications.headline.belowPrice');
      case 'BACK_IN_STOCK': return translate(currentLocale(), 'notifications.headline.backInStock');
      case 'PRICE_INCREASE': return translate(currentLocale(), 'notifications.headline.priceIncrease');
      default: return translate(currentLocale(), 'notifications.headline.default');
    }
  }

  function getAlertAction(type: string): string {
    return type === 'PRICE_INCREASE' ? translate(currentLocale(), 'notifications.action.viewProduct') : translate(currentLocale(), 'notifications.action.buyNow');
  }

  function getAlertColor(type: string): { text: string; bg: string } {
    switch (type) {
      case 'PRICE_DECREASE':
      case 'HISTORICAL_LOW':
      case 'BELOW_PRICE': return { text: 'text-[var(--success)]', bg: 'bg-[var(--success-subtle)]' };
      case 'BACK_IN_STOCK': return { text: 'text-[var(--info)]', bg: 'bg-[var(--info-subtle)]' };
      default: return { text: 'text-[var(--fg-muted)]', bg: 'bg-[var(--bg-subtle)]' };
    }
  }
</script>

<svelte:head>
  <title>{$t('notifications.pageTitle')}</title>
</svelte:head>

<div class="space-y-5">
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div>
      <h1 class="page-title">{$t('notifications.title')}</h1>
      <p class="text-[13px] text-[var(--fg-muted)] mt-0.5">{$t('notifications.subtitle')}</p>
    </div>
    <a href="/settings" class="btn btn-secondary">
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
      {$t('notifications.channels')}
    </a>
  </div>

  {#if initialLoading}
    <LoadingState />
  {:else if error}
    <ErrorState message={error} retry={() => loadAlerts(true)} />
  {:else if visibleAlerts.length === 0}
    <div class="flex flex-col items-center justify-center py-20 text-center">
      <div class="w-12 h-12 rounded-[var(--radius-lg)] bg-[var(--bg-subtle)] border border-[var(--border)] flex items-center justify-center mb-5">
        <svg class="w-6 h-6 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
      </div>
      <h3 class="title-md mb-1.5">{$t('notifications.empty.title')}</h3>
      <p class="text-[13px] text-[var(--fg-muted)] max-w-[300px] leading-relaxed">
        {$t(triggered.length > 0 ? 'notifications.empty.allSeen' : 'notifications.empty.awaiting')}
      </p>
    </div>
  {:else}
    <div class="space-y-2 {refreshing ? 'refreshing' : ''}">
      {#each visibleAlerts as alert, i (alert.id)}
        {@const colors = getAlertColor(alert.alert_type)}
        <div class="card p-4 animate-in" style="animation-delay: {Math.min(i, 8) * 30}ms">
          <div class="flex items-start gap-4">
            {#if alert.product_image}
              <img src={alert.product_image} alt={alert.product_name || ''} class="w-11 h-11 rounded-[var(--radius-md)] bg-[var(--bg-subtle)] object-contain p-0.5 flex-shrink-0" loading="lazy" />
            {:else}
              <div class="w-11 h-11 rounded-[var(--radius-md)] bg-[var(--bg-subtle)] flex items-center justify-center flex-shrink-0">
                <svg class="w-5 h-5 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
                </svg>
              </div>
            {/if}

            <div class="flex-1 min-w-0">
              <div class="flex flex-wrap items-center gap-2 mb-1">
                <span class="text-[11px] font-semibold px-1.5 py-0.5 rounded-[var(--radius-sm)] {colors.text} {colors.bg}">
                  {getAlertTypeLabel(alert.alert_type)}
                </span>
                {#if alert.store_name}
                  <span class="text-[11px] text-[var(--fg-muted)]">{alert.store_name}</span>
                {/if}
                <span class="text-[11px] text-[var(--fg-muted)]">· {timeAgo(alert.triggered_at!)}</span>
              </div>
              <p class="text-[14px] font-semibold truncate">{getAlertHeadline(alert.alert_type)}</p>
              <p class="text-[13px] text-[var(--fg-secondary)] truncate mt-0.5">{alert.product_name || $t('table.product')}</p>
              {#if alert.threshold_price}
                <p class="num text-[12px] text-[var(--fg-muted)] mt-1">
                  {$t('notifications.targetReached')} <span class="font-semibold text-[var(--fg)]">{formatPrice(alert.threshold_price, 'CLP')}</span>
                </p>
              {/if}
            </div>
          </div>

          <div class="flex items-center justify-end gap-2 mt-3 pt-3 border-t border-[var(--border-subtle)]">
            <button
              onclick={() => dismiss(alert.id)}
              class="btn btn-ghost btn-sm"
            >
              {$t('notifications.seen')}
            </button>
            {#if alert.offer_url}
              <a
                href={alert.offer_url}
                target="_blank"
                rel="noopener noreferrer"
                class="btn btn-primary btn-sm"
              >
                {getAlertAction(alert.alert_type)}
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3" />
                </svg>
              </a>
            {/if}
          </div>
        </div>
      {/each}
    </div>

    <Pagination {currentPage} {totalPages} loading={refreshing} onPageChange={handlePageChange} />
  {/if}
</div>
