/**
 * Domain Model → React Flow Model mapper.
 * The only layer that knows React Flow's node/edge types.
 *
 * Each machine of the core becomes a section (group node) containing its
 * state nodes — unless the core has a single machine, which renders flat.
 * Geometry (positions and routes) is computed later by the LayoutEngine;
 * this layer provides the node dimensions the engine needs.
 */

import type { Edge, Node } from '@xyflow/react';
import type { DomainCore, DomainMachine } from '../domain/types';
import { wildcardStateId } from '../domain/types';
import { entryState, stateRole } from '../domain/stateRole';

export interface FlowModel {
  nodes: Node[];
  edges: Edge[];
}

/**
 * Chrome this layer has to render but must not author.
 *
 * Localization is a presentation concern: the mapper receives already-
 * translated text from the component boundary instead of importing the message
 * catalog, which keeps this layer (and the domain below it) language-free.
 * It matters for geometry too — node widths are estimated from the label, so
 * the *translated* string is the one that has to be measured.
 */
export interface FlowLabels {
  /** Label of the wildcard pseudo-node. */
  anyState: string;
}

const NODE_HEIGHT = 44;
const NODE_MIN_WIDTH = 110;
const NODE_PADDING_X = 44;
/** Average glyph width of the node label font (14px system-ui). */
const NODE_CHAR_WIDTH = 7.6;
/** Extra room for the initial-state dot rendered before the label. */
const INITIAL_MARKER_WIDTH = 14;

export function toFlowModel(core: DomainCore, labels: FlowLabels): FlowModel {
  const grouped = core.machines.length > 1;
  const nodes: Node[] = [];
  const edges: Edge[] = [];

  for (const machine of core.machines) {
    if (grouped) {
      nodes.push({
        id: machine.id,
        type: 'machineGroup',
        // clicking the section stands for clicking its entry state
        data: { label: machine.name, entryStateId: entryState(machine)?.id },
        position: { x: 0, y: 0 },
      });
    }
    nodes.push(...machineNodes(machine, grouped ? machine.id : undefined, labels));
    edges.push(...machineEdges(machine));
  }

  return { nodes, edges };
}

function machineNodes(
  machine: DomainMachine,
  parentId: string | undefined,
  labels: FlowLabels,
): Node[] {
  const base = { parentId, position: { x: 0, y: 0 } };

  const nodes: Node[] = machine.states.map((state) => {
    // Composite leaves ("Active/Loading") read better with spaced separators.
    const label = state.name.replace(/\//g, ' / ');
    const role = stateRole(machine, state);
    return {
      ...base,
      id: state.id,
      type: 'state',
      data: { label, initial: role.initial, failure: role.failure, final: role.final },
      // the initial marker (a dot before the label) needs its own room
      width: nodeWidth(label) + (role.initial ? INITIAL_MARKER_WIDTH : 0),
      height: NODE_HEIGHT,
    };
  });

  if (machine.hasWildcard) {
    nodes.push({
      ...base,
      id: wildcardStateId(machine.id),
      type: 'anyState',
      data: { label: labels.anyState },
      width: nodeWidth(labels.anyState),
      height: 36,
    });
  }

  return nodes;
}

function machineEdges(machine: DomainMachine): Edge[] {
  return machine.transitions.map((transition) => ({
    id: transition.id,
    type: 'routed',
    source: transition.from,
    target: transition.to,
    label: transition.event,
    // The arrowhead color is theme-dependent and applied by the renderer:
    // SVG marker attributes cannot read CSS variables.
    markerEnd: {
      type: 'arrowclosed' as const,
      width: 14,
      height: 14,
    },
  }));
}

function nodeWidth(label: string): number {
  return Math.max(NODE_MIN_WIDTH, Math.round(label.length * NODE_CHAR_WIDTH) + NODE_PADDING_X);
}
