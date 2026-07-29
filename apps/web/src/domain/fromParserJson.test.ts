import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson, machineOf } from './fromParserJson';

const project = fromParserJson(parseProjectJson(rawProject));

describe('fromParserJson', () => {
  it('maps project, cores and machines', () => {
    expect(project.name).toBe('Audio Recorder');
    expect(project.cores.map((c) => c.name)).toEqual(['Recorder', 'Authentication', 'Sync']);
    expect(project.cores[0].machines.map((m) => m.name)).toEqual([
      'RecorderState',
      'InputState',
    ]);
  });

  it('generates unique, machine-scoped ids', () => {
    const recorder = project.cores[0];
    const sync = project.cores[2];

    // "Idle" exists in Recorder and Sync machines — the ids must not collide
    const recorderIdle = recorder.machines[0].states.find((s) => s.name === 'Idle')!;
    const syncIdle = sync.machines[0].states.find((s) => s.name === 'Idle')!;
    expect(recorderIdle.id).not.toBe(syncIdle.id);

    const allIds = project.cores.flatMap((c) =>
      c.machines.flatMap((m) => [
        ...m.states.map((s) => s.id),
        ...m.transitions.map((t) => t.id),
      ]),
    );
    expect(new Set(allIds).size).toBe(allIds.length);
  });

  it('derives incoming/outgoing for each state', () => {
    const machine = project.cores[0].machines[0];
    const recording = machine.states.find((s) => s.name === 'Recording')!;

    expect(recording.incoming.map((t) => t.event).sort()).toEqual([
      'RecordPressed',
      'ResumePressed',
    ]);
    expect(recording.outgoing.map((t) => t.event).sort()).toEqual([
      'PausePressed',
      'StopPressed',
    ]);
  });

  it('maps effects and wildcard sources', () => {
    const recorder = project.cores[0].machines[0];
    expect(recorder.transitions[0].effects).toEqual(['AudioOperation::Start']);
    expect(recorder.hasWildcard).toBe(false);

    const inputs = project.cores[0].machines[1];
    expect(inputs.hasWildcard).toBe(true);
    const wildcard = inputs.transitions.find((t) => t.fromName === '*')!;
    expect(wildcard.from).toBe(`${inputs.id}/*`);
    expect(wildcard.effects).toEqual([]);
    // wildcard transitions are not listed as any state's outgoing
    for (const state of inputs.states) {
      expect(state.outgoing).not.toContain(wildcard);
    }
  });

  it('machineOf resolves the owner of a state or transition id', () => {
    const core = project.cores[0];
    const inputs = core.machines[1];
    expect(machineOf(core, inputs.states[0].id)?.name).toBe('InputState');
    expect(machineOf(core, inputs.transitions[0].id)?.name).toBe('InputState');
    expect(machineOf(core, 'nonsense')).toBeNull();
  });
});

describe('parseProjectJson', () => {
  it('rejects JSON outside the contract', () => {
    expect(() => parseProjectJson({ cores: [] })).toThrow(/project/);
    expect(() =>
      parseProjectJson({
        project: 'X',
        cores: [{ name: 'A', machines: [{ name: 'M', states: ['S'], transitions: [{ from: 'S' }] }] }],
      }),
    ).toThrow(/transition/);
    expect(() =>
      parseProjectJson({
        project: 'X',
        cores: [{ name: 'A', states: [], transitions: [] }],
      }),
    ).toThrow(/machines/);
  });
});
