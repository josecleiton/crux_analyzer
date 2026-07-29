/**
 * The CSP is generated from the HTML being written, so what needs pinning is
 * that the generator sees the scripts that are actually there — a `script-src`
 * whose hash does not match the inline script blocks the page entirely, and it
 * fails at load time in a browser rather than at build time here.
 */

import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { describe, expect, it } from 'vitest';
import { inlineScripts, policyFor } from '../csp';

const INDEX_HTML = readFileSync(new URL('../index.html', import.meta.url), 'utf8');

describe('inlineScripts', () => {
  it('finds the pre-paint scripts in the real index.html', () => {
    const bodies = inlineScripts(INDEX_HTML);
    expect(bodies).toHaveLength(1);
    // The theme and locale blocks live in that one script element.
    expect(bodies[0]).toContain('crux-analyzer:theme');
    expect(bodies[0]).toContain('crux-analyzer:locale');
  });

  it('ignores scripts with a src', () => {
    expect(inlineScripts('<script type="module" src="/main.js"></script>')).toEqual([]);
    expect(inlineScripts('<script nonce="x" src="/a.js"></script>')).toEqual([]);
  });

  it('finds every inline script, with attributes or without', () => {
    const bodies = inlineScripts('<script>a()</script><script defer>b()</script>');
    expect(bodies).toEqual(['a()', 'b()']);
  });
});

describe('policyFor', () => {
  it('hashes each inline script so the real index.html would load', () => {
    const policy = policyFor(INDEX_HTML);
    for (const body of inlineScripts(INDEX_HTML)) {
      const hash = createHash('sha256').update(body, 'utf8').digest('base64');
      expect(policy).toContain(`'sha256-${hash}'`);
    }
  });

  it('never allows inline or eval for scripts', () => {
    const policy = policyFor(INDEX_HTML);
    const scriptSrc = policy.split('; ').find((d) => d.startsWith('script-src'))!;
    expect(scriptSrc).not.toContain("'unsafe-inline'");
    expect(scriptSrc).not.toContain("'unsafe-eval'");
    expect(scriptSrc).not.toContain('*');
  });

  it('denies everything not explicitly allowed', () => {
    const policy = policyFor(INDEX_HTML);
    expect(policy).toContain("default-src 'none'");
    expect(policy).toContain("base-uri 'none'");
    expect(policy).toContain("form-action 'none'");
    // The model fetch needs this; nothing else is permitted to talk out.
    expect(policy).toContain("connect-src 'self'");
  });

  it('changes when the inline scripts change, so a stale hash cannot survive', () => {
    const edited = INDEX_HTML.replace('crux-analyzer:theme', 'crux-analyzer:theme2');
    expect(policyFor(edited)).not.toBe(policyFor(INDEX_HTML));
  });
});
