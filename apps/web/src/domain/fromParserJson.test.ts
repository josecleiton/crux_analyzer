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
    // The contract's `resolvesWith` becomes this client's `answers`, and both
    // authored forms of an effect arrive as one record.
    expect(recorder.transitions[0].effects).toEqual([
      {
        name: 'AudioOperation::Start',
        capability: 'Audio',
        answers: ['RecordingStarted'],
        conditional: false,
      },
    ]);
    expect(recorder.transitions[1].effects).toEqual([
      {
        name: 'AudioOperation::Pause',
        capability: undefined,
        answers: [],
        conditional: false,
      },
    ]);
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

  it('carries the documentation authored on a state', () => {
    const auth = project.cores[1].machines[0];
    const failed = auth.states.find((s) => s.name === 'Failed')!;
    expect(failed.doc).toMatch(/refused by the server/);
    expect(failed.markers).toEqual(['failure']);
    expect(failed.tags).toEqual(['retryable']);
  });

  it('leaves an undocumented state without metadata', () => {
    const recording = project.cores[0].machines[0].states.find((s) => s.name === 'Recording')!;
    expect(recording.doc).toBeUndefined();
    expect(recording.markers).toEqual([]);
    expect(recording.tags).toEqual([]);
  });

  it('carries the description authored on the state enum', () => {
    expect(project.cores[2].machines[0].doc).toMatch(/one device at a time/);
    expect(project.cores[0].machines[0].doc).toBeUndefined();
  });

  it('carries documented events and effects as per-core lookup maps', () => {
    const recorder = project.cores[0];
    expect(recorder.eventDocs).toEqual({
      RecordPressed: 'The user hit the record button on the main screen.',
    });
    expect(recorder.effectDocs['AudioOperation::Start']).toMatch(/Begins capturing/);
    // cores that document nothing get empty maps, not absent ones
    expect(project.cores[1].eventDocs).toEqual({});
  });

  it('generates the same id and graph for a state however it was authored', () => {
    // `Failed` is written as an object and `Authenticating` as a bare string in
    // the same machine: the authored form must not leak into ids or wiring.
    const auth = project.cores[1].machines[0];
    const failed = auth.states.find((s) => s.name === 'Failed')!;
    expect(failed.id).toBe(`${auth.id}/Failed`);
    expect(failed.incoming.map((t) => t.event)).toEqual(['AuthFailed']);
    expect(failed.outgoing.map((t) => t.event)).toEqual(['RetryPressed']);
  });
});
