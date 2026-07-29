/**
 * Current UI selection — a state, a transition, or nothing.
 * Kept outside the components so the future Simulation Engine can drive
 * selection/highlighting without touching the Graph.
 */

export type Selection =
  | { kind: 'state'; id: string }
  | { kind: 'transition'; id: string }
  | null;
