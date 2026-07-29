import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson } from '../domain/fromParserJson';
import { toFlowModel } from './toFlowModel';

// The mapper receives already-translated chrome; these tests pin the English
// label so a locale change cannot silently alter node geometry expectations.
const labels = { anyState: 'any state' };

const project = fromParserJson(parseProjectJson(rawProject));
const recorderCore = project.cores[0]; // two machines
const syncCore = project.cores[2]; // one machine

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
