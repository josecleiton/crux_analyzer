/**
 * Generates `THIRD-PARTY-NOTICES.md` for the built web bundle.
 *
 * Every dependency in this bundle is permissive, and every permissive license
 * still has one obligation: the notice travels with the code. MIT requires it
 * "in all copies or substantial portions", BSD-3-Clause clause 2 requires binary
 * redistributions to reproduce it "in the documentation or other materials", and
 * elkjs's EPL-2.0 §3.1(a) requires a statement of where its source can be had.
 * The bundle is distributed on GitHub Pages and inside every VSIX, and it
 * carried none of that.
 *
 * ## Why this is driven by the bundler
 *
 * The list comes from the chunks rolldown actually emitted, not from what is
 * installed. Two reasons: the installed tree includes 15 `@types/*` packages
 * that contribute zero bytes to the artifact and therefore have nothing to
 * notice, and `pnpm licenses list` reports store paths that **do not resolve**
 * in this install layout. What shipped is the only correct scope.
 *
 * A missing license is a hard error. A notices file that quietly skips a package
 * is worse than no file at all, because it looks complete.
 */

import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import type { Plugin } from 'vite';

/** What one bundled dependency contributes to the notices file. */
export interface PackageNotice {
  name: string;
  version: string;
  /** SPDX id as declared by the package. */
  license: string;
  /** The `Copyright …` line from the package's own license file, when it has one. */
  copyright: string | null;
  /** Full text of the package's license file, when it ships one. */
  licenseText: string | null;
  homepage: string | null;
}

/** Minimal `package.json` shape this reads. */
interface PackageJson {
  name?: string;
  version?: string;
  license?: string | { type?: string };
  author?: string | { name?: string };
  homepage?: string;
}

/**
 * The package directory a bundled module belongs to, or `null` for our own
 * source and for virtual modules.
 *
 * Uses the *last* `node_modules` segment so that pnpm's
 * `.pnpm/pkg@1.0.0/node_modules/pkg` layout resolves to the innermost package
 * rather than the store entry.
 */
export function packageDirOf(moduleId: string): string | null {
  const id = moduleId.split('\\').join('/');
  // Virtual modules (`\0…`, `virtual:…`) belong to no package.
  if (id.includes('\0') || !id.includes('/node_modules/')) return null;
  const marker = '/node_modules/';
  const start = id.lastIndexOf(marker) + marker.length;
  const rest = id.slice(start);
  if (rest === '' || rest.startsWith('.')) return null;
  const segments = rest.split('/');
  const depth = segments[0].startsWith('@') ? 2 : 1;
  if (segments.length < depth) return null;
  return id.slice(0, start) + segments.slice(0, depth).join('/');
}

/** The `Copyright …` line of a license text, if it states one. */
export function copyrightLine(licenseText: string): string | null {
  for (const raw of licenseText.split('\n')) {
    const line = raw.trim();
    if (/^copyright\b/i.test(line) && /\d{4}|\(c\)|©/i.test(line)) {
      return line.replace(/\s+/g, ' ');
    }
  }
  return null;
}

function declaredLicense(pkg: PackageJson): string | null {
  const license = pkg.license;
  if (typeof license === 'string') return license;
  if (license && typeof license.type === 'string') return license.type;
  return null;
}

function authorName(pkg: PackageJson): string | null {
  const author = pkg.author;
  if (typeof author === 'string') return author.replace(/\s*<[^>]*>/g, '').trim() || null;
  if (author && typeof author.name === 'string') return author.name;
  return null;
}

/** Reads one package's notice. Throws when the package declares no license. */
export function readPackageNotice(dir: string): PackageNotice {
  const pkg: PackageJson = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'));
  const name = pkg.name ?? dir;
  const license = declaredLicense(pkg);
  if (!license) {
    throw new Error(
      `${name} declares no license in package.json. It is bundled into the ` +
        `artifact, so it cannot be listed without one — establish the license ` +
        `and add it to the notices, or drop the dependency.`,
    );
  }

  let licenseText: string | null = null;
  try {
    const file = readdirSync(dir).find((entry) => /^(LICEN[CS]E|COPYING)/i.test(entry));
    if (file) licenseText = readFileSync(join(dir, file), 'utf8').trim();
  } catch {
    // Unreadable directory: handled as "ships no license file" below.
  }

  return {
    name,
    version: pkg.version ?? '0.0.0',
    license,
    copyright: (licenseText && copyrightLine(licenseText)) ?? authorName(pkg),
    licenseText,
    homepage: pkg.homepage ?? null,
  };
}

/** Sorted, de-duplicated notices for a set of bundled module ids. */
export function collectNotices(
  moduleIds: Iterable<string>,
  read: (dir: string) => PackageNotice = readPackageNotice,
): PackageNotice[] {
  const dirs = new Set<string>();
  for (const id of moduleIds) {
    const dir = packageDirOf(id);
    if (dir) dirs.add(dir);
  }
  const byKey = new Map<string, PackageNotice>();
  for (const dir of dirs) {
    const notice = read(dir);
    byKey.set(`${notice.name}@${notice.version}`, notice);
  }
  return [...byKey.values()].sort((a, b) => a.name.localeCompare(b.name));
}

/** elkjs is the one dependency whose license asks for more than a notice. */
function elkSection(notices: PackageNotice[]): string {
  const elk = notices.find((n) => n.name === 'elkjs');
  if (!elk) return '';
  return [
    '## elkjs — Eclipse Public License 2.0',
    '',
    `crux_analyzer bundles **elkjs ${elk.version}**, used unmodified as its graph`,
    'layout engine. elkjs is available under `EPL-2.0 OR GPL-3.0-or-later`, and',
    '**this project elects the EPL-2.0**.',
    '',
    'As EPL-2.0 §3.1(a) requires: the source code of elkjs is available under the',
    'Eclipse Public License 2.0, and can be obtained from',
    '<https://github.com/kieler/elkjs> and from the npm registry',
    `(\`npm pack elkjs@${elk.version}\`). elkjs is emitted as its own chunk`,
    '(`assets/elk-*.js`), separate from this project\'s own code.',
    '',
    'The full text of the EPL-2.0 appears in the license texts below.',
    '',
  ].join('\n');
}

/** The notices file. */
export function renderNotices(notices: PackageNotice[]): string {
  if (notices.length === 0) {
    throw new Error(
      'No third-party packages were found in the bundle. That cannot be right — ' +
        'the notices generator is not seeing the bundled modules.',
    );
  }

  const lines: string[] = [
    '# Third-party notices',
    '',
    'crux_analyzer is MIT licensed (see `LICENSE`). This file covers the',
    'third-party code **bundled into the web application** — the static',
    'documentation site and the web view inside the VS Code extension.',
    '',
    'It is generated from the chunks the bundler actually emitted, so it lists',
    'what ships and nothing else. Regenerate with `just notices`.',
    '',
    `${notices.length} packages are bundled.`,
    '',
  ];

  const elk = elkSection(notices);
  if (elk) lines.push(elk);

  lines.push('## Bundled packages', '', '| Package | Version | License | Copyright |', '| --- | --- | --- | --- |');
  for (const notice of notices) {
    const name = notice.homepage ? `[${notice.name}](${notice.homepage})` : notice.name;
    lines.push(
      `| ${name} | ${notice.version} | ${notice.license} | ${escapeCell(notice.copyright)} |`,
    );
  }

  // One full text per distinct license file, so every copyright holder's own
  // wording survives rather than being replaced by a canonical template.
  lines.push('', '## License texts', '');
  const byText = new Map<string, PackageNotice[]>();
  for (const notice of notices) {
    if (!notice.licenseText) continue;
    const existing = byText.get(notice.licenseText);
    if (existing) existing.push(notice);
    else byText.set(notice.licenseText, [notice]);
  }

  const noFile = notices.filter((n) => !n.licenseText);
  for (const [text, holders] of byText) {
    const names = holders.map((h) => `${h.name} ${h.version}`).join(', ');
    lines.push(`### ${holders[0].license} — ${names}`, '', '```', text, '```', '');
  }

  if (noFile.length > 0) {
    lines.push(
      '### Packages that ship no license file',
      '',
      'These declare a license in `package.json` but include no license file of',
      'their own. The standard text of the license they declare applies; the',
      'declaration is reproduced here with the package.',
      '',
      '| Package | Version | License |',
      '| --- | --- | --- |',
      ...noFile.map((n) => `| ${n.name} | ${n.version} | ${n.license} |`),
      '',
    );
  }

  return lines.join('\n');
}

function escapeCell(value: string | null): string {
  if (!value) return '—';
  return value.replace(/\|/g, '\\|');
}

/**
 * Emits `THIRD-PARTY-NOTICES.md` into the bundle, so every artifact built from
 * it — the Pages site, the VSIX's `media/web` — carries its own notices.
 *
 * It deliberately does *not* write the committed copy at the repository root.
 * That file is the union of this and the Rust binary's notices, assembled by
 * `just notices`; if a plain `web-build` overwrote it with only half, the
 * `notices-current` gate would fail on every build that is not a full one.
 */
export function noticesPlugin(): Plugin {
  return {
    name: 'crux-analyzer-notices',
    generateBundle(_outputOptions, bundle) {
      const moduleIds = new Set<string>();
      for (const output of Object.values(bundle)) {
        if (output.type !== 'chunk') continue;
        for (const id of Object.keys(output.modules)) moduleIds.add(id);
      }
      const source = renderNotices(collectNotices(moduleIds));
      this.emitFile({ type: 'asset', fileName: 'THIRD-PARTY-NOTICES.md', source });
    },
  };
}
