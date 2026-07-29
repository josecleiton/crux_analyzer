import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson } from '../domain/fromParserJson';
import { toFlowModel } from './toFlowModel';

const recorder = fromParserJson(parseProjectJson(rawProject)).cores[0];
const { nodes, edges } = toFlowModel(recorder);

describe('toFlowModel', () => {
  it('turns each state into a node labeled with its name', () => {
    expect(nodes).toHaveLength(recorder.states.length);
    expect(nodes.map((n) => n.data.label)).toEqual([
      'Idle',
      'Recording',
      'Paused',
      'Uploading',
      'Completed',
    ]);
  });

  it('turns each transition into an edge labeled with its event', () => {
    expect(edges).toHaveLength(recorder.transitions.length);
    const nodeIds = new Set(nodes.map((n) => n.id));
    for (const edge of edges) {
      expect(nodeIds.has(edge.source)).toBe(true);
      expect(nodeIds.has(edge.target)).toBe(true);
    }
    const record = edges.find((e) => e.label === 'RecordPressed')!;
    expect(record.source).toContain('Idle');
    expect(record.target).toContain('Recording');
  });

  it('edge ids match the domain transition ids', () => {
    expect(edges.map((e) => e.id)).toEqual(recorder.transitions.map((t) => t.id));
  });
});
