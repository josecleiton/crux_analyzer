/**
 * Domain Model → React Flow Model mapper.
 * The only layer that knows React Flow's node/edge types.
 * Geometry (positions and routes) is computed later by the LayoutEngine;
 * this layer provides the node dimensions the engine needs.
 */

import type { Edge, Node } from '@xyflow/react';
import type { DomainCore } from '../domain/types';

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
  const nodes: Node[] = core.states.map((state) => ({
    id: state.id,
    type: 'state',
    data: { label: state.name },
    position: { x: 0, y: 0 },
    width: nodeWidth(state.name),
    height: NODE_HEIGHT,
  }));

  const edges: Edge[] = core.transitions.map((transition) => ({
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

  return { nodes, edges };
}

function nodeWidth(label: string): number {
  return Math.max(NODE_MIN_WIDTH, Math.round(label.length * NODE_CHAR_WIDTH) + NODE_PADDING_X);
}
