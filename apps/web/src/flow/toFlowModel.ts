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

export interface FlowModel {
  nodes: Node[];
  edges: Edge[];
}

const NODE_HEIGHT = 44;
const NODE_MIN_WIDTH = 110;
const NODE_PADDING_X = 44;
/** Average glyph width of the node label font (14px system-ui). */
const NODE_CHAR_WIDTH = 7.6;

export function toFlowModel(core: DomainCore): FlowModel {
  const grouped = core.machines.length > 1;
  const nodes: Node[] = [];
  const edges: Edge[] = [];

  for (const machine of core.machines) {
    if (grouped) {
      nodes.push({
        id: machine.id,
        type: 'machineGroup',
        data: { label: machine.name },
        position: { x: 0, y: 0 },
      });
    }
    nodes.push(...machineNodes(machine, grouped ? machine.id : undefined));
    edges.push(...machineEdges(machine));
  }

  return { nodes, edges };
}

function machineNodes(machine: DomainMachine, parentId: string | undefined): Node[] {
  const base = { parentId, position: { x: 0, y: 0 } };

  const nodes: Node[] = machine.states.map((state) => ({
    ...base,
    id: state.id,
    type: 'state',
    data: { label: state.name },
    width: nodeWidth(state.name),
    height: NODE_HEIGHT,
  }));

  if (machine.hasWildcardSource) {
    nodes.push({
      ...base,
      id: wildcardStateId(machine.id),
      type: 'anyState',
      data: { label: 'any state' },
      width: nodeWidth('any state'),
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
    markerEnd: {
      type: 'arrowclosed' as const,
      width: 14,
      height: 14,
      color: '#8792a2',
    },
  }));
}

function nodeWidth(label: string): number {
  return Math.max(NODE_MIN_WIDTH, Math.round(label.length * NODE_CHAR_WIDTH) + NODE_PADDING_X);
}
