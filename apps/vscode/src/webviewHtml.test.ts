import { describe, expect, it } from 'vitest';
import { buildWebviewHtml } from './webviewHtml';

// The shape Vite actually emits: inline pre-paint scripts in <head>, one entry
// module script, `modulepreload` links for the chunks it imports (the bundle is
// code-split — elkjs has its own chunk), and one stylesheet, all root-absolute.
const INDEX_HTML = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/favicon.svg" />
    <title>crux_analyzer</title>
    <script>
      document.documentElement.dataset.theme = 'dark';
    </script>
    <script type="module" crossorigin src="/assets/index-abc123.js"></script>
    <link rel="modulepreload" crossorigin href="/assets/elk-ghi789.js">
    <link rel="stylesheet" crossorigin href="/assets/index-def456.css">
  </head>
  <body>
    <div id="root"></div>
  </body>
</html>`;

function build(model: unknown = { project: 'P', cores: [] }) {
  return buildWebviewHtml({
    indexHtml: INDEX_HTML,
    webRootUri: 'vscode-resource://ext/media/web',
    cspSource: 'vscode-resource://ext',
    nonce: 'NONCE',
    model,
  });
}

describe('buildWebviewHtml', () => {
  it('re-roots every root-absolute URL onto the webview base', () => {
    const html = build();
    expect(html).toContain('src="vscode-resource://ext/media/web/assets/index-abc123.js"');
    expect(html).toContain('href="vscode-resource://ext/media/web/assets/index-def456.css"');
    expect(html).toContain('href="vscode-resource://ext/media/web/favicon.svg"');
    expect(html).not.toMatch(/(src|href)="\//);
  });

  it('stamps the nonce on every script, the inline pre-paint ones included', () => {
    const html = build();
    const scripts = html.match(/<script/g) ?? [];
    const nonced = html.match(/<script nonce="NONCE"/g) ?? [];
    expect(scripts.length).toBe(3); // pre-paint + module + injected model
    expect(nonced.length).toBe(scripts.length);
  });

  it('injects the model before any other script and locks the CSP to the nonce', () => {
    const html = build({ project: 'My App', cores: [] });
    const modelAt = html.indexOf('window.__CRUX_MODEL__ = {"project":"My App"');
    const firstOtherScript = html.indexOf('dataset.theme');
    expect(modelAt).toBeGreaterThan(-1);
    expect(modelAt).toBeLessThan(firstOtherScript);
    expect(html).toContain("script-src 'nonce-NONCE'");
    expect(html).toContain("default-src 'none'");
  });

  /**
   * The bundle is code-split, so the entry chunk statically *imports* other
   * chunks. A nonce authorizes the element it sits on and does not extend to
   * modules that element imports — so a nonce-only `script-src` blocks every
   * split chunk and the webview renders an empty page. Verified in a browser
   * before this was allowed: 0 nodes with the nonce alone, the full graph with
   * the bundle origin permitted.
   */
  it('lets the bundle load its own split chunks, not just the nonced scripts', () => {
    const html = build();
    const csp = html.match(/content="([^"]*default-src[^"]*)"/)![1];
    const scriptSrc = csp.split('; ').find((d) => d.startsWith('script-src'))!;
    expect(scriptSrc).toContain("'nonce-NONCE'");
    expect(scriptSrc).toContain('vscode-resource://ext');
    // Still no blanket permission: inline script needs the nonce.
    expect(scriptSrc).not.toContain("'unsafe-inline'");
    expect(scriptSrc).not.toContain("'unsafe-eval'");
    expect(scriptSrc).not.toMatch(/(^|\s)\*/);
  });

  it('re-roots the modulepreload of a split chunk', () => {
    // A preload the webview cannot fetch is a chunk the page cannot import.
    expect(build()).toContain(
      'href="vscode-resource://ext/media/web/assets/elk-ghi789.js"',
    );
  });

  it('keeps author prose from closing the injection script', () => {
    // a doc comment is author data and may contain literal HTML
    const html = build({ project: 'P', doc: 'never write </script> in prose' });
    expect(html).not.toContain('</script> in prose');
    expect(html).toContain('\\u003c/script> in prose');
  });

  it('escapes the line terminators JSON.stringify leaves raw', () => {
    // U+2028/U+2029 are line terminators to a JavaScript parser but legal
    // unescaped inside a JSON string, so they would break the statement.
    const html = build({ project: 'P', doc: 'before\u2028after\u2029end' });
    expect(html).not.toContain('\u2028');
    expect(html).not.toContain('\u2029');
    expect(html).toContain('before\\u2028after\\u2029end');
  });

  it('produces a parseable injection for hostile prose', () => {
    // The real requirement behind the escaping: the script still runs.
    const doc = '</script><script>alert(1)</script>\u2028\u2029 "quotes" \\ backslash';
    const html = build({ project: 'P', doc });
    const injected = html.match(/window\.__CRUX_MODEL__ = (.*);<\/script>/)!;
    expect(injected).not.toBeNull();
    const parsed = JSON.parse(injected[1].replace(/\\u003c/g, '<'));
    expect(parsed.doc).toBe(doc);
  });
});
