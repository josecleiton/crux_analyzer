/**
 * Domain Model → React Flow Model mapper.
 * The only layer that knows React Flow's node/edge types.
 * Actual positions are computed later by the LayoutEngine.
 */

import type { Edge, Node } from '@xyflow/react';
import type { DomainCore } from '../domain/types';

export interface FlowModel {
  nodes: Node[];
  edges: Edge[];
}

export function toFlowModel(core: DomainCore): FlowModel {
  const nodes: Node[] = core.states.map((state) => ({
    id: state.id,
    data: { label: state.name },
    position: { x: 0, y: 0 },
  }));

  const edges: Edge[] = core.transitions.map((transition) => ({
    id: transition.id,
    source: transition.from,
    target: transition.to,
    label: transition.event,
    // smoothstep keeps back edges (upward transitions) close to the nodes
    // instead of the default bezier's long off-viewport curves
    type: 'smoothstep' as const,
    markerEnd: { type: 'arrowclosed' as const },
  }));

  return { nodes, edges };
}
