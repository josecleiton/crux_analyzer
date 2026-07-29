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
  /**
   * Documentation authored on event enum variants, by the event name
   * transitions carry. Only documented events appear. Author prose — never
   * translated.
   */
  eventDocs: Record<string, string>;
  /** Same for effects, by the label transitions carry (`Enum::Variant`). */
  effectDocs: Record<string, string>;
}

export interface DomainMachine {
  id: string;
  name: string;
  /** Description authored on the state enum's own doc comment. */
  doc?: string;
  /** Markers declared on the state enum — they describe the whole region. */
  markers: StateMarker[];
  /** Free-form tag names declared on the state enum. */
  tags: string[];
  states: DomainState[];
  transitions: DomainTransition[];
  /** Whether any transition uses the wildcard "any state" (source or target). */
  hasWildcard: boolean;
}

export interface DomainState {
  id: string;
  name: string;
  /**
   * Description authored in the analyzed source's doc comment. May span
   * several paragraphs. Prose from the analyzed app — never translated.
   */
  doc?: string;
  /**
   * Markers the author declared. Derived roles (`initial`, `final`) are NOT
   * here — see `stateRole.ts`, which is the only module that reads this.
   */
  markers: StateMarker[];
  /** Free-form `@tag` names, verbatim: data from the analyzed app. */
  tags: string[];
  /** Transitions arriving at this state (wildcard-sourced ones included). */
  incoming: DomainTransition[];
  /** Transitions leaving specifically this state (wildcards not repeated). */
  outgoing: DomainTransition[];
}

/** A marker declared in the analyzed source (`@failure`, `@deprecated`). */
export type StateMarker = 'failure' | 'deprecated';

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
