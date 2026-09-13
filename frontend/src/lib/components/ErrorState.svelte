<script lang="ts">
  import { t } from '$lib/i18n';
  interface Props {
    message?: string;
    retry?: () => void;
  }

  let { message = '', retry }: Props = $props();
</script>

<div class="flex flex-col items-center justify-center py-20 text-center">
  <div class="w-12 h-12 rounded-[var(--radius-lg)] bg-[var(--danger-subtle)] flex items-center justify-center mb-5">
    <svg class="w-6 h-6 text-[var(--danger)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
    </svg>
  </div>
  <h3 class="title-md text-[var(--fg)] mb-1.5">{$t('error.title')}</h3>
  <p class="text-[14px] text-[var(--fg-muted)] max-w-[320px] leading-relaxed mb-5">{message || $t('error.default')}</p>
  {#if retry}
    <button onclick={retry} class="btn btn-secondary">
      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
      </svg>
      {$t('error.retry')}
    </button>
  {/if}
</div>
