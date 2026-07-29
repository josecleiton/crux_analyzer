/**
 * Theme contract: the two color schemes, how the active one is resolved
 * (explicit user choice → OS preference) and how it is applied to the DOM.
 *
 * The `data-theme` attribute on <html> is the single switch: every color in
 * `index.css` comes from a token defined per theme there, so components never
 * hardcode colors. The few colors that must reach SVG presentation attributes
 * (marker arrowheads, which cannot read CSS variables) are read back from the
 * same tokens through `readGraphColors`, keeping the CSS the single source.
 */

export type Theme = 'light' | 'dark';

/** Must stay in sync with the pre-paint script in `index.html`. */
export const THEME_STORAGE_KEY = 'crux-analyzer:theme';

export function storedTheme(): Theme | null {
  try {
    const value = localStorage.getItem(THEME_STORAGE_KEY);
    return value === 'light' || value === 'dark' ? value : null;
  } catch {
    return null; // storage disabled (private mode): fall back to the OS
  }
}

export function storeTheme(theme: Theme): void {
  try {
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // not persisting is acceptable; the session still honors the choice
  }
}

export function systemTheme(): Theme {
  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

/** The theme already applied by the pre-paint script, or the resolved one. */
export function currentTheme(): Theme {
  const applied = document.documentElement.dataset.theme;
  if (applied === 'light' || applied === 'dark') return applied;
  return storedTheme() ?? systemTheme();
}

export function applyTheme(theme: Theme): void {
  document.documentElement.dataset.theme = theme;
}

/** Graph colors that SVG attributes need as literal values. */
export interface GraphColors {
  edge: string;
  edgeSelected: string;
  edgeHighlighted: string;
  edgeFailure: string;
}

export function readGraphColors(): GraphColors {
  const style = getComputedStyle(document.documentElement);
  const token = (name: string, fallback: string) =>
    style.getPropertyValue(name).trim() || fallback;
  return {
    edge: token('--edge-stroke', '#8792a2'),
    edgeSelected: token('--edge-stroke-selected', '#6366f1'),
    edgeHighlighted: token('--edge-stroke-highlighted', '#059669'),
    edgeFailure: token('--edge-stroke-failure', '#dc2626'),
  };
}
