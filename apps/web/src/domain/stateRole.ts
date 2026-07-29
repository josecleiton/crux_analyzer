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
 * `failure` — a naming heuristic (a guess), so it lives here in the UI and
 * never in the parser: a state is read as a failure when one of its words is
 * a failure word. Nothing in the model marks failure, and inventing a parser
 * flag for it would break the parser honesty rule.
 */

import type { DomainMachine, DomainState } from './types';

export interface StateRole {
  initial: boolean;
  failure: boolean;
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

export function stateRole(machine: DomainMachine, state: DomainState): StateRole {
  return {
    initial: isInitial(machine, state),
    failure: isFailureName(state.name),
    final: state.outgoing.length === 0,
  };
}
