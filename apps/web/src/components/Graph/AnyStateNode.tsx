/**
 * Pseudo-state representing "any state" — the source of wildcard transitions.
 */

import { Handle, Position } from '@xyflow/react';
import type { NodeProps } from '@xyflow/react';

export function AnyStateNode({ data }: NodeProps) {
  return (
    <div className="any-state-node">
      <Handle type="target" position={Position.Top} className="state-node-handle" />
      {String(data.label)}
      <Handle type="source" position={Position.Bottom} className="state-node-handle" />
    </div>
  );
}
