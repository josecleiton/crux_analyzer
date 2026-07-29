/**
 * UI domain model — independent from the parser format and from React Flow.
 * Components (Sidebar, Inspector), the Simulation Engine and any future
 * client depend only on these types.
 */

export interface DomainProject {
  name: string;
  cores: DomainCore[];
}

export interface DomainCore {
  id: string;
  name: string;
  /** State machines (orthogonal regions / "modules") of this core. */
  machines: DomainMachine[];
}

export interface DomainMachine {
  id: string;
  name: string;
  states: DomainState[];
  transitions: DomainTransition[];
  /** Whether any transition uses the wildcard "any state" (source or target). */
  hasWildcard: boolean;
}

export interface DomainState {
  id: string;
  name: string;
  /** Transitions arriving at this state (wildcard-sourced ones included). */
  incoming: DomainTransition[];
  /** Transitions leaving specifically this state (wildcards not repeated). */
  outgoing: DomainTransition[];
}

export interface DomainTransition {
  id: string;
  event: string;
  /** id of the source state — or the machine's wildcard pseudo-state id. */
  from: string;
  /** id of the target state. */
  to: string;
  /** human-readable names (for the Inspector); fromName is "*" for wildcards. */
  fromName: string;
  toName: string;
  /** Effects requested when this transition fires. */
  effects: string[];
}

/** Wildcard source name used in the contract. */
export const ANY_STATE_NAME = '*';

/** id of a machine's wildcard pseudo-state. */
export function wildcardStateId(machineId: string): string {
  return `${machineId}/*`;
}
