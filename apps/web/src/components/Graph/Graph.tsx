/**
 * Pure graph renderer: receives nodes and edges with geometry already
 * computed by the LayoutEngine, plus the selection and optional highlights,
 * and only emits selection events. It knows nothing about domain, layout or
 * data source — the Simulation Engine drives highlights through props.
 */

import { ReactFlow, Background, Controls } from '@xyflow/react';
import type { Edge, Node } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useMemo, useState } from 'react';
import type { Selection } from '../../state/selection';
import type { Theme } from '../../theme/theme';
import { useTranslate } from '../../i18n/useI18n';
import { useGraphColors } from '../../theme/useTheme';
import { StateNode } from './StateNode';
import { AnyStateNode } from './AnyStateNode';
import { MachineGroupNode } from './MachineGroupNode';
import { RoutedEdge } from './RoutedEdge';
import { ViewportFocus } from './ViewportFocus';
import type { FitRequest } from './ViewportFocus';

const nodeTypes = { state: StateNode, anyState: AnyStateNode, machineGroup: MachineGroupNode };
const edgeTypes = { routed: RoutedEdge };
/** Room left around the framed content, on load and on every framing. */
const FIT_PADDING = 0.15;

/**
 * Ids to emphasize. The simulation drives three tiers of emphasis, so the
 * replay reads as a path instead of a single lit-up state:
 *
 * - `nodeIds`/`edgeIds` — the here and now: current state, last transition.
 * - `visited` — everything already traveled (bold).
 * - `available` — what can fire from the current state, and where it lands.
 *
 * With `dimOthers`, states and transitions outside those sets fade back.
 */
export interface GraphHighlight {
  nodeIds: string[];
  edgeIds: string[];
  visited?: { nodeIds: string[]; edgeIds: string[] };
  available?: { nodeIds: string[]; edgeIds: string[] };
  dimOthers?: boolean;
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
  const t = useTranslate();
  // Framing a section is a viewport reaction to a click, not graph state: a
  // fresh request object per click re-frames even the same section.
  const [fitRequest, setFitRequest] = useState<FitRequest | null>(null);

  // The simulation already tells us where it is; the camera tags along.
  const currentStateId = highlight?.nodeIds[0];
  const step = highlight?.step ?? 0;
  const follow = useMemo(
    () => (currentStateId ? { nodeId: currentStateId, step } : null),
    [currentStateId, step],
  );

  // React Flow ships English accessible names for its own controls; without
  // this they would stay English while the rest of the UI is translated.
  const ariaLabelConfig = useMemo(
    () => ({
      'controls.ariaLabel': t('graph.a11y.controls'),
      'controls.zoomIn.ariaLabel': t('graph.a11y.zoomIn'),
      'controls.zoomOut.ariaLabel': t('graph.a11y.zoomOut'),
      'controls.fitView.ariaLabel': t('graph.a11y.fitView'),
    }),
    [t],
  );

  // Alternating pulse class: re-adding the same class would not restart the
  // arrival animation when a transition loops back to the current state.
  const pulseClass = (highlight?.step ?? 0) % 2 === 0 ? 'pulse-a' : 'pulse-b';
  const failureClass = highlight?.failure ? ' is-failure' : '';

  const visitedNodes = new Set(highlight?.visited?.nodeIds ?? []);
  const visitedEdges = new Set(highlight?.visited?.edgeIds ?? []);
  const availableNodes = new Set(highlight?.available?.nodeIds ?? []);
  const availableEdges = new Set(highlight?.available?.edgeIds ?? []);

  /** Emphasis tier of a graph element, from the highlight sets. */
  function tier(id: string, current: boolean, visited: Set<string>, available: Set<string>) {
    const classes: string[] = [];
    if (visited.has(id)) classes.push('visited');
    if (available.has(id)) classes.push('available');
    if (highlight?.dimOthers && !current && classes.length === 0) classes.push('dimmed');
    return classes;
  }

  const styledNodes = nodes.map((node) => {
    const current = highlight?.nodeIds.includes(node.id) ?? false;
    // group containers are scenery: they never take part in the emphasis
    const classes =
      node.type === 'machineGroup' ? [] : tier(node.id, current, visitedNodes, availableNodes);
    if (current) classes.push('highlighted', pulseClass, ...(failureClass ? ['is-failure'] : []));
    return {
      ...node,
      selected: selection?.kind === 'state' && selection.id === node.id,
      className: classes.length > 0 ? classes.join(' ') : undefined,
    };
  });
  const styledEdges = edges.map((edge) => {
    const selected = selection?.kind === 'transition' && selection.id === edge.id;
    const highlighted = highlight?.edgeIds.includes(edge.id) ?? false;
    const classes = tier(edge.id, highlighted, visitedEdges, availableEdges);
    if (highlighted) {
      classes.push('highlighted', ...(failureClass ? ['is-failure'] : []));
    }
    // keep the arrowhead in sync with the stroke color of its state
    const stroke = selected
      ? colors.edgeSelected
      : highlighted
        ? highlight?.failure
          ? colors.edgeFailure
          : colors.edgeHighlighted
        : visitedEdges.has(edge.id)
          ? colors.edgeHighlighted
          : colors.edge;
    return {
      ...edge,
      selected,
      className: classes.length > 0 ? classes.join(' ') : undefined,
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
        if (node.type === 'state') {
          onSelect({ kind: 'state', id: node.id });
          return;
        }
        // a machine section resolves to the entry state the mapper put in it,
        // and frames the machine it stands for
        const entryStateId = node.data?.entryStateId;
        if (node.type === 'machineGroup' && typeof entryStateId === 'string') {
          onSelect({ kind: 'state', id: entryStateId });
          setFitRequest({ nodeId: node.id });
        }
      }}
      onEdgeClick={(_, edge) => onSelect({ kind: 'transition', id: edge.id })}
      onPaneClick={() => onSelect(null)}
      nodesDraggable={false}
      nodesConnectable={false}
      colorMode={theme}
      ariaLabelConfig={ariaLabelConfig}
      fitView
      fitViewOptions={{ padding: FIT_PADDING }}
      proOptions={{ hideAttribution: true }}
    >
      <Background gap={20} />
      <Controls showInteractive={false} />
      <ViewportFocus fit={fitRequest} follow={follow} padding={FIT_PADDING} />
    </ReactFlow>
  );
}
