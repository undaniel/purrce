<script lang="ts">
  import { SIGNAL_META, type BuySignal } from '$lib/utils/signal';
  import { t } from '$lib/i18n';

  interface Props {
    signal: BuySignal;
    label?: string;
    reason?: string;
    size?: 'xs' | 'sm' | 'md';
    showIcon?: boolean;
  }

  let { signal, label, reason, size = 'sm', showIcon = true }: Props = $props();

  const meta = $derived(SIGNAL_META[signal]);

  const sizeClass = $derived(
    size === 'md' ? 'text-[13px] px-2.5 py-1 gap-1.5' :
    size === 'xs' ? 'text-[10px] px-1.5 py-0.5 gap-1' :
    'text-[11px] px-2 py-0.5 gap-1'
  );

  const iconSize = $derived(size === 'md' ? 'w-3.5 h-3.5' : 'w-3 h-3');
</script>

<span
  class="inline-flex items-center rounded-full font-semibold leading-none whitespace-nowrap {sizeClass}"
  style="color: {meta.color}; background: {meta.bg}; border: 1px solid {meta.border};"
  title={reason ?? $t(meta.label)}
>
  {#if showIcon}
    <svg class={iconSize} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={meta.icon} />
    </svg>
  {/if}
  {label ?? $t(meta.label)}
</span>
