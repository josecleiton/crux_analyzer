/**
 * Message lookup and interpolation.
 *
 * Deliberately DOM-free so it is unit-testable in the project's default
 * `node` test environment — the DOM and storage side of localization lives in
 * `locale.ts`.
 */

import type { Locale } from './locale';
import { DEFAULT_LOCALE } from './locale';
import type { Catalog, MessageKey } from './messages/en';
import { en } from './messages/en';
import { ptBR } from './messages/pt-BR';

export type { Catalog, MessageKey };

export const catalogs: Record<Locale, Catalog> = {
  en,
  'pt-BR': ptBR,
};

export type MessageParams = Record<string, string | number>;

export type Translate = (key: MessageKey, params?: MessageParams) => string;

/**
 * Builds the lookup for `locale`.
 *
 * The catalogs are type-checked for parity, so the fallbacks here only matter
 * if a catalog is reached from untyped code; they keep the UI readable instead
 * of blank in that case.
 */
export function makeTranslate(locale: Locale): Translate {
  const catalog = catalogs[locale] ?? catalogs[DEFAULT_LOCALE];
  const fallback = catalogs[DEFAULT_LOCALE];
  return (key, params) => interpolate(catalog[key] ?? fallback[key] ?? key, params);
}

/** Replaces every `{name}` with the matching param. */
export function interpolate(template: string, params?: MessageParams): string {
  if (typeof template !== 'string') return String(template ?? '');
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in params ? String(params[name]) : whole,
  );
}

/**
 * Picks the right form for a count, via the platform's own plural rules.
 *
 * No message needs this yet. It exists so the first counted string is written
 * as plural forms instead of concatenated by hand — `"1 state(s)"` and
 * `n + ' states'` are both wrong in locales with more than two categories.
 */
export function plural(
  locale: Locale,
  count: number,
  forms: Partial<Record<Intl.LDMLPluralRule, string>>,
): string {
  const category = new Intl.PluralRules(locale).select(count);
  const form = forms[category] ?? forms.other ?? '';
  return interpolate(form, { count });
}
