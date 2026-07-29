/**
 * Node renderer for a state. Handles are present (React Flow requires them
 * to bind edges) but invisible — edge geometry comes from the LayoutEngine,
 * not from handle anchors.
 *
 * Initial, failure, deprecated and final states are told apart by the classes
 * the flow mapper's role flags produce (see domain/stateRole.ts); the colors
 * live in the CSS tokens, so both themes are covered. Roles are always
 * painted, with or without a running simulation.
 *
 * A documented state gets a small mark and a native `title` tooltip. `title`
 * rather than a hover card on purpose: React Flow scales its node pane, so a
 * card inside a node blurs and one outside needs a portal positioned against
 * the transform — and a tooltip needs no reduced-motion handling.
 */

import { Handle, Position } from '@xyflow/react';
import type { NodeProps } from '@xyflow/react';

export function StateNode({ data, selected }: NodeProps) {
  const className = [
    'state-node',
    data.initial ? 'state-initial' : '',
    data.deprecated ? 'state-deprecated' : '',
    data.failure ? 'state-failure' : '',
    data.final ? 'state-final' : '',
    selected ? 'selected' : '',
  ]
    .filter(Boolean)
    .join(' ');
  // The analyzed app's own prose: shown verbatim, never localized.
  const doc = typeof data.doc === 'string' ? data.doc : undefined;

  return (
    <div className={className} title={doc}>
      <Handle type="target" position={Position.Top} className="state-node-handle" />
      {data.initial ? <span className="state-initial-dot" aria-hidden="true" /> : null}
      {String(data.label)}
      {doc ? <span className="state-doc-mark" aria-hidden="true" /> : null}
      <Handle type="source" position={Position.Bottom} className="state-node-handle" />
    </div>
  );
}
