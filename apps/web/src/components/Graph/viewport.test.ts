import { describe, expect, it } from 'vitest';
import { isRectInView, rectCenter } from './viewport';

/** A 1000×800 pane showing the canvas 1:1, origin at the top-left. */
const pane = { x: 0, y: 0, zoom: 1, width: 1000, height: 800 };

describe('rectCenter', () => {
  it('returns the middle of the rectangle', () => {
    expect(rectCenter({ x: 100, y: 50, width: 200, height: 40 })).toEqual({ x: 200, y: 70 });
  });
});

describe('isRectInView', () => {
  it('accepts a rectangle well inside the pane', () => {
    expect(isRectInView({ x: 400, y: 300, width: 120, height: 44 }, pane)).toBe(true);
  });

  it('rejects a rectangle past any edge', () => {
    expect(isRectInView({ x: 980, y: 300, width: 120, height: 44 }, pane)).toBe(false);
    expect(isRectInView({ x: -200, y: 300, width: 120, height: 44 }, pane)).toBe(false);
    expect(isRectInView({ x: 400, y: 900, width: 120, height: 44 }, pane)).toBe(false);
    expect(isRectInView({ x: 400, y: -60, width: 120, height: 44 }, pane)).toBe(false);
  });

  it('demands the margin as breathing room, not just visibility', () => {
    // 20px from the right edge: on screen, but too close to count as in view
    const hugging = { x: 860, y: 300, width: 120, height: 44 };
    expect(isRectInView(hugging, pane)).toBe(false);
    expect(isRectInView(hugging, pane, 0)).toBe(true);
  });

  it('takes the pane transform into account', () => {
    const rect = { x: 400, y: 300, width: 120, height: 44 };
    // panned far left, the same rectangle leaves the pane
    expect(isRectInView(rect, { ...pane, x: -900 })).toBe(false);
    // zoomed in, it grows past the bottom
    expect(isRectInView(rect, { ...pane, zoom: 3 })).toBe(false);
  });
});
