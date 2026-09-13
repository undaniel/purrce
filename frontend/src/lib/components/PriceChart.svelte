<script lang="ts">
  import type { PriceHistory } from '$lib/types';
  import { formatPrice, formatDate, filterHistoryByRange, type TimeRange } from '$lib/utils';
  import TimeFilter from './TimeFilter.svelte';
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import { t, translate, currentLocale } from '$lib/i18n';

  interface ChartEvent {
    date: string;
    label: string;
    price: number;
  }

  export interface ChartSeries {
    name: string;
    data: PriceHistory[];
    highlighted?: boolean;
  }

  interface Props {
    series: ChartSeries[];
    currency?: string;
    events?: ChartEvent[];
  }

  let { series, currency = 'CLP', events = [] }: Props = $props();
  let chartContainer: HTMLDivElement;
  let chart = $state.raw<any>(undefined);
  let echartsLib: any = undefined;
  let resizeObserver: ResizeObserver | undefined;
  let selectedRange: TimeRange = $state('all');

  // Paleta con significado: la serie enfocada usa el acento de marca y las
  // demás caen a neutros para que el ojo sepa dónde mirar.
  const ACCENT_LIGHT = '#e8590c';
  const ACCENT_DARK = '#fb923c';
  const NEUTRALS_LIGHT = ['#78716c', '#a8a29e', '#57534e', '#d6d3d1'];
  const NEUTRALS_DARK = ['#a8a29e', '#78716c', '#57534e', '#44403c'];

  const filteredSeries = $derived(
    series.map(s => ({
      ...s,
      data: [...(filterHistoryByRange(s.data, selectedRange) as PriceHistory[])]
        .sort((a, b) => new Date(a.observed_at).getTime() - new Date(b.observed_at).getTime())
    }))
  );

  const visible = $derived(filteredSeries.filter(s => s.data.length > 0));

  const focusSeries = $derived(
    visible.find(s => s.highlighted) ?? visible[0] ?? null
  );

  const stats = $derived.by(() => {
    if (!focusSeries || focusSeries.data.length === 0) return null;
    const prices = focusSeries.data.map(h => h.price).filter(p => p > 0);
    if (prices.length === 0) return null;
    return {
      min: Math.min(...prices),
      max: Math.max(...prices),
      avg: prices.reduce((a, b) => a + b, 0) / prices.length,
      count: focusSeries.data.length
    };
  });

  function colorFor(i: number, highlighted: boolean | undefined, total: number, isDark: boolean) {
    const accent = isDark ? ACCENT_DARK : ACCENT_LIGHT;
    if (highlighted || total <= 1) return accent;
    const neutrals = isDark ? NEUTRALS_DARK : NEUTRALS_LIGHT;
    return neutrals[i % neutrals.length];
  }

  onMount(async () => {
    if (!browser) return;
    try {
      echartsLib = await import('echarts');
      if (chartContainer && visible.length > 0) initChart();
      resizeObserver = new ResizeObserver(() => {
        if (chart) { try { chart.resize(); } catch {} }
      });
      if (chartContainer) resizeObserver.observe(chartContainer);
    } catch (e) {
      console.error('Error loading echarts:', e);
    }
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    if (chart) { try { chart.dispose(); } catch {} chart = undefined; }
  });

  function initChart() {
    if (!chartContainer || !echartsLib) return;
    try {
      if (chart) chart.dispose();
      chart = echartsLib.init(chartContainer);
      updateChart();
    } catch (e) {
      console.error('Error initializing chart:', e);
    }
  }

  function updateChart() {
    if (!chart || !echartsLib || visible.length === 0) return;

    const isDark = document.documentElement.classList.contains('dark');
    const textColor = isDark ? '#a8a29e' : '#78716c';
    const axisLine = isDark ? '#292524' : '#e7e5e4';
    const splitLine = isDark ? '#211e1c' : '#f1efed';
    const tooltipBg = isDark ? '#1c1917' : '#ffffff';
    const tooltipBorder = isDark ? '#292524' : '#e7e5e4';
    const tooltipFg = isDark ? '#fafaf9' : '#191714';
    const mono = 'Geist Mono, ui-monospace, monospace';

    // Lookup for tooltip deltas
    const lookup: Record<string, { ts: number; price: number }[]> = {};
    for (const s of visible) {
      lookup[s.name] = s.data.map(h => ({ ts: new Date(h.observed_at).getTime(), price: h.price }));
    }

    const chartSeries = visible.map((s, i) => {
      const color = colorFor(i, s.highlighted, visible.length, isDark);
      const dim = !s.highlighted && visible.length > 1;
      const points = s.data.map(h => [new Date(h.observed_at).getTime(), h.price]);
      const base: any = {
        name: s.name,
        type: 'line',
        data: points,
        smooth: 0.32,
        symbol: 'circle',
        symbolSize: 5,
        showSymbol: s.data.length <= 24,
        connectNulls: true,
        lineStyle: { color, width: s.highlighted ? 2.5 : 1.4, opacity: dim ? 0.5 : 1 },
        itemStyle: { color, borderColor: isDark ? '#171412' : '#ffffff', borderWidth: 2 },
        emphasis: { focus: 'series', lineStyle: { width: 2.6, opacity: 1 } },
        z: s.highlighted ? 5 : 3
      };

      // Banda mín/máx + líneas guía solo en la serie enfocada
      if (s.highlighted && stats) {
        base.markArea = {
          silent: true,
          itemStyle: { color: isDark ? 'rgba(255,255,255,0.04)' : 'rgba(28,25,23,0.035)' },
          data: [[{ yAxis: stats.min }, { yAxis: stats.max }]]
        };
        base.markLine = {
          silent: true,
          symbol: 'none',
          data: [
            { yAxis: stats.min, lineStyle: { color: isDark ? '#4ade80' : '#15803d', type: 'dashed', width: 1 },
              label: { formatter: translate(currentLocale(), 'chart.minLine', { price: formatPrice(stats.min, currency) }), position: 'insideEndTop', color: isDark ? '#4ade80' : '#15803d', fontSize: 10, fontFamily: mono } },
            { yAxis: stats.max, lineStyle: { color: isDark ? '#f87171' : '#b91c1c', type: 'dashed', width: 1 },
              label: { formatter: translate(currentLocale(), 'chart.maxLine', { price: formatPrice(stats.max, currency) }), position: 'insideEndBottom', color: isDark ? '#f87171' : '#b91c1c', fontSize: 10, fontFamily: mono } }
          ]
        };

        if (events && events.length > 0) {
          base.markPoint = {
            symbol: 'pin',
            symbolSize: 26,
            label: { fontSize: 9, color: '#fff' },
            itemStyle: { color: isDark ? '#f87171' : '#b91c1c' },
            data: events.map(ev => {
              const idx = s.data.findIndex(h => h.observed_at >= ev.date);
              const i2 = idx >= 0 ? idx : s.data.length - 1;
              return { coord: [new Date(s.data[i2]?.observed_at).getTime(), s.data[i2]?.price ?? ev.price], name: ev.label };
            })
          };
        }
      }

      return base;
    });

    try {
      chart.clear();
      chart.setOption({
        animation: true,
        animationDuration: 500,
        animationEasing: 'cubicOut',
        color: [isDark ? ACCENT_DARK : ACCENT_LIGHT, ...(isDark ? NEUTRALS_DARK : NEUTRALS_LIGHT)],
        legend: visible.length > 1 ? {
          top: 0,
          left: 0,
          icon: 'roundRect',
          itemWidth: 10,
          itemHeight: 3,
          itemGap: 14,
          textStyle: { color: textColor, fontSize: 11, fontFamily: mono }
        } : undefined,
        tooltip: {
          trigger: 'axis',
          backgroundColor: tooltipBg,
          borderColor: tooltipBorder,
          borderWidth: 1,
          padding: [10, 12],
          extraCssText: 'border-radius:14px;box-shadow:0 12px 32px rgba(0,0,0,0.16);backdrop-filter:blur(8px);',
          textStyle: { color: tooltipFg, fontSize: 12, fontFamily: mono },
          formatter: function(params: any) {
            if (!params || params.length === 0) return '';
            const ts = params[0].value[0];
            const rows = params
              .filter((p: any) => p.value && p.value[1] != null)
              .map((p: any) => {
                const arr = lookup[p.seriesName] || [];
                const idx = arr.findIndex(d => d.ts === p.value[0]);
                let delta = '';
                if (idx > 0 && arr[idx - 1].price > 0) {
                  const diff = arr[idx].price - arr[idx - 1].price;
                  const pct = (diff / arr[idx - 1].price) * 100;
                  if (diff !== 0) {
                    const col = diff < 0 ? (isDark ? '#4ade80' : '#15803d') : (isDark ? '#f87171' : '#b91c1c');
                    delta = `<span style="color:${col};margin-left:6px">${diff < 0 ? '↓' : '↑'}${Math.abs(pct).toFixed(1)}%</span>`;
                  }
                }
                return `<div style="display:flex;align-items:center;gap:6px;margin-top:4px">
                  <span style="width:8px;height:8px;border-radius:2px;background:${p.color}"></span>
                  <span style="color:${textColor}">${p.seriesName}</span>
                  <span style="margin-left:auto;font-weight:600">${formatPrice(p.value[1], currency)}</span>${delta}
                </div>`;
              })
              .join('');
            return `<div style="font-size:11px;color:${textColor}">${formatDate(new Date(ts).toISOString())}</div>${rows}`;
          }
        },
        grid: { left: 8, right: 12, top: visible.length > 1 ? 32 : 12, bottom: 8, containLabel: true },
        xAxis: {
          type: 'time',
          boundaryGap: false,
          axisLabel: { formatter: (v: number) => formatDate(new Date(v).toISOString()), color: textColor, fontSize: 10, fontFamily: mono, hideOverlap: true },
          axisLine: { lineStyle: { color: axisLine } },
          axisTick: { show: false },
          splitLine: { show: false }
        },
        yAxis: {
          type: 'value',
          scale: true,
          axisLabel: { formatter: (v: number) => formatPrice(v, currency), color: textColor, fontSize: 10, fontFamily: mono },
          splitLine: { lineStyle: { color: splitLine, type: 'dashed' } },
          axisLine: { show: false },
          axisTick: { show: false }
        },
        series: chartSeries
      });
    } catch (e) {
      console.error('Error setting chart options:', e);
    }
  }

  // Un único efecto: `visible` ya depende de `selectedRange` y de los datos,
  // y `chart` es reactivo (raw) para reinicializar cuando esté listo.
  $effect(() => {
    void visible;
    void selectedRange;
    if (chart && visible.length > 0) updateChart();
  });
</script>

<div class="card p-4 sm:p-5">
  <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 mb-4">
    <div>
      <h3 class="title-sm">{$t('chart.historyTitle')}</h3>
      {#if visible.length > 1}
        <p class="text-[11px] text-[var(--fg-muted)] mt-0.5">{$t('chart.comparison', { count: visible.length })}</p>
      {/if}
    </div>
    <TimeFilter selected={selectedRange} onchange={(range) => selectedRange = range} />
  </div>

  {#if visible.length > 0}
    <div bind:this={chartContainer} class="w-full h-60 sm:h-72"></div>
    {#if stats}
      <div class="flex flex-wrap items-center justify-between gap-3 mt-3 pt-3 border-t border-[var(--border)]">
        <div class="flex gap-4 text-[12px]">
          <span class="text-[var(--fg-muted)]">{$t('chart.min')} <span class="num font-medium text-[var(--success)]">{formatPrice(stats.min, currency)}</span></span>
          <span class="text-[var(--fg-muted)]">{$t('chart.max')} <span class="num font-medium text-[var(--danger)]">{formatPrice(stats.max, currency)}</span></span>
          <span class="text-[var(--fg-muted)]">{$t('chart.avg')} <span class="num font-medium">{formatPrice(stats.avg, currency)}</span></span>
        </div>
        <span class="text-[12px] text-[var(--fg-muted)]">{$t('chart.records', { count: stats.count })}</span>
      </div>
    {/if}
  {:else}
    <div class="w-full h-60 sm:h-72 flex items-center justify-center text-[13px] text-[var(--fg-muted)]">
      {$t('chart.noData')}
    </div>
  {/if}
</div>
