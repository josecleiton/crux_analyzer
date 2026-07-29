/**
 * Parser JSON → Domain Model mapper.
 * Generates stable ids and derives each state's incoming/outgoing transitions.
 */

import type { ParserMachineJson, ParserProjectJson } from '../schema/parserJson';
import { ANY_STATE } from '../schema/parserJson';
import type { DomainCore, DomainMachine, DomainProject, DomainState, DomainTransition } from './types';
import { wildcardStateId } from './types';

export function fromParserJson(json: ParserProjectJson): DomainProject {
  return {
    name: json.project,
    cores: json.cores.map((core) => ({
      id: core.name,
      name: core.name,
      machines: core.machines.map((machine) => mapMachine(core.name, machine)),
      eventDocs: Object.fromEntries(core.events.map((entry) => [entry.name, entry.doc])),
      effectDocs: Object.fromEntries(core.effects.map((entry) => [entry.name, entry.doc])),
    })),
  };
}

function mapMachine(coreId: string, machine: ParserMachineJson): DomainMachine {
  const machineId = `${coreId}/${machine.name}`;
  const stateId = (stateName: string) =>
    stateName === ANY_STATE ? wildcardStateId(machineId) : `${machineId}/${stateName}`;

  const transitions: DomainTransition[] = machine.transitions.map((t, index) => ({
    id: `${machineId}/t${index}:${t.from}-${t.event}->${t.to}`,
    event: t.event,
    from: stateId(t.from),
    to: stateId(t.to),
    fromName: t.from,
    toName: t.to,
    effects: t.effects ?? [],
  }));

  // `doc`, `markers` and `tags` pass through verbatim: normalizing prose or
  // author-chosen tag names here would be the mapper inventing semantics.
  const states: DomainState[] = machine.states.map((state) => {
    const id = stateId(state.name);
    return {
      id,
      name: state.name,
      doc: state.doc,
      markers: state.markers,
      tags: state.tags,
      incoming: transitions.filter((t) => t.to === id),
      outgoing: transitions.filter((t) => t.from === id),
    };
  });

  return {
    id: machineId,
    name: machine.name,
    doc: machine.doc,
    markers: machine.markers,
    tags: machine.tags,
    states,
    transitions,
    hasWildcard: machine.transitions.some(
      (t) => t.from === ANY_STATE || t.to === ANY_STATE,
    ),
  };
}

/** Finds the machine that owns a state or transition id. */
export function machineOf(core: DomainCore, id: string): DomainMachine | null {
  return core.machines.find((machine) => id.startsWith(`${machine.id}/`)) ?? null;
}
