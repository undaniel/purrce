<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { api } from '$lib/api/client';
  import type { Product, Tag } from '$lib/types';
  import { t, translate, currentLocale } from '$lib/i18n';
  import TagInput from './TagInput.svelte';

  interface Props {
    open: boolean;
    onclose: () => void;
    onsuccess: (product: Product) => void;
  }

  let { open, onclose, onsuccess }: Props = $props();
  let url = $state('');
  let loading = $state(false);
  let error = $state('');
  let watch = $state(true);
  let interval = $state(3600);
  let step = $state<'url' | 'manual'>('url');
  let pendingProduct = $state<Product | null>(null);
  let manualName = $state('');
  let manualPrice = $state('');
  let manualImage = $state('');
  let selectedTags: Tag[] = $state([]);
  let loadingMessage = $state(translate(currentLocale(), 'add.loading.detecting'));
  let loadingSeconds = $state(0);
  let loadingTimer: ReturnType<typeof setInterval> | null = null;

  const LOADING_STEPS = [
    { at: 0,  msg: 'add.loading.connecting' },
    { at: 3,  msg: 'add.loading.analyzing' },
    { at: 8,  msg: 'add.loading.detectingPrice' },
    { at: 15, msg: 'add.loading.browser' },
    { at: 30, msg: 'add.loading.almost' },
  ];

  // Fake progress bar: reaches ~90% at 40s, never hits 100 until done.
  let loadingProgress = $derived(
    Math.min(90, Math.round((loadingSeconds / 40) * 90))
  );

  let loadingDomain = $derived.by(() => {
    try { return new URL(url).hostname.replace('www.', ''); } catch { return ''; }
  });

  function startLoadingTimer() {
    loadingSeconds = 0;
    loadingMessage = translate(currentLocale(), LOADING_STEPS[0].msg);
    loadingTimer = setInterval(() => {
      loadingSeconds++;
      const step = [...LOADING_STEPS].reverse().find(s => loadingSeconds >= s.at);
      if (step) loadingMessage = translate(currentLocale(), step.msg);
    }, 1000);
  }

  function stopLoadingTimer() {
    if (loadingTimer) {
      clearInterval(loadingTimer);
      loadingTimer = null;
    }
    loadingSeconds = 0;
  }

  async function handleSubmit() {
    if (!url.trim()) return;

    loading = true;
    error = '';
    startLoadingTimer();

    try {
      const tagNames = selectedTags.map(t => t.name);
      const product = await api.products.import(url, watch, interval, tagNames.length > 0 ? tagNames : undefined);
      onsuccess(product);
      resetAndClose();
    } catch (e: any) {
      // 422 = scraper blocked or failed — show manual form, no DB record was created
      if (e?.status === 422) {
        step = 'manual';
        // Keep the message visible in the manual form header
        error = e?.code === 'BLOCKED'
          ? translate(currentLocale(), 'add.error.blocked')
          : translate(currentLocale(), 'add.error.detectFailed');
        return;
      }
      error = e instanceof Error ? e.message : translate(currentLocale(), 'add.error.import');
    } finally {
      loading = false;
      stopLoadingTimer();
    }
  }

  async function handleManualSubmit() {
    if (!manualName.trim()) return;

    loading = true;
    error = ''; // clear the block warning on submit attempt
    startLoadingTimer();

    try {
      const price = manualPrice ? parseFloat(manualPrice.replace(/\./g, '').replace(',', '.')) : undefined;
      const tagNames = selectedTags.map(t => t.name);
      const product = await api.products.import(
        url, watch, interval,
        tagNames.length > 0 ? tagNames : undefined,
        manualName,
        price && !isNaN(price) && price > 0 ? price : undefined,
        manualImage || undefined
      );
      onsuccess(product);
      resetAndClose();
    } catch (e) {
      error = e instanceof Error ? e.message : translate(currentLocale(), 'add.error.save');
    } finally {
      loading = false;
      stopLoadingTimer();
    }
  }

  function resetAndClose() {
    url = '';
    step = 'url';
    pendingProduct = null;
    manualName = '';
    manualPrice = '';
    manualImage = '';
    selectedTags = [];
    error = '';
    stopLoadingTimer();
    onclose();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      resetAndClose();
    }
  }

  const inputClasses = 'input';
</script>

{#if open}
  <div 
    class="fixed inset-0 m3-scrim flex items-center justify-center z-50 p-4"
    onclick={handleBackdropClick}
    onkeydown={(e) => { if (e.key === 'Escape') resetAndClose(); }}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    transition:fade={{ duration: 150 }}
  >
    <div class="m3-dialog p-6 w-full max-w-[440px] max-h-[90vh] overflow-y-auto" transition:fly={{ y: 14, duration: 220 }}>
      {#if loading}
        <div class="flex flex-col items-center justify-center py-8 text-center gap-5">
          <div class="relative w-14 h-14">
            <svg class="w-14 h-14 animate-spin text-[var(--accent)]" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-20" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3"></circle>
              <path class="opacity-80" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
            </svg>
          </div>
          <div>
            {#if loadingDomain}
              <p class="text-[13px] text-[var(--fg-muted)] mb-1">{$t('add.analyzing')} <span class="font-medium text-[var(--fg)]">{loadingDomain}</span></p>
            {/if}
            <p class="text-[15px] font-medium text-[var(--fg)]">{loadingMessage}</p>
          </div>
          <div class="w-full bg-[var(--border)] rounded-full h-1.5 overflow-hidden">
            <div
              class="h-full bg-[var(--accent)] rounded-full transition-all duration-1000 ease-out"
              style="width: {loadingProgress}%"
            ></div>
          </div>
          <button
            type="button"
            onclick={resetAndClose}
            class="text-[13px] text-[var(--fg-muted)] hover:text-[var(--fg)] transition-colors"
          >
            {$t('dash.cancel')}
          </button>
        </div>
      {:else if step === 'url'}
        <h2 class="text-[17px] font-semibold mb-1">{$t('dash.addProduct')}</h2>
        <p class="text-[13px] text-[var(--fg-muted)] mb-5">{$t('add.pasteUrl')}</p>

        <form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
          <div class="space-y-4">
            <div>
              <label for="url" class="block text-[13px] font-medium text-[var(--fg-secondary)] mb-1.5">
                {$t('add.urlLabel')}
              </label>
              <input
                id="url"
                type="url"
                bind:value={url}
                placeholder="https://tienda.cl/producto/123"
                class={inputClasses}
                required
              />
            </div>

            <div>
              <p class="block text-[13px] font-medium text-[var(--fg-secondary)] mb-1.5">
                Tags <span class="text-[var(--fg-muted)] font-normal">{$t('add.optional')}</span>
              </p>
              <TagInput
                tags={selectedTags}
                onTagsChange={(tags) => selectedTags = tags}
              />
            </div>

            <div class="flex items-center gap-2.5">
              <input
                type="checkbox"
                id="watch"
                bind:checked={watch}
                class="w-4 h-4 rounded border-[var(--border)] text-[var(--fg)] focus:ring-[var(--accent)]"
              />
              <label for="watch" class="text-[13px] font-medium text-[var(--fg-secondary)]">{$t('add.watchPrice')}</label>
            </div>

            {#if watch}
              <div>
                <label for="interval" class="block text-[13px] font-medium text-[var(--fg-secondary)] mb-1.5">
                  {$t('add.frequency')}
                </label>
                <select
                  id="interval"
                  bind:value={interval}
                  class={inputClasses}
                >
                  <option value={3600}>{$t('add.everyHour')}</option>
                  <option value={21600}>{$t('add.every6h')}</option>
                  <option value={43200}>{$t('add.every12h')}</option>
                  <option value={86400}>{$t('add.every24h')}</option>
                </select>
              </div>
            {/if}

            {#if error}
              <div class="p-3 rounded-[var(--radius-lg)] bg-[var(--danger-subtle)] border border-[var(--danger)]/20">
                <p class="text-[13px] text-[var(--danger)]">{error}</p>
              </div>
            {/if}
          </div>

          <div class="flex justify-end gap-2.5 mt-6">
            <button
              type="button"
              onclick={resetAndClose}
              class="btn btn-ghost"
            >
              {$t('dash.cancel')}
            </button>
            <button
              type="submit"
              disabled={!url.trim()}
              class="btn btn-primary"
            >
              {$t('dash.addProduct')}
            </button>
          </div>
        </form>
      {:else}
        <h2 class="text-[17px] font-semibold mb-1">{$t('add.completeData')}</h2>
        {#if error}
          <div class="p-3 rounded-[var(--radius-lg)] bg-[var(--warning-subtle)] border border-[var(--warning)]/30 mb-4">
            <p class="text-[13px] text-[var(--warning)]">{error}</p>
          </div>
        {:else}
          <p class="text-[13px] text-[var(--fg-muted)] mb-5">
            {$t('add.manualHint')}
          </p>
        {/if}

        <form onsubmit={(e) => { e.preventDefault(); handleManualSubmit(); }}>
          <div class="space-y-4">
            <div>
              <label for="name" class="block text-[13px] font-medium text-[var(--fg-secondary)] mb-1.5">
                {$t('add.nameLabel')} <span class="text-[var(--danger)]">*</span>
              </label>
              <input
                id="name"
                type="text"
                bind:value={manualName}
                placeholder={$t('add.namePlaceholder')}
                class={inputClasses}
                required
              />
            </div>

            <div>
              <label for="price" class="block text-[13px] font-medium text-[var(--fg-secondary)] mb-1.5">
                {$t('add.currentPrice')}
              </label>
              <input
                id="price"
                type="text"
                bind:value={manualPrice}
                placeholder={$t('add.pricePlaceholder')}
                class={inputClasses}
              />
            </div>

            <div>
              <label for="image" class="block text-[13px] font-medium text-[var(--fg-secondary)] mb-1.5">
                {$t('add.imageLabel')} <span class="text-[var(--fg-muted)] font-normal">{$t('add.optional')}</span>
              </label>
              <input
                id="image"
                type="url"
                bind:value={manualImage}
                placeholder="https://ejemplo.com/imagen.jpg"
                class={inputClasses}
              />
            </div>

            <div>
              <p class="block text-[13px] font-medium text-[var(--fg-secondary)] mb-1.5">
                Tags <span class="text-[var(--fg-muted)] font-normal">{$t('add.optional')}</span>
              </p>
              <TagInput
                tags={selectedTags}
                onTagsChange={(tags) => selectedTags = tags}
              />
            </div>

            {#if error}
              <div class="p-3 rounded-[var(--radius-lg)] bg-[var(--danger-subtle)] border border-[var(--danger)]/20">
                <p class="text-[13px] text-[var(--danger)]">{error}</p>
              </div>
            {/if}
          </div>

          <div class="flex justify-end gap-2.5 mt-6">
            <button
              type="button"
              onclick={resetAndClose}
              class="btn btn-ghost"
            >
              {$t('dash.cancel')}
            </button>
            <button
              type="submit"
              disabled={!manualName.trim()}
              class="btn btn-primary"
            >
              {$t('add.saveProduct')}
            </button>
          </div>
        </form>
      {/if}
    </div>
  </div>
{/if}
