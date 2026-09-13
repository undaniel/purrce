import { currentLocale, translate } from '$lib/i18n';

function dateLocale(): string {
  return currentLocale() === 'en' ? 'en-US' : 'es-CL';
}

function getLocaleForCurrency(currency: string): string {
  switch (currency.toUpperCase()) {
    case 'USD': return 'en-US';
    case 'EUR': return 'de-DE';
    case 'GBP': return 'en-GB';
    case 'BRL': return 'pt-BR';
    case 'ARS': return 'es-AR';
    case 'MXN': return 'es-MX';
    case 'COP': return 'es-CO';
    case 'PEN': return 'es-PE';
    case 'UYU': return 'es-UY';
    case 'CLP':
    default: return 'es-CL';
  }
}

function getFractionDigits(currency: string): number {
  switch (currency.toUpperCase()) {
    case 'CLP':
    case 'ARS':
    case 'UYU':
    case 'COP':
      return 0;
    case 'USD':
    case 'EUR':
    case 'GBP':
    case 'BRL':
    case 'MXN':
    case 'PEN':
    default: return 2;
  }
}

export function formatPrice(price: number, currency: string = 'CLP'): string {
  const locale = getLocaleForCurrency(currency);
  const digits = getFractionDigits(currency);
  return new Intl.NumberFormat(locale, {
    style: 'currency',
    currency: currency.toUpperCase(),
    minimumFractionDigits: digits,
    maximumFractionDigits: digits
  }).format(price);
}

/** Etiqueta corta de moneda para mostrar junto al precio (p. ej. CLP, COP). */
export function currencyLabel(currency: string | null | undefined): string {
  return (currency || 'CLP').toUpperCase();
}

/**
 * Capitaliza la primera letra de cada palabra solo si está en minúscula,
 * preservando siglas y nombres ya formateados (p. ej. "PC Factory").
 */
export function titleCase(value: string | null | undefined): string {
  if (!value) return '';
  return value
    .trim()
    .replace(/(^|[\s\-/])([a-záéíóúüñ])/g, (_m, sep, ch: string) => sep + ch.toLocaleUpperCase('es-CL'));
}

export function formatDate(date: string): string {
  return new Date(date).toLocaleDateString(dateLocale(), {
    day: '2-digit',
    month: 'short',
    year: 'numeric'
  });
}

export function formatDateTime(date: string): string {
  return new Date(date).toLocaleString(dateLocale(), {
    day: '2-digit',
    month: 'short',
    hour: '2-digit',
    minute: '2-digit'
  });
}

export function timeAgo(date: string, capitalize = true): string {
  const now = new Date();
  const past = new Date(date);
  const diffMs = now.getTime() - past.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  const l = currentLocale();
  const suffix = capitalize ? '' : 'Lower';

  if (diffMins < 1) return translate(l, capitalize ? 'time.now' : 'time.nowLower');
  if (diffMins < 60) return translate(l, `time.minutes${suffix}`, { count: diffMins });
  if (diffHours < 24) return translate(l, `time.hours${suffix}`, { count: diffHours });
  if (diffDays < 7) return translate(l, `time.days${suffix}`, { count: diffDays });
  return formatDate(date);
}

export function priceChangePercent(current: number, previous: number): number {
  if (previous === 0) return 0;
  return ((current - previous) / previous) * 100;
}

export function getStatusColor(status: string): string {
  switch (status) {
    case 'ACTIVE': return 'text-[var(--success)]';
    case 'CHECKING': return 'text-[var(--accent)]';
    case 'ERROR': return 'text-[var(--danger)]';
    case 'CAPTCHA_REQUIRED': return 'text-[var(--warning)]';
    case 'BLOCKED': return 'text-[var(--danger)]';
    case 'DISABLED': return 'text-[var(--fg-muted)]';
    default: return 'text-[var(--fg-muted)]';
  }
}

export function getStatusLabel(status: string): string {
  const key = `status.${status}`;
  const label = translate(currentLocale(), key);
  return label === key ? status : label;
}

export function formatNumber(num: number): string {
  return new Intl.NumberFormat(dateLocale()).format(num);
}

export function formatPercent(value: number): string {
  return `${value > 0 ? '+' : ''}${value.toFixed(1)}%`;
}

export function getChangeColor(change: number): string {
  if (change < 0) return 'text-[var(--success)]';
  if (change > 0) return 'text-[var(--danger)]';
  return 'text-[var(--fg-muted)]';
}

export function getChangeBg(change: number): string {
  if (change < 0) return 'bg-[var(--success-subtle)]';
  if (change > 0) return 'bg-[var(--danger-subtle)]';
  return 'bg-[var(--bg-subtle)]';
}

export function getChangeIcon(change: number): string {
  if (change < 0) return '↓';
  if (change > 0) return '↑';
  return '—';
}

export type TimeRange = '7d' | '30d' | '3m' | '6m' | '1y' | 'all';

export function filterHistoryByRange(history: { observed_at: string }[], range: TimeRange): { observed_at: string }[] {
  if (range === 'all') return history;
  
  const now = new Date();
  let cutoffDate: Date;
  
  switch (range) {
    case '7d':
      cutoffDate = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
      break;
    case '30d':
      cutoffDate = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);
      break;
    case '3m':
      cutoffDate = new Date(now.getTime() - 90 * 24 * 60 * 60 * 1000);
      break;
    case '6m':
      cutoffDate = new Date(now.getTime() - 180 * 24 * 60 * 60 * 1000);
      break;
    case '1y':
      cutoffDate = new Date(now.getTime() - 365 * 24 * 60 * 60 * 1000);
      break;
    default:
      return history;
  }
  
  return history.filter(h => new Date(h.observed_at) >= cutoffDate);
}

export function calculateDelta(current: number, previous: number): { absolute: number; percent: number } {
  const absolute = current - previous;
  const percent = previous === 0 ? 0 : (absolute / previous) * 100;
  return { absolute, percent };
}

export function formatDelta(delta: { absolute: number; percent: number }, currency: string = 'CLP'): string {
  const sign = delta.absolute >= 0 ? '+' : '';
  return `${sign}${formatPrice(delta.absolute, currency)} (${sign}${delta.percent.toFixed(1)}%)`;
}
