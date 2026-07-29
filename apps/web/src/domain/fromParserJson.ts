/**
 * Parser JSON → Domain Model mapper.
 * Generates stable ids and derives each state's incoming/outgoing transitions.
 */

import type { ParserProjectJson } from '../schema/parserJson';
import type { DomainCore, DomainProject, DomainState, DomainTransition } from './types';

export function fromParserJson(json: ParserProjectJson): DomainProject {
  return {
    name: json.project,
    cores: json.cores.map(mapCore),
  };
}

function mapCore(core: {
  name: string;
  states: string[];
  transitions: { from: string; event: string; to: string }[];
}): DomainCore {
  const coreId = core.name;
  const stateId = (stateName: string) => `${coreId}/${stateName}`;

  const transitions: DomainTransition[] = core.transitions.map((t, index) => ({
    id: `${coreId}/t${index}:${t.from}-${t.event}->${t.to}`,
    event: t.event,
    from: stateId(t.from),
    to: stateId(t.to),
    fromName: t.from,
    toName: t.to,
  }));

  const states: DomainState[] = core.states.map((name) => {
    const id = stateId(name);
    return {
      id,
      name,
      incoming: transitions.filter((t) => t.to === id),
      outgoing: transitions.filter((t) => t.from === id),
    };
  });

  return { id: coreId, name: core.name, states, transitions };
}
