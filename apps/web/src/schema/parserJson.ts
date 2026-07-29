/**
 * Types of the raw JSON emitted by the parser — they mirror the contract in
 * `shared/schema/crux-model.schema.json`.
 *
 * This is the ONLY UI layer that knows the parser format. Swapping the
 * parser (or evolving the contract) must only affect this module and the
 * `domain/fromParserJson.ts` mapper.
 */

export interface ParserProjectJson {
  project: string;
  cores: ParserCoreJson[];
}

export interface ParserCoreJson {
  name: string;
  states: string[];
  transitions: ParserTransitionJson[];
}

export interface ParserTransitionJson {
  from: string;
  event: string;
  to: string;
}

/** Light structural validation — fail early if the JSON breaks the contract. */
export function parseProjectJson(raw: unknown): ParserProjectJson {
  if (!isRecord(raw)) throw invalid('root must be an object');
  if (typeof raw.project !== 'string') throw invalid('"project" must be a string');
  if (!Array.isArray(raw.cores)) throw invalid('"cores" must be an array');

  for (const core of raw.cores) {
    if (!isRecord(core)) throw invalid('core must be an object');
    if (typeof core.name !== 'string') throw invalid('core.name must be a string');
    if (!Array.isArray(core.states) || core.states.some((s) => typeof s !== 'string')) {
      throw invalid(`core "${core.name}": states must be an array of strings`);
    }
    if (!Array.isArray(core.transitions)) {
      throw invalid(`core "${core.name}": transitions must be an array`);
    }
    for (const t of core.transitions) {
      if (
        !isRecord(t) ||
        typeof t.from !== 'string' ||
        typeof t.event !== 'string' ||
        typeof t.to !== 'string'
      ) {
        throw invalid(`core "${core.name}": transition must have from/event/to strings`);
      }
    }
  }
  return raw as unknown as ParserProjectJson;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function invalid(detail: string): Error {
  return new Error(`Invalid parser JSON: ${detail}`);
}
