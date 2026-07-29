/**
 * Simulation Engine: replays events through a machine's transitions.
 *
 * Pure domain logic — it knows nothing about React, React Flow or layout.
 * The UI drives it and projects its state onto the Graph via highlight
 * props, exactly as the architecture promised: no graph changes needed.
 */

import type { DomainEffect, DomainMachine, DomainTransition } from '../domain/types';
import { wildcardStateId } from '../domain/types';

export interface SimulationStep {
  transitionId: string;
  event: string;
  fromName: string;
  toName: string;
  /** What firing this asked the shell to do. */
  effects: DomainEffect[];
}

/**
 * A request the replay has made and the shell has not answered yet.
 *
 * This is the half of Crux's loop a state graph cannot show: firing an event
 * requests an effect, and the *shell* decides which event comes back. Only
 * requests that declare an answer wait here — a fire-and-forget request is done
 * the moment it is made.
 */
export interface InFlightEffect {
  name: string;
  /** Step (1-based, as in the trail) that requested it. */
  step: number;
  /** Events the shell can answer it with. */
  answers: string[];
}

export interface Simulation {
  machineId: string;
  /** Where the replay started — the first state of the traveled path. */
  initialStateId: string;
  currentStateId: string;
  trail: SimulationStep[];
  /** Requests still waiting for the shell, oldest first. */
  inFlight: InFlightEffect[];
}

/** Starts a simulation at `initialStateId` (or the machine's first state). */
export function startSimulation(machine: DomainMachine, initialStateId?: string): Simulation {
  const initial =
    machine.states.find((s) => s.id === initialStateId) ?? machine.states[0] ?? null;
  if (!initial) throw new Error(`machine ${machine.id} has no states to simulate`);
  return {
    machineId: machine.id,
    initialStateId: initial.id,
    currentStateId: initial.id,
    trail: [],
    inFlight: [],
  };
}

/**
 * Transitions that can fire from the current state: its outgoing ones plus
 * every wildcard-sourced transition of the machine. Transitions whose target
 * is decided at runtime (`to: "*"`) cannot be replayed and are excluded.
 */
export function availableTransitions(
  machine: DomainMachine,
  simulation: Simulation,
): DomainTransition[] {
  const wildcardId = wildcardStateId(machine.id);
  return machine.transitions.filter(
    (t) =>
      (t.from === simulation.currentStateId || t.from === wildcardId) &&
      t.to !== wildcardId,
  );
}

/**
 * Transitions that fire from the current state but land on a runtime-decided
 * target (`to: "*"`) — real behavior the replay cannot follow. Surfaced so
 * the panel can say so instead of silently hiding them.
 */
export function unreplayableTransitions(
  machine: DomainMachine,
  simulation: Simulation,
): DomainTransition[] {
  const wildcardId = wildcardStateId(machine.id);
  return machine.transitions.filter(
    (t) =>
      (t.from === simulation.currentStateId || t.from === wildcardId) &&
      t.to === wildcardId,
  );
}

/** Fires a transition, moving the simulation to its target state. */
export function fire(
  machine: DomainMachine,
  simulation: Simulation,
  transitionId: string,
): Simulation {
  const transition = availableTransitions(machine, simulation).find(
    (t) => t.id === transitionId,
  );
  if (!transition) return simulation; // not fireable from here — ignore

  const current = machine.states.find((s) => s.id === simulation.currentStateId);
  const step = simulation.trail.length + 1;
  return {
    machineId: simulation.machineId,
    initialStateId: simulation.initialStateId,
    currentStateId: transition.to,
    trail: [
      ...simulation.trail,
      {
        transitionId: transition.id,
        event: transition.event,
        fromName: current?.name ?? transition.fromName,
        toName: transition.toName,
        effects: transition.effects,
      },
    ],
    // This event answers whatever was waiting for it, and the requests this
    // transition makes start waiting in turn.
    inFlight: [
      ...simulation.inFlight.filter((pending) => !pending.answers.includes(transition.event)),
      ...transition.effects
        .filter((effect) => effect.answers.length > 0)
        .map((effect) => ({ name: effect.name, step, answers: effect.answers })),
    ],
  };
}

/**
 * The events the shell owes the replay, oldest request first: what can arrive
 * next without the user doing anything.
 *
 * `fireable` is what separates an answer the graph accounts for from one it does
 * not: a callback event with no transition from the current state is real
 * behavior that changes no state (a confirmation the core just renders), and
 * saying so beats hiding it.
 */
export interface Answer {
  event: string;
  /** The request this answers. */
  effect: string;
  /** Transition this answer would fire from the current state, if any. */
  transitionId: string | null;
}

export function pendingAnswers(machine: DomainMachine, simulation: Simulation): Answer[] {
  const available = availableTransitions(machine, simulation);
  const answers: Answer[] = [];
  for (const pending of simulation.inFlight) {
    for (const event of pending.answers) {
      if (answers.some((answer) => answer.event === event)) continue;
      answers.push({
        event,
        effect: pending.name,
        transitionId: available.find((t) => t.event === event)?.id ?? null,
      });
    }
  }
  return answers;
}

/** The last fired transition, for highlighting. */
export function lastStep(simulation: Simulation): SimulationStep | null {
  return simulation.trail[simulation.trail.length - 1] ?? null;
}

/** States and transitions the replay has already been through. */
export interface TraveledPath {
  stateIds: string[];
  transitionIds: string[];
}

export function traveledPath(machine: DomainMachine, simulation: Simulation): TraveledPath {
  const stateIds = new Set<string>([simulation.initialStateId]);
  const transitionIds: string[] = [];

  for (const step of simulation.trail) {
    const transition = machine.transitions.find((t) => t.id === step.transitionId);
    if (!transition) continue;
    transitionIds.push(transition.id);
    // `from` can be the wildcard pseudo-state: it is a graph node all the same
    stateIds.add(transition.from);
    stateIds.add(transition.to);
  }

  return { stateIds: [...stateIds], transitionIds };
}
