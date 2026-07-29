/**
 * LayoutEngine implementation using ELKJS ("layered" algorithm, top-down).
 */

import ELK from 'elkjs/lib/elk.bundled.js';
import type { Edge, Node } from '@xyflow/react';
import type { LayoutEngine } from './LayoutEngine';

const NODE_WIDTH = 160;
const NODE_HEIGHT = 44;

export class ElkLayoutEngine implements LayoutEngine {
  private elk = new ELK();

  async layout(nodes: Node[], edges: Edge[]): Promise<Node[]> {
    if (nodes.length === 0) return [];

    const graph = await this.elk.layout({
      id: 'root',
      layoutOptions: {
        'elk.algorithm': 'layered',
        'elk.direction': 'DOWN',
        'elk.layered.spacing.nodeNodeBetweenLayers': '80',
        'elk.spacing.nodeNode': '60',
        'elk.edgeRouting': 'SPLINES',
      },
      children: nodes.map((node) => ({
        id: node.id,
        width: NODE_WIDTH,
        height: NODE_HEIGHT,
      })),
      edges: edges.map((edge) => ({
        id: edge.id,
        sources: [edge.source],
        targets: [edge.target],
      })),
    });

    const positions = new Map(
      (graph.children ?? []).map((child) => [child.id, { x: child.x ?? 0, y: child.y ?? 0 }]),
    );

    return nodes.map((node) => ({
      ...node,
      position: positions.get(node.id) ?? node.position,
    }));
  }
}
