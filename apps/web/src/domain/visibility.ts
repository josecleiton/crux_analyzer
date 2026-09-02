/**
 * Reader-driven visibility: which states are drawn at all.
 *
 * This is the second half of reader control over the canvas, and it is
 * deliberately a different channel from `focus.ts`. Focus answers "which states
 * matter right now" by dimming the rest — everything stays on screen. Visibility
 * answers "which states am I reading about at all": a deselected state leaves
 * the canvas, and so do the transitions that can no longer be drawn.
 *
 * Hidden states are held as a set of ids, so the default — nothing hidden,
 * everything visible — costs nothing and needs no per-state bookkeeping. Pure
 * functions over the domain model, testable without React.
 */

import type { DomainCore, DomainMachine } from './types';
import { wildcardStateId } from './types';

/** Nothing hidden: the default canvas. */
export const NOTHING_HIDDEN: ReadonlySet<string> = new Set<string>();

/**
 * How much of a group of states is on the canvas — what a group's checkbox
 * shows. `some` is the mixed (indeterminate) reading.
 */
export type GroupVisibility = 'all' | 'none' | 'some';

export function groupVisibility(
  stateIds: readonly string[],
  hidden: ReadonlySet<string>,
): GroupVisibility {
  if (stateIds.length === 0) return 'all';
  const hiddenCount = stateIds.filter((id) => hidden.has(id)).length;
  if (hiddenCount === 0) return 'all';
  return hiddenCount === stateIds.length ? 'none' : 'some';
}

/**
 * The set with `stateIds` hidden (or shown again). Returns the set it was given
 * when nothing changed, so React state does not churn on a no-op toggle.
 */
export function withHidden(
  hidden: ReadonlySet<string>,
  stateIds: readonly string[],
  hide: boolean,
): ReadonlySet<string> {
  const changed = stateIds.filter((id) => hidden.has(id) !== hide);
  if (changed.length === 0) return hidden;
  const next = new Set(hidden);
  for (const id of changed) {
    if (hide) next.add(id);
    else next.delete(id);
  }
  return next;
}

/**
 * The set with only `stateIds` visible inside `core`: what a row's *name* does
 * in the outline — read this, and nothing else. The scope is the core and not
 * the clicked row's machine, because the canvas draws a whole core: leaving
 * another machine untouched would not read as "only this".
 *
 * Same identity contract as `withHidden`: the set it was given comes back when
 * nothing changes.
 */
export function withOnlyVisible(
  hidden: ReadonlySet<string>,
  core: DomainCore,
  stateIds: readonly string[],
): ReadonlySet<string> {
  const keep = new Set(stateIds);
  const ids = coreStateIds(core);
  const shown = withHidden(
    hidden,
    ids.filter((id) => !keep.has(id)),
    true,
  );
  return withHidden(
    shown,
    ids.filter((id) => keep.has(id)),
    false,
  );
}

/** Ids of the core's states that are currently hidden. */
export function hiddenInCore(core: DomainCore, hidden: ReadonlySet<string>): string[] {
  return core.machines.flatMap((machine) =>
    machine.states.filter((state) => hidden.has(state.id)).map((state) => state.id),
  );
}

/** Every state id of a machine — what a region-level toggle acts on. */
export function machineStateIds(machine: DomainMachine): string[] {
  return machine.states.map((state) => state.id);
}

/** Every state id the core declares — what a core-wide operation acts on. */
export function coreStateIds(core: DomainCore): string[] {
  return core.machines.flatMap((machine) => machineStateIds(machine));
}

/**
 * Whether a selected id still has everything it needs to be shown: a state has
 * to be visible, and a transition needs both of its endpoints. The wildcard
 * pseudo-state cannot be hidden — "any state" is not a state — but its edges
 * still go when the real state at the other end does.
 */
export function isOnCanvas(
  core: DomainCore,
  kind: 'state' | 'transition',
  id: string,
  hidden: ReadonlySet<string>,
): boolean {
  if (kind === 'state') return !hidden.has(id);
  for (const machine of core.machines) {
    const transition = machine.transitions.find((candidate) => candidate.id === id);
    if (!transition) continue;
    // A machine with nothing visible left is off the canvas whole: its wildcards
    // have no states left to stand for, `* → *` included.
    if (machine.states.every((state) => hidden.has(state.id))) return false;
    const wildcard = wildcardStateId(machine.id);
    const endpoints = [transition.from, transition.to].filter((end) => end !== wildcard);
    return endpoints.every((end) => !hidden.has(end));
  }
  // Not a transition of this core: nothing to hide it.
  return true;
}
