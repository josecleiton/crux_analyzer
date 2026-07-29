/**
 * Documentation read out of the analyzed application, shared by the Inspector
 * and the simulation panel.
 *
 * The text is the analyzed app's own prose, so it is never translated — only
 * the headings around it are. It *is* rendered as Markdown now, like the
 * generated document always did: `///` prose full of `backticks` and lists
 * was showing its raw syntax here. react-markdown renders to React elements
 * (no HTML injection), leaves raw HTML in prose unrendered, and treats
 * single newlines as soft breaks — which is exactly the hard-wrap rejoining
 * `docParagraphs` used to do by hand. Tag names are the author's
 * identifiers, hence monospace: in this UI, monospace is data.
 */

import Markdown from 'react-markdown';
import type { DomainMachine } from '../../domain/types';
import { useTranslate } from '../../i18n/useI18n';

export function DocText({ doc }: { doc: string }) {
  if (doc.trim() === '') return null;
  return (
    <div className="doc-text">
      <Markdown>{doc}</Markdown>
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
