/**
 * Simulation Engine: replays events through a machine's transitions.
 *
 * Pure domain logic — it knows nothing about React, React Flow or layout.
 * The UI drives it and projects its state onto the Graph via highlight
 * props, exactly as the architecture promised: no graph changes needed.
 */

import type { DomainMachine, DomainTransition } from '../domain/types';
import { wildcardStateId } from '../domain/types';

export interface SimulationStep {
  transitionId: string;
  event: string;
  fromName: string;
  toName: string;
}

export interface Simulation {
  machineId: string;
  /** Where the replay started — the first state of the traveled path. */
  initialStateId: string;
  currentStateId: string;
  trail: SimulationStep[];
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
      },
    ],
  };
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
