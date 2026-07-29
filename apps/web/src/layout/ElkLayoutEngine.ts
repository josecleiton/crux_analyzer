/**
 * LayoutEngine implementation using ELKJS ("layered" algorithm, top-down,
 * orthogonal edge routing). ELK computes node positions, full edge routes
 * (bend points) and inline label positions; everything is passed through to
 * the renderer untouched.
 *
 * Supports one level of grouping (machine sections): nodes with a `parentId`
 * are laid out inside their group's compound node. ELK reports child
 * positions relative to the parent — exactly what React Flow expects — while
 * edge routes are shifted to absolute canvas coordinates for the renderer.
 */

import ELK from 'elkjs/lib/elk.bundled.js';
import type { ElkExtendedEdge, ElkNode } from 'elkjs/lib/elk.bundled.js';
import type { Edge, Node } from '@xyflow/react';
import type { EdgeRoute, LayoutEngine, LayoutResult, Point } from './LayoutEngine';

const DEFAULT_NODE_WIDTH = 160;
const DEFAULT_NODE_HEIGHT = 44;
/** Space reserved at the top of a group for its title bar. */
const GROUP_PADDING = '[top=52,left=24,bottom=24,right=24]';

const COMMON_OPTIONS = {
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
};

export class ElkLayoutEngine implements LayoutEngine {
  private elk = new ELK();

  async layout(nodes: Node[], edges: Edge[]): Promise<LayoutResult> {
    if (nodes.length === 0) return { nodes: [], edges: [] };

    const groups = nodes.filter((node) => !node.parentId && this.isGroup(node, nodes));
    const rootNodes = nodes.filter((node) => !node.parentId && !this.isGroup(node, nodes));

    // An edge lives in the container of its endpoints (ELK requires edges to
    // be declared in the lowest common ancestor). Machine edges never cross
    // machine boundaries, so the source's parent decides.
    const parentOf = new Map(nodes.map((node) => [node.id, node.parentId]));
    const edgesOf = (containerId: string | undefined) =>
      edges.filter((edge) => parentOf.get(edge.source) === containerId).map((e) => this.elkEdge(e));

    const graph = await this.elk.layout({
      id: 'root',
      layoutOptions: {
        ...COMMON_OPTIONS,
        // Sections flow left-to-right next to each other.
        ...(groups.length > 0 ? { 'elk.direction': 'RIGHT', 'elk.spacing.nodeNode': '56' } : {}),
      },
      children: [
        ...groups.map((group) => ({
          id: group.id,
          layoutOptions: { ...COMMON_OPTIONS, 'elk.padding': GROUP_PADDING },
          children: this.elkChildren(nodes, group.id),
          edges: edgesOf(group.id),
        })),
        ...rootNodes.map((node) => this.elkNode(node)),
      ],
      edges: edgesOf(undefined),
    });

    return this.applyGeometry(graph, nodes, edges);
  }

  private isGroup(node: Node, all: Node[]): boolean {
    return all.some((candidate) => candidate.parentId === node.id);
  }

  private elkChildren(nodes: Node[], parentId: string) {
    return nodes.filter((node) => node.parentId === parentId).map((node) => this.elkNode(node));
  }

  private elkNode(node: Node) {
    return {
      id: node.id,
      width: node.width ?? DEFAULT_NODE_WIDTH,
      height: node.height ?? DEFAULT_NODE_HEIGHT,
    };
  }

  private elkEdge(edge: Edge) {
    return {
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
    };
  }

  private applyGeometry(graph: ElkNode, nodes: Node[], edges: Edge[]): LayoutResult {
    const positions = new Map<string, { x: number; y: number; width?: number; height?: number }>();
    const routes = new Map<string, EdgeRoute | undefined>();
    // ELK edge/child coordinates are relative to their container node.
    collectGeometry(graph, { x: 0, y: 0 }, positions, routes);

    return {
      nodes: nodes.map((node) => {
        const geometry = positions.get(node.id);
        if (!geometry) return node;
        return {
          ...node,
          position: { x: geometry.x, y: geometry.y },
          // Groups get their size from the layout.
          ...(geometry.width !== undefined && node.type === 'machineGroup'
            ? { width: geometry.width, height: geometry.height }
            : {}),
        };
      }),
      edges: edges.map((edge) => ({
        ...edge,
        data: { ...edge.data, route: routes.get(edge.id) },
      })),
    };
  }
}

function collectGeometry(
  container: ElkNode,
  origin: Point,
  positions: Map<string, { x: number; y: number; width?: number; height?: number }>,
  routes: Map<string, EdgeRoute | undefined>,
) {
  for (const child of container.children ?? []) {
    positions.set(child.id, {
      // React Flow child positions are relative to the parent, like ELK's.
      x: child.x ?? 0,
      y: child.y ?? 0,
      width: child.width,
      height: child.height,
    });
    const absolute = { x: origin.x + (child.x ?? 0), y: origin.y + (child.y ?? 0) };
    collectGeometry(child, absolute, positions, routes);
  }
  for (const edge of container.edges ?? []) {
    routes.set(edge.id, toRoute(edge, origin));
  }
}

function toRoute(edge: ElkExtendedEdge, origin: Point): EdgeRoute | undefined {
  const section = edge.sections?.[0];
  if (!section) return undefined;

  const points: Point[] = [
    section.startPoint,
    ...(section.bendPoints ?? []),
    section.endPoint,
  ].map((p) => ({ x: origin.x + p.x, y: origin.y + p.y }));

  const label = edge.labels?.[0];
  return {
    points,
    label:
      label && label.x !== undefined && label.y !== undefined
        ? {
            x: origin.x + label.x,
            y: origin.y + label.y,
            width: label.width ?? 0,
            height: label.height ?? 0,
          }
        : undefined,
  };
}

/** Rough width of an edge label rendered at 11px monospace + padding. */
function estimateLabelWidth(text: string): number {
  return Math.round(text.length * 6.8) + 16;
}
