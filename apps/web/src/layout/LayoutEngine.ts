/**
 * Layout engine contract. The rest of the app depends only on this
 * interface — swapping ELK for another algorithm only changes the
 * implementation.
 *
 * The engine owns the whole geometry: node positions AND edge routes.
 * Routed edges carry an [`EdgeRoute`] in `edge.data.route`, which the
 * Graph's edge component renders verbatim — no client-side re-routing.
 */

import type { Edge, Node } from '@xyflow/react';

export interface Point {
  x: number;
  y: number;
}

/** Geometry computed by the engine for one edge, in canvas coordinates. */
export interface EdgeRoute {
  /** Polyline from source to target (start, bends, end). */
  points: Point[];
  /** Box reserved for the label, when the edge has one. */
  label?: { x: number; y: number; width: number; height: number };
}

export interface LayoutResult {
  nodes: Node[];
  edges: Edge[];
}

export interface LayoutEngine {
  /**
   * Returns nodes with computed positions and edges with computed routes.
   * Does not mutate the inputs.
   */
  layout(nodes: Node[], edges: Edge[]): Promise<LayoutResult>;
}
