/**
 * The localization context itself, kept apart from both the provider component
 * and the hooks so each module has a single kind of export (which is also what
 * keeps React Fast Refresh working).
 */

import { createContext } from 'react';
import type { Locale } from './locale';
import type { Translate } from './translate';

export interface I18n {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: Translate;
}

export const I18nContext = createContext<I18n | null>(null);
