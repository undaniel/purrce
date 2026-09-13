<script lang="ts">
  import { formatPrice } from '$lib/utils';

  interface Props {
    price: number;
    currency?: string;
    change?: number | null;
    size?: 'sm' | 'md' | 'lg';
  }

  let { price, currency = 'CLP', change = null, size = 'md' }: Props = $props();

  const sizeClasses = {
    sm: 'text-sm',
    md: 'text-lg',
    lg: 'text-2xl sm:text-3xl'
  };

  const changeColor = $derived.by(() => {
    if (change === null) return '';
    return change < 0 ? 'text-[var(--success)]' : change > 0 ? 'text-[var(--danger)]' : 'text-[var(--fg-muted)]';
  });

  const changeBg = $derived.by(() => {
    if (change === null) return '';
    return change < 0 ? 'bg-[var(--success-subtle)]' : change > 0 ? 'bg-[var(--danger-subtle)]' : 'bg-[var(--bg-subtle)]';
  });
</script>

<div class="flex items-baseline gap-2">
  <span class="font-bold tracking-tight {sizeClasses[size]}">{formatPrice(price, currency)}</span>
  {#if change !== null && change !== 0}
    <span class="inline-flex items-center gap-0.5 text-xs font-medium px-2 py-0.5 rounded-full {changeColor()} {changeBg()}">
      {change > 0 ? '↑' : '↓'}
      {Math.abs(change).toFixed(1)}%
    </span>
  {/if}
</div>
