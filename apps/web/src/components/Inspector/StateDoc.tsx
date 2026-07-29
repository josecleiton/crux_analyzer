/**
 * Documentation read out of the analyzed application, shared by the Inspector
 * and the simulation panel.
 *
 * The text is the analyzed app's own prose, so it is never translated — only
 * the headings around it are. It *is* rendered as Markdown, like the generated
 * document always did: `///` prose full of `backticks` and lists was showing
 * its raw syntax here. react-markdown renders to React elements (no HTML
 * injection), leaves raw HTML in prose unrendered, and treats single newlines
 * as soft breaks — which is exactly the hard-wrap rejoining `docParagraphs`
 * used to do by hand. Tag names are the author's identifiers, hence monospace:
 * in this UI, monospace is data.
 *
 * ## Why the options below are spelled out
 *
 * This prose is untrusted input (`docs/security.md`): it comes from whatever
 * repository the analyzer was pointed at, and it renders both on a public site
 * and inside a VS Code webview. react-markdown's defaults are already safe, but
 * "safe by default" is a property of the dependency, not of this file — so the
 * three things that matter are stated explicitly here and pinned by
 * `StateDoc.test.tsx`:
 *
 * - **no raw HTML.** `rehype-raw` must never be added, and `skipHtml` stays
 *   off so markup in prose renders as visible text rather than vanishing.
 * - **`urlTransform`** narrows the protocol allowlist to the three schemes this
 *   UI has any use for. The default also permits `irc`, `ircs` and `xmpp`,
 *   which hand a custom-scheme handler to author-controlled text for nothing.
 * - **`a` and `img` are overridden.** Links open with `noopener noreferrer
 *   nofollow`; images are not fetched at all — a `![](https://tracker/x.png)`
 *   in a doc comment is a read beacon that would report every viewer of a
 *   published document, so the alt text stands in for it.
 */

import Markdown from 'react-markdown';
import type { Components } from 'react-markdown';
import type { DomainMachine } from '../../domain/types';
import { useTranslate } from '../../i18n/useI18n';

/** The only URL schemes author prose may produce. */
const SAFE_PROTOCOLS = ['http:', 'https:', 'mailto:'];

/**
 * Keeps same-page and relative URLs, allows the three safe schemes, drops
 * everything else — `javascript:`, `data:`, `vscode:`, unknown handlers.
 */
export function safeUrl(url: string): string {
  const trimmed = url.trim();
  if (trimmed === '' || /^[#/?]/.test(trimmed)) return trimmed;
  // A scheme is only a scheme if the colon precedes any `/`, `?` or `#`.
  const colon = trimmed.indexOf(':');
  if (colon === -1) return trimmed;
  if (/[/?#]/.test(trimmed.slice(0, colon))) return trimmed;
  const protocol = trimmed.slice(0, colon + 1).toLowerCase();
  return SAFE_PROTOCOLS.includes(protocol) ? trimmed : '';
}

const DOC_COMPONENTS: Components = {
  a: ({ href, children, ...rest }) => (
    <a {...rest} href={href} target="_blank" rel="noopener noreferrer nofollow">
      {children}
    </a>
  ),
  // Never issues a request: an external image in author prose would tell its
  // host who read the document.
  img: ({ alt, src }) => <em className="doc-image">{alt || String(src ?? '')}</em>,
};

export function DocText({ doc }: { doc: string }) {
  // The model is validated, but this is also reached with values keyed by
  // identifiers from the analyzed app — a non-string here must not throw.
  if (typeof doc !== 'string' || doc.trim() === '') return null;
  return (
    <div className="doc-text">
      <Markdown urlTransform={safeUrl} components={DOC_COMPONENTS}>
        {doc}
      </Markdown>
    </div>
  );
}

export function StateTags({ tags }: { tags: string[] }) {
  const t = useTranslate();
  if (tags.length === 0) return null;
  return (
    <>
      <h4>{t('inspector.tags')}</h4>
      <div className="state-tags">
        {tags.map((tag) => (
          <span className="state-tag" key={tag}>
            {tag}
          </span>
        ))}
      </div>
    </>
  );
}

/**
 * What the state enum itself declares: its description, and any markers or
 * tags that describe the whole region. Unlike the machine *name*, this shows
 * even for a single-machine core — a lone region still has a description worth
 * reading.
 */
export function MachineDoc({ machine }: { machine: DomainMachine }) {
  const t = useTranslate();
  const marked = machine.markers.length > 0 || machine.tags.length > 0;
  if (!machine.doc && !marked) return null;
  return (
    <>
      <h4>{t('inspector.aboutMachine')}</h4>
      {machine.markers.length > 0 ? (
        <div className="state-badges">
          {machine.markers.map((marker) => (
            <span className={`state-badge ${marker}`} key={marker}>
              {t(marker === 'failure' ? 'badge.failure' : 'badge.deprecated')}
            </span>
          ))}
        </div>
      ) : null}
      {machine.doc ? <DocText doc={machine.doc} /> : null}
      <StateTags tags={machine.tags} />
    </>
  );
}
