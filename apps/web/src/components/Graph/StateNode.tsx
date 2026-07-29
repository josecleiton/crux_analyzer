/**
 * Node renderer for a state. Handles are present (React Flow requires them
 * to bind edges) but invisible — edge geometry comes from the LayoutEngine,
 * not from handle anchors.
 */

import { Handle, Position } from '@xyflow/react';
import type { NodeProps } from '@xyflow/react';

export function StateNode({ data, selected }: NodeProps) {
  return (
    <div className={selected ? 'state-node selected' : 'state-node'}>
      <Handle type="target" position={Position.Top} className="state-node-handle" />
      {String(data.label)}
      <Handle type="source" position={Position.Bottom} className="state-node-handle" />
    </div>
  );
}
