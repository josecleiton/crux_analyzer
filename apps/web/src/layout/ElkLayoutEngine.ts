/**
 * LayoutEngine implementation using ELKJS ("layered" algorithm, top-down,
 * orthogonal edge routing). ELK computes node positions, full edge routes
 * (bend points) and inline label positions; everything is passed through to
 * the renderer untouched.
 *
 * Grouping is arbitrary-depth over `parentId` — machine sections holding
 * composite containers holding states. ELK reports child positions relative
 * to the parent — exactly what React Flow expects — while edge routes are
 * shifted to absolute canvas coordinates for the renderer.
 *
 * Two hierarchy rules make composite edges work. An edge is declared in the
 * lowest common ancestor of its endpoints (ELK's requirement), and each
 * machine's subtree is laid out as one run (`INCLUDE_CHILDREN`) so an edge
 * may cross a composite's boundary; the sections themselves stay separate
 * runs, keeping the side-by-side flow with top-down insides.
 */

import ELK from 'elkjs/lib/elk.bundled.js';
import type { ElkExtendedEdge, ElkNode } from 'elkjs/lib/elk.bundled.js';
import type { Edge, Node } from '@xyflow/react';
import type { EdgeRoute, LayoutEngine, LayoutResult, Point } from './LayoutEngine';

const DEFAULT_NODE_WIDTH = 160;
const DEFAULT_NODE_HEIGHT = 44;
/** Space reserved at the top of a machine section for its title bar. */
const GROUP_PADDING = '[top=52,left=24,bottom=24,right=24]';
/** Composite containers have a smaller title, so less headroom. */
const COMPOSITE_PADDING = '[top=44,left=20,bottom=20,right=20]';

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

    const childrenOf = new Map<string | undefined, Node[]>();
    for (const node of nodes) {
      const siblings = childrenOf.get(node.parentId) ?? [];
      siblings.push(node);
      childrenOf.set(node.parentId, siblings);
    }
    const parentOf = new Map(nodes.map((node) => [node.id, node.parentId]));

    // An edge is declared in the lowest common ancestor of its endpoints
    // (ELK's requirement); `undefined` is the root.
    const ancestorsOf = (id: string): (string | undefined)[] => {
      const chain: (string | undefined)[] = [];
      let current = parentOf.get(id);
      while (current !== undefined) {
        chain.push(current);
        current = parentOf.get(current);
      }
      chain.push(undefined);
      return chain;
    };
    const edgesByContainer = new Map<string | undefined, ElkExtendedEdge[]>();
    for (const edge of edges) {
      const targetAncestors = new Set(ancestorsOf(edge.target));
      const container = ancestorsOf(edge.source).find((a) => targetAncestors.has(a));
      const list = edgesByContainer.get(container) ?? [];
      list.push(this.elkEdge(edge));
      edgesByContainer.set(container, list);
    }

    const build = (node: Node): ElkNode => {
      const children = childrenOf.get(node.id);
      if (!children) return this.elkNode(node);
      return {
        id: node.id,
        layoutOptions: {
          ...COMMON_OPTIONS,
          'elk.padding': node.type === 'machineGroup' ? GROUP_PADDING : COMPOSITE_PADDING,
          // one layout run per machine section, so edges may cross the
          // composite containers inside it
          ...(node.type === 'machineGroup' ? { 'elk.hierarchyHandling': 'INCLUDE_CHILDREN' } : {}),
        },
        children: children.map(build),
        edges: edgesByContainer.get(node.id) ?? [],
      };
    };

    const roots = childrenOf.get(undefined) ?? [];
    const hasSections = roots.some((node) => node.type === 'machineGroup');
    const graph = await this.elk.layout({
      id: 'root',
      layoutOptions: {
        ...COMMON_OPTIONS,
        // Sections flow left-to-right next to each other, each an isolated
        // run; a flat (single-machine) core is itself one hierarchical run.
        ...(hasSections
          ? { 'elk.direction': 'RIGHT', 'elk.spacing.nodeNode': '56' }
          : { 'elk.hierarchyHandling': 'INCLUDE_CHILDREN' }),
      },
      children: roots.map(build),
      edges: edgesByContainer.get(undefined) ?? [],
    });

    return this.applyGeometry(graph, nodes, edges);
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
          ...(geometry.width !== undefined &&
          (node.type === 'machineGroup' || node.type === 'compositeGroup')
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
