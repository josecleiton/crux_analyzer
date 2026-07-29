/**
 * "What does entering `Uploading` actually do?" — the union of the effects
 * requested by a state's incoming transitions, in first-seen order.
 *
 * A union, not an intersection, and presented as such: each incoming
 * transition requests its own effects, so this list reads "arriving here
 * fires some of these", never "all of these fire".
 *
 * The same request reached by two arrivals is one entry, and what the arrivals
 * disagree about is resolved the honest way: the answers are pooled (the shell
 * can send any of them), and the request counts as conditional only when every
 * arrival that makes it says so.
 */

import type { DomainEffect, DomainState } from './types';

export function entryEffects(state: DomainState): DomainEffect[] {
  const byName = new Map<string, DomainEffect>();
  for (const transition of state.incoming) {
    for (const effect of transition.effects) {
      const kept = byName.get(effect.name);
      if (!kept) {
        byName.set(effect.name, { ...effect, answers: [...effect.answers] });
        continue;
      }
      kept.conditional = kept.conditional && effect.conditional;
      for (const answer of effect.answers) {
        if (!kept.answers.includes(answer)) kept.answers.push(answer);
      }
    }
  }
  return [...byName.values()];
}
