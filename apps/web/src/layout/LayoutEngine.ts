/**
 * Layout engine contract. The rest of the app depends only on this
 * interface — swapping ELK for another algorithm only changes the
 * implementation.
 */

import type { Edge, Node } from '@xyflow/react';

export interface LayoutEngine {
  /** Returns the nodes with computed positions. Does not mutate the originals. */
  layout(nodes: Node[], edges: Edge[]): Promise<Node[]>;
}
