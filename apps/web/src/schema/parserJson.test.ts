import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from './parserJson';

/** A minimal contract-shaped project with one machine, for focused cases. */
function projectWith(machine: Record<string, unknown>) {
  return { project: 'X', cores: [{ name: 'A', machines: [machine] }] };
}

function statesOf(raw: unknown) {
  return parseProjectJson(raw).cores[0].machines[0].states;
}

describe('parseProjectJson', () => {
  it('accepts the bundled contract example', () => {
    // Regression guard: the example now mixes bare and annotated states, and
    // the old validator rejected anything that was not a string.
    expect(() => parseProjectJson(rawProject)).not.toThrow();
  });

  it('normalizes a bare state name into the annotated shape', () => {
    const [state] = statesOf(projectWith({ name: 'M', states: ['Idle'], transitions: [] }));
    expect(state).toEqual({ name: 'Idle', doc: undefined, markers: [], tags: [] });
  });

  it('carries doc, markers and tags of an annotated state', () => {
    const [state] = statesOf(
      projectWith({
        name: 'M',
        states: [{ name: 'Failed', doc: 'It broke.', markers: ['failure'], tags: ['retryable'] }],
        transitions: [],
      }),
    );
    expect(state).toEqual({
      name: 'Failed',
      doc: 'It broke.',
      markers: ['failure'],
      tags: ['retryable'],
    });
  });

  it('carries the machine description, markers and tags', () => {
    const machine = parseProjectJson(
      projectWith({
        name: 'M',
        doc: 'What this region is for.',
        markers: ['deprecated'],
        tags: ['legacy'],
        states: ['Idle'],
        transitions: [],
      }),
    ).cores[0].machines[0];
    expect(machine.doc).toBe('What this region is for.');
    expect(machine.markers).toEqual(['deprecated']);
    expect(machine.tags).toEqual(['legacy']);
  });

  it('defaults markers and tags to empty arrays', () => {
    const machine = parseProjectJson(
      projectWith({ name: 'M', states: [{ name: 'Idle' }], transitions: [] }),
    ).cores[0].machines[0];
    expect(machine.markers).toEqual([]);
    expect(machine.tags).toEqual([]);
    expect(machine.states[0].markers).toEqual([]);
    expect(machine.states[0].tags).toEqual([]);
  });

  it('drops a marker it does not understand instead of rejecting the model', () => {
    // Forward compatibility: a newer parser must never blank an older UI.
    const [state] = statesOf(
      projectWith({
        name: 'M',
        states: [{ name: 'Failed', markers: ['failure', 'experimental'] }],
        transitions: [],
      }),
    );
    expect(state.markers).toEqual(['failure']);
  });

  it('rejects JSON outside the contract', () => {
    expect(() => parseProjectJson({ cores: [] })).toThrow(/project/);
    expect(() =>
      parseProjectJson(
        projectWith({ name: 'M', states: ['S'], transitions: [{ from: 'S' }] }),
      ),
    ).toThrow(/transition/);
    expect(() =>
      parseProjectJson({ project: 'X', cores: [{ name: 'A', states: [], transitions: [] }] }),
    ).toThrow(/machines/);
  });

  it('rejects malformed state documentation', () => {
    expect(() =>
      statesOf(projectWith({ name: 'M', states: [{ doc: 'no name' }], transitions: [] })),
    ).toThrow(/state\.name/);
    expect(() =>
      statesOf(projectWith({ name: 'M', states: [{ name: 'S', doc: 42 }], transitions: [] })),
    ).toThrow(/doc/);
    expect(() =>
      statesOf(projectWith({ name: 'M', states: [{ name: 'S', markers: 'failure' }], transitions: [] })),
    ).toThrow(/markers/);
    expect(() =>
      statesOf(projectWith({ name: 'M', states: [{ name: 'S', tags: [1] }], transitions: [] })),
    ).toThrow(/tags/);
    expect(() =>
      statesOf(projectWith({ name: 'M', states: [42], transitions: [] })),
    ).toThrow(/state must be a string or an object/);
  });
});
