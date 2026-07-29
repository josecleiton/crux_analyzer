import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson } from '../domain/fromParserJson';
import { availableTransitions, fire, lastStep, startSimulation } from './engine';

const project = fromParserJson(parseProjectJson(rawProject));
const recorder = project.cores[0].machines[0]; // RecorderState
const inputs = project.cores[0].machines[1]; // InputState (has wildcard)

describe('simulation engine', () => {
  it('starts at the first state by default, or at a chosen state', () => {
    expect(startSimulation(recorder).currentStateId).toBe(recorder.states[0].id);
    const paused = recorder.states.find((s) => s.name === 'Paused')!;
    expect(startSimulation(recorder, paused.id).currentStateId).toBe(paused.id);
  });

  it('offers the outgoing transitions of the current state', () => {
    const sim = startSimulation(recorder); // Idle
    expect(availableTransitions(recorder, sim).map((t) => t.event)).toEqual(['RecordPressed']);
  });

  it('always offers wildcard-sourced transitions', () => {
    const sim = startSimulation(inputs); // Ready
    const events = availableTransitions(inputs, sim).map((t) => t.event);
    expect(events).toContain('InputSelected'); // outgoing of Ready
    expect(events).toContain('InputsInvalidated'); // wildcard
  });

  it('fires transitions, moves state and records the trail', () => {
    let sim = startSimulation(recorder); // Idle
    const record = availableTransitions(recorder, sim)[0];
    sim = fire(recorder, sim, record.id); // → Recording

    const recording = recorder.states.find((s) => s.name === 'Recording')!;
    expect(sim.currentStateId).toBe(recording.id);
    expect(sim.trail).toHaveLength(1);
    expect(lastStep(sim)).toMatchObject({ event: 'RecordPressed', toName: 'Recording' });

    const stop = availableTransitions(recorder, sim).find((t) => t.event === 'StopPressed')!;
    sim = fire(recorder, sim, stop.id); // → Uploading
    expect(sim.trail.map((s) => s.event)).toEqual(['RecordPressed', 'StopPressed']);
  });

  it('ignores transitions that cannot fire from the current state', () => {
    const sim = startSimulation(recorder); // Idle
    const pause = recorder.transitions.find((t) => t.event === 'PausePressed')!;
    expect(fire(recorder, sim, pause.id)).toBe(sim);
  });
});
