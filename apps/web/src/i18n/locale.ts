/**
 * Locale contract: the supported languages, how the active one is resolved
 * (explicit user choice → browser preference) and how it is applied to the DOM.
 *
 * Mirrors `theme/theme.ts` on purpose — same resolution order, same
 * storage-may-be-disabled handling. `data-locale` on <html> is the switch the
 * app reads back (set by the pre-paint script in `index.html`); `lang` is set
 * alongside it because that is the attribute assistive technology and CSS
 * hyphenation actually consume.
 */

export type Locale = 'en' | 'pt-BR';

/** In display order. */
export const LOCALES: readonly Locale[] = ['en', 'pt-BR'];

/** The source locale, and the fallback whenever resolution fails. */
export const DEFAULT_LOCALE: Locale = 'en';

/** Must stay in sync with the pre-paint script in `index.html`. */
export const LOCALE_STORAGE_KEY = 'crux-analyzer:locale';

export function isLocale(value: unknown): value is Locale {
  return typeof value === 'string' && (LOCALES as readonly string[]).includes(value);
}

/**
 * A locale's name for itself, for the language switcher. Endonyms are proper
 * names: they read the same regardless of the active locale.
 */
export function localeEndonym(locale: Locale): string {
  return locale === 'pt-BR' ? 'Português (Brasil)' : 'English';
}

/** Short label for a compact toggle. */
export function localeShortLabel(locale: Locale): string {
  return locale === 'pt-BR' ? 'PT' : 'EN';
}

export function storedLocale(): Locale | null {
  try {
    const value = localStorage.getItem(LOCALE_STORAGE_KEY);
    return isLocale(value) ? value : null;
  } catch {
    return null; // storage disabled (private mode): fall back to the browser
  }
}

export function storeLocale(locale: Locale): void {
  try {
    localStorage.setItem(LOCALE_STORAGE_KEY, locale);
  } catch {
    // not persisting is acceptable; the session still honors the choice
  }
}

/**
 * The best match for the browser's languages.
 *
 * Any Portuguese maps to pt-BR — it is the only Portuguese catalog shipped,
 * and showing it beats falling back to English for a pt-PT reader.
 */
export function systemLocale(): Locale {
  const preferences = navigator.languages?.length
    ? navigator.languages
    : [navigator.language].filter(Boolean);
  for (const preference of preferences) {
    const language = preference.toLowerCase().split('-')[0];
    if (language === 'pt') return 'pt-BR';
    if (language === 'en') return 'en';
  }
  return DEFAULT_LOCALE;
}

/** The locale already applied by the pre-paint script, or the resolved one. */
export function currentLocale(): Locale {
  const applied = document.documentElement.dataset.locale;
  if (isLocale(applied)) return applied;
  return storedLocale() ?? systemLocale();
}

export function applyLocale(locale: Locale): void {
  document.documentElement.dataset.locale = locale;
  document.documentElement.lang = locale;
}
