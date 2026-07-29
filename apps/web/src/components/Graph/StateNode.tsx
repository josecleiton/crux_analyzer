/**
 * Node renderer for a state. Handles are present (React Flow requires them
 * to bind edges) but invisible — edge geometry comes from the LayoutEngine,
 * not from handle anchors.
 *
 * Initial, failure and final states are told apart by the classes the flow
 * mapper's role flags produce (see domain/stateRole.ts); the colors live in
 * the CSS tokens, so both themes are covered. Roles are always painted, with
 * or without a running simulation.
 */

import { Handle, Position } from '@xyflow/react';
import type { NodeProps } from '@xyflow/react';

export function StateNode({ data, selected }: NodeProps) {
  const className = [
    'state-node',
    data.initial ? 'state-initial' : '',
    data.failure ? 'state-failure' : '',
    data.final ? 'state-final' : '',
    selected ? 'selected' : '',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div className={className}>
      <Handle type="target" position={Position.Top} className="state-node-handle" />
      {data.initial ? <span className="state-initial-dot" aria-hidden="true" /> : null}
      {String(data.label)}
      <Handle type="source" position={Position.Bottom} className="state-node-handle" />
    </div>
  );
}
