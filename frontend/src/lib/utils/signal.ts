import type { PriceHistory } from '$lib/types';
import { currentLocale, translate } from '$lib/i18n';

export type BuySignal = 'buy' | 'fair' | 'high' | 'unknown';

export interface PriceSignal {
  signal: BuySignal;
  label: string;
  reason: string;
  min: number;
  max: number;
  avg: number;
  current: number;
  /** 0 = mínimo histórico, 1 = máximo histórico */
  percentile: number;
  /** avg - current. Positivo = estás ahorrando respecto al precio habitual */
  savingsVsAvg: number;
  isHistoricalLow: boolean;
  samples: number;
}

export const SIGNAL_META: Record<BuySignal, { label: string; icon: string; color: string; bg: string; border: string }> = {
  buy: {
    label: 'signal.buy',
    icon: 'M13 17h8m0 0V9m0 8l-8-8-4 4-6-6',
    color: 'var(--success)',
    bg: 'var(--success-subtle)',
    border: 'var(--success-muted)'
  },
  fair: {
    label: 'signal.fair',
    icon: 'M5 12h14',
    color: 'var(--fg-secondary)',
    bg: 'var(--bg-subtle)',
    border: 'var(--border)'
  },
  high: {
    label: 'signal.high',
    icon: 'M11 7h8m0 0V15m0-8l-8 8-4-4-6 6',
    color: 'var(--danger)',
    bg: 'var(--danger-subtle)',
    border: 'var(--danger-muted)'
  },
  unknown: {
    label: 'signal.unknown',
    icon: 'M8 12h.01M12 12h.01M16 12h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z',
    color: 'var(--fg-muted)',
    bg: 'var(--bg-subtle)',
    border: 'var(--border)'
  }
};

/**
 * Decide si conviene comprar un producto comparando el precio actual
 * contra su histórico: posición en el rango, distancia al mínimo y
 * desviación respecto al promedio.
 */
export function computePriceSignal(
  history: PriceHistory[] | undefined,
  currentPrice: number
): PriceSignal | null {
  if (!history || history.length < 3 || currentPrice <= 0) return null;

  const prices = history.map(h => h.price).filter(p => p > 0);
  if (prices.length < 3) return null;

  const min = Math.min(...prices);
  const max = Math.max(...prices);
  const avg = prices.reduce((a, b) => a + b, 0) / prices.length;
  const range = max - min;
  const percentile = range > 0 ? (currentPrice - min) / range : 0;
  const isHistoricalLow = currentPrice <= min;
  const savingsVsAvg = avg - currentPrice;

  const l = currentLocale();
  let signal: BuySignal;
  let reason: string;

  if (isHistoricalLow) {
    signal = 'buy';
    reason = translate(l, 'signal.reason.historicalLow');
  } else if (percentile <= 0.25) {
    signal = 'buy';
    reason = translate(l, 'signal.reason.nearLow', { percent: Math.round(percentile * 100) });
  } else if (percentile >= 0.8 || currentPrice >= avg * 1.12) {
    signal = 'high';
    reason = translate(l, 'signal.reason.waitDrop');
  } else {
    signal = 'fair';
    reason = translate(l, 'signal.reason.usual');
  }

  return {
    signal,
    label: translate(l, SIGNAL_META[signal].label),
    reason,
    min,
    max,
    avg,
    current: currentPrice,
    percentile,
    savingsVsAvg,
    isHistoricalLow,
    samples: prices.length
  };
}
