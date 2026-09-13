<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import type { Product, Tag, PriceHistory, Offer } from '$lib/types';
  import ProductCard from '$lib/components/ProductCard.svelte';
  import ProductGridSkeleton from '$lib/components/ProductGridSkeleton.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import ErrorState from '$lib/components/ErrorState.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import { t, translate, currentLocale } from '$lib/i18n';

  let products: Product[] = $state([]);
  let priceHistories: Record<string, PriceHistory[]> = $state({});
  let allTags: Tag[] = $state([]);
  let initialLoading = $state(true);
  let refreshing = $state(false);
  let error = $state('');
  let currentPage = $state(1);
  let totalPages = $state(1);
  let totalProducts = $state(0);
  let searchQuery = $state('');
  let searchDebounce: ReturnType<typeof setTimeout> | null = null;
  let selectedTag = $state<string | null>(null);
  const pageSize = 8;

  let deletingId = $state<string | null>(null);
  let showDeleteConfirm = $state(false);
  let productToDelete = $state<Product | null>(null);

  onMount(async () => {
    await Promise.all([loadProducts(true), loadTags()]);
  });

  async function loadProducts(initial = false) {
    if (initial) initialLoading = true;
    else refreshing = true;
    error = '';
    try {
      const response = await api.products.list(
        currentPage,
        pageSize,
        searchQuery || undefined,
        selectedTag || undefined
      );
      products = response.data;
      totalPages = response.total_pages;
      totalProducts = response.total;
      await loadHistories();
    } catch (e) {
      error = e instanceof Error ? e.message : translate(currentLocale(), 'dash.loadError');
    } finally {
      initialLoading = false;
      refreshing = false;
    }
  }

  function getBestOffer(product: Product): Offer | null {
    if (product.offers.length === 0) return null;
    return product.offers.reduce((best, offer) =>
      offer.current_price > 0 && (best.current_price === 0 || offer.current_price < best.current_price) ? offer : best
    );
  }

  async function loadHistories() {
    const bestOffers = products.map(p => getBestOffer(p)).filter((o): o is Offer => !!o);
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

  async function loadTags() {
    try {
      allTags = await api.tags.list();
    } catch (e) {
      console.error('Error loading tags:', e);
    }
  }

  function handlePageChange(page: number) {
    currentPage = page;
    loadProducts();
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function handleSearch() {
    if (searchDebounce) clearTimeout(searchDebounce);
    searchDebounce = setTimeout(() => {
      currentPage = 1;
      loadProducts();
    }, 300);
  }

  function toggleTag(tagName: string) {
    selectedTag = selectedTag === tagName ? null : tagName;
    currentPage = 1;
    loadProducts();
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
      await loadProducts();
    } catch (e) {
      console.error('Error deleting product:', e);
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
  <title>{$t('products.pageTitle')}</title>
</svelte:head>

<div class="space-y-5">
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div>
      <h1 class="page-title">{$t('nav.products')}</h1>
      <p class="text-[13px] text-[var(--fg-muted)] mt-0.5">{$t('dash.productsMonitored', { count: totalProducts })}</p>
    </div>

    <div class="relative">
      <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
      </svg>
      <input
        type="text"
        bind:value={searchQuery}
        oninput={handleSearch}
        placeholder={$t('products.searchPlaceholder')}
        class="input pl-9 sm:w-72"
      />
    </div>
  </div>

  {#if allTags.length > 0}
    <div class="flex flex-wrap gap-2">
      <button
        onclick={() => toggleTag('')}
        class="px-2.5 py-1 rounded-full text-[12px] font-medium border transition-colors
          {!selectedTag
            ? 'bg-[var(--accent-subtle)] border-[var(--accent)]/40 text-[var(--accent)]'
            : 'bg-[var(--card)] border-[var(--border)] text-[var(--fg-muted)] hover:text-[var(--fg)] hover:border-[var(--border-strong)]'}"
      >
        {$t('products.allTags')}
      </button>
      {#each allTags as tag}
        <button
          onclick={() => toggleTag(tag.name)}
          class="px-2.5 py-1 rounded-full text-[12px] font-medium border transition-colors
            {selectedTag === tag.name
              ? 'border-transparent text-white'
              : 'bg-[var(--card)] border-[var(--border)] text-[var(--fg-muted)] hover:text-[var(--fg)] hover:border-[var(--border-strong)]'}"
          style={selectedTag === tag.name ? `background-color: ${tag.color}` : ''}
        >
          {tag.name}
        </button>
      {/each}
    </div>
  {/if}

  {#if initialLoading}
    <ProductGridSkeleton cards={pageSize} />
  {:else if error}
    <ErrorState message={error} retry={() => loadProducts(true)} />
  {:else if products.length === 0}
    <EmptyState
      title={$t('empty.title')}
      message={searchQuery && selectedTag
        ? $t('products.emptyFilteredSearchTag', { query: searchQuery, tag: selectedTag })
        : searchQuery
          ? $t('products.emptyFilteredSearch', { query: searchQuery })
          : selectedTag
            ? $t('products.emptyFilteredTag', { tag: selectedTag })
            : $t('products.emptyNoProducts')}
    />
  {:else}
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 {refreshing ? 'refreshing' : ''}">
      {#each products as product, i (product.id)}
        {@const best = getBestOffer(product)}
        <div class="relative group animate-in" style="animation-delay: {Math.min(i, 8) * 35}ms">
          <ProductCard {product} history={best ? priceHistories[best.id] : undefined} />
          <button
            onclick={(e) => { e.preventDefault(); requestDelete(product); }}
            disabled={deletingId === product.id}
            class="absolute top-2 right-2 p-1.5 rounded-[var(--radius-md)] bg-[var(--card)] border border-[var(--border)] text-[var(--fg-muted)] opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-all hover:text-[var(--danger)] hover:border-[var(--danger)]/40 disabled:opacity-70"
            title={$t('dash.deleteTitle')}
          >
            {#if deletingId === product.id}
              <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
            {:else}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
              </svg>
            {/if}
          </button>
        </div>
      {/each}
    </div>

    <Pagination
      {currentPage}
      {totalPages}
      loading={refreshing}
      onPageChange={handlePageChange}
    />
  {/if}
</div>

<ConfirmDialog
  open={showDeleteConfirm}
  title={$t('dash.deleteTitle')}
  message={$t('dash.deleteMessage', { name: productToDelete?.name ?? '' })}
  confirmLabel={$t('dash.delete')}
  cancelLabel={$t('dash.cancel')}
  onconfirm={confirmDelete}
  oncancel={cancelDelete}
/>
