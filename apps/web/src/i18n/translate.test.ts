import { describe, expect, it } from 'vitest';
import { LOCALES } from './locale';
import { catalogs, interpolate, makeTranslate, plural } from './translate';
import { en } from './messages/en';

describe('catalogs', () => {
  it('ships one catalog per supported locale', () => {
    expect(Object.keys(catalogs).sort()).toEqual([...LOCALES].sort());
  });

  // `tsc` already enforces this through the `Catalog` type; asserting it here
  // catches drift for anyone running tests without a type-check.
  it('gives every locale exactly the source locale key set', () => {
    const expected = Object.keys(en).sort();
    for (const locale of LOCALES) {
      expect(Object.keys(catalogs[locale]).sort(), locale).toEqual(expected);
    }
  });

  it('has no empty messages', () => {
    for (const locale of LOCALES) {
      for (const [key, value] of Object.entries(catalogs[locale])) {
        expect(value.trim(), `${locale}/${key}`).not.toBe('');
      }
    }
  });

  it('actually translates the prose away from English', () => {
    // Symbols and abbreviations legitimately match across locales.
    const shared = new Set(['inspector.none', 'simulation.unknownState', 'badge.final']);
    const untranslated = Object.keys(en).filter(
      (key) =>
        !shared.has(key) &&
        catalogs['pt-BR'][key as keyof typeof en] === catalogs.en[key as keyof typeof en],
    );
    expect(untranslated).toEqual([]);
  });
});

describe('makeTranslate', () => {
  it('looks messages up in the requested locale', () => {
    expect(makeTranslate('en')('toolbar.simulate')).toBe('Simulate');
    expect(makeTranslate('pt-BR')('toolbar.simulate')).toBe('Simular');
  });

  it('substitutes params', () => {
    expect(makeTranslate('en')('localeToggle.switchTo', { language: 'Português (Brasil)' })).toBe(
      'Switch to Português (Brasil)',
    );
    expect(makeTranslate('pt-BR')('localeToggle.switchTo', { language: 'English' })).toBe(
      'Mudar para English',
    );
  });
});

describe('interpolate', () => {
  it('replaces every occurrence and accepts numbers', () => {
    expect(interpolate('{a} and {a} and {b}', { a: 'x', b: 2 })).toBe('x and x and 2');
  });

  it('leaves unknown placeholders untouched rather than printing undefined', () => {
    expect(interpolate('{known} {unknown}', { known: 'ok' })).toBe('ok {unknown}');
  });

  it('is a no-op without params', () => {
    expect(interpolate('{a}')).toBe('{a}');
  });
});

describe('plural', () => {
  it('selects the form matching the locale rules', () => {
    const forms = { one: '{count} state', other: '{count} states' };
    expect(plural('en', 1, forms)).toBe('1 state');
    expect(plural('en', 0, forms)).toBe('0 states');
    expect(plural('en', 2, forms)).toBe('2 states');
  });

  it('falls back to `other` when a category is not provided', () => {
    expect(plural('pt-BR', 1, { other: '{count} estados' })).toBe('1 estados');
  });
});
