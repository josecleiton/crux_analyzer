/** React bindings for the theme: the active theme, the toggle, and the
 *  token-derived colors the graph needs as literal SVG values. */

import { useCallback, useEffect, useState } from 'react';
import type { GraphColors, Theme } from './theme';
import { applyTheme, currentTheme, readGraphColors, storeTheme, storedTheme } from './theme';

export function useTheme(): { theme: Theme; toggleTheme: () => void } {
  const [theme, setTheme] = useState<Theme>(currentTheme);

  // The DOM attribute is written before the re-render (not in an effect) so
  // that components reading the color tokens already see the new theme.
  const select = useCallback((next: Theme) => {
    applyTheme(next);
    setTheme(next);
  }, []);

  useEffect(() => {
    applyTheme(theme); // first paint / recovery if the attribute is missing
  }, [theme]);

  // Follow the OS while the user has not made an explicit choice.
  useEffect(() => {
    if (storedTheme()) return;
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const onChange = (event: MediaQueryListEvent) => select(event.matches ? 'dark' : 'light');
    media.addEventListener('change', onChange);
    return () => media.removeEventListener('change', onChange);
  }, [theme, select]);

  const toggleTheme = useCallback(() => {
    const next: Theme = theme === 'dark' ? 'light' : 'dark';
    storeTheme(next);
    select(next);
  }, [theme, select]);

  return { theme, toggleTheme };
}

/** Re-reads the color tokens whenever the theme changes. */
export function useGraphColors(theme: Theme): GraphColors {
  const [colors, setColors] = useState<GraphColors>(readGraphColors);
  useEffect(() => {
    setColors(readGraphColors());
  }, [theme]);
  return colors;
}
