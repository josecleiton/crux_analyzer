/**
 * "What does entering `Uploading` actually do?" — the union of the effects
 * requested by a state's incoming transitions, in first-seen order.
 *
 * A union, not an intersection, and presented as such: each incoming
 * transition requests its own effects, so this list reads "arriving here
 * fires some of these", never "all of these fire".
 */

import type { DomainState } from './types';

export function entryEffects(state: DomainState): string[] {
  const seen = new Set<string>();
  const effects: string[] = [];
  for (const transition of state.incoming) {
    for (const effect of transition.effects) {
      if (!seen.has(effect)) {
        seen.add(effect);
        effects.push(effect);
      }
    }
  }
  return effects;
}
