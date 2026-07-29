/**
 * The selection as a URL: `#state=Core/Machine/Name`, `#transition=<id>`,
 * `#core=<name>` — so "this state of this machine" is a link that can be
 * pasted in a review. Hash-based on purpose: the app deploys as a static
 * bundle with no router and no SPA fallback rule, and a hash survives any
 * host untouched.
 *
 * Pure encode/decode/resolve, so the mapping is testable without a browser;
 * `App` owns the actual `location` wiring.
 */

import type { DomainProject } from '../domain/types';
import type { Selection } from './selection';

export interface UrlState {
  coreId: string | null;
  selection: Selection;
}

export function toHash(state: UrlState): string {
  if (state.selection) {
    return `#${state.selection.kind}=${encodeId(state.selection.id)}`;
  }
  if (state.coreId) return `#core=${encodeId(state.coreId)}`;
  return '';
}

export function fromHash(hash: string): UrlState {
  const raw = hash.replace(/^#/, '');
  const separator = raw.indexOf('=');
  if (separator < 0) return { coreId: null, selection: null };
  const key = raw.slice(0, separator);
  let value: string;
  try {
    value = decodeURIComponent(raw.slice(separator + 1));
  } catch {
    return { coreId: null, selection: null }; // malformed escape: not ours
  }
  if (value === '') return { coreId: null, selection: null };

  if (key === 'state' || key === 'transition') {
    // ids are `${core}/${machine}/...`, so the link carries its core
    return { coreId: value.split('/', 1)[0], selection: { kind: key, id: value } };
  }
  if (key === 'core') return { coreId: value, selection: null };
  return { coreId: null, selection: null };
}

/**
 * Validates a decoded URL state against the loaded project — a stale or
 * foreign link falls back to "nothing selected" instead of a broken UI.
 */
export function resolveUrlState(project: DomainProject, url: UrlState): UrlState {
  const core = project.cores.find((candidate) => candidate.id === url.coreId);
  if (!core) return { coreId: null, selection: null };
  if (!url.selection) return { coreId: core.id, selection: null };

  const exists = core.machines.some((machine) =>
    url.selection!.kind === 'state'
      ? machine.states.some((state) => state.id === url.selection!.id)
      : machine.transitions.some((transition) => transition.id === url.selection!.id),
  );
  return { coreId: core.id, selection: exists ? url.selection : null };
}

/** Escape the id but keep its `/` separators readable in the address bar. */
function encodeId(id: string): string {
  return encodeURIComponent(id).replace(/%2F/gi, '/');
}
