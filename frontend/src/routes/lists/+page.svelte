<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import type { ProductList, ListWithTotal, Product } from '$lib/types';
  import { formatPrice } from '$lib/utils';
  import LoadingState from '$lib/components/LoadingState.svelte';
  import ErrorState from '$lib/components/ErrorState.svelte';
  import { t, translate, currentLocale } from '$lib/i18n';

  let lists: ProductList[] = $state([]);
  let selected: ListWithTotal | null = $state(null);
  let initialLoading = $state(true);
  let refreshing = $state(false);
  let error = $state('');
  let showNew = $state(false);
  let newName = $state('');
  let creating = $state(false);
  let loadingList = $state(false);
  let showAddProduct = $state(false);
  let allProducts: Product[] = $state([]);
  let addingProduct = $state<string | null>(null);
  let deletingListId = $state<string | null>(null);
  let removingItem = $state<string | null>(null);
  let productSearch = $state('');
  let updatingQuantity = $state<string | null>(null);

  let renamingId = $state<string | null>(null);
  let renameValue = $state('');

  onMount(async () => { await load(true); });

  async function load(initial = false) {
    if (initial) initialLoading = true;
    else refreshing = true;
    error = '';
    try {
      lists = await api.lists.list();
    } catch (e) {
      error = e instanceof Error ? e.message : translate(currentLocale(), 'lists.loadError');
    } finally {
      initialLoading = false;
      refreshing = false;
    }
  }

  async function selectList(id: string) {
    loadingList = true;
    renamingId = null;
    try {
      selected = await api.lists.get(id);
    } catch (e) {
      console.error(e);
    } finally {
      loadingList = false;
    }
  }

  async function createList() {
    if (!newName.trim()) return;
    creating = true;
    try {
      const list = await api.lists.create(newName.trim());
      lists = [...lists, list];
      newName = '';
      showNew = false;
      await selectList(list.id);
    } catch (e) {
      console.error(e);
    } finally {
      creating = false;
    }
  }

  async function deleteList(id: string) {
    deletingListId = id;
    try {
      await api.lists.delete(id);
      lists = lists.filter(l => l.id !== id);
      if (selected?.list.id === id) selected = null;
    } catch (e) {
      console.error(e);
    } finally {
      deletingListId = null;
    }
  }

  function startRename(list: ProductList) {
    renamingId = list.id;
    renameValue = list.name;
  }

  async function commitRename(id: string) {
    if (!renameValue.trim() || renameValue.trim() === lists.find(l => l.id === id)?.name) {
      renamingId = null;
      return;
    }
    try {
      const updated = await api.lists.rename(id, renameValue.trim());
      lists = lists.map(l => l.id === id ? updated : l);
      if (selected?.list.id === id) selected = { ...selected, list: updated };
    } catch (e) {
      console.error(e);
    } finally {
      renamingId = null;
    }
  }

  async function openAddProduct() {
    try {
      allProducts = await api.products.listAll();
    } catch {}
    productSearch = '';
    showAddProduct = true;
  }

  async function addProduct(productId: string) {
    if (!selected) return;
    addingProduct = productId;
    try {
      await api.lists.addItem(selected.list.id, productId);
      selected = await api.lists.get(selected.list.id);
    } catch (e) {
      console.error(e);
    } finally {
      addingProduct = null;
    }
  }

  async function updateQuantity(productId: string, qty: number) {
    if (!selected || qty < 1) return;
    updatingQuantity = productId;
    try {
      await api.lists.updateQuantity(selected.list.id, productId, qty);
      selected = await api.lists.get(selected.list.id);
    } catch (e) {
      console.error(e);
    } finally {
      updatingQuantity = null;
    }
  }

  async function removeItem(productId: string) {
    if (!selected) return;
    removingItem = productId;
    try {
      await api.lists.removeItem(selected.list.id, productId);
      selected = await api.lists.get(selected.list.id);
    } catch (e) {
      console.error(e);
    } finally {
      removingItem = null;
    }
  }

  const alreadyInList = $derived(new Set(selected?.items.map(i => i.product_id) ?? []));
  const filteredProducts = $derived(
    productSearch.trim()
      ? allProducts.filter(p => p.name.toLowerCase().includes(productSearch.toLowerCase()))
      : allProducts
  );
</script>

<svelte:head>
  <title>{$t('lists.pageTitle')}</title>
</svelte:head>

<div class="space-y-5">
  <div class="flex items-center justify-between gap-3">
    <div>
      <h1 class="page-title">{$t('nav.lists')}</h1>
      <p class="text-[13px] text-[var(--fg-muted)] mt-0.5">{$t('lists.subtitle')}</p>
    </div>
    <button onclick={() => showNew = true} class="btn btn-primary">
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.2" d="M12 4v16m8-8H4" />
      </svg>
      {$t('lists.new')}
    </button>
  </div>

  {#if initialLoading}
    <LoadingState />
  {:else if error}
    <ErrorState message={error} retry={() => load(true)} />
  {:else if lists.length === 0}
    <div class="flex flex-col items-center justify-center py-20 text-center gap-4">
      <div class="w-14 h-14 rounded-[var(--radius-lg)] bg-[var(--bg-subtle)] border border-[var(--border)] flex items-center justify-center">
        <svg class="w-7 h-7 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
        </svg>
      </div>
      <div>
        <p class="font-semibold">{$t('lists.emptyTitle')}</p>
        <p class="text-[13px] text-[var(--fg-muted)] mt-1 max-w-xs">{$t('lists.emptyMessage')}</p>
      </div>
      <button onclick={() => showNew = true} class="btn btn-primary">{$t('lists.createFirst')}</button>
    </div>
  {:else}
    <div class="grid grid-cols-1 sm:grid-cols-[240px_1fr] gap-4 {refreshing ? 'refreshing' : ''}">
      <!-- Sidebar -->
      <div class="space-y-1">
        {#each lists as list, i (list.id)}
          <div
            class="group flex items-center gap-1 rounded-[var(--radius-md)] border transition-colors animate-in
              {selected?.list.id === list.id
                ? 'bg-[var(--accent-subtle)] border-[var(--accent)]/40 text-[var(--accent)]'
                : 'border-transparent hover:bg-[var(--bg-subtle)] text-[var(--fg-secondary)]'}"
            style="animation-delay: {Math.min(i, 8) * 25}ms"
          >
            {#if renamingId === list.id}
              <input
                type="text"
                bind:value={renameValue}
                class="flex-1 px-3 py-2.5 bg-transparent text-[13px] font-medium outline-none"
                onblur={() => commitRename(list.id)}
                onkeydown={(e) => {
                  if (e.key === 'Enter') commitRename(list.id);
                  if (e.key === 'Escape') renamingId = null;
                }}
                onclick={(e) => e.stopPropagation()}
                autofocus
              />
            {:else}
              <span
                class="flex-1 px-3 py-2.5 text-[13px] font-medium truncate min-w-0"
                onclick={() => selectList(list.id)}
                role="button"
                tabindex="0"
                onkeydown={(e) => e.key === 'Enter' && selectList(list.id)}
              >{list.name}</span>
              <button
                onclick={(e) => { e.stopPropagation(); startRename(list); }}
                class="opacity-0 group-hover:opacity-100 p-1.5 rounded-[var(--radius-sm)] hover:bg-[var(--bg-subtle)] transition-all"
                title={$t('lists.rename')}
              >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                </svg>
              </button>
              <button
                onclick={(e) => { e.stopPropagation(); deleteList(list.id); }}
                disabled={deletingListId === list.id}
                class="opacity-0 group-hover:opacity-100 p-1.5 mr-1 rounded-[var(--radius-sm)] hover:bg-[var(--danger-subtle)] hover:text-[var(--danger)] transition-all disabled:opacity-50"
                title={$t('lists.deleteTitle')}
              >
                {#if deletingListId === list.id}
                  <svg class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/>
                  </svg>
                {:else}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                {/if}
              </button>
            {/if}
          </div>
        {/each}
      </div>

      <!-- Detalle -->
      <div>
        {#if loadingList}
          <LoadingState />
        {:else if selected}
          <div class="card overflow-hidden">
            <div class="p-4 border-b border-[var(--border)] flex items-center justify-between gap-3 min-w-0">
              <h2 class="title-md truncate min-w-0 flex-1">{selected.list.name}</h2>
              <button
                onclick={openAddProduct}
                class="inline-flex items-center gap-1.5 px-3 py-1.5 text-[12px] font-medium border border-dashed border-[var(--border)] rounded-[var(--radius-md)] text-[var(--fg-muted)] hover:text-[var(--fg)] hover:border-[var(--border-strong)] transition-colors"
              >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                </svg>
                {$t('dash.addProduct')}
              </button>
            </div>

            {#if selected.items.length === 0}
              <div class="flex flex-col items-center gap-3 p-10 text-center">
                <p class="text-[13px] text-[var(--fg-muted)]">{$t('lists.empty')}</p>
                <button onclick={openAddProduct} class="text-[13px] text-[var(--accent)] hover:underline">{$t('dash.addFirstProduct')} →</button>
              </div>
            {:else}
              <div class="divide-y divide-[var(--border)]">
                {#each selected.items as item}
                  <div class="flex items-center gap-3 p-3 hover:bg-[var(--bg-subtle)] transition-colors overflow-hidden">
                    <div class="flex-shrink-0 w-10 h-10 rounded-[var(--radius-md)] bg-[var(--bg-subtle)] border border-[var(--border)] overflow-hidden flex items-center justify-center">
                      {#if item.image_url}
                        <img src={item.image_url} alt={item.product_name} class="w-full h-full object-contain p-0.5" loading="lazy" />
                      {:else}
                        <svg class="w-5 h-5 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
                        </svg>
                      {/if}
                    </div>

                    <div class="flex-1 min-w-0">
                      <a href="/products/{item.product_id}" class="text-[13px] font-medium hover:text-[var(--accent)] transition-colors truncate block">
                        {item.product_name}
                      </a>
                      {#if item.best_price}
                        <p class="num text-[12px] text-[var(--fg-muted)]">
                          {formatPrice(item.best_price, item.currency || 'CLP')} {$t('lists.each')}
                          {#if item.quantity > 1}
                            · <span class="font-medium text-[var(--fg)]">{formatPrice(item.best_price * item.quantity, item.currency || 'CLP')}</span>
                          {/if}
                        </p>
                      {:else}
                        <p class="text-[12px] text-[var(--fg-muted)]">{$t('table.noPrice')}</p>
                      {/if}
                    </div>

                    <div class="flex items-center gap-1 flex-shrink-0">
                      <button
                        onclick={() => updateQuantity(item.product_id, item.quantity - 1)}
                        disabled={item.quantity <= 1 || updatingQuantity === item.product_id}
                        class="w-6 h-6 rounded-[var(--radius-sm)] flex items-center justify-center border border-[var(--border)] text-[var(--fg-muted)] hover:text-[var(--fg)] hover:border-[var(--border-strong)] disabled:opacity-30 transition-colors text-[13px] font-medium"
                      >−</button>
                      <span class="num w-6 text-center text-[13px] font-medium">
                        {updatingQuantity === item.product_id ? '…' : item.quantity}
                      </span>
                      <button
                        onclick={() => updateQuantity(item.product_id, item.quantity + 1)}
                        disabled={updatingQuantity === item.product_id}
                        class="w-6 h-6 rounded-[var(--radius-sm)] flex items-center justify-center border border-[var(--border)] text-[var(--fg-muted)] hover:text-[var(--fg)] hover:border-[var(--border-strong)] disabled:opacity-30 transition-colors text-[13px] font-medium"
                      >+</button>
                    </div>

                    <button
                      onclick={() => removeItem(item.product_id)}
                      disabled={removingItem === item.product_id}
                      class="p-1.5 rounded-[var(--radius-sm)] hover:bg-[var(--danger-subtle)] hover:text-[var(--danger)] transition-colors text-[var(--fg-muted)] disabled:opacity-50 flex-shrink-0"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                      </svg>
                    </button>
                  </div>
                {/each}
              </div>

              <div class="p-4 border-t border-[var(--border)] flex items-center justify-between bg-[var(--bg-subtle)]">
                <span class="text-[13px] text-[var(--fg-muted)]">
                  {$t('lists.units', { count: selected.items.reduce((s, i) => s + i.quantity, 0) })} · {selected.items.length === 1 ? $t('lists.productCountOne', { count: selected.items.length }) : $t('lists.productCountMany', { count: selected.items.length })}
                </span>
                <div class="text-right">
                  <p class="text-[11px] text-[var(--fg-muted)]">{$t('lists.totalCost')}</p>
                  <p class="num text-[16px] font-semibold">{formatPrice(selected.total_cost, selected.items[0]?.currency || 'CLP')}</p>
                </div>
              </div>
            {/if}
          </div>
        {:else}
          <div class="flex items-center justify-center h-40 text-[13px] text-[var(--fg-muted)]">
            {$t('lists.selectPrompt')}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<!-- Modal: nueva lista -->
{#if showNew}
  <div class="fixed inset-0 m3-scrim flex items-center justify-center z-50 p-4" onclick={() => showNew = false}>
    <div class="m3-dialog w-full max-w-sm p-5 animate-in" onclick={(e) => e.stopPropagation()}>
      <h2 class="title-md mb-4">{$t('lists.new')}</h2>
      <form onsubmit={(e) => { e.preventDefault(); createList(); }} class="space-y-4">
        <input type="text" bind:value={newName} placeholder={$t('lists.namePlaceholder')} class="input" autofocus />
        <div class="flex gap-2 justify-end">
          <button type="button" onclick={() => showNew = false} class="btn btn-ghost">{$t('dash.cancel')}</button>
          <button type="submit" disabled={!newName.trim() || creating} class="btn btn-primary">
            {creating ? $t('lists.creating') : $t('lists.create')}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Modal: agregar producto -->
{#if showAddProduct && selected}
  <div class="fixed inset-0 m3-scrim flex items-center justify-center z-50 p-4" onclick={() => showAddProduct = false}>
    <div class="m3-dialog w-full max-w-md p-5 flex flex-col gap-4 animate-in" onclick={(e) => e.stopPropagation()}>
      <h2 class="title-md">{$t('lists.addTo', { name: selected.list.name })}</h2>

      <div class="relative">
        <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <input type="text" bind:value={productSearch} placeholder={$t('lists.searchPlaceholder')} class="input pl-9" autofocus />
      </div>

      <div class="max-h-64 overflow-y-auto space-y-0.5 -mx-1 px-1">
        {#if filteredProducts.length === 0}
          <p class="text-[13px] text-[var(--fg-muted)] text-center py-6">{$t('lists.noResults')}</p>
        {:else}
          {#each filteredProducts as product}
            {@const inList = alreadyInList.has(product.id)}
            <button
              onclick={() => !inList && addProduct(product.id)}
              disabled={inList || addingProduct === product.id}
              class="w-full flex items-center gap-3 px-3 py-2 rounded-[var(--radius-md)] text-left transition-colors min-w-0
                {inList ? 'opacity-50 cursor-not-allowed' : 'hover:bg-[var(--bg-subtle)]'}"
            >
              <div class="w-8 h-8 flex-shrink-0 rounded-[var(--radius-sm)] bg-[var(--bg-subtle)] border border-[var(--border)] flex items-center justify-center overflow-hidden">
                {#if product.image_url}
                  <img src={product.image_url} alt={product.name} class="w-full h-full object-contain p-0.5" />
                {:else}
                  <svg class="w-4 h-4 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
                  </svg>
                {/if}
              </div>
              <span class="text-[13px] font-medium flex-1 truncate min-w-0">{product.name}</span>
              {#if inList}
                <span class="text-[11px] text-[var(--success)] font-medium flex-shrink-0">{$t('lists.inList')}</span>
              {:else if addingProduct === product.id}
                <span class="text-[11px] text-[var(--fg-muted)] flex-shrink-0">{$t('lists.adding')}</span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>

      <div class="flex justify-end border-t border-[var(--border)] pt-3">
        <button onclick={() => showAddProduct = false} class="btn btn-ghost">{$t('lists.close')}</button>
      </div>
    </div>
  </div>
{/if}
