/**
 * Types of the raw JSON emitted by the parser — they mirror the contract in
 * `shared/schema/crux-model.schema.json`.
 *
 * This is the ONLY UI layer that knows the parser format. Swapping the
 * parser (or evolving the contract) must only affect this module and the
 * `domain/fromParserJson.ts` mapper.
 *
 * A state is authored in the contract either as a bare name or as an object
 * carrying the documentation written on it in the analyzed source.
 * `parseProjectJson` collapses both into one record, so that union stops here
 * and nothing above ever branches on the shape of a state.
 */

export interface ParserProjectJson {
  project: string;
  cores: ParserCoreJson[];
}

export interface ParserCoreJson {
  name: string;
  machines: ParserMachineJson[];
  /** Documented events, keyed by the name transitions use. Always an array. */
  events: ParserDocumentedNameJson[];
  /** Documented effects (`AudioOperation::Start`, `Render`). Always an array. */
  effects: ParserDocumentedNameJson[];
}

/** A name from the analyzed source with the documentation authored on it. */
export interface ParserDocumentedNameJson {
  name: string;
  doc: string;
}

export interface ParserMachineJson {
  name: string;
  /** Documentation authored on the state enum itself. */
  doc?: string;
  markers: ParserMarker[];
  tags: string[];
  states: ParserStateJson[];
  transitions: ParserTransitionJson[];
}

/**
 * A state after normalization — the only state shape that leaves this module.
 *
 * `markers` and `tags` are always arrays so no consumer needs `?? []`, while
 * `doc` stays optional because "absent" and "empty" differ to a renderer.
 */
export interface ParserStateJson {
  name: string;
  doc?: string;
  markers: ParserMarker[];
  tags: string[];
}

/**
 * Markers this client understands. These are contract identifiers, never
 * prose — the UI renders its own localized label for each.
 */
export type ParserMarker = 'failure' | 'deprecated';

const KNOWN_MARKERS = new Set<string>(['failure', 'deprecated']);

export interface ParserTransitionJson {
  from: string;
  event: string;
  to: string;
  effects?: string[];
}

/** Wildcard source state: the transition fires from any state. */
export const ANY_STATE = '*';

/**
 * Structural validation plus state normalization — fail early if the JSON
 * breaks the contract, and hand the rest of the app one state shape.
 */
export function parseProjectJson(raw: unknown): ParserProjectJson {
  if (!isRecord(raw)) throw invalid('root must be an object');
  if (typeof raw.project !== 'string') throw invalid('"project" must be a string');
  if (!Array.isArray(raw.cores)) throw invalid('"cores" must be an array');

  return {
    project: raw.project,
    cores: raw.cores.map((core) => parseCore(core)),
  };
}

function parseCore(raw: unknown): ParserCoreJson {
  if (!isRecord(raw)) throw invalid('core must be an object');
  if (typeof raw.name !== 'string') throw invalid('core.name must be a string');
  if (!Array.isArray(raw.machines)) {
    throw invalid(`core "${raw.name}": machines must be an array`);
  }
  return {
    name: raw.name,
    machines: raw.machines.map((machine) => parseMachine(raw.name as string, machine)),
    events: parseDocumentedNames(`core "${raw.name}": events`, raw.events),
    effects: parseDocumentedNames(`core "${raw.name}": effects`, raw.effects),
  };
}

function parseDocumentedNames(what: string, raw: unknown): ParserDocumentedNameJson[] {
  if (raw === undefined) return [];
  if (!Array.isArray(raw)) throw invalid(`${what} must be an array`);
  return raw.map((entry) => {
    if (!isRecord(entry) || typeof entry.name !== 'string' || typeof entry.doc !== 'string') {
      throw invalid(`${what}: entries must have name/doc strings`);
    }
    return { name: entry.name, doc: entry.doc };
  });
}

function parseMachine(coreName: string, raw: unknown): ParserMachineJson {
  if (!isRecord(raw)) throw invalid(`core "${coreName}": machine must be an object`);
  if (typeof raw.name !== 'string') {
    throw invalid(`core "${coreName}": machine.name must be a string`);
  }
  const name = raw.name;
  if (!Array.isArray(raw.states)) {
    throw invalid(`machine "${name}": states must be an array`);
  }
  if (!Array.isArray(raw.transitions)) {
    throw invalid(`machine "${name}": transitions must be an array`);
  }
  return {
    name,
    doc: parseDoc(`machine "${name}"`, raw.doc),
    markers: parseMarkers(`machine "${name}"`, raw.markers),
    tags: parseStrings(`machine "${name}": tags`, raw.tags),
    states: raw.states.map((state) => parseState(name, state)),
    transitions: raw.transitions.map((transition) => parseTransition(name, transition)),
  };
}

/**
 * Both authored forms collapse here: a bare `"Failed"` and an annotated
 * `{ "name": "Failed", ... }` become the same record.
 */
function parseState(machineName: string, raw: unknown): ParserStateJson {
  if (typeof raw === 'string') return { name: raw, markers: [], tags: [] };
  if (!isRecord(raw)) {
    throw invalid(`machine "${machineName}": state must be a string or an object`);
  }
  if (typeof raw.name !== 'string') {
    throw invalid(`machine "${machineName}": state.name must be a string`);
  }
  const name = raw.name;
  return {
    name,
    doc: parseDoc(`state "${name}"`, raw.doc),
    markers: parseMarkers(`state "${name}"`, raw.markers),
    tags: parseStrings(`state "${name}": tags`, raw.tags),
  };
}

function parseTransition(machineName: string, raw: unknown): ParserTransitionJson {
  if (
    !isRecord(raw) ||
    typeof raw.from !== 'string' ||
    typeof raw.event !== 'string' ||
    typeof raw.to !== 'string'
  ) {
    throw invalid(`machine "${machineName}": transition must have from/event/to strings`);
  }
  const effects =
    raw.effects === undefined
      ? undefined
      : parseStrings(`machine "${machineName}": effects`, raw.effects);
  return { from: raw.from, event: raw.event, to: raw.to, effects };
}

function parseDoc(what: string, raw: unknown): string | undefined {
  if (raw === undefined) return undefined;
  if (typeof raw !== 'string') throw invalid(`${what}: doc must be a string`);
  return raw;
}

function parseMarkers(what: string, raw: unknown): ParserMarker[] {
  const values = parseStrings(`${what}: markers`, raw);
  // A marker a newer parser invented must not invalidate the whole model for
  // an older UI: drop what we cannot render, keep what we can.
  return values.filter((value): value is ParserMarker => KNOWN_MARKERS.has(value));
}

function parseStrings(what: string, raw: unknown): string[] {
  if (raw === undefined) return [];
  if (!Array.isArray(raw) || raw.some((value) => typeof value !== 'string')) {
    throw invalid(`${what} must be an array of strings`);
  }
  return raw as string[];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  // Arrays excluded explicitly: they are objects, and while a missing required
  // field would reject one anyway, "is this a record" should not answer yes.
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function invalid(detail: string): Error {
  return new Error(`Invalid parser JSON: ${detail}`);
}
