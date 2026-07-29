/**
 * Language switcher, sitting next to the theme toggle.
 *
 * The button shows the locale it will switch *to*, and its accessible name
 * uses that locale's endonym — a language is named in its own language, so the
 * name is substituted untranslated.
 */

import { LOCALES, localeEndonym, localeShortLabel } from '../../i18n/locale';
import { useI18n } from '../../i18n/useI18n';

export function LocaleToggle() {
  const { locale, setLocale, t } = useI18n();
  const next = LOCALES[(LOCALES.indexOf(locale) + 1) % LOCALES.length];
  const label = t('localeToggle.switchTo', { language: localeEndonym(next) });

  return (
    <button
      className="locale-toggle"
      onClick={() => setLocale(next)}
      title={label}
      aria-label={label}
    >
      {localeShortLabel(next)}
    </button>
  );
}
