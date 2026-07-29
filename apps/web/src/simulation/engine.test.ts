import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson } from '../domain/fromParserJson';
import {
  availableTransitions,
  fire,
  lastStep,
  startSimulation,
  traveledPath,
  unreplayableTransitions,
} from './engine';

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

  it('separates runtime-target transitions instead of hiding them', () => {
    // A machine with a `to: "*"` transition: real behavior the replay cannot
    // follow, so it must be reported, never offered.
    const runtime = fromParserJson(
      parseProjectJson({
        project: 'R',
        cores: [
          {
            name: 'C',
            machines: [
              {
                name: 'State',
                states: ['Idle', 'Busy'],
                transitions: [
                  { from: 'Idle', event: 'Start', to: 'Busy' },
                  { from: 'Idle', event: 'Restore', to: '*' },
                ],
              },
            ],
          },
        ],
      }),
    ).cores[0].machines[0];

    const sim = startSimulation(runtime); // Idle
    expect(availableTransitions(runtime, sim).map((t) => t.event)).toEqual(['Start']);
    expect(unreplayableTransitions(runtime, sim).map((t) => t.event)).toEqual(['Restore']);
    // ...and from a state it does not leave, it is not reported either
    const busy = startSimulation(runtime, runtime.states[1].id);
    expect(unreplayableTransitions(runtime, busy)).toEqual([]);
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

  it('reports the traveled path, starting state included', () => {
    let sim = startSimulation(recorder); // Idle
    const idle = recorder.states.find((s) => s.name === 'Idle')!;
    expect(traveledPath(recorder, sim)).toEqual({ stateIds: [idle.id], transitionIds: [] });

    const record = availableTransitions(recorder, sim)[0];
    sim = fire(recorder, sim, record.id); // → Recording
    const stop = availableTransitions(recorder, sim).find((t) => t.event === 'StopPressed')!;
    sim = fire(recorder, sim, stop.id); // → Uploading

    const path = traveledPath(recorder, sim);
    expect(path.transitionIds).toEqual([record.id, stop.id]);
    expect(path.stateIds).toEqual([idle.id, record.to, stop.to]);
  });

  it('keeps the traveled path when a wildcard transition fired', () => {
    let sim = startSimulation(inputs); // Ready
    const wildcard = availableTransitions(inputs, sim).find(
      (t) => t.event === 'InputsInvalidated',
    )!;
    sim = fire(inputs, sim, wildcard.id);
    // the wildcard pseudo-state is part of the path: it is a graph node
    expect(traveledPath(inputs, sim).stateIds).toContain(wildcard.from);
  });

  it('ignores transitions that cannot fire from the current state', () => {
    const sim = startSimulation(recorder); // Idle
    const pause = recorder.transitions.find((t) => t.event === 'PausePressed')!;
    expect(fire(recorder, sim, pause.id)).toBe(sim);
  });
});
