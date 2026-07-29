/**
 * Section container for one state machine (orthogonal region) of a core.
 * Purely visual: a titled box sized by the LayoutEngine.
 *
 * The state enum's description is a tooltip and nothing more — this box is
 * sized by ELK from its children, so putting content in it would move a
 * geometry decision into a component.
 */

import type { NodeProps } from '@xyflow/react';

export function MachineGroupNode({ data }: NodeProps) {
  const doc = typeof data.doc === 'string' ? data.doc : undefined;
  return (
    <div className="machine-group" title={doc}>
      <div className="machine-group-title">{String(data.label)}</div>
    </div>
  );
}
