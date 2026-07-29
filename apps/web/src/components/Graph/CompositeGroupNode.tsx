/**
 * Container for a composite state's children ("Active" holding "Loading",
 * "Ready") — the web counterpart of Mermaid's nested blocks. Purely visual
 * and sized by the LayoutEngine, like the machine sections.
 *
 * A composite parent is never a state of its own (the parser fans patterns
 * out over the leaves), so unlike a section this box selects nothing.
 */

import type { NodeProps } from '@xyflow/react';

export function CompositeGroupNode({ data }: NodeProps) {
  return (
    <div className="composite-group">
      <div className="composite-group-title">{String(data.label)}</div>
    </div>
  );
}
