import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson } from './fromParserJson';
import { families, machineTree } from './hierarchy';

/** A single-machine core with the given state names, as the parser emits it. */
function machineWith(states: string[]) {
  return fromParserJson(
    parseProjectJson({
      project: 'Composite',
      cores: [
        {
          name: 'C',
          machines: [
            { name: 'State', states, transitions: [{ from: states[0], event: 'Go', to: states[1] }] },
          ],
        },
      ],
    }),
  ).cores[0].machines[0];
}

const project = fromParserJson(parseProjectJson(rawProject));

describe('machineTree', () => {
  it('keeps a flat machine flat, in declaration order', () => {
    const tree = machineTree(project.cores[2].machines[0]);
    expect(tree.entries.map((entry) => (entry.kind === 'state' ? entry.label : entry.name))).toEqual(
      ['Idle', 'Syncing', 'Conflict', 'Done'],
    );
    expect(families(tree)).toHaveLength(0);
  });

  it('groups a composite family under its parent, leaves labeled by their leaf name', () => {
    const tree = machineTree(machineWith(['Idle', 'Active/Loading', 'Active/Ready']));
    expect(tree.entries.map((entry) => entry.kind)).toEqual(['state', 'family']);

    const [family] = families(tree);
    expect(family.name).toBe('Active');
    expect(family.children.map((leaf) => leaf.label)).toEqual(['Loading', 'Ready']);
    // the placement of a leaf names the family it belongs to
    expect(tree.placement.get(family.children[0].state.id)).toEqual({
      label: 'Loading',
      family: 'Active',
    });
  });

  it('takes the position of a family from its first leaf', () => {
    const tree = machineTree(machineWith(['Active/Loading', 'Idle', 'Active/Ready']));
    expect(tree.entries.map((entry) => entry.kind)).toEqual(['family', 'state']);
    expect(families(tree)[0].children.map((leaf) => leaf.label)).toEqual(['Loading', 'Ready']);
  });

  it('keeps a family flat when a plain state collides with the parent name', () => {
    // Nesting a state inside a state is not a hierarchy: the leaf keeps its
    // whole, spaced name at the machine's own level.
    const tree = machineTree(machineWith(['Idle', 'Active', 'Active/Loading']));
    expect(families(tree)).toHaveLength(0);
    expect(tree.entries.map((entry) => (entry.kind === 'state' ? entry.label : entry.name))).toEqual(
      ['Idle', 'Active', 'Active / Loading'],
    );
  });

  it('nests one level deep, keeping the rest of the path in the label', () => {
    const tree = machineTree(machineWith(['Idle', 'Active/Sub/Deep']));
    expect(families(tree)[0].children.map((leaf) => leaf.label)).toEqual(['Sub / Deep']);
  });
});
