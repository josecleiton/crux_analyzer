import type { FlowModel } from '../flow/toFlowModel';
import type { ChangeSet } from './types';

/**
 * Decorates a FlowModel's nodes and edges with data.changeType ('added' | 'modified' | 'removed')
 * based on the provided ChangeSet.
 */
export function annotateFlowModel(flowModel: FlowModel, changeSet: ChangeSet): FlowModel {
  if (!changeSet || changeSet.totalChanges === 0) {
    return flowModel;
  }

  // Collect modified state IDs
  const modifiedStateIds = new Set<string>();
  for (const m of changeSet.machines) {
    for (const s of m.states.modified) {
      modifiedStateIds.add(s.stateId);
    }
  }

  // Collect added and modified transition keys (from-event-to)
  const addedEdgeIds = new Set<string>();
  const modifiedEdgeIds = new Set<string>();

  for (const m of changeSet.machines) {
    for (const t of m.transitions.added) {
      const edgeId = `${t.fromName}-${t.event}-${t.toName}`;
      addedEdgeIds.add(edgeId);
    }
    for (const t of m.transitions.modified) {
      const edgeId = `${t.key.from}-${t.key.event}-${t.key.to}`;
      modifiedEdgeIds.add(edgeId);
    }
  }

  const annotatedNodes = flowModel.nodes.map((node) => {
    if (modifiedStateIds.has(node.id)) {
      return {
        ...node,
        data: {
          ...node.data,
          changeType: 'modified' as const,
        },
      };
    }
    return node;
  });

  const annotatedEdges = flowModel.edges.map((edge) => {
    // Edge IDs in toFlowModel are transition.id (`${from}-${event}-${to}`)
    if (addedEdgeIds.has(edge.id)) {
      return {
        ...edge,
        data: {
          ...edge.data,
          changeType: 'added' as const,
        },
      };
    }
    if (modifiedEdgeIds.has(edge.id)) {
      return {
        ...edge,
        data: {
          ...edge.data,
          changeType: 'modified' as const,
        },
      };
    }
    return edge;
  });

  return {
    nodes: annotatedNodes,
    edges: annotatedEdges,
  };
}
