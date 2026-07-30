import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson } from './fromParserJson';
import {
  NOTHING_HIDDEN,
  groupVisibility,
  hiddenInCore,
  isOnCanvas,
  machineStateIds,
  withHidden,
} from './visibility';

const project = fromParserJson(parseProjectJson(rawProject));
const core = project.cores[0]; // two machines
const machine = core.machines[0];
const ids = machineStateIds(machine);

describe('groupVisibility', () => {
  it('reads all, none and mixed', () => {
    expect(groupVisibility(ids, NOTHING_HIDDEN)).toBe('all');
    expect(groupVisibility(ids, new Set(ids))).toBe('none');
    expect(groupVisibility(ids, new Set([ids[0]]))).toBe('some');
  });

  it('reads an empty group as visible', () => {
    // A machine with no states hides nothing, so its toggle is not "off".
    expect(groupVisibility([], new Set(ids))).toBe('all');
  });
});

describe('withHidden', () => {
  it('hides and shows the ids it is given', () => {
    const hidden = withHidden(NOTHING_HIDDEN, [ids[0], ids[1]], true);
    expect([...hidden].sort()).toEqual([ids[0], ids[1]].sort());
    expect([...withHidden(hidden, [ids[0]], false)]).toEqual([ids[1]]);
  });

  it('returns the same set when nothing changes', () => {
    // Identity is the contract: a no-op toggle must not re-render the canvas.
    const hidden = withHidden(NOTHING_HIDDEN, [ids[0]], true);
    expect(withHidden(hidden, [ids[0]], true)).toBe(hidden);
    expect(withHidden(hidden, [ids[1]], false)).toBe(hidden);
  });

  it('leaves the set it was given untouched', () => {
    const hidden = new Set([ids[0]]);
    withHidden(hidden, [ids[1]], true);
    expect([...hidden]).toEqual([ids[0]]);
  });
});

describe('hiddenInCore', () => {
  it('lists only the hidden states of that core', () => {
    const other = project.cores[1].machines[0].states[0].id;
    const hidden = new Set([ids[0], other]);
    expect(hiddenInCore(core, hidden)).toEqual([ids[0]]);
    expect(hiddenInCore(project.cores[1], hidden)).toEqual([other]);
  });
});

describe('isOnCanvas', () => {
  it('drops a hidden state', () => {
    expect(isOnCanvas(core, 'state', ids[0], new Set([ids[0]]))).toBe(false);
    expect(isOnCanvas(core, 'state', ids[1], new Set([ids[0]]))).toBe(true);
  });

  it('drops a transition that lost either endpoint', () => {
    const transition = machine.transitions.find((t) => t.fromName !== '*')!;
    expect(isOnCanvas(core, 'transition', transition.id, NOTHING_HIDDEN)).toBe(true);
    expect(isOnCanvas(core, 'transition', transition.id, new Set([transition.from]))).toBe(false);
    expect(isOnCanvas(core, 'transition', transition.id, new Set([transition.to]))).toBe(false);
  });

  it('drops every transition of a machine with nothing visible left', () => {
    // Including one whose both ends are the wildcard pseudo-state: that machine
    // is off the canvas whole.
    const allHidden = new Set(machineStateIds(machine));
    for (const transition of machine.transitions) {
      expect(isOnCanvas(core, 'transition', transition.id, allHidden)).toBe(false);
    }
  });

  it('keeps a wildcard transition alive through its real endpoint', () => {
    const wildcardMachine = core.machines[1];
    const wildcard = wildcardMachine.transitions.find((t) => t.fromName === '*')!;
    // "any state" is not a state the reader can deselect: only the target can
    // take this edge off the canvas.
    expect(isOnCanvas(core, 'transition', wildcard.id, new Set([wildcard.from]))).toBe(true);
    expect(isOnCanvas(core, 'transition', wildcard.id, new Set([wildcard.to]))).toBe(false);
  });
});
