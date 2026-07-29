import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson } from './fromParserJson';

const project = fromParserJson(parseProjectJson(rawProject));

describe('fromParserJson', () => {
  it('maps project and cores', () => {
    expect(project.name).toBe('Audio Recorder');
    expect(project.cores.map((c) => c.name)).toEqual(['Recorder', 'Authentication', 'Sync']);
  });

  it('generates unique, core-scoped ids', () => {
    const recorder = project.cores[0];
    const sync = project.cores[2];

    // "Idle" exists in both Recorder and Sync — the ids must not collide
    const recorderIdle = recorder.states.find((s) => s.name === 'Idle')!;
    const syncIdle = sync.states.find((s) => s.name === 'Idle')!;
    expect(recorderIdle.id).not.toBe(syncIdle.id);

    const allIds = project.cores.flatMap((c) => [
      ...c.states.map((s) => s.id),
      ...c.transitions.map((t) => t.id),
    ]);
    expect(new Set(allIds).size).toBe(allIds.length);
  });

  it('derives incoming/outgoing for each state', () => {
    const recorder = project.cores[0];
    const recording = recorder.states.find((s) => s.name === 'Recording')!;

    expect(recording.incoming.map((t) => t.event).sort()).toEqual([
      'RecordPressed',
      'ResumePressed',
    ]);
    expect(recording.outgoing.map((t) => t.event).sort()).toEqual([
      'PausePressed',
      'StopPressed',
    ]);

    const idle = recorder.states.find((s) => s.name === 'Idle')!;
    expect(idle.incoming).toHaveLength(0);
    expect(idle.outgoing.map((t) => t.event)).toEqual(['RecordPressed']);
  });

  it('transitions reference existing state ids and keep readable names', () => {
    for (const core of project.cores) {
      const stateIds = new Set(core.states.map((s) => s.id));
      for (const t of core.transitions) {
        expect(stateIds.has(t.from)).toBe(true);
        expect(stateIds.has(t.to)).toBe(true);
      }
    }
    const first = project.cores[0].transitions[0];
    expect(first.fromName).toBe('Idle');
    expect(first.event).toBe('RecordPressed');
    expect(first.toName).toBe('Recording');
  });
});

describe('parseProjectJson', () => {
  it('rejects JSON outside the contract', () => {
    expect(() => parseProjectJson({ cores: [] })).toThrow(/project/);
    expect(() =>
      parseProjectJson({
        project: 'X',
        cores: [{ name: 'A', states: ['S'], transitions: [{ from: 'S' }] }],
      }),
    ).toThrow(/transition/);
  });
});
