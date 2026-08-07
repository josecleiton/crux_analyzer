/**
 * Domain Model → React Flow Model mapper.
 * The only layer that knows React Flow's node/edge types.
 *
 * Each machine of the core becomes a section (group node) containing its
 * state nodes — unless the core has a single machine, which renders flat.
 * Geometry (positions and routes) is computed later by the LayoutEngine;
 * this layer provides the node dimensions the engine needs.
 *
 * Reader-hidden states (`domain/visibility.ts`) are dropped here rather than
 * further down: what the reader deselected never reaches the layout engine, so
 * the remaining graph is laid out as if those states did not exist instead of
 * keeping a gap where they were.
 */

import type { Edge, Node } from '@xyflow/react';
import type { DomainCore, DomainMachine, DomainTransition, DomainEffect } from '../domain/types';
import { wildcardStateId } from '../domain/types';
import { families, familyId, machineTree } from '../domain/hierarchy';
import { NOTHING_HIDDEN } from '../domain/visibility';
import { entryState, stateRole } from '../domain/stateRole';

export interface FlowModel {
  nodes: Node[];
  edges: Edge[];
}

/**
 * What a `state` node needs to render itself.
 *
 * `doc` is the analyzed app's own prose — data, not chrome, so it does not go
 * through `FlowLabels` and the width allowance it earns is the same in every
 * locale. `tags` deliberately stay out: they belong to the panels, and putting
 * them in a node would make its geometry depend on how many an author wrote.
 */
export interface StateNodeData extends Record<string, unknown> {
  label: string;
  initial: boolean;
  failure: boolean;
  deprecated: boolean;
  final: boolean;
  doc?: string;
}

/**
 * Chrome this layer has to render but must not author.
 *
 * Localization is a presentation concern: the mapper receives already-
 * translated text from the component boundary instead of importing the message
 * catalog, which keeps this layer (and the domain below it) language-free.
 * It matters for geometry too — node widths are estimated from the label, so
 * the *translated* string is the one that has to be measured.
 */
export interface FlowLabels {
  /** Label of the wildcard pseudo-node. */
  anyState: string;
}

const NODE_HEIGHT = 44;
const NODE_MIN_WIDTH = 110;
const NODE_PADDING_X = 44;
/** Average glyph width of the node label font (14px system-ui). */
const NODE_CHAR_WIDTH = 7.6;
/** Extra room for the initial-state dot rendered before the label. */
const INITIAL_MARKER_WIDTH = 14;
/**
 * Extra room for the documentation mark rendered after the label: a 10px
 * shape plus its 7px gap. A shape and not a glyph, so this is the same in
 * every locale.
 */
export const DOC_MARK_WIDTH = 17;

export function toFlowModel(
  core: DomainCore,
  labels: FlowLabels,
  hidden: ReadonlySet<string> = NOTHING_HIDDEN,
  showEffects: boolean = false,
): FlowModel {
  // Sections come from what the core *declares*, not from what survives the
  // reader's filter: hiding one machine entirely must not re-flatten another.
  const grouped = core.machines.length > 1;
  const nodes: Node[] = [];
  const edges: Edge[] = [];

  for (const machine of core.machines) {
    // A machine with no state left leaves the canvas altogether — including its
    // wildcards. "Any state" stands for the machine's visible states, so with
    // none of them on screen it stands for nothing: a `* → *` transition would
    // otherwise keep a section alive with an edge between two pseudo-nodes.
    if (machine.states.every((state) => hidden.has(state.id))) continue;

    const machineEdgeList = machineEdges(machine, hidden, showEffects);
    const wildcard = wildcardStateId(machine.id);
    // The pseudo-node exists to carry wildcard edges; with none left to draw
    // (their real endpoints hidden) it has nothing to say.
    const wildcardUsed =
      machine.hasWildcard &&
      machineEdgeList.some((edge) => edge.source === wildcard || edge.target === wildcard);
    const stateNodes = machineNodes(
      machine,
      grouped ? machine.id : undefined,
      labels,
      hidden,
      wildcardUsed,
    );
    if (stateNodes.length === 0) continue;

    if (grouped) {
      nodes.push({
        id: machine.id,
        type: 'machineGroup',
        // clicking the section stands for clicking its entry state
        data: {
          label: machine.name,
          entryStateId: entryState(machine)?.id,
          doc: machine.doc,
        },
        position: { x: 0, y: 0 },
      });
    }
    nodes.push(...stateNodes);
    edges.push(...machineEdgeList);
  }

  return { nodes, edges };
}

function machineNodes(
  machine: DomainMachine,
  parentId: string | undefined,
  labels: FlowLabels,
  hidden: ReadonlySet<string>,
  wildcardUsed: boolean,
): Node[] {
  const base = { parentId, position: { x: 0, y: 0 } };

  // Composite parents ("Active" in "Active/Loading") become containers, the
  // same nesting Mermaid renders — and the same reading the sidebar outline
  // gives them, which is why it is `domain/hierarchy.ts` that decides it.
  const tree = machineTree(machine);

  // containers first: React Flow requires a parent before its children
  const nodes: Node[] = families(tree)
    // a family whose every leaf is hidden has nothing left to contain
    .filter((family) => family.children.some((leaf) => !hidden.has(leaf.state.id)))
    .map((family) => ({
      ...base,
      id: familyId(machine.id, family.name),
      type: 'compositeGroup',
      data: { label: family.name },
    }));

  for (const state of machine.states) {
    if (hidden.has(state.id)) continue;
    // Inside a container the parent's name is the container's title, so the
    // node keeps only the leaf; spaced separators either way ("A / B").
    const { label, family } = tree.placement.get(state.id)!;
    const role = stateRole(machine, state);
    const data: StateNodeData = {
      label,
      initial: role.initial,
      failure: role.failure,
      deprecated: role.deprecated,
      final: role.final,
      doc: state.doc,
    };
    nodes.push({
      ...base,
      ...(family ? { parentId: familyId(machine.id, family) } : {}),
      id: state.id,
      type: 'state',
      data,
      // the initial marker (a dot before the label) and the documentation mark
      // (a shape after it) each need their own room
      width:
        nodeWidth(label) +
        (role.initial ? INITIAL_MARKER_WIDTH : 0) +
        (state.doc ? DOC_MARK_WIDTH : 0),
      height: NODE_HEIGHT,
    });
  }

  if (wildcardUsed) {
    nodes.push({
      ...base,
      id: wildcardStateId(machine.id),
      type: 'anyState',
      data: { label: labels.anyState },
      width: nodeWidth(labels.anyState),
      height: 36,
    });
  }

  return nodes;
}

/**
 * A transition needs both of its ends drawn. The wildcard pseudo-state is never
 * hidden — "any state" is not a state the reader can deselect — so only the real
 * endpoint of a wildcard edge decides whether it survives.
 */
function machineEdges(machine: DomainMachine, hidden: ReadonlySet<string>, showEffects: boolean): Edge[] {
  return machine.transitions
    .filter((transition) => !hidden.has(transition.from) && !hidden.has(transition.to))
    .map((transition) => ({
      id: transition.id,
      type: 'routed',
      source: transition.from,
      target: transition.to,
      label: formatEdgeLabel(transition, showEffects),
      data: { event: transition.event, effects: showEffects ? transition.effects : [] },
      // The arrowhead color is theme-dependent and applied by the renderer:
      // SVG marker attributes cannot read CSS variables.
      markerEnd: {
        type: 'arrowclosed' as const,
        width: 14,
        height: 14,
      },
    }));
}

function nodeWidth(label: string): number {
  return Math.max(NODE_MIN_WIDTH, Math.round(label.length * NODE_CHAR_WIDTH) + NODE_PADDING_X);
}

function formatEdgeLabel(transition: DomainTransition, showEffects: boolean): string {
  if (!showEffects || transition.effects.length === 0) {
    return transition.event;
  }
  const effectsStr = transition.effects
    .map((e: DomainEffect) => (e.conditional ? `${e.name}?` : e.name))
    .join(', ');
  return `${transition.event} / ${effectsStr}`;
}
