<script lang="ts">
  import { formatPrice } from '$lib/utils';
  import type { PriceSignal } from '$lib/utils/signal';
  import { t } from '$lib/i18n';

  interface Props {
    signal: PriceSignal;
    currency?: string;
  }

  let { signal, currency = 'CLP' }: Props = $props();

  const markerPct = $derived(Math.min(98, Math.max(2, signal.percentile * 100)));
  const isLow = $derived(signal.signal === 'buy');
  const isHigh = $derived(signal.signal === 'high');
  const markerColor = $derived(isLow ? 'var(--success)' : isHigh ? 'var(--danger)' : 'var(--fg-secondary)');
</script>

<div class="w-full">
  <div class="relative h-8">
    <!-- pista -->
    <div class="absolute inset-x-0 top-1/2 -translate-y-1/2 h-2 rounded-full overflow-hidden bg-[var(--bg-subtle)] border border-[var(--border)]">
      <div class="absolute inset-y-0 left-0 w-1/4 bg-[var(--success-subtle)]"></div>
      <div class="absolute inset-y-0 right-0 w-1/5 bg-[var(--danger-subtle)]"></div>
    </div>

    <!-- marcador del precio actual -->
    <div
      class="absolute top-1/2 -translate-y-1/2 -translate-x-1/2 flex flex-col items-center"
      style="left: {markerPct}%"
    >
      <span class="num text-[10px] font-semibold whitespace-nowrap mb-0.5 px-1 rounded" style="color: {markerColor}; background: var(--bg-elevated)">
        {formatPrice(signal.current, currency)}
      </span>
      <span class="w-3 h-3 rounded-full border-2 border-[var(--bg-elevated)] shadow-[var(--shadow-xs)]" style="background: {markerColor}"></span>
    </div>
  </div>

  <div class="flex items-center justify-between mt-1.5">
    <span class="num text-[11px] text-[var(--fg-muted)]">{$t('range.min', { price: formatPrice(signal.min, currency) })}</span>
    <span class="num text-[11px] text-[var(--fg-muted)]">{$t('range.max', { price: formatPrice(signal.max, currency) })}</span>
  </div>
</div>
