/**
 * Reader-driven focus: which states stay at full strength while the rest of
 * the graph dims. This is the domain half of tag filtering and of the
 * undocumented-states highlight — pure functions over the domain model, so
 * they can be tested without React and reused by any client.
 *
 * Matching is a reader-side convenience (case-insensitive, substring), not
 * model semantics: the model carries the author's verbatim tag names and this
 * module never rewrites them.
 */

import type { DomainCore } from './types';
import { wildcardStateId } from './types';

/** What the reader asked to focus on. Both criteria compose (intersection). */
export interface FocusCriteria {
  /** Fragment matched against declared tag names; empty means "no filter". */
  tagQuery: string;
  /** Keep only states with no authored description. */
  undocumentedOnly: boolean;
}

/** Ids that survive the criteria. `stateIds` includes wildcard pseudo-states. */
export interface FocusSet {
  stateIds: string[];
  transitionIds: string[];
}

/**
 * Computes the focus for a core, or `null` when no criterion is active —
 * `null` rather than "everything kept", so callers can tell "not filtering"
 * from "filtering and everything matched".
 */
export function focusFor(core: DomainCore, criteria: FocusCriteria): FocusSet | null {
  const query = criteria.tagQuery.trim().toLowerCase();
  if (query === '' && !criteria.undocumentedOnly) return null;

  const stateIds: string[] = [];
  const transitionIds: string[] = [];

  for (const machine of core.machines) {
    // A tag on the state enum describes the whole region, so it keeps every
    // state of the machine — the same reading the Inspector gives it.
    const regionTagged = query !== '' && matchesTag(machine.tags, query);

    const kept = new Set<string>();
    for (const state of machine.states) {
      const tagOk = query === '' || regionTagged || matchesTag(state.tags, query);
      const docOk = !criteria.undocumentedOnly || !state.doc;
      if (tagOk && docOk) kept.add(state.id);
    }

    // A transition survives when everything it connects survives. The
    // wildcard pseudo-state counts as kept on either end — "any state"
    // includes the kept ones — but a wildcard-only edge stays out: at least
    // one endpoint has to be a state the reader actually asked for.
    const wildcard = wildcardStateId(machine.id);
    let wildcardTouched = false;
    for (const transition of machine.transitions) {
      const fromOk = kept.has(transition.from) || transition.from === wildcard;
      const toOk = kept.has(transition.to) || transition.to === wildcard;
      if (fromOk && toOk && (kept.has(transition.from) || kept.has(transition.to))) {
        transitionIds.push(transition.id);
        if (transition.from === wildcard || transition.to === wildcard) wildcardTouched = true;
      }
    }

    stateIds.push(...kept);
    if (wildcardTouched) stateIds.push(wildcard);
  }

  return { stateIds, transitionIds };
}

/**
 * Every tag declared in the core (regions and states), for the filter input's
 * suggestions — most-used first (declaration count), ties alphabetical, so
 * the tags that structure the machine surface before one-off labels. An
 * empty list is also the filter's reason not to render: a core with nothing
 * to filter by gets no filter.
 */
export function declaredTags(core: DomainCore): string[] {
  const uses = new Map<string, number>();
  const count = (tag: string) => uses.set(tag, (uses.get(tag) ?? 0) + 1);
  for (const machine of core.machines) {
    machine.tags.forEach(count);
    machine.states.forEach((state) => state.tags.forEach(count));
  }
  return [...uses.entries()]
    .sort(([tagA, usesA], [tagB, usesB]) => usesB - usesA || tagA.localeCompare(tagB))
    .map(([tag]) => tag);
}

function matchesTag(tags: string[], query: string): boolean {
  return tags.some((tag) => tag.toLowerCase().includes(query));
}
