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
    // The nonce authorizes the *inline* scripts (the pre-paint blocks and the
    // model injection). `cspSource` is needed for the bundle's own module
    // files, because a nonce does not extend to modules a nonced script
    // imports — and the bundle is code-split, so the entry chunk statically
    // imports `elk-*.js` and the rolldown runtime. Verified: with the nonce
    // alone, both imports are blocked and the webview renders nothing.
    //
    // This is not a widening of what may run: `cspSource` is the webview's own
    // resource origin, and `localResourceRoots` limits that to the bundle
    // directory. Arbitrary inline script still needs the nonce.
    `script-src 'nonce-${nonce}' ${cspSource}`,
  ].join('; ');

  // `</script>` inside author prose must not close the injection script;
  // escaping `<` inside JSON strings is lossless. U+2028/U+2029 go too:
  // `JSON.stringify` leaves them raw and they are line terminators to a
  // JavaScript parser, so prose containing one would break the statement on any
  // engine older than ES2019.
  const modelJson = JSON.stringify(model)
    .replace(/</g, '\\u003c')
    .replace(/\u2028/g, '\\u2028')
    .replace(/\u2029/g, '\\u2029');

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
