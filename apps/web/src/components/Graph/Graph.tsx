/**
 * Pure graph renderer: receives already-positioned nodes/edges and the
 * selection via props, and only emits selection events. It knows nothing
 * about domain, layout or data source — the future Simulation Engine will
 * drive highlights through props.
 */

import { ReactFlow, Background, Controls } from '@xyflow/react';
import type { Edge, Node } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import type { Selection } from '../../state/selection';

interface GraphProps {
  nodes: Node[];
  edges: Edge[];
  selection: Selection;
  onSelect: (selection: Selection) => void;
}

export function Graph({ nodes, edges, selection, onSelect }: GraphProps) {
  const styledNodes = nodes.map((node) => ({
    ...node,
    selected: selection?.kind === 'state' && selection.id === node.id,
  }));
  const styledEdges = edges.map((edge) => ({
    ...edge,
    selected: selection?.kind === 'transition' && selection.id === edge.id,
  }));

  return (
    <ReactFlow
      nodes={styledNodes}
      edges={styledEdges}
      onNodeClick={(_, node) => onSelect({ kind: 'state', id: node.id })}
      onEdgeClick={(_, edge) => onSelect({ kind: 'transition', id: edge.id })}
      onPaneClick={() => onSelect(null)}
      nodesDraggable
      nodesConnectable={false}
      fitView
      proOptions={{ hideAttribution: true }}
    >
      <Background />
      <Controls />
    </ReactFlow>
  );
}
