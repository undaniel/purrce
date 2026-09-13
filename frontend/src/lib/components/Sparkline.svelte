<script lang="ts">
  interface Props {
    data: number[];
    width?: number;
    height?: number;
    color?: string;
    positiveColor?: string;
    negativeColor?: string;
    showDelta?: boolean;
    currency?: string;
  }

  let {
    data,
    width = 80,
    height = 24,
    color = 'var(--fg-muted)',
    positiveColor = 'var(--success)',
    negativeColor = 'var(--danger)',
    showDelta = false,
    currency = 'CLP'
  }: Props = $props();

  const geom = $derived.by(() => {
    if (data.length < 2) return null;

    const min = Math.min(...data);
    const max = Math.max(...data);
    const range = max - min || 1;
    const pad = 2;
    const stepX = (width - pad * 2) / (data.length - 1);

    const pts = data.map((value, i) => {
      const x = pad + i * stepX;
      const y = height - pad - ((value - min) / range) * (height - pad * 2);
      return { x, y };
    });

    const line = pts.map(p => `${p.x},${p.y}`).join(' ');
    const area = `${pad},${height} ${line} ${width - pad},${height}`;
    return { line, area };
  });

  const trend = $derived.by(() => {
    if (data.length < 2) return 'neutral';
    const first = data[0];
    const last = data[data.length - 1];
    if (last < first) return 'down';
    if (last > first) return 'up';
    return 'neutral';
  });

  const strokeColor = $derived(
    trend === 'down' ? positiveColor :
    trend === 'up' ? negativeColor :
    color
  );

  const delta = $derived.by(() => {
    if (data.length < 2) return null;
    const first = data[0];
    const last = data[data.length - 1];
    const abs = last - first;
    const pct = first === 0 ? 0 : (abs / first) * 100;
    return { abs, pct };
  });
</script>

{#if geom}
  <div class="inline-flex items-center gap-1.5">
    <svg {width} {height} viewBox="0 0 {width} {height}" class="inline-block flex-shrink-0" preserveAspectRatio="none">
      <polygon points={geom.area} fill={strokeColor} opacity="0.08" />
      <polyline
        points={geom.line}
        fill="none"
        stroke={strokeColor}
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
    {#if showDelta && delta}
      <span class="num text-[11px] font-medium" style="color: {delta.pct < 0 ? 'var(--success)' : delta.pct > 0 ? 'var(--danger)' : 'var(--fg-muted)'}">
        {delta.pct < 0 ? '↓' : delta.pct > 0 ? '↑' : ''}{Math.abs(delta.pct).toFixed(1)}%
      </span>
    {/if}
  </div>
{:else}
  <span class="text-[var(--fg-muted)] text-xs">—</span>
{/if}
