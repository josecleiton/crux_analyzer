/**
 * Turns the built web bundle's `index.html` into webview-servable HTML.
 * Pure string → string, so the transformation is unit-testable without an
 * extension host.
 *
 * Three things separate a webview from the static site the bundle was built
 * for, and each gets one rewrite:
 *
 * - webviews cannot serve `/assets/...` — root-absolute URLs are re-rooted
 *   onto the `asWebviewUri` base the host passes in;
 * - webviews want a strict CSP — scripts run under a nonce, which is also
 *   stamped onto the bundle's own inline pre-paint scripts (theme/locale);
 * - webviews have no HTTP origin to fetch `model.json` from — the model is
 *   injected as `window.__CRUX_MODEL__`, the embedding contract the web
 *   app's `loadProject` honors before it ever tries to fetch.
 */

export interface WebviewHtmlOptions {
  /** The built bundle's index.html, verbatim. */
  indexHtml: string;
  /** Webview URI of the bundle directory (no trailing slash). */
  webRootUri: string;
  /** The webview's `cspSource`. */
  cspSource: string;
  /** Nonce authorizing every script on the page. */
  nonce: string;
  /** The parser model to inject, still unparsed — passed through verbatim. */
  model: unknown;
}

export function buildWebviewHtml(options: WebviewHtmlOptions): string {
  const { indexHtml, webRootUri, cspSource, nonce, model } = options;

  const csp = [
    "default-src 'none'",
    `img-src ${cspSource} data:`,
    // the bundle sets style attributes at runtime (React, React Flow)
    `style-src ${cspSource} 'unsafe-inline'`,
    `font-src ${cspSource}`,
    `script-src 'nonce-${nonce}'`,
  ].join('; ');

  // `</script>` inside author prose must not close the injection script;
  // escaping `<` inside JSON strings is lossless.
  const modelJson = JSON.stringify(model).replace(/</g, '\\u003c');

  return (
    indexHtml
      // root-absolute asset URLs → webview URIs
      .replace(/(src|href)="\//g, `$1="${webRootUri}/`)
      // every script — the bundle's module and its inline pre-paint blocks —
      // runs under the same nonce
      .replace(/<script(?![^>]*\bnonce=)/g, `<script nonce="${nonce}"`)
      // CSP and the injected model enter right after <head>, so the model
      // exists before any script runs
      .replace(
        /<head>/,
        `<head>\n    <meta http-equiv="Content-Security-Policy" content="${csp}" />\n` +
          `    <script nonce="${nonce}">window.__CRUX_MODEL__ = ${modelJson};</script>`,
      )
  );
}
