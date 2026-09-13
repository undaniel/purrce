import { derived, get, writable } from 'svelte/store';
import { messages, type Locale } from './messages';

export type { Locale } from './messages';

type Vars = Record<string, string | number>;

declare global {
  interface Window {
    __purrceLocale?: Locale;
  }
}

function initialLocale(): Locale {
  if (typeof window !== 'undefined' && window.__purrceLocale) return window.__purrceLocale;
  return 'es';
}

export const locale = writable<Locale>(initialLocale());

export function translate(l: Locale, key: string, vars?: Vars): string {
  let text = messages[l]?.[key] ?? messages.es[key] ?? key;
  if (vars) {
    for (const k in vars) text = text.replaceAll(`{${k}}`, String(vars[k]));
  }
  return text;
}

/**
 * Store derivado: úsalo como `$t('clave')` en los componentes para que
 * se re-rendericen al cambiar de idioma.
 */
export const t = derived(locale, ($locale) => (key: string, vars?: Vars) => translate($locale, key, vars));

export function detectLocale(): Locale {
  if (typeof navigator === 'undefined') return 'es';
  const lang = (navigator.languages && navigator.languages[0]) || navigator.language || 'es';
  return lang.toLowerCase().startsWith('es') ? 'es' : 'en';
}

function applyLang(l: Locale) {
  if (typeof document !== 'undefined') document.documentElement.lang = l;
}

export function setLocale(l: Locale) {
  locale.set(l);
  applyLang(l);
  try { localStorage.setItem('locale', l); } catch {}
}

export function initLocale() {
  if (typeof window === 'undefined') return;
  let stored: Locale | null = null;
  try { stored = localStorage.getItem('locale') as Locale | null; } catch {}
  const l = stored ?? detectLocale();
  locale.set(l);
  applyLang(l);
}

export function currentLocale(): Locale {
  return get(locale);
}
