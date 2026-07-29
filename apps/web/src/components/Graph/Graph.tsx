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
import type { Theme } from '../../theme/theme';
import { useGraphColors } from '../../theme/useTheme';
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
  /** Paints the highlight red — the simulation sits in a failure state. */
  failure?: boolean;
  /** Bumped on every step so the arrival animation replays (even on self-loops). */
  step?: number;
}

interface GraphProps {
  nodes: Node[];
  edges: Edge[];
  selection: Selection;
  onSelect: (selection: Selection) => void;
  highlight?: GraphHighlight;
  theme: Theme;
}

export function Graph({ nodes, edges, selection, onSelect, highlight, theme }: GraphProps) {
  const colors = useGraphColors(theme);

  // Alternating pulse class: re-adding the same class would not restart the
  // arrival animation when a transition loops back to the current state.
  const pulseClass = (highlight?.step ?? 0) % 2 === 0 ? 'pulse-a' : 'pulse-b';
  const failureClass = highlight?.failure ? ' is-failure' : '';

  const styledNodes = nodes.map((node) => ({
    ...node,
    selected: selection?.kind === 'state' && selection.id === node.id,
    className: highlight?.nodeIds.includes(node.id)
      ? `highlighted ${pulseClass}${failureClass}`
      : undefined,
  }));
  const styledEdges = edges.map((edge) => {
    const selected = selection?.kind === 'transition' && selection.id === edge.id;
    const highlighted = highlight?.edgeIds.includes(edge.id) ?? false;
    // keep the arrowhead in sync with the stroke color of its state
    const stroke = selected
      ? colors.edgeSelected
      : highlighted
        ? highlight?.failure
          ? colors.edgeFailure
          : colors.edgeHighlighted
        : colors.edge;
    return {
      ...edge,
      selected,
      className: highlighted ? `highlighted${failureClass}` : undefined,
      // the traveling pulse is drawn by the edge itself, which needs to know
      data: highlighted ? { ...edge.data, flowing: true } : edge.data,
      markerEnd:
        typeof edge.markerEnd === 'object' ? { ...edge.markerEnd, color: stroke } : edge.markerEnd,
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
      colorMode={theme}
      fitView
      fitViewOptions={{ padding: 0.15 }}
      proOptions={{ hideAttribution: true }}
    >
      <Background gap={20} />
      <Controls showInteractive={false} />
    </ReactFlow>
  );
}
