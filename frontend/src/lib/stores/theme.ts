import { writable } from 'svelte/store';

export const theme = writable<'light' | 'dark' | 'system'>('system');

export function initTheme() {
  if (typeof window !== 'undefined') {
    const stored = localStorage.getItem('theme') as 'light' | 'dark' | 'system' | null;
    if (stored) {
      theme.set(stored);
      applyTheme(stored);
    } else {
      applyTheme('system');
    }
  }
}

export function setTheme(t: 'light' | 'dark' | 'system') {
  theme.set(t);
  if (typeof window !== 'undefined') {
    localStorage.setItem('theme', t);
    applyTheme(t);
  }
}

export function toggleTheme() {
  if (typeof window !== 'undefined') {
    const isDark = document.documentElement.classList.contains('dark');
    const newTheme = isDark ? 'light' : 'dark';
    setTheme(newTheme);
  }
}

function applyTheme(t: 'light' | 'dark' | 'system') {
  if (typeof document !== 'undefined') {
    const isDark = t === 'dark' || (t === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
    document.documentElement.classList.toggle('dark', isDark);
  }
}
