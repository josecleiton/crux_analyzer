import { describe, expect, it } from 'vitest';
import { isContained, resolveSourceDir } from './sourceDir';

const never = () => false;

describe('resolveSourceDir', () => {
  it('honors an explicit setting even when the directory is missing', () => {
    // the analyzer's own error beats silently analyzing somewhere else
    expect(resolveSourceDir('backend/src', '/ws', never)).toBe('/ws/backend/src');
  });

  it('falls back to the conventional Crux layout, then to src', () => {
    expect(resolveSourceDir('', '/ws', (p) => p === '/ws/shared/src')).toBe('/ws/shared/src');
    expect(resolveSourceDir('', '/ws', (p) => p === '/ws/src')).toBe('/ws/src');
  });

  it('prefers shared/src when both exist', () => {
    expect(resolveSourceDir('', '/ws', () => true)).toBe('/ws/shared/src');
  });

  it('returns null when nothing is configured and nothing conventional exists', () => {
    expect(resolveSourceDir('  ', '/ws', never)).toBeNull();
  });

  it('normalizes the joining slashes', () => {
    // A leading separator reads as workspace-relative, and redundant separators
    // collapse — including the trailing one.
    expect(resolveSourceDir('/lib/core/', '/ws/', never)).toBe('/ws/lib/core');
    expect(resolveSourceDir('lib//core', '/ws', never)).toBe('/ws/lib/core');
  });

  /**
   * `cruxAnalyzer.src` is workspace-scoped, so a cloned repository's
   * `.vscode/settings.json` sets it. It must not be able to point the analyzer —
   * or the file watcher that follows it — outside the folder the user opened.
   */
  it('refuses a setting that escapes the workspace root', () => {
    for (const escape of [
      '../secrets',
      '../../etc',
      'shared/../../etc',
      './../etc',
      '..\\windows\\system32',
      '/../etc',
    ]) {
      expect(resolveSourceDir(escape, '/ws', () => true), escape).toBeNull();
    }
  });

  it('allows a traversal that stays inside', () => {
    expect(resolveSourceDir('shared/../src', '/ws', never)).toBe('/ws/src');
    expect(resolveSourceDir('./src', '/ws', never)).toBe('/ws/src');
  });
});

describe('isContained', () => {
  it('accepts paths that stay in the workspace', () => {
    for (const path of ['', 'src', 'shared/src', './src', 'a/../b', '/lib/core']) {
      expect(isContained(path), path).toBe(true);
    }
  });

  it('rejects paths that climb out', () => {
    for (const path of ['..', '../x', 'a/../../x', '..\\x']) {
      expect(isContained(path), path).toBe(false);
    }
  });
});
