/**
 * Language switcher, sitting next to the theme toggle.
 *
 * The button shows the **active** locale: a text label reads as state, and
 * showing the destination instead made the UI look mislabeled. Where it leads
 * is the accessible name and the tooltip, using the target locale's endonym —
 * a language is named in its own language, so the name goes in untranslated.
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
      {localeShortLabel(locale)}
    </button>
  );
}
