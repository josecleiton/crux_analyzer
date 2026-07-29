/**
 * Section container for one state machine (orthogonal region) of a core.
 * Purely visual: a titled box sized by the LayoutEngine.
 */

import type { NodeProps } from '@xyflow/react';

export function MachineGroupNode({ data }: NodeProps) {
  return (
    <div className="machine-group">
      <div className="machine-group-title">{String(data.label)}</div>
    </div>
  );
}
