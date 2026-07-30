/**
 * A machine's states, read as a tree.
 *
 * `Parent/Child` names are the parser's hierarchical (composite) states: the
 * family is a container and its leaves are the real states. Two clients need
 * exactly the same reading — the canvas nests them as group nodes, the sidebar
 * lists them as an outline — so the detection lives here once instead of being
 * re-derived by each of them.
 *
 * A composite parent is never a state of its own: the parser fans wildcard
 * patterns out over the children. A machine that nonetheless declares a plain
 * state with a parent's name keeps that family flat, rather than nesting a
 * state inside a state.
 *
 * Every label here is built from names the analyzed application declares —
 * data, never translated.
 */

import type { DomainMachine, DomainState } from './types';

/** A state as it appears at its own level of the tree. */
export interface TreeLeaf {
  state: DomainState;
  /**
   * What to show at this level: the leaf name inside a family (`Loading` for
   * `Active/Loading`), the whole name otherwise. Separators kept in a name are
   * spaced out (`A / B`).
   */
  label: string;
}

/** A top-level entry: a plain state, or a composite family with its leaves. */
export type TreeEntry =
  | ({ kind: 'state' } & TreeLeaf)
  | { kind: 'family'; name: string; children: TreeLeaf[] };

/** Where a state sits, by state id. */
export interface StatePlacement {
  label: string;
  /** Composite parent name, absent for a state at the machine's own level. */
  family?: string;
}

export interface MachineTree {
  /** Top-level entries, in declaration order. */
  entries: TreeEntry[];
  /** Placement of every state of the machine, by id. */
  placement: Map<string, StatePlacement>;
}

export function machineTree(machine: DomainMachine): MachineTree {
  const plainNames = new Set(
    machine.states.filter((state) => !state.name.includes('/')).map((state) => state.name),
  );

  const entries: TreeEntry[] = [];
  const families = new Map<string, Extract<TreeEntry, { kind: 'family' }>>();
  const placement = new Map<string, StatePlacement>();

  for (const state of machine.states) {
    const parent = state.name.split('/', 1)[0];
    const nested = state.name.includes('/') && !plainNames.has(parent);
    const label = spaced(nested ? state.name.slice(parent.length + 1) : state.name);
    placement.set(state.id, { label, family: nested ? parent : undefined });

    if (!nested) {
      entries.push({ kind: 'state', state, label });
      continue;
    }
    let family = families.get(parent);
    if (!family) {
      // A family takes the position of its first leaf, so the outline and the
      // canvas both follow declaration order.
      family = { kind: 'family', name: parent, children: [] };
      families.set(parent, family);
      entries.push(family);
    }
    family.children.push({ state, label });
  }

  return { entries, placement };
}

/** The composite families of a tree, in declaration order. */
export function families(tree: MachineTree): Extract<TreeEntry, { kind: 'family' }>[] {
  return tree.entries.filter((entry): entry is Extract<TreeEntry, { kind: 'family' }> =>
    entry.kind === 'family',
  );
}

/** id of the container node/row of a composite family. */
export function familyId(machineId: string, family: string): string {
  return `${machineId}/${family}`;
}

function spaced(name: string): string {
  return name.replace(/\//g, ' / ');
}
