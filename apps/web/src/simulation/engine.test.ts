import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson } from '../domain/fromParserJson';
import {
  availableTransitions,
  fire,
  goToStep,
  lastStep,
  pendingAnswers,
  recordedRun,
  startSimulation,
  traveledPath,
  unreplayableTransitions,
} from './engine';

const project = fromParserJson(parseProjectJson(rawProject));
const recorder = project.cores[0].machines[0]; // RecorderState
const inputs = project.cores[0].machines[1]; // InputState (has wildcard)

describe('standing at another point of the run', () => {
  const fireEvent = (
    machine: typeof recorder,
    sim: ReturnType<typeof startSimulation>,
    event: string,
  ) => fire(machine, sim, availableTransitions(machine, sim).find((t) => t.event === event)!.id);

  /** Idle → Recording → Paused → Recording. */
  const threeSteps = () => {
    let sim = startSimulation(recorder);
    for (const event of ['RecordPressed', 'PausePressed', 'ResumePressed']) {
      sim = fireEvent(recorder, sim, event);
    }
    return sim;
  };

  it('goes back without throwing away what was done', () => {
    const back = goToStep(recorder, threeSteps(), 1);

    // Standing after step 1, with the rest recorded and not taken.
    expect(back.trail.map((s) => s.event)).toEqual(['RecordPressed']);
    expect(back.ahead.map((s) => s.event)).toEqual(['PausePressed', 'ResumePressed']);
    expect(recordedRun(back).map((s) => s.event)).toEqual([
      'RecordPressed',
      'PausePressed',
      'ResumePressed',
    ]);
    expect(back.currentStateId).toBe(recorder.states.find((s) => s.name === 'Recording')!.id);
  });

  it('rebuilds everything that hangs off the position', () => {
    // Rewinding past the request that was waiting un-waits it, and the traveled
    // path shrinks with it — both derived, neither stored.
    const back = goToStep(recorder, threeSteps(), 0);
    expect(back.inFlight).toEqual([]);
    expect(back.trail).toEqual([]);
    expect(traveledPath(recorder, back).transitionIds).toEqual([]);
    expect(lastStep(back)).toBeNull();

    const one = goToStep(recorder, threeSteps(), 1);
    expect(one.inFlight.map((p) => p.name)).toEqual(['AudioOperation::Start']);
    expect(traveledPath(recorder, one).transitionIds).toHaveLength(1);
  });

  it('walks back into the recorded steps when the same event fires', () => {
    const back = goToStep(recorder, threeSteps(), 1);
    const forward = fireEvent(recorder, back, 'PausePressed');

    expect(forward.trail.map((s) => s.event)).toEqual(['RecordPressed', 'PausePressed']);
    expect(forward.ahead.map((s) => s.event)).toEqual(['ResumePressed']);
  });

  it('replaces the recorded steps when a different move is made', () => {
    // From Recording, stopping instead of pausing is a different run.
    const back = goToStep(recorder, threeSteps(), 1);
    const diverged = fireEvent(recorder, back, 'StopPressed');

    expect(diverged.trail.map((s) => s.event)).toEqual(['RecordPressed', 'StopPressed']);
    expect(diverged.ahead).toEqual([]);
  });

  it('goes forward again through the trail it kept', () => {
    const run = threeSteps();
    const back = goToStep(recorder, run, 1);
    expect(goToStep(recorder, back, 3)).toEqual(run);
  });

  it('clamps out-of-range positions instead of breaking', () => {
    const run = threeSteps();
    expect(goToStep(recorder, run, -5).trail).toEqual([]);
    expect(goToStep(recorder, run, 99)).toEqual(run);
  });
});

describe('effects in flight', () => {
  const fireEvent = (machine: typeof recorder, sim: ReturnType<typeof startSimulation>, event: string) =>
    fire(machine, sim, availableTransitions(machine, sim).find((t) => t.event === event)!.id);

  it('records what each step asked the shell to do', () => {
    const sim = fireEvent(recorder, startSimulation(recorder), 'RecordPressed');
    expect(lastStep(sim)!.effects.map((e) => e.name)).toEqual(['AudioOperation::Start']);
  });

  it('keeps a request waiting until an event answers it', () => {
    // RecordPressed requests Start, answered by RecordingStarted — an event no
    // transition of this machine carries, so it stays waiting.
    let sim = fireEvent(recorder, startSimulation(recorder), 'RecordPressed');
    expect(sim.inFlight.map((p) => p.name)).toEqual(['AudioOperation::Start']);
    expect(sim.inFlight[0].step).toBe(1);

    // Pausing and resuming are fire-and-forget: nothing new waits, and Start
    // still does.
    sim = fireEvent(recorder, sim, 'PausePressed');
    sim = fireEvent(recorder, sim, 'ResumePressed');
    expect(sim.inFlight.map((p) => p.name)).toEqual(['AudioOperation::Start']);

    // Stopping requests the upload, which UploadFinished answers.
    sim = fireEvent(recorder, sim, 'StopPressed');
    expect(sim.inFlight.map((p) => p.name)).toEqual([
      'AudioOperation::Start',
      'HttpOperation::Upload',
    ]);
    sim = fireEvent(recorder, sim, 'UploadFinished');
    expect(sim.inFlight.map((p) => p.name)).toEqual(['AudioOperation::Start']);
  });

  it('says which answers a transition handles and which are inert', () => {
    const sim = fireEvent(recorder, startSimulation(recorder), 'RecordPressed');
    // The shell can answer with RecordingStarted; no transition from Recording
    // carries it, so the replay reports it instead of offering it.
    expect(pendingAnswers(recorder, sim)).toEqual([
      { event: 'RecordingStarted', effect: 'AudioOperation::Start', transitionId: null },
    ]);

    // In the input machine the answer *is* a transition from here.
    const switching = fireEvent(inputs, startSimulation(inputs), 'InputSelected');
    const answers = pendingAnswers(inputs, switching);
    const handled = answers.find((a) => a.event === 'InputSwitched')!;
    expect(handled.transitionId).toBe(
      availableTransitions(inputs, switching).find((t) => t.event === 'InputSwitched')!.id,
    );
    expect(answers.find((a) => a.event === 'InputSwitchFailed')!.transitionId).toBeNull();
  });

  it('starts and restarts with nothing in flight', () => {
    expect(startSimulation(recorder).inFlight).toEqual([]);
  });
});

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
