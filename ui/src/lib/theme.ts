import type { ThemeMode } from './types';

let mediaQuery: MediaQueryList | null = null;
let activeMode: ThemeMode = 'system';

function resolveTheme(mode: ThemeMode): 'light' | 'dark' {
  if (mode === 'light' || mode === 'dark') return mode;
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

export function applyTheme(mode: ThemeMode): void {
  activeMode = mode;
  document.documentElement.setAttribute('data-theme', resolveTheme(mode));
}

export function initThemeListener(): void {
  if (typeof window === 'undefined' || !window.matchMedia) return;
  mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
  mediaQuery.addEventListener('change', () => {
    if (activeMode === 'system') {
      applyTheme('system');
    }
  });
}
