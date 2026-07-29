/**
 * Visual role of a state, derived from the domain model. Roles are static
 * facts about the machine, so they are painted whether or not a simulation
 * is running.
 *
 * `initial` — the machine's entry point: a state nothing transitions into.
 * When every state has an incoming transition (a fully cyclic machine), the
 * machine's first state is used, which is exactly where the Simulation
 * Engine starts.
 *
 * `final` — a dead end: no outgoing transition of its own. A machine-wide
 * wildcard transition (`from: "*"`) may still leave it; that escape is drawn
 * from the "any state" pseudo-node, so it stays visible instead of erasing
 * every final state of machines that have one.
 *
 * `failure` — declared, then guessed. A state whose doc comment in the
 * analyzed source carries `@failure` *is* a failure: that is the author of
 * that application speaking, and it reaches us as model data, so the parser is
 * reporting rather than inventing. When no state of the machine declares one,
 * the naming heuristic stands in — a state is read as a failure when one of
 * its words is a failure word. That heuristic is a guess, which is why it
 * lives here in the UI and never in the parser; and one `@failure` anywhere in
 * a machine silences it for that whole machine, because from then on an
 * unmarked sibling is unmarked on purpose.
 *
 * `deprecated` — declared only (`@deprecated`). There is no heuristic for it
 * and there must not be one: nothing in a state's name says it is on its way
 * out.
 */

import type { DomainMachine, DomainState } from './types';

export interface StateRole {
  initial: boolean;
  failure: boolean;
  deprecated: boolean;
  final: boolean;
}

const FAILURE_WORDS = new Set([
  'fail',
  'failed',
  'failing',
  'failure',
  'error',
  'errored',
  'denied',
  'rejected',
  'invalid',
  'unauthorized',
  'forbidden',
  'timeout',
  'crash',
  'crashed',
  'aborted',
  'broken',
]);

/** Splits `Active/RecordingFailed` or `recording_failed` into lowercase words. */
function words(name: string): string[] {
  return name
    .split(/[^A-Za-z0-9]+/)
    .flatMap((part) => part.split(/(?=[A-Z])/))
    .map((word) => word.toLowerCase())
    .filter(Boolean);
}

export function isFailureName(name: string): boolean {
  const parts = words(name);
  if (parts.some((word) => FAILURE_WORDS.has(word))) return true;
  // "TimedOut" / "timed_out" read as two words
  return parts.some((word, i) => word === 'timed' && parts[i + 1] === 'out');
}

function isInitial(machine: DomainMachine, state: DomainState): boolean {
  if (state.incoming.length === 0) return true;
  const anyEntryPoint = machine.states.some((s) => s.incoming.length === 0);
  return !anyEntryPoint && machine.states[0]?.id === state.id;
}

/**
 * The machine's entry point: the first state carrying the `initial` role.
 * It stands for the machine as a whole — clicking a machine section selects
 * it, and it is where the Simulation Engine starts by default.
 */
export function entryState(machine: DomainMachine): DomainState | null {
  return machine.states.find((state) => isInitial(machine, state)) ?? machine.states[0] ?? null;
}

/**
 * Declared, then guessed. A machine that declares even one `@failure` silences
 * the heuristic for all of its states: from then on an unmarked state is
 * unmarked on purpose, and guessing over that would contradict a declaration.
 */
function isFailure(machine: DomainMachine, state: DomainState): boolean {
  if (state.markers.includes('failure')) return true;
  return !declaresFailure(machine) && isFailureName(state.name);
}

function declaresFailure(machine: DomainMachine): boolean {
  return machine.states.some((s) => s.markers.includes('failure'));
}

export function stateRole(machine: DomainMachine, state: DomainState): StateRole {
  return {
    initial: isInitial(machine, state),
    failure: isFailure(machine, state),
    deprecated: state.markers.includes('deprecated'),
    final: state.outgoing.length === 0,
  };
}
