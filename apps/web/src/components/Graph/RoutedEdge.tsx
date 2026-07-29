/**
 * Edge renderer that draws the exact route computed by the LayoutEngine
 * (orthogonal polyline with rounded corners) and places the label in the box
 * the engine reserved for it — no client-side re-routing, so edges never
 * cross nodes and labels never overlap.
 *
 * The label is SVG inside the edge group, so clicking it selects the edge.
 */

import { BaseEdge } from '@xyflow/react';
import type { EdgeProps } from '@xyflow/react';
import type { EdgeRoute, Point } from '../../layout/LayoutEngine';

const CORNER_RADIUS = 8;

export function RoutedEdge({ id, data, label, selected, markerEnd }: EdgeProps) {
  const route = data?.route as EdgeRoute | undefined;
  if (!route) return null; // geometry not computed yet (first paint)

  const path = roundedPolylinePath(route.points, CORNER_RADIUS);
  const selectedClass = selected ? ' selected' : '';

  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        markerEnd={markerEnd}
        className={`routed-edge${selectedClass}`}
      />
      {label && route.label ? (
        <g className={`edge-label${selectedClass}`}>
          <rect
            className="edge-label-box"
            x={route.label.x}
            y={route.label.y}
            width={route.label.width}
            height={route.label.height}
            rx={4}
          />
          <text
            className="edge-label-text"
            x={route.label.x + route.label.width / 2}
            y={route.label.y + route.label.height / 2}
          >
            {String(label)}
          </text>
        </g>
      ) : null}
    </>
  );
}

/** SVG path following the polyline, with corners rounded via quadratic curves. */
function roundedPolylinePath(points: Point[], radius: number): string {
  if (points.length === 0) return '';
  if (points.length === 1) return `M ${points[0].x} ${points[0].y}`;

  let path = `M ${points[0].x} ${points[0].y}`;
  for (let i = 1; i < points.length - 1; i++) {
    const previous = points[i - 1];
    const corner = points[i];
    const next = points[i + 1];

    const inLength = distance(previous, corner);
    const outLength = distance(corner, next);
    const inRadius = Math.min(radius, inLength / 2);
    const outRadius = Math.min(radius, outLength / 2);

    const entry = pointTowards(corner, previous, inRadius);
    const exit = pointTowards(corner, next, outRadius);

    path += ` L ${entry.x} ${entry.y} Q ${corner.x} ${corner.y} ${exit.x} ${exit.y}`;
  }
  const last = points[points.length - 1];
  path += ` L ${last.x} ${last.y}`;
  return path;
}

function distance(a: Point, b: Point): number {
  return Math.hypot(b.x - a.x, b.y - a.y) || 1;
}

/** The point `amount` away from `from`, in the direction of `towards`. */
function pointTowards(from: Point, towards: Point, amount: number): Point {
  const length = distance(from, towards);
  return {
    x: from.x + ((towards.x - from.x) / length) * amount,
    y: from.y + ((towards.y - from.y) / length) * amount,
  };
}
