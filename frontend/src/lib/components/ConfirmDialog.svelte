<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { t } from '$lib/i18n';

  interface Props {
    title: string;
    message: string;
    confirmLabel?: string;
    cancelLabel?: string;
    onconfirm: () => void;
    oncancel: () => void;
    open: boolean;
  }

  let { title, message, confirmLabel = '', cancelLabel = '', onconfirm, oncancel, open }: Props = $props();

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      oncancel();
    }
  }
</script>

{#if open}
  <div
    class="fixed inset-0 m3-scrim flex items-center justify-center z-50 p-4"
    onclick={handleBackdropClick}
    onkeydown={(e) => { if (e.key === 'Escape') oncancel(); }}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    transition:fade={{ duration: 150 }}
  >
    <div class="m3-dialog p-6 w-full max-w-[400px]" transition:fly={{ y: 14, duration: 220 }}>
      <h3 class="title-md mb-2">{title}</h3>
      <p class="text-[13px] text-[var(--fg-secondary)] mb-6 leading-relaxed">{message}</p>
      <div class="flex justify-end gap-2">
        <button onclick={oncancel} class="btn btn-ghost">{cancelLabel || $t('dialog.cancel')}</button>
        <button onclick={onconfirm} class="btn btn-danger">{confirmLabel || $t('dialog.confirm')}</button>
      </div>
    </div>
  </div>
{/if}
