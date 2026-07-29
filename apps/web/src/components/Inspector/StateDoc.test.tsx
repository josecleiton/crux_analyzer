/**
 * The regression gate for the one invariant this app cannot afford to lose:
 * **author prose never becomes markup.**
 *
 * Doc comments come from whatever repository the analyzer was pointed at, and
 * `DocText` renders them through a Markdown library on a public site and inside
 * a VS Code webview. react-markdown's defaults are safe today; a `rehype-raw`
 * added for a legitimate-sounding reason would silently revoke that, and this
 * file is what fails when it does.
 *
 * Rendered with `renderToStaticMarkup` rather than a DOM harness on purpose: it
 * exercises the real render path and needs no jsdom, so the security gate costs
 * the project no new dependency.
 */

import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { DocText, safeUrl } from './StateDoc';

const render = (doc: string) => renderToStaticMarkup(<DocText doc={doc} />);

describe('DocText', () => {
  it('renders raw HTML in prose as visible text, not as elements', () => {
    const html = render('<script>alert(1)</script> and <img src=x onerror=alert(1)>');
    // No element is created: the payload survives only as escaped text, where
    // `onerror` is a word on the page rather than an attribute.
    expect(html).not.toContain('<script');
    expect(html).not.toContain('<img');
    expect(html).toContain('&lt;script&gt;');
    expect(html).toContain('&lt;img src=x onerror=alert(1)&gt;');
    // The only tag in the output is DocText's own wrapper.
    expect(html.match(/<[a-zA-Z]/g)).toEqual(['<d']);
  });

  it('does not create an element for HTML that looks harmless either', () => {
    // `skipHtml` is off, so this must appear as text rather than disappear.
    const html = render('<b>bold?</b>');
    expect(html).not.toContain('<b>');
    expect(html).toContain('&lt;b&gt;');
  });

  it('keeps generics readable — the case an HTML sanitizer would eat', () => {
    expect(render('Holds a Vec<String> of ids.')).toContain('Vec&lt;String&gt;');
  });

  it('strips javascript: and data: URLs from links', () => {
    for (const scheme of [
      'javascript:alert(1)',
      'JavaScript:alert(1)',
      'data:text/html,<script>alert(1)</script>',
      'vbscript:msgbox(1)',
    ]) {
      const html = render(`[click](${scheme})`);
      expect(html.toLowerCase()).not.toContain('javascript:');
      expect(html.toLowerCase()).not.toContain('data:text/html');
      expect(html.toLowerCase()).not.toContain('vbscript:');
    }
  });

  it('marks external links noopener, noreferrer and nofollow', () => {
    const html = render('[docs](https://example.com)');
    expect(html).toContain('href="https://example.com"');
    expect(html).toContain('rel="noopener noreferrer nofollow"');
  });

  it('never emits an img element — an external image is a read beacon', () => {
    const html = render('![alt text](https://tracker.example.com/pixel.png)');
    expect(html).not.toContain('<img');
    expect(html).not.toContain('tracker.example.com');
    expect(html).toContain('alt text');
  });

  it('still renders the Markdown the feature exists for', () => {
    const html = render('Uses `Vec` and **bold**\n\n- one\n- two');
    expect(html).toContain('<code>Vec</code>');
    expect(html).toContain('<strong>bold</strong>');
    expect(html).toContain('<li>one</li>');
  });

  it('renders nothing for blank or non-string documentation', () => {
    expect(render('')).toBe('');
    expect(render('   \n  ')).toBe('');
    // Reachable through a doc map keyed by identifiers from the analyzed app:
    // a variant named `constructor` used to resolve to an inherited function.
    expect(renderToStaticMarkup(<DocText doc={undefined as unknown as string} />)).toBe('');
    expect(renderToStaticMarkup(<DocText doc={(() => 'x') as unknown as string} />)).toBe('');
  });
});

describe('safeUrl', () => {
  it('allows only http, https and mailto', () => {
    expect(safeUrl('https://example.com')).toBe('https://example.com');
    expect(safeUrl('http://example.com')).toBe('http://example.com');
    expect(safeUrl('mailto:someone@example.com')).toBe('mailto:someone@example.com');
    expect(safeUrl('MAILTO:someone@example.com')).toBe('MAILTO:someone@example.com');
  });

  it('drops every other scheme, including the ones the default permits', () => {
    // react-markdown's default also allows irc, ircs and xmpp; this UI does not.
    for (const url of [
      'javascript:alert(1)',
      'data:text/html,x',
      'vbscript:x',
      'file:///etc/passwd',
      'irc://example.com',
      'xmpp:someone@example.com',
      'vscode://command',
    ]) {
      expect(safeUrl(url), url).toBe('');
    }
  });

  it('leaves relative and same-page links alone', () => {
    expect(safeUrl('#section')).toBe('#section');
    expect(safeUrl('/docs/page')).toBe('/docs/page');
    expect(safeUrl('page.html')).toBe('page.html');
    // A colon after a path separator is not a scheme.
    expect(safeUrl('path/to:thing')).toBe('path/to:thing');
  });
});
