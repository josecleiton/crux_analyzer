/**
 * Provides the active locale and the `t` lookup to the whole app.
 *
 * This is the one place the localization layer diverges from `theme/useTheme`.
 * The theme threads through props because only two components need it, while
 * `t` is needed by every panel — so it comes through context instead, provided
 * once in `main.tsx`.
 */

import { useCallback, useEffect, useMemo, useState } from 'react';
import type { ReactNode } from 'react';
import type { I18n } from './context';
import { I18nContext } from './context';
import type { Locale } from './locale';
import { applyLocale, currentLocale, storeLocale, storedLocale, systemLocale } from './locale';
import { makeTranslate } from './translate';

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(currentLocale);

  // The DOM attributes are written before the re-render (not in an effect) so
  // that anything reading `lang` already sees the new locale.
  const select = useCallback((next: Locale) => {
    applyLocale(next);
    setLocaleState(next);
  }, []);

  useEffect(() => {
    applyLocale(locale); // first paint / recovery if the attribute is missing
  }, [locale]);

  // Follow the browser while the user has not made an explicit choice. Unlike
  // `prefers-color-scheme` there is no change event for `navigator.languages`,
  // so this resolves once per mount rather than live.
  useEffect(() => {
    if (storedLocale()) return;
    const preferred = systemLocale();
    if (preferred !== locale) select(preferred);
  }, [locale, select]);

  const setLocale = useCallback(
    (next: Locale) => {
      storeLocale(next);
      select(next);
    },
    [select],
  );

  const value = useMemo<I18n>(
    () => ({ locale, setLocale, t: makeTranslate(locale) }),
    [locale, setLocale],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}
