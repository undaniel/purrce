<script lang="ts">
  import { t } from '$lib/i18n';
  interface Props {
    currentPage: number;
    totalPages: number;
    loading?: boolean;
    onPageChange: (page: number) => void;
  }

  let { currentPage, totalPages, loading = false, onPageChange }: Props = $props();

  const pages = $derived.by(() => {
    const items: (number | string)[] = [];
    const maxVisible = 7;

    if (totalPages <= maxVisible) {
      for (let i = 1; i <= totalPages; i++) items.push(i);
    } else {
      items.push(1);
      if (currentPage > 3) items.push('...');
      const start = Math.max(2, currentPage - 1);
      const end = Math.min(totalPages - 1, currentPage + 1);
      for (let i = start; i <= end; i++) items.push(i);
      if (currentPage < totalPages - 2) items.push('...');
      items.push(totalPages);
    }
    return items;
  });
</script>

{#if totalPages > 1}
  <nav class="flex items-center justify-center gap-1 mt-7" aria-label={$t('pagination.label')}>
    <button
      onclick={() => onPageChange(currentPage - 1)}
      disabled={currentPage === 1 || loading}
      class="pressable p-2 rounded-[var(--radius-md)] text-[var(--fg-muted)] hover:bg-[var(--bg-subtle)] hover:text-[var(--fg)] disabled:opacity-30 disabled:cursor-not-allowed"
      aria-label={$t('pagination.prev')}
    >
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M15 19l-7-7 7-7" />
      </svg>
    </button>

    {#each pages as page, i (page === '...' ? `gap-${i}` : page)}
      {#if page === '...'}
        <span class="px-2 py-1.5 text-[13px] text-[var(--fg-muted)]">…</span>
      {:else}
        <button
          onclick={() => onPageChange(page as number)}
          disabled={loading && currentPage !== page}
          aria-current={currentPage === page ? 'page' : undefined}
          class="pressable num min-w-[30px] h-8 px-2 rounded-[var(--radius-md)] text-[13px] font-medium inline-flex items-center justify-center gap-1
            {currentPage === page
              ? 'bg-[var(--accent)] text-[var(--accent-fg)] shadow-[var(--shadow-xs)]'
              : 'text-[var(--fg-secondary)] hover:bg-[var(--bg-subtle)] hover:text-[var(--fg)]'}"
        >
          {#if loading && currentPage === page}
            <svg class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
          {:else}
            {page}
          {/if}
        </button>
      {/if}
    {/each}

    <button
      onclick={() => onPageChange(currentPage + 1)}
      disabled={currentPage === totalPages || loading}
      class="pressable p-2 rounded-[var(--radius-md)] text-[var(--fg-muted)] hover:bg-[var(--bg-subtle)] hover:text-[var(--fg)] disabled:opacity-30 disabled:cursor-not-allowed"
      aria-label={$t('pagination.next')}
    >
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M9 5l7 7-7 7" />
      </svg>
    </button>
  </nav>
{/if}
