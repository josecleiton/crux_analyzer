import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson } from './fromParserJson';
import { entryState, isFailureName, stateRole } from './stateRole';
import type { DomainMachine, DomainState } from './types';

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
      deprecated: false,
      final: false,
    });
    expect(role(recorderMachine, 'Completed')).toEqual({
      initial: false,
      failure: false,
      deprecated: false,
      final: true, // no outgoing transition
    });
    expect(role(recorderMachine, 'Recording')).toEqual({
      initial: false,
      failure: false,
      deprecated: false,
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
      deprecated: false,
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

/** A bare extra state, for the heuristic-silencing cases. */
function extraState(name: string, markers: DomainState['markers'] = []): DomainState {
  return {
    id: `x/${name}`,
    name,
    markers,
    tags: [],
    isDefault: false,
    incoming: [],
    outgoing: [],
  };
}

function withState(machine: DomainMachine, state: DomainState): DomainMachine {
  return { ...machine, states: [...machine.states, state] };
}

describe('stateRole with declared markers', () => {
  it('trusts a declared @failure', () => {
    // AuthState's "Failed" declares one in the bundled example.
    const failed = authMachine.states.find((s) => s.name === 'Failed')!;
    expect(failed.markers).toEqual(['failure']);
    expect(role(authMachine, 'Failed').failure).toBe(true);
  });

  it('marks a declared @deprecated and never infers one', () => {
    // SyncState's "Done" declares it; nothing else does.
    const sync = project.cores[2].machines[0];
    expect(role(sync, 'Done').deprecated).toBe(true);
    expect(role(sync, 'Idle').deprecated).toBe(false);
    // No name ever implies it — that would be a guess, and there is no
    // heuristic for "on its way out".
    const machine = withState(recorderMachine, extraState('DeprecatedIdle'));
    expect(stateRole(machine, machine.states.at(-1)!).deprecated).toBe(false);
  });

  it('stops guessing failures in a machine that declares one', () => {
    // AuthState declares @failure, so an unmarked failure-shaped sibling is
    // unmarked on purpose.
    const declaring = withState(authMachine, extraState('UploadError'));
    expect(stateRole(declaring, declaring.states.at(-1)!).failure).toBe(false);

    // RecorderState declares none, so the heuristic still stands in.
    const guessing = withState(recorderMachine, extraState('UploadError'));
    expect(stateRole(guessing, guessing.states.at(-1)!).failure).toBe(true);
  });

  it('does not let @deprecated silence the failure heuristic', () => {
    // Only a declared *failure* is a statement about failures.
    const machine = withState(
      withState(recorderMachine, extraState('LegacyIdle', ['deprecated'])),
      extraState('UploadError'),
    );
    expect(stateRole(machine, machine.states.at(-1)!).failure).toBe(true);
  });
});

describe('stateRole with a declared default', () => {
  /** The same machine, with `#[default]` moved onto `name`. */
  function declaring(machine: DomainMachine, name: string): DomainMachine {
    return {
      ...machine,
      states: machine.states.map((s) => ({ ...s, isDefault: s.name === name })),
    };
  }

  it('takes the entry point from the declaration in a cyclic machine', () => {
    // AuthState is a cycle, so declaration order says nothing — which is the
    // whole reason the parser reports `#[default]`.
    const machine = declaring(authMachine, 'Authenticating');
    expect(role(machine, 'Authenticating').initial).toBe(true);
    expect(role(machine, 'SignedOut').initial).toBe(false);
    expect(entryState(machine)?.name).toBe('Authenticating');
  });

  it('outranks a state nothing transitions into', () => {
    // Idle is RecorderState's entry point by shape; the source is still
    // entitled to say the machine starts somewhere else.
    const machine = declaring(recorderMachine, 'Recording');
    expect(role(machine, 'Recording').initial).toBe(true);
    expect(role(machine, 'Idle').initial).toBe(false);
  });

  it('leaves the final role alone', () => {
    const machine = declaring(recorderMachine, 'Completed');
    expect(role(machine, 'Completed')).toEqual({
      initial: true,
      failure: false,
      deprecated: false,
      final: true,
    });
  });
});

describe('entryState', () => {
  it('returns the state carrying the initial role', () => {
    expect(entryState(recorderMachine)?.name).toBe('Idle');
    // fully cyclic machine: the first state stands in as the entry point
    expect(entryState(authMachine)?.name).toBe('SignedOut');
  });

  it('returns null for a machine with no states', () => {
    expect(entryState({ ...recorderMachine, states: [] })).toBeNull();
  });
});
