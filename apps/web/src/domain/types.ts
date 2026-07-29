/**
 * UI domain model — independent from the parser format and from React Flow.
 * Components (Sidebar, Inspector) and the future Simulation Engine depend
 * only on these types.
 */

export interface DomainProject {
  name: string;
  cores: DomainCore[];
}

export interface DomainCore {
  id: string;
  name: string;
  states: DomainState[];
  transitions: DomainTransition[];
}

export interface DomainState {
  id: string;
  name: string;
  /** Transitions arriving at this state. */
  incoming: DomainTransition[];
  /** Transitions leaving this state. */
  outgoing: DomainTransition[];
}

export interface DomainTransition {
  id: string;
  event: string;
  /** id of the source state. */
  from: string;
  /** id of the target state. */
  to: string;
  /** human-readable names (for the Inspector). */
  fromName: string;
  toName: string;
}
