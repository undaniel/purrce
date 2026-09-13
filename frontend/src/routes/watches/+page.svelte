<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import type { Watch } from '$lib/types';
  import { formatPrice, getStatusLabel, timeAgo, titleCase } from '$lib/utils';
  import WatchListSkeleton from '$lib/components/WatchListSkeleton.svelte';
  import ErrorState from '$lib/components/ErrorState.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import { t, translate, currentLocale } from '$lib/i18n';

  let watches: Watch[] = $state([]);
  let initialLoading = $state(true);
  let refreshing = $state(false);
  let error = $state('');
  let togglingId = $state<string | null>(null);
  let deletingId = $state<string | null>(null);
  let checkingId = $state<string | null>(null);
  let resumingId = $state<string | null>(null);
  let currentPage = $state(1);
  let totalPages = $state(1);
  let totalWatches = $state(0);
  const pageSize = 10;

  function handlePageChange(page: number) {
    currentPage = page;
    loadWatches();
  }

  onMount(async () => {
    await loadWatches(true);
  });

  async function loadWatches(initial = false) {
    if (initial) initialLoading = true;
    else refreshing = true;
    error = '';
    try {
      const response = await api.watches.list(currentPage, pageSize);
      watches = response.data;
      totalPages = response.total_pages;
      totalWatches = response.total;
    } catch (e) {
      error = e instanceof Error ? e.message : translate(currentLocale(), 'watches.loadError');
    } finally {
      initialLoading = false;
      refreshing = false;
    }
  }

  async function toggleWatch(watch: Watch) {
    togglingId = watch.id;
    try {
      await api.watches.update(watch.id, { enabled: !watch.enabled });
      await loadWatches();
    } catch (e) {
      console.error('Error toggling watch:', e);
    } finally {
      togglingId = null;
    }
  }

  async function checkNow(id: string) {
    checkingId = id;
    try {
      await api.watches.check(id);
      await loadWatches();
    } catch (e) {
      console.error('Error checking watch:', e);
    } finally {
      checkingId = null;
    }
  }

  async function resumeWatch(id: string) {
    resumingId = id;
    try {
      await api.watches.resume(id);
      await loadWatches();
    } catch (e) {
      console.error('Error resuming watch:', e);
    } finally {
      resumingId = null;
    }
  }

  async function deleteWatch(id: string) {
    if (!confirm(translate(currentLocale(), 'watches.deleteConfirm'))) return;
    deletingId = id;
    try {
      await api.watches.delete(id);
      watches = watches.filter(w => w.id !== id);
    } catch (e) {
      console.error('Error deleting watch:', e);
    } finally {
      deletingId = null;
    }
  }

  function getStatusConfig(status: string) {
    switch (status) {
      case 'ACTIVE':
        return { color: 'text-[var(--success)]', bg: 'bg-[var(--success-subtle)]', dot: 'bg-[var(--success)]' };
      case 'CHECKING':
        return { color: 'text-[var(--accent)]', bg: 'bg-[var(--accent-subtle)]', dot: 'bg-[var(--accent)]' };
      case 'ERROR':
      case 'BLOCKED':
        return { color: 'text-[var(--danger)]', bg: 'bg-[var(--danger-subtle)]', dot: 'bg-[var(--danger)]' };
      case 'CAPTCHA_REQUIRED':
        return { color: 'text-[var(--warning)]', bg: 'bg-[var(--warning-subtle)]', dot: 'bg-[var(--warning)]' };
      default:
        return { color: 'text-[var(--fg-muted)]', bg: 'bg-[var(--bg-subtle)]', dot: 'bg-[var(--fg-muted)]' };
    }
  }
</script>

<svelte:head>
  <title>{$t('watches.pageTitle')}</title>
</svelte:head>

<div class="space-y-5">
  <div>
    <h1 class="page-title">{$t('nav.watches')}</h1>
    <p class="text-[13px] text-[var(--fg-muted)] mt-0.5">{$t('watches.subtitle', { count: totalWatches })}</p>
  </div>

  {#if initialLoading}
    <WatchListSkeleton rows={pageSize} />
  {:else if error}
    <ErrorState message={error} retry={() => loadWatches(true)} />
  {:else if watches.length === 0}
    <EmptyState
      title={$t('watches.emptyTitle')}
      message={$t('watches.emptyMessage')}
    />
  {:else}
    <div class="space-y-2 {refreshing ? 'refreshing' : ''}">
      {#each watches as watch, i (watch.id)}
        {@const statusConfig = getStatusConfig(watch.status)}
        <div class="card p-3.5 hover:border-[var(--border-strong)] transition-colors animate-in" style="animation-delay: {Math.min(i, 8) * 30}ms">
          <div class="flex items-center justify-between gap-3">
            <div class="flex items-center gap-3 min-w-0">
              <span class="w-2 h-2 rounded-full flex-shrink-0 {watch.enabled ? statusConfig.dot : 'bg-[var(--fg-muted)]'} {watch.status === 'CHECKING' ? 'animate-pulse' : ''}"></span>
              <div class="min-w-0">
                {#if watch.product_id}
                  <a
                    href={`/products/${watch.product_id}`}
                    class="block font-medium text-[13px] truncate hover:text-[var(--accent)] transition-colors"
                    title={watch.product_name}
                  >
                    {watch.product_name ?? $t('table.product')}
                  </a>
                {:else}
                  <p class="font-medium text-[13px] truncate">{$t('watches.fallbackName', { id: watch.id.slice(0, 8) })}</p>
                {/if}
                <p class="num text-[11px] text-[var(--fg-muted)] mt-0.5 flex items-center gap-1.5 flex-wrap">
                  {#if watch.store_name}
                    <span class="truncate">{titleCase(watch.store_name)}</span>
                    <span class="text-[var(--fg-faint)]">·</span>
                  {/if}
                  {#if watch.current_price != null && watch.current_price > 0}
                    <span class="text-[var(--fg)] font-medium">{formatPrice(watch.current_price, watch.currency)}</span>
                    <span class="text-[var(--fg-faint)]">·</span>
                  {/if}
                  {#if watch.availability === false}
                    <span class="text-[var(--danger)]">{$t('watches.outOfStock')}</span>
                    <span class="text-[var(--fg-faint)]">·</span>
                  {/if}
                  <span>{$t('watches.every', { hours: watch.interval_seconds / 3600 })}</span>
                  {#if watch.last_check_at}
                    <span class="text-[var(--fg-faint)]">·</span>
                    <span>{timeAgo(watch.last_check_at)}</span>
                  {/if}
                </p>
              </div>
            </div>
            <div class="flex items-center gap-2 flex-shrink-0">
              <span class="inline-flex items-center gap-1.5 px-2 py-1 rounded-[var(--radius-sm)] text-[11px] font-medium {statusConfig.color} {statusConfig.bg}">
                <span class="w-1.5 h-1.5 rounded-full {statusConfig.dot} {watch.status === 'CHECKING' || checkingId === watch.id ? 'animate-pulse' : ''}"></span>
                {checkingId === watch.id ? $t('watches.checking') : getStatusLabel(watch.status)}
              </span>
              {#if ['CAPTCHA_REQUIRED', 'BLOCKED', 'ERROR'].includes(watch.status)}
                <button
                  onclick={() => resumeWatch(watch.id)}
                  disabled={resumingId === watch.id}
                  class="px-2.5 py-1.5 rounded-[var(--radius-md)] text-[11px] font-medium bg-[var(--warning-subtle)] text-[var(--warning)] hover:bg-[var(--warning-muted)] transition-colors disabled:opacity-50"
                >
                  {resumingId === watch.id ? $t('watches.resuming') : $t('watches.resume')}
                </button>
              {/if}
              {#if watch.offer_url}
                <a
                  href={watch.offer_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  title={$t('watches.openInStore')}
                  class="p-1.5 rounded-[var(--radius-md)] text-[var(--fg-muted)] hover:text-[var(--accent)] hover:bg-[var(--accent-subtle)] transition-colors"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                  </svg>
                </a>
              {/if}
              <button
                onclick={() => checkNow(watch.id)}
                disabled={checkingId === watch.id || togglingId === watch.id || deletingId === watch.id}
                title={$t('watches.checkNow')}
                class="p-1.5 rounded-[var(--radius-md)] text-[var(--fg-muted)] hover:text-[var(--accent)] hover:bg-[var(--accent-subtle)] transition-colors disabled:opacity-50"
              >
                {#if checkingId === watch.id}
                  <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                  </svg>
                {:else}
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                  </svg>
                {/if}
              </button>
              <button
                onclick={() => toggleWatch(watch)}
                disabled={togglingId === watch.id || deletingId === watch.id || checkingId === watch.id}
                class="px-3 py-1.5 rounded-[var(--radius-md)] text-[11px] font-medium transition-colors disabled:opacity-50
                  {watch.enabled
                    ? 'bg-[var(--bg-subtle)] text-[var(--fg-muted)] hover:text-[var(--fg)]'
                    : 'bg-[var(--success-subtle)] text-[var(--success)] hover:bg-[var(--success-muted)]'}"
              >
                {#if togglingId === watch.id}
                  <svg class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                  </svg>
                {:else}
                  {watch.enabled ? $t('watches.pause') : $t('watches.activate')}
                {/if}
              </button>
              <button
                onclick={() => deleteWatch(watch.id)}
                disabled={togglingId === watch.id || deletingId === watch.id || checkingId === watch.id}
                class="p-1.5 rounded-[var(--radius-md)] text-[var(--fg-muted)] hover:text-[var(--danger)] hover:bg-[var(--danger-subtle)] transition-colors disabled:opacity-50"
                title={$t('table.delete')}
              >
                {#if deletingId === watch.id}
                  <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                  </svg>
                {:else}
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                {/if}
              </button>
            </div>
          </div>
        </div>
      {/each}
    </div>
    <Pagination {currentPage} {totalPages} loading={refreshing} onPageChange={handlePageChange} />
  {/if}
</div>
