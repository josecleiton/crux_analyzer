import { describe, expect, it } from 'vitest';
import { resolveSourceDir } from './sourceDir';

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
    expect(resolveSourceDir('/lib/core/', '/ws/', never)).toBe('/ws/lib/core/');
  });
});
