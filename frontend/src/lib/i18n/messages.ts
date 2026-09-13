export type Locale = 'es' | 'en';

const esFiles = import.meta.glob<{ default: Record<string, string> }>('./locales/es/*.json', { eager: true });
const enFiles = import.meta.glob<{ default: Record<string, string> }>('./locales/en/*.json', { eager: true });

function merge(files: Record<string, { default: Record<string, string> }>): Record<string, string> {
  const out: Record<string, string> = {};
  for (const path of Object.keys(files).sort()) {
    Object.assign(out, files[path].default ?? files[path]);
  }
  return out;
}

export const messages: Record<Locale, Record<string, string>> = {
  es: merge(esFiles),
  en: merge(enFiles),
};
