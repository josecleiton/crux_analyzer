/** React hooks for localization. The provider lives in `I18nProvider.tsx`. */

import { useContext } from 'react';
import type { I18n } from './context';
import { I18nContext } from './context';
import type { Translate } from './translate';

export function useI18n(): I18n {
  const context = useContext(I18nContext);
  if (!context) throw new Error('useI18n must be used inside an I18nProvider');
  return context;
}

/** Shorthand for components that only need the lookup. */
export function useTranslate(): Translate {
  return useI18n().t;
}
