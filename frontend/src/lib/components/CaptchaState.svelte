<script lang="ts">
  import { t } from '$lib/i18n';

  interface Props {
    onResolve?: () => void;
  }

  let { onResolve }: Props = $props();
</script>

<div class="flex flex-col items-center justify-center py-16 sm:py-24 text-center">
  <div class="w-16 h-16 rounded-2xl bg-[var(--warning-subtle)] flex items-center justify-center mb-5">
    <svg class="w-8 h-8 text-[var(--warning)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
    </svg>
  </div>
  <h3 class="text-lg font-semibold text-[var(--fg)] mb-2">{$t('captcha.title')}</h3>
  <p class="text-[var(--fg-muted)] max-w-sm text-sm leading-relaxed mb-5">
    {$t('captcha.message')}
  </p>
  {#if onResolve}
    <button
      onclick={onResolve}
      class="inline-flex items-center gap-2 px-4 py-2.5 bg-[var(--accent)] text-[var(--accent-fg)] rounded-lg text-sm font-medium hover:bg-[var(--accent-hover)] transition-colors"
    >
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      {$t('captcha.continue')}
    </button>
  {/if}
</div>
