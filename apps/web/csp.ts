/**
 * Injects a Content-Security-Policy meta tag into the built `index.html`.
 *
 * The VS Code webview has had a strict nonce CSP since it shipped; the static
 * site — the `just site` deployment, the GitHub Pages preview — had none, and it
 * is the deployment that renders author prose through a Markdown library for an
 * audience that did not write it. Nothing in the app has an HTML-injection sink
 * (see `StateDoc.test.tsx`), so this is defence in depth: it is what holds if
 * one is ever introduced.
 *
 * The hashes are **computed from the file being written**, not pasted in. The
 * bundle carries two inline pre-paint scripts (theme and locale) which must run
 * before first paint, so they cannot move to an external file; hand-maintained
 * hashes would go stale the first time someone edits them, and a stale
 * `script-src` hash breaks the whole page silently. `csp.test.ts` pins that the
 * generated policy matches the scripts actually present.
 */

import { createHash } from 'node:crypto';
import type { Plugin } from 'vite';

/** `<script>`…`</script>` bodies with no `src`, in document order. */
export function inlineScripts(html: string): string[] {
  const bodies: string[] = [];
  const pattern = /<script(?![^>]*\bsrc=)[^>]*>([\s\S]*?)<\/script>/gi;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(html)) !== null) {
    bodies.push(match[1]);
  }
  return bodies;
}

function sha256(source: string): string {
  return `'sha256-${createHash('sha256').update(source, 'utf8').digest('base64')}'`;
}

/**
 * The policy for `html`.
 *
 * - `default-src 'none'` — everything is opt-in below.
 * - `script-src` is `'self'` (the bundle) plus a hash per inline pre-paint
 *   script. No `'unsafe-inline'`, no `'unsafe-eval'`.
 * - `style-src` needs `'unsafe-inline'`: React and React Flow set style
 *   attributes at runtime. This is the one concession, and it is the same one
 *   the webview CSP already makes.
 * - `connect-src 'self'` allows the `model.json` fetch and nothing else.
 * - `img-src` allows the bundle's own assets and data URIs. Author prose cannot
 *   reach an `<img>` at all (`StateDoc` renders alt text instead), so no remote
 *   host is needed.
 * - no `frame-src`, `object-src` or `form-action` beyond `default-src 'none'`,
 *   which already denies them.
 */
export function policyFor(html: string): string {
  const hashes = inlineScripts(html).map(sha256);
  return [
    "default-src 'none'",
    `script-src 'self' ${hashes.join(' ')}`.trimEnd(),
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' data:",
    "font-src 'self'",
    "connect-src 'self'",
    "base-uri 'none'",
    "form-action 'none'",
  ].join('; ');
}

export function cspPlugin(): Plugin {
  return {
    name: 'crux-analyzer-csp',
    // `post` so the tag is added after Vite has finished rewriting script tags:
    // hashing before that would hash markup the browser never sees.
    transformIndexHtml: {
      order: 'post',
      handler(html) {
        const meta = `<meta http-equiv="Content-Security-Policy" content="${policyFor(html)}" />`;
        return html.replace(/<head>/, `<head>\n    ${meta}`);
      },
    },
  };
}
