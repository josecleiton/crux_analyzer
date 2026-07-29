/**
 * Viewport driver: brings a requested target into view. Renders nothing.
 *
 * It sits *inside* `<ReactFlow>` because that is where React Flow's store lives
 * (the component wraps its children in a provider when there is none above it),
 * the same reason `<Background>` and `<Controls>` are children. Both requests
 * arrive as props, so the Graph keeps only reacting to what it is given and the
 * simulation still drives the canvas through props alone.
 */

import { useEffect } from 'react';
import { useReactFlow, useStoreApi } from '@xyflow/react';
import { isRectInView, rectCenter } from './viewport';

/** Frame this node: pan *and* zoom until it fills the pane. */
export interface FitRequest {
  nodeId: string;
}

/** Keep this node in sight, without ever changing the zoom. */
export interface FollowRequest {
  nodeId: string;
  /** Simulation step, so landing on the same state again still re-centers. */
  step: number;
}

const DURATION = 400;

/**
 * Viewport tweens are JS-driven, so the CSS `prefers-reduced-motion` block
 * cannot silence them: with the preference set, the camera jumps instead.
 */
function animationDuration(): number {
  return window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ? 0 : DURATION;
}

interface ViewportFocusProps {
  fit: FitRequest | null;
  follow: FollowRequest | null;
  /** Padding of the framing, shared with the canvas' initial fitView. */
  padding: number;
}

export function ViewportFocus({ fit, follow, padding }: ViewportFocusProps) {
  const { fitView, setCenter, getZoom, getViewport, getInternalNode } = useReactFlow();
  // Read (never subscribe to) the pane size: a window resize must not pan.
  const store = useStoreApi();

  useEffect(() => {
    if (!fit) return;
    // maxZoom keeps a small section from being blown up to fill the screen.
    void fitView({
      nodes: [{ id: fit.nodeId }],
      padding,
      maxZoom: 1,
      duration: animationDuration(),
    });
  }, [fit, fitView, padding]);

  useEffect(() => {
    if (!follow) return;
    const node = getInternalNode(follow.nodeId);
    if (!node) return;

    const rect = {
      x: node.internals.positionAbsolute.x,
      y: node.internals.positionAbsolute.y,
      width: node.measured.width ?? 0,
      height: node.measured.height ?? 0,
    };
    const { width, height } = store.getState();
    if (isRectInView(rect, { ...getViewport(), width, height })) return;

    const center = rectCenter(rect);
    void setCenter(center.x, center.y, { zoom: getZoom(), duration: animationDuration() });
  }, [follow, getInternalNode, getViewport, getZoom, setCenter, store]);

  return null;
}
