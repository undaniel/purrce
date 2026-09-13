<script lang="ts">
  import { t } from '$lib/i18n';
  interface Props {
    title?: string;
    message?: string;
    ctaLabel?: string;
    oncta?: () => void;
    showOnboarding?: boolean;
  }

  let { title = '', message = '', ctaLabel, oncta, showOnboarding = false }: Props = $props();

  const features = [
    { icon: 'M13 7h8m0 0v8m0-8l-8 8-4-4-6 6', label: 'empty.feature1.title', desc: 'empty.feature1.desc' },
    { icon: 'M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9', label: 'empty.feature2.title', desc: 'empty.feature2.desc' },
    { icon: 'M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z', label: 'empty.feature3.title', desc: 'empty.feature3.desc' },
  ];
</script>

<div class="flex flex-col items-center justify-center py-16 text-center">
  <div class="w-12 h-12 rounded-[var(--radius-lg)] bg-[var(--bg-subtle)] border border-[var(--border)] flex items-center justify-center mb-5">
    <svg class="w-6 h-6 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
    </svg>
  </div>
  <h3 class="title-md text-[var(--fg)] mb-1.5">{title || $t('empty.title')}</h3>
  <p class="text-[14px] text-[var(--fg-muted)] max-w-[300px] leading-relaxed">{message || $t('empty.message')}</p>
  {#if oncta && ctaLabel}
    <button
      onclick={oncta}
      class="btn btn-primary mt-5"
    >
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
      </svg>
      {ctaLabel}
    </button>
  {/if}

  {#if showOnboarding}
    <div class="mt-10 grid grid-cols-1 sm:grid-cols-3 gap-3 max-w-2xl text-left">
      {#each features as f}
        <div class="card p-4">
          <div class="w-8 h-8 rounded-[var(--radius-md)] bg-[var(--bg-subtle)] flex items-center justify-center mb-3">
            <svg class="w-4 h-4 text-[var(--fg-secondary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d={f.icon} />
            </svg>
          </div>
          <p class="text-[13px] font-semibold text-[var(--fg)] mb-1">{$t(f.label)}</p>
          <p class="text-[12px] text-[var(--fg-muted)] leading-relaxed">{$t(f.desc)}</p>
        </div>
      {/each}
    </div>
  {/if}
</div>
