import { describe, expect, it } from 'vitest';
import { buildWebviewHtml } from './webviewHtml';

// The shape Vite actually emits: inline pre-paint scripts in <head>, one
// module script and one stylesheet, all root-absolute.
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

  it('keeps author prose from closing the injection script', () => {
    // a doc comment is author data and may contain literal HTML
    const html = build({ project: 'P', doc: 'never write </script> in prose' });
    expect(html).not.toContain('</script> in prose');
    expect(html).toContain('\\u003c/script> in prose');
  });
});
