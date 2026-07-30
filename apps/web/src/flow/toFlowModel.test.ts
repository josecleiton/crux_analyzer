import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson } from '../domain/fromParserJson';
import { DOC_MARK_WIDTH, toFlowModel } from './toFlowModel';

// The mapper receives already-translated chrome; these tests pin the English
// label so a locale change cannot silently alter node geometry expectations.
const labels = { anyState: 'any state' };

const project = fromParserJson(parseProjectJson(rawProject));
const recorderCore = project.cores[0]; // two machines
const syncCore = project.cores[2]; // one machine

/** A single-machine core with a composite family, the shape the parser emits. */
function compositeCore(states: string[] = ['Idle', 'Active/Loading', 'Active/Ready']) {
  return fromParserJson(
    parseProjectJson({
      project: 'Composite',
      cores: [
        {
          name: 'C',
          machines: [
            {
              name: 'State',
              states,
              transitions: [
                { from: 'Idle', event: 'Start', to: 'Active/Loading' },
                { from: 'Active/Loading', event: 'Loaded', to: 'Active/Ready' },
                { from: 'Active/Ready', event: 'Stop', to: 'Idle' },
              ],
            },
          ],
        },
      ],
    }),
  ).cores[0];
}

describe('toFlowModel', () => {
  it('renders a multi-machine core as sections (group nodes)', () => {
    const { nodes } = toFlowModel(recorderCore, labels);
    const groups = nodes.filter((n) => n.type === 'machineGroup');
    expect(groups.map((g) => g.data.label)).toEqual(['RecorderState', 'InputState']);

    // every state node belongs to its machine's group
    for (const machine of recorderCore.machines) {
      for (const state of machine.states) {
        const node = nodes.find((n) => n.id === state.id)!;
        expect(node.parentId).toBe(machine.id);
      }
    }
  });

  it('points each section at the entry state of its machine', () => {
    const { nodes } = toFlowModel(recorderCore, labels);
    const groups = nodes.filter((n) => n.type === 'machineGroup');
    const entryNames = groups.map(
      (group) => nodes.find((n) => n.id === group.data.entryStateId)!.data.label,
    );
    expect(entryNames).toEqual(['Idle', 'Ready']);
  });

  it('renders a single-machine core flat (no groups)', () => {
    const { nodes } = toFlowModel(syncCore, labels);
    expect(nodes.every((n) => n.type !== 'machineGroup')).toBe(true);
    expect(nodes.every((n) => n.parentId === undefined)).toBe(true);
    expect(nodes.map((n) => n.data.label)).toEqual(['Idle', 'Syncing', 'Conflict', 'Done']);
  });

  it('adds a pseudo-node for wildcard sources', () => {
    const { nodes, edges } = toFlowModel(recorderCore, labels);
    const anyNode = nodes.find((n) => n.type === 'anyState')!;
    expect(anyNode).toBeDefined();
    expect(anyNode.parentId).toBe(recorderCore.machines[1].id);
    // the wildcard transition edge starts at the pseudo-node
    const wildcardEdge = edges.find((e) => e.source === anyNode.id)!;
    expect(wildcardEdge.label).toBe('InputsInvalidated');
  });

  it('labels the wildcard pseudo-node from the injected chrome, and sizes it to match', () => {
    const translated = { anyState: 'qualquer estado' };
    const english = toFlowModel(recorderCore, labels).nodes.find((n) => n.type === 'anyState')!;
    const ptBR = toFlowModel(recorderCore, translated).nodes.find((n) => n.type === 'anyState')!;

    expect(english.data.label).toBe('any state');
    expect(ptBR.data.label).toBe('qualquer estado');
    // The longer label must widen the node: the width estimate has to be
    // derived from the translated string, not from a hardcoded English one.
    expect(ptBR.width!).toBeGreaterThan(english.width!);
  });

  it('carries a state description and the deprecated role into node data', () => {
    const { nodes } = toFlowModel(syncCore, labels);
    const conflict = nodes.find((n) => n.data.label === 'Conflict')!;
    expect(conflict.data.doc).toMatch(/someone has to choose/);
    expect(nodes.find((n) => n.data.label === 'Done')!.data.deprecated).toBe(true);
    expect(conflict.data.deprecated).toBe(false);
    expect(nodes.find((n) => n.data.label === 'Idle')!.data.doc).toBeUndefined();
  });

  it('keeps tags out of the flow model', () => {
    // Tags are inspector-only: a node's geometry must not depend on how many
    // an author wrote.
    const { nodes } = toFlowModel(project.cores[1], labels);
    const failed = nodes.find((n) => n.data.label === 'Failed')!;
    expect(failed.data.tags).toBeUndefined();
  });

  it('gives a documented state room for its mark', () => {
    const machine = syncCore.machines[0];
    const undocumented = {
      ...syncCore,
      machines: [{ ...machine, states: machine.states.map((s) => ({ ...s, doc: undefined })) }],
    };
    const withDoc = toFlowModel(syncCore, labels).nodes.find((n) => n.data.label === 'Conflict')!;
    const without = toFlowModel(undocumented, labels).nodes.find(
      (n) => n.data.label === 'Conflict',
    )!;
    expect(withDoc.width! - without.width!).toBe(DOC_MARK_WIDTH);
  });

  it('sizes a documented node identically in every locale', () => {
    // The mark is a CSS shape and the description is untranslated data, so
    // this feature adds no locale-dependent geometry.
    const english = toFlowModel(syncCore, labels).nodes.find((n) => n.data.label === 'Conflict')!;
    const ptBR = toFlowModel(syncCore, { anyState: 'qualquer estado' }).nodes.find(
      (n) => n.data.label === 'Conflict',
    )!;
    expect(ptBR.width).toBe(english.width);
  });

  it('carries the machine description onto its section node', () => {
    const { nodes } = toFlowModel(recorderCore, labels);
    const groups = nodes.filter((n) => n.type === 'machineGroup');
    // Neither Recorder machine is documented in the example.
    expect(groups.every((g) => g.data.doc === undefined)).toBe(true);

    const documented = toFlowModel(
      { ...recorderCore, machines: [{ ...recorderCore.machines[0], doc: 'A region.' }, recorderCore.machines[1]] },
      labels,
    ).nodes.filter((n) => n.type === 'machineGroup');
    expect(documented[0].data.doc).toBe('A region.');
  });

  it('nests composite leaves inside a container named after their parent', () => {
    const composite = compositeCore();
    const { nodes } = toFlowModel(composite, labels);

    const container = nodes.find((n) => n.type === 'compositeGroup')!;
    expect(container.data.label).toBe('Active');
    // containers come before their children (a React Flow requirement)
    expect(nodes.indexOf(container)).toBeLessThan(
      nodes.findIndex((n) => n.parentId === container.id),
    );

    const loading = nodes.find((n) => n.id.endsWith('Active/Loading'))!;
    const ready = nodes.find((n) => n.id.endsWith('Active/Ready'))!;
    expect(loading.parentId).toBe(container.id);
    expect(ready.parentId).toBe(container.id);
    // inside the container the parent's name is the title, not the label
    expect(loading.data.label).toBe('Loading');
    // flat states stay at the machine level
    expect(nodes.find((n) => n.data.label === 'Idle')!.parentId).toBeUndefined();
  });

  it('keeps a family flat when a plain state collides with the parent name', () => {
    const collided = compositeCore(['Idle', 'Active', 'Active/Loading']);
    const { nodes } = toFlowModel(collided, labels);
    expect(nodes.every((n) => n.type !== 'compositeGroup')).toBe(true);
    // the composite leaf keeps its full, spaced name
    expect(nodes.map((n) => n.data.label)).toContain('Active / Loading');
  });

  it('leaves out hidden states and the transitions that touch them', () => {
    const machine = syncCore.machines[0];
    const syncing = machine.states.find((s) => s.name === 'Syncing')!;
    const { nodes, edges } = toFlowModel(syncCore, labels, new Set([syncing.id]));

    expect(nodes.some((n) => n.id === syncing.id)).toBe(false);
    expect(nodes.map((n) => n.data.label)).toEqual(['Idle', 'Conflict', 'Done']);
    // an edge with a missing end cannot be drawn at all
    expect(edges.some((e) => e.source === syncing.id || e.target === syncing.id)).toBe(false);
    const nodeIds = new Set(nodes.map((n) => n.id));
    for (const edge of edges) {
      expect(nodeIds.has(edge.source)).toBe(true);
      expect(nodeIds.has(edge.target)).toBe(true);
    }
  });

  it('drops a composite container once its every leaf is hidden', () => {
    const composite = compositeCore();
    const leaves = composite.machines[0].states.filter((s) => s.name.startsWith('Active/'));

    const partly = toFlowModel(composite, labels, new Set([leaves[0].id]));
    expect(partly.nodes.some((n) => n.type === 'compositeGroup')).toBe(true);

    const whole = toFlowModel(composite, labels, new Set(leaves.map((s) => s.id)));
    // An empty container is a box around nothing.
    expect(whole.nodes.some((n) => n.type === 'compositeGroup')).toBe(false);
    expect(whole.nodes.map((n) => n.data.label)).toEqual(['Idle']);
  });

  it('drops a machine section when the whole machine is hidden, keeping the others sectioned', () => {
    const [first, second] = recorderCore.machines;
    const { nodes, edges } = toFlowModel(
      recorderCore,
      labels,
      new Set(first.states.map((s) => s.id)),
    );

    const groups = nodes.filter((n) => n.type === 'machineGroup');
    // The remaining machine keeps its section: sections come from what the core
    // declares, not from what survived the filter.
    expect(groups.map((g) => g.data.label)).toEqual([second.name]);
    expect(nodes.every((n) => !n.id.startsWith(`${first.id}/`))).toBe(true);
    expect(edges.every((e) => e.id.startsWith(`${second.id}/`))).toBe(true);
  });

  it('drops a machine whose every state is hidden, wildcards included', () => {
    // A `* → *` transition (wildcard source, runtime target) touches no real
    // state, so nothing but this rule takes it off the canvas: "any state" has
    // to stand for at least one state the reader can see.
    const runtime = fromParserJson(
      parseProjectJson({
        project: 'Runtime',
        cores: [
          {
            name: 'C',
            machines: [
              {
                name: 'State',
                states: ['Idle', 'Busy'],
                transitions: [
                  { from: 'Idle', event: 'Start', to: 'Busy' },
                  { from: '*', event: 'Restored', to: '*' },
                ],
              },
            ],
          },
        ],
      }),
    ).cores[0];

    const all = runtime.machines[0].states.map((state) => state.id);
    expect(toFlowModel(runtime, labels, new Set([all[0]])).nodes).not.toHaveLength(0);

    const { nodes, edges } = toFlowModel(runtime, labels, new Set(all));
    expect(nodes).toEqual([]);
    expect(edges).toEqual([]);
  });

  it('drops the wildcard pseudo-node when no wildcard edge is left to draw', () => {
    const machine = recorderCore.machines[1]; // the one with a wildcard source
    const wildcardTargets = machine.transitions
      .filter((t) => t.fromName === '*')
      .map((t) => t.to);

    const { nodes } = toFlowModel(recorderCore, labels, new Set(wildcardTargets));
    // "any state" only exists to carry those edges.
    expect(nodes.some((n) => n.type === 'anyState')).toBe(false);
  });

  it('turns each transition into an edge labeled with its event', () => {
    const machine = syncCore.machines[0];
    const { nodes, edges } = toFlowModel(syncCore, labels);
    expect(edges).toHaveLength(machine.transitions.length);
    const nodeIds = new Set(nodes.map((n) => n.id));
    for (const edge of edges) {
      expect(nodeIds.has(edge.source)).toBe(true);
      expect(nodeIds.has(edge.target)).toBe(true);
    }
    expect(edges.map((e) => e.id)).toEqual(machine.transitions.map((t) => t.id));
  });
});
