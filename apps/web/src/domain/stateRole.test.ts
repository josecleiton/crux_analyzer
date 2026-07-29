import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson } from './fromParserJson';
import { isFailureName, stateRole } from './stateRole';
import type { DomainMachine } from './types';

const project = fromParserJson(parseProjectJson(rawProject));
const recorderMachine = project.cores[0].machines[0]; // RecorderState
const inputMachine = project.cores[0].machines[1]; // InputState (wildcard source)
const authMachine = project.cores[1].machines[0]; // AuthState (has "Failed")

function role(machine: DomainMachine, name: string) {
  const state = machine.states.find((s) => s.name === name)!;
  expect(state, name).toBeDefined();
  return stateRole(machine, state);
}

describe('isFailureName', () => {
  it('reads failure words in any casing style', () => {
    for (const name of [
      'Failed',
      'RecordingFailure',
      'upload_error',
      'Active/AuthDenied',
      'RequestRejected',
      'TimedOut',
      'Timeout',
      'InvalidInput',
    ]) {
      expect(isFailureName(name), name).toBe(true);
    }
  });

  it('does not read healthy states as failures', () => {
    for (const name of ['Idle', 'Recording', 'Completed', 'Retrying', 'Available']) {
      expect(isFailureName(name), name).toBe(false);
    }
  });
});

describe('stateRole', () => {
  it('marks the entry point as initial and the dead end as final', () => {
    expect(role(recorderMachine, 'Idle')).toEqual({
      initial: true, // nothing transitions into Idle
      failure: false,
      final: false,
    });
    expect(role(recorderMachine, 'Completed')).toEqual({
      initial: false,
      failure: false,
      final: true, // no outgoing transition
    });
    expect(role(recorderMachine, 'Recording')).toEqual({
      initial: false,
      failure: false,
      final: false,
    });
  });

  it('falls back to the first state when every state has an incoming transition', () => {
    // AuthState is fully cyclic: SignedOut is both a target and the first state
    expect(authMachine.states.every((s) => s.incoming.length > 0)).toBe(true);
    expect(role(authMachine, 'SignedOut').initial).toBe(true);
    expect(role(authMachine, 'Authenticating').initial).toBe(false);
  });

  it('marks failure states independently of being final', () => {
    // "Failed" has a RetryPressed way out: a failure, but not a final
    expect(role(authMachine, 'Failed')).toEqual({
      initial: false,
      failure: true,
      final: false,
    });
  });

  it('keeps a dead end final even when a wildcard transition could leave it', () => {
    // InputState has "* -> Ready"; Switching still has its own way out
    expect(role(inputMachine, 'Switching').final).toBe(false);
    // both states are transitioned into (Ready also from the wildcard), so
    // the first state carries the initial role
    expect(role(inputMachine, 'Ready').initial).toBe(true);
    expect(role(inputMachine, 'Switching').initial).toBe(false);
  });
});
