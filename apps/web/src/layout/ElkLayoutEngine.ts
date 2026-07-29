/**
 * LayoutEngine implementation using ELKJS ("layered" algorithm, top-down,
 * orthogonal edge routing). ELK computes node positions, full edge routes
 * (bend points) and inline label positions; everything is passed through to
 * the renderer untouched.
 */

import ELK from 'elkjs/lib/elk.bundled.js';
import type { ElkExtendedEdge, ElkNode } from 'elkjs/lib/elk.bundled.js';
import type { Edge, Node } from '@xyflow/react';
import type { EdgeRoute, LayoutEngine, LayoutResult, Point } from './LayoutEngine';

const DEFAULT_NODE_WIDTH = 160;
const DEFAULT_NODE_HEIGHT = 44;

export class ElkLayoutEngine implements LayoutEngine {
  private elk = new ELK();

  async layout(nodes: Node[], edges: Edge[]): Promise<LayoutResult> {
    if (nodes.length === 0) return { nodes: [], edges: [] };

    const graph = await this.elk.layout({
      id: 'root',
      layoutOptions: {
        'elk.algorithm': 'layered',
        'elk.direction': 'DOWN',
        'elk.edgeRouting': 'ORTHOGONAL',
        'elk.layered.spacing.nodeNodeBetweenLayers': '64',
        'elk.spacing.nodeNode': '48',
        'elk.spacing.edgeNode': '28',
        'elk.spacing.edgeEdge': '16',
        'elk.spacing.edgeLabel': '6',
        // Inline labels sit on the edge itself; ELK reserves room for them.
        'elk.edgeLabels.inline': 'true',
      },
      children: nodes.map((node) => ({
        id: node.id,
        width: node.width ?? DEFAULT_NODE_WIDTH,
        height: node.height ?? DEFAULT_NODE_HEIGHT,
      })),
      edges: edges.map((edge) => ({
        id: edge.id,
        sources: [edge.source],
        targets: [edge.target],
        labels: edge.label
          ? [
              {
                text: String(edge.label),
                width: estimateLabelWidth(String(edge.label)),
                height: 20,
              },
            ]
          : [],
      })),
    });

    const positions = new Map(
      (graph.children ?? []).map((child: ElkNode) => [
        child.id,
        { x: child.x ?? 0, y: child.y ?? 0 },
      ]),
    );
    const routes = new Map(
      (graph.edges ?? []).map((edge: ElkExtendedEdge) => [edge.id, toRoute(edge)]),
    );

    return {
      nodes: nodes.map((node) => ({
        ...node,
        position: positions.get(node.id) ?? node.position,
      })),
      edges: edges.map((edge) => ({
        ...edge,
        data: { ...edge.data, route: routes.get(edge.id) },
      })),
    };
  }
}

function toRoute(edge: ElkExtendedEdge): EdgeRoute | undefined {
  const section = edge.sections?.[0];
  if (!section) return undefined;

  const points: Point[] = [
    section.startPoint,
    ...(section.bendPoints ?? []),
    section.endPoint,
  ].map((p) => ({ x: p.x, y: p.y }));

  const label = edge.labels?.[0];
  return {
    points,
    label:
      label && label.x !== undefined && label.y !== undefined
        ? { x: label.x, y: label.y, width: label.width ?? 0, height: label.height ?? 0 }
        : undefined,
  };
}

/** Rough width of an edge label rendered at 11px monospace + padding. */
function estimateLabelWidth(text: string): number {
  return Math.round(text.length * 6.8) + 16;
}
