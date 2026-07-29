/**
 * The notices file is a license obligation, so what needs pinning is that it
 * cannot become *quietly* wrong: a package silently missing, a scope that drifts
 * to the installed tree instead of the bundle, or elkjs's EPL-2.0 §3.1(a)
 * statement disappearing. The last block reads the committed file itself, the
 * way `csp.test.ts` reads the real `index.html`.
 */

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import {
  collectNotices,
  copyrightLine,
  packageDirOf,
  renderNotices,
  type PackageNotice,
} from '../notices';

const NOTICES = readFileSync(new URL('../../../THIRD-PARTY-NOTICES.md', import.meta.url), 'utf8');

const notice = (over: Partial<PackageNotice> = {}): PackageNotice => ({
  name: 'thing',
  version: '1.0.0',
  license: 'MIT',
  copyright: 'Copyright (c) 2020 Someone',
  licenseText: 'MIT License\n\nCopyright (c) 2020 Someone',
  homepage: null,
  ...over,
});

describe('packageDirOf', () => {
  it('resolves the innermost package in a pnpm layout', () => {
    // The store path repeats the name; the *last* node_modules is the package.
    expect(packageDirOf('/w/node_modules/.pnpm/react@19.0.0/node_modules/react/index.js')).toBe(
      '/w/node_modules/.pnpm/react@19.0.0/node_modules/react',
    );
  });

  it('keeps both segments of a scoped package', () => {
    expect(packageDirOf('/w/node_modules/@xyflow/react/dist/index.js')).toBe(
      '/w/node_modules/@xyflow/react',
    );
  });

  it('ignores our own source and virtual modules', () => {
    expect(packageDirOf('/w/apps/web/src/App.tsx')).toBeNull();
    expect(packageDirOf('\0vite/preload-helper')).toBeNull();
    expect(packageDirOf('virtual:something')).toBeNull();
  });

  it('handles Windows separators', () => {
    expect(packageDirOf('C:\\w\\node_modules\\react\\index.js')).toBe('C:/w/node_modules/react');
  });
});

describe('copyrightLine', () => {
  it('finds the copyright line in a license text', () => {
    expect(copyrightLine('MIT License\n\nCopyright (c) 2016 Someone\n\nPermission...')).toBe(
      'Copyright (c) 2016 Someone',
    );
    expect(copyrightLine('Copyright 2010-2021 Mike Bostock')).toBe(
      'Copyright 2010-2021 Mike Bostock',
    );
  });

  it('ignores prose that merely mentions copyright', () => {
    // No year and no (c): this is the license body, not the notice.
    expect(copyrightLine('The above copyright notice shall be included')).toBeNull();
    expect(copyrightLine('MIT License\n\nPermission is hereby granted')).toBeNull();
  });
});

describe('collectNotices', () => {
  it('lists each bundled package once, sorted, skipping our own modules', () => {
    const ids = [
      '/w/node_modules/b-pkg/index.js',
      '/w/node_modules/b-pkg/other.js', // same package twice
      '/w/node_modules/a-pkg/index.js',
      '/w/apps/web/src/main.tsx', // ours
    ];
    const read = (dir: string) => notice({ name: dir.split('/').pop()! });
    expect(collectNotices(ids, read).map((n) => n.name)).toEqual(['a-pkg', 'b-pkg']);
  });

  it('propagates a package that declares no license', () => {
    const read = () => {
      throw new Error('no license');
    };
    expect(() => collectNotices(['/w/node_modules/x/i.js'], read)).toThrow('no license');
  });
});

describe('renderNotices', () => {
  it('refuses to emit an empty file', () => {
    // An empty list means the generator lost sight of the bundle — a file
    // claiming zero dependencies is worse than a failed build.
    expect(() => renderNotices([])).toThrow(/not seeing the bundled modules/);
  });

  it('states the EPL-2.0 election and where to get elkjs source', () => {
    const out = renderNotices([notice({ name: 'elkjs', version: '0.12.0', license: 'EPL-2.0 OR GPL-3.0-or-later' })]);
    expect(out).toContain('elects the EPL-2.0');
    expect(out).toContain('§3.1(a)');
    expect(out).toContain('https://github.com/kieler/elkjs');
    expect(out).toContain('elkjs@0.12.0');
  });

  it('reproduces each license text and does not repeat a shared one', () => {
    const shared = 'MIT License\n\nCopyright (c) 2015 Titus Wormer';
    const out = renderNotices([
      notice({ name: 'bail', licenseText: shared }),
      notice({ name: 'unified', licenseText: shared }),
      notice({ name: 'other', licenseText: 'ISC License\n\nCopyright (c) 2021 Someone Else' }),
    ]);
    expect(out).toContain('bail 1.0.0, unified 1.0.0');
    expect(out.split('Copyright (c) 2015 Titus Wormer').length - 1).toBeLessThanOrEqual(2);
    expect(out).toContain('Someone Else');
  });

  it('escapes a pipe in a copyright holder so the table survives', () => {
    const out = renderNotices([notice({ copyright: 'Copyright (c) A | B' })]);
    const row = out.split('\n').find((l) => l.startsWith('| thing'))!;
    // Only unescaped pipes are column delimiters: four columns means five.
    const delimiters = [...row].filter((c, i) => c === '|' && row[i - 1] !== '\\').length;
    expect(delimiters).toBe(5);
    expect(row).toContain('A \\| B');
  });

  it('names the packages that ship no license file rather than dropping them', () => {
    const out = renderNotices([notice({ name: 'bare', licenseText: null, copyright: null })]);
    expect(out).toContain('ship no license file');
    expect(out).toContain('| bare | 1.0.0 | MIT |');
  });
});

describe('the committed THIRD-PARTY-NOTICES.md', () => {
  it('covers the dependencies that actually ship', () => {
    for (const pkg of ['elkjs', 'react', 'react-dom', '@xyflow/react', 'react-markdown']) {
      expect(NOTICES, pkg).toMatch(new RegExp(`\\| (\\[)?${pkg.replace('/', '\\/')}(\\])?[( ]`));
    }
  });

  it('carries elkjs’s EPL-2.0 statement and the full license text', () => {
    expect(NOTICES).toContain('elects the EPL-2.0');
    expect(NOTICES).toContain('https://github.com/kieler/elkjs');
    // Enough of the EPL body to prove the whole text is there, not a summary.
    expect(NOTICES).toContain('Eclipse Public License - v 2.0');
    expect(NOTICES).toContain('3.1 If a Contributor Distributes the Program in any form');
    expect(NOTICES).toContain('Exhibit A');
  });

  it('reproduces the copyright of a package whose header the minifier drops', () => {
    expect(NOTICES).toContain('Copyright (c) Meta Platforms, Inc. and affiliates.');
  });

  it('lists no @types package — they contribute no code to the artifact', () => {
    expect(NOTICES).not.toContain('@types/');
  });

  it('covers every license the bundle actually contains', () => {
    for (const license of ['MIT', 'ISC', 'BSD-3-Clause', 'EPL-2.0']) {
      expect(NOTICES, license).toContain(license);
    }
  });
});
