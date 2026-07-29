/**
 * Pure graph renderer: receives nodes and edges with geometry already
 * computed by the LayoutEngine, plus the selection and optional highlights,
 * and only emits selection events. It knows nothing about domain, layout or
 * data source — the Simulation Engine drives highlights through props.
 */

import { ReactFlow, Background, Controls } from '@xyflow/react';
import type { Edge, Node } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import type { Selection } from '../../state/selection';
import { StateNode } from './StateNode';
import { AnyStateNode } from './AnyStateNode';
import { MachineGroupNode } from './MachineGroupNode';
import { RoutedEdge } from './RoutedEdge';

const nodeTypes = { state: StateNode, anyState: AnyStateNode, machineGroup: MachineGroupNode };
const edgeTypes = { routed: RoutedEdge };

/** Ids to emphasize (e.g. the simulation's current state and last transition). */
export interface GraphHighlight {
  nodeIds: string[];
  edgeIds: string[];
}

interface GraphProps {
  nodes: Node[];
  edges: Edge[];
  selection: Selection;
  onSelect: (selection: Selection) => void;
  highlight?: GraphHighlight;
}

export function Graph({ nodes, edges, selection, onSelect, highlight }: GraphProps) {
  const styledNodes = nodes.map((node) => ({
    ...node,
    selected: selection?.kind === 'state' && selection.id === node.id,
    className: highlight?.nodeIds.includes(node.id) ? 'highlighted' : undefined,
  }));
  const styledEdges = edges.map((edge) => {
    const selected = selection?.kind === 'transition' && selection.id === edge.id;
    return {
      ...edge,
      selected,
      className: highlight?.edgeIds.includes(edge.id) ? 'highlighted' : undefined,
      // keep the arrowhead in sync with the selected stroke color
      markerEnd:
        selected && typeof edge.markerEnd === 'object'
          ? { ...edge.markerEnd, color: '#6366f1' }
          : edge.markerEnd,
    };
  });

  return (
    <ReactFlow
      nodes={styledNodes}
      edges={styledEdges}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      onNodeClick={(_, node) => {
        if (node.type === 'state') onSelect({ kind: 'state', id: node.id });
      }}
      onEdgeClick={(_, edge) => onSelect({ kind: 'transition', id: edge.id })}
      onPaneClick={() => onSelect(null)}
      nodesDraggable={false}
      nodesConnectable={false}
      fitView
      fitViewOptions={{ padding: 0.15 }}
      proOptions={{ hideAttribution: true }}
    >
      <Background gap={20} />
      <Controls showInteractive={false} />
    </ReactFlow>
  );
}
