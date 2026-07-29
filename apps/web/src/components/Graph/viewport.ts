/**
 * Viewport math for the graph renderer. Pure geometry, no React and no React
 * Flow types, so it can be unit-tested on its own.
 *
 * Layout geometry is owned by the LayoutEngine; this is a different concern —
 * *where the camera looks*, computed from rectangles the renderer already has.
 */

/** A rectangle in flow (canvas) coordinates. */
export interface FlowRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** The visible pane: its transform plus its size in screen pixels. */
export interface ViewportState {
  x: number;
  y: number;
  zoom: number;
  width: number;
  height: number;
}

export function rectCenter(rect: FlowRect): { x: number; y: number } {
  return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
}

/**
 * Whether `rect` sits entirely inside the pane, with `margin` screen pixels to
 * spare on every side.
 *
 * The simulation follows the current state only when this is false, so a replay
 * through states that are already on screen does not shake the canvas at every
 * step.
 */
export function isRectInView(rect: FlowRect, viewport: ViewportState, margin = 60): boolean {
  const left = rect.x * viewport.zoom + viewport.x;
  const top = rect.y * viewport.zoom + viewport.y;
  const right = left + rect.width * viewport.zoom;
  const bottom = top + rect.height * viewport.zoom;

  return (
    left >= margin &&
    top >= margin &&
    right <= viewport.width - margin &&
    bottom <= viewport.height - margin
  );
}
