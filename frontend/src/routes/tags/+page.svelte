<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import type { TagWithUsage } from '$lib/types';
  import LoadingState from '$lib/components/LoadingState.svelte';
  import ErrorState from '$lib/components/ErrorState.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import { t, translate, currentLocale } from '$lib/i18n';

  const COLORS = [
    '#e8590c', '#15803d', '#1d4ed8', '#b45309', '#b91c1c',
    '#7c3aed', '#0d9488', '#db2777', '#6366f1', '#57534e'
  ];

  let tags: TagWithUsage[] = $state([]);
  let initialLoading = $state(true);
  let refreshing = $state(false);
  let error = $state('');
  let search = $state('');

  let showCreate = $state(false);
  let newName = $state('');
  let newColor = $state(COLORS[0]);
  let creating = $state(false);
  let createError = $state('');

  let editingId = $state<string | null>(null);
  let editName = $state('');
  let editColor = $state('');
  let savingId = $state<string | null>(null);

  let deleteTarget = $state<TagWithUsage | null>(null);
  let showDelete = $state(false);
  let deleting = $state(false);

  let mergeSource = $state<TagWithUsage | null>(null);
  let showMerge = $state(false);
  let mergeTargetId = $state('');
  let merging = $state(false);

  const filtered = $derived(
    tags.filter(t => t.name.toLowerCase().includes(search.trim().toLowerCase()))
  );
  const unusedCount = $derived(tags.filter(t => t.product_count === 0).length);
  const mergeCandidates = $derived.by(() => {
    const source = mergeSource;
    if (!source) return [];
    return tags.filter(t => t.id !== source.id);
  });

  onMount(() => loadTags(true));

  async function loadTags(initial = false) {
    if (initial) initialLoading = true;
    else refreshing = true;
    error = '';
    try {
      tags = await api.tags.listWithUsage();
    } catch (e) {
      error = e instanceof Error ? e.message : translate(currentLocale(), 'tags.loadError');
    } finally {
      initialLoading = false;
      refreshing = false;
    }
  }

  async function createTag() {
    const name = newName.trim();
    if (!name) return;
    creating = true;
    createError = '';
    try {
      await api.tags.create(name, newColor);
      newName = '';
      newColor = COLORS[0];
      showCreate = false;
      await loadTags();
    } catch (e) {
      createError = e instanceof Error ? e.message : translate(currentLocale(), 'tags.createError');
    } finally {
      creating = false;
    }
  }

  function startEdit(tag: TagWithUsage) {
    editingId = tag.id;
    editName = tag.name;
    editColor = tag.color;
  }

  function cancelEdit() {
    editingId = null;
    editName = '';
    editColor = '';
  }

  async function saveEdit(id: string) {
    const name = editName.trim();
    if (!name) return;
    savingId = id;
    try {
      await api.tags.update(id, { name, color: editColor });
      await loadTags();
      cancelEdit();
    } catch (e) {
      alert(e instanceof Error ? e.message : translate(currentLocale(), 'tags.saveError'));
    } finally {
      savingId = null;
    }
  }

  function requestDelete(tag: TagWithUsage) {
    deleteTarget = tag;
    showDelete = true;
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    deleting = true;
    try {
      await api.tags.delete(deleteTarget.id);
      showDelete = false;
      deleteTarget = null;
      await loadTags();
    } catch (e) {
      alert(e instanceof Error ? e.message : translate(currentLocale(), 'tags.deleteError'));
    } finally {
      deleting = false;
    }
  }

  function openMerge(tag: TagWithUsage) {
    mergeSource = tag;
    mergeTargetId = '';
    showMerge = true;
  }

  async function confirmMerge() {
    if (!mergeSource || !mergeTargetId) return;
    merging = true;
    try {
      await api.tags.merge(mergeSource.id, mergeTargetId);
      showMerge = false;
      mergeSource = null;
      mergeTargetId = '';
      await loadTags();
    } catch (e) {
      alert(e instanceof Error ? e.message : translate(currentLocale(), 'tags.mergeError'));
    } finally {
      merging = false;
    }
  }
</script>

<svelte:head>
  <title>{$t('tags.pageTitle')}</title>
</svelte:head>

<div class="space-y-5">
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div>
      <h1 class="page-title">{$t('nav.tags')}</h1>
      <p class="text-[13px] text-[var(--fg-muted)] mt-0.5">
        {$t('tags.count', { count: tags.length })}{#if unusedCount > 0} · {$t('tags.unusedCount', { count: unusedCount })}{/if}
      </p>
    </div>
    <button onclick={() => { showCreate = true; createError = ''; }} class="btn btn-primary">
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.2" d="M12 4v16m8-8H4" />
      </svg>
      {$t('tags.new')}
    </button>
  </div>

  <div class="relative">
    <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
    </svg>
    <input type="text" bind:value={search} placeholder={$t('tags.searchPlaceholder')} class="input pl-9 sm:w-72" />
  </div>

  {#if initialLoading}
    <LoadingState />
  {:else if error}
    <ErrorState message={error} retry={() => loadTags(true)} />
  {:else if tags.length === 0}
    <div class="card p-10 text-center">
      <div class="w-12 h-12 mx-auto mb-4 rounded-full bg-[var(--accent-subtle)] flex items-center justify-center">
        <svg class="w-6 h-6 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 7h.01M7 3h5a2 2 0 011.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A2 2 0 013 12V7a4 4 0 014-4z" />
        </svg>
      </div>
      <h3 class="font-semibold mb-1">{$t('tags.emptyTitle')}</h3>
      <p class="text-[13px] text-[var(--fg-muted)]">{$t('tags.emptyMessage')}</p>
    </div>
  {:else if filtered.length === 0}
    <div class="card p-8 text-center">
      <p class="text-[13px] text-[var(--fg-muted)]">{$t('tags.noResults', { query: search })}</p>
    </div>
  {:else}
    <div class="card overflow-hidden divide-y divide-[var(--border)] {refreshing ? 'refreshing' : ''}">
      {#each filtered as tag, i (tag.id)}
        <div class="p-3.5 sm:p-4 animate-in" style="animation-delay: {Math.min(i, 8) * 25}ms">
          {#if editingId === tag.id}
            <div class="flex flex-col sm:flex-row sm:items-center gap-3">
              <div class="flex items-center gap-3 flex-1 min-w-0">
                <span class="w-8 h-8 rounded-[var(--radius-md)] flex-shrink-0 border border-[var(--border)]" style="background-color: {editColor}20">
                  <span class="block w-3 h-3 rounded-full m-[10px]" style="background-color: {editColor}"></span>
                </span>
                <input
                  type="text"
                  bind:value={editName}
                  onkeydown={(e) => { if (e.key === 'Enter') saveEdit(tag.id); if (e.key === 'Escape') cancelEdit(); }}
                  class="input flex-1"
                  autofocus
                />
              </div>
              <div class="flex items-center gap-1.5 flex-wrap">
                {#each COLORS as c}
                  <button
                    type="button"
                    onclick={() => editColor = c}
                    class="w-6 h-6 rounded-full border-2 transition-transform hover:scale-110 {editColor === c ? 'border-[var(--fg)]' : 'border-transparent'}"
                    style="background-color: {c}"
                    aria-label={$t('tags.colorAria', { color: c })}
                  ></button>
                {/each}
              </div>
              <div class="flex items-center gap-2">
                <button onclick={cancelEdit} class="btn btn-ghost btn-sm">{$t('dash.cancel')}</button>
                <button onclick={() => saveEdit(tag.id)} disabled={savingId === tag.id || !editName.trim()} class="btn btn-primary btn-sm">
                  {savingId === tag.id ? $t('tags.saving') : $t('tags.save')}
                </button>
              </div>
            </div>
          {:else}
            <div class="flex items-center gap-3">
              <span class="w-8 h-8 rounded-[var(--radius-md)] flex-shrink-0 border border-[var(--border)]" style="background-color: {tag.color}20">
                <span class="block w-3 h-3 rounded-full m-[10px]" style="background-color: {tag.color}"></span>
              </span>

              <div class="flex-1 min-w-0">
                <p class="text-[14px] font-medium truncate">{tag.name}</p>
                <p class="text-[11px] text-[var(--fg-muted)] mt-0.5">
                  {#if tag.product_count === 0}
                    {$t('tags.unused')}
                  {:else}
                    {tag.product_count === 1 ? $t('tags.productOne', { count: tag.product_count }) : $t('tags.productMany', { count: tag.product_count })}
                  {/if}
                </p>
              </div>

              <div class="flex items-center gap-1 flex-shrink-0">
                <button onclick={() => startEdit(tag)} class="btn btn-ghost btn-sm" title={$t('tags.edit')}>
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                  </svg>
                  {$t('tags.edit')}
                </button>
                {#if tags.length > 1}
                  <button onclick={() => openMerge(tag)} class="btn btn-ghost btn-sm" title={$t('tags.mergeWith')}>
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
                    </svg>
                    {$t('tags.merge')}
                  </button>
                {/if}
                <button
                  onclick={() => requestDelete(tag)}
                  class="p-1.5 rounded-[var(--radius-md)] text-[var(--fg-muted)] hover:text-[var(--danger)] hover:bg-[var(--danger-subtle)] transition-colors"
                  title={$t('table.delete')}
                >
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Crear tag -->
{#if showCreate}
  <div class="fixed inset-0 m3-scrim flex items-center justify-center z-50 p-4" onclick={(e) => { if (e.target === e.currentTarget) showCreate = false; }}>
    <div class="m3-dialog w-full max-w-[400px] p-5 animate-in">
      <h2 class="title-md mb-4">{$t('tags.new')}</h2>
      <form onsubmit={(e) => { e.preventDefault(); createTag(); }} class="space-y-4">
        <div>
          <label for="tag-name" class="block text-[12px] font-medium text-[var(--fg-secondary)] mb-1.5">{$t('tags.name')}</label>
          <input id="tag-name" type="text" bind:value={newName} placeholder={$t('tags.namePlaceholder')} class="input" required autofocus />
        </div>
        <div>
          <p class="block text-[12px] font-medium text-[var(--fg-secondary)] mb-2">{$t('tags.color')}</p>
          <div class="flex flex-wrap gap-2">
            {#each COLORS as c}
              <button
                type="button"
                onclick={() => newColor = c}
                class="w-7 h-7 rounded-full border-2 transition-transform hover:scale-110 {newColor === c ? 'border-[var(--fg)]' : 'border-transparent'}"
                style="background-color: {c}"
                aria-label={$t('tags.colorAria', { color: c })}
              ></button>
            {/each}
          </div>
        </div>
        {#if createError}
          <div class="p-3 rounded-[var(--radius-md)] bg-[var(--danger-subtle)] border border-[var(--danger)]/20">
            <p class="text-[13px] text-[var(--danger)]">{createError}</p>
          </div>
        {/if}
        <div class="flex justify-end gap-2">
          <button type="button" onclick={() => showCreate = false} class="btn btn-ghost">{$t('dash.cancel')}</button>
          <button type="submit" disabled={creating || !newName.trim()} class="btn btn-primary">
            {creating ? $t('tags.creating') : $t('tags.create')}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Fusionar tag -->
{#if showMerge && mergeSource}
  <div class="fixed inset-0 m3-scrim flex items-center justify-center z-50 p-4" onclick={(e) => { if (e.target === e.currentTarget) showMerge = false; }}>
    <div class="m3-dialog w-full max-w-[420px] p-5 animate-in">
      <h2 class="title-md mb-1">{$t('tags.mergeTitle')}</h2>
      <p class="text-[13px] text-[var(--fg-muted)] mb-4">
        {$t('tags.mergeMessageBefore', { count: mergeSource.product_count })} <span class="font-medium text-[var(--fg)]">{mergeSource.name}</span> {$t('tags.mergeMessageAfter')}
      </p>
      <form onsubmit={(e) => { e.preventDefault(); confirmMerge(); }} class="space-y-4">
        <div>
          <label for="merge-target" class="block text-[12px] font-medium text-[var(--fg-secondary)] mb-1.5">{$t('tags.mergeInto')}</label>
          <select id="merge-target" bind:value={mergeTargetId} class="input" required>
            <option value="">{$t('tags.mergeSelect')}</option>
            {#each mergeCandidates as t}
              <option value={t.id}>{t.name} ({t.product_count})</option>
            {/each}
          </select>
        </div>
        <div class="flex justify-end gap-2">
          <button type="button" onclick={() => showMerge = false} class="btn btn-ghost">{$t('dash.cancel')}</button>
          <button type="submit" disabled={merging || !mergeTargetId} class="btn btn-primary">
            {merging ? $t('tags.merging') : $t('tags.merge')}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<ConfirmDialog
  open={showDelete}
  title={$t('tags.deleteTitle')}
  message={deleteTarget?.product_count === 1
    ? $t('tags.deleteMessageOne', { name: deleteTarget?.name ?? '', count: deleteTarget?.product_count ?? 0 })
    : $t('tags.deleteMessageMany', { name: deleteTarget?.name ?? '', count: deleteTarget?.product_count ?? 0 })}
  confirmLabel={deleting ? $t('tags.deleting') : $t('dash.delete')}
  cancelLabel={$t('dash.cancel')}
  onconfirm={confirmDelete}
  oncancel={() => { showDelete = false; deleteTarget = null; }}
/>
