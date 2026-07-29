import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson } from './fromParserJson';
import { declaredTags, focusFor } from './focus';
import type { DomainCore } from './types';
import { wildcardStateId } from './types';

const project = fromParserJson(parseProjectJson(rawProject));
const recorderCore = project.cores[0]; // RecorderState + InputState (wildcard)
const authCore = project.cores[1]; // AuthState — "Failed" tagged "retryable"
const syncCore = project.cores[2]; // SyncState — "Conflict" tagged and documented

function names(core: DomainCore, stateIds: string[]): string[] {
  const all = core.machines.flatMap((machine) => machine.states);
  return stateIds.map((id) => all.find((state) => state.id === id)?.name ?? id).sort();
}

describe('focusFor with a tag query', () => {
  it('keeps the states carrying the tag and drops the rest', () => {
    const focus = focusFor(authCore, { tagQuery: 'retryable', undocumentedOnly: false })!;
    expect(names(authCore, focus.stateIds)).toEqual(['Failed']);
    // both edges of "Failed" touch an unmatched state, so they dim too
    expect(focus.transitionIds).toEqual([]);
  });

  it('matches case-insensitively and by fragment, but never rewrites tags', () => {
    const focus = focusFor(authCore, { tagQuery: '  RETRY', undocumentedOnly: false })!;
    expect(names(authCore, focus.stateIds)).toEqual(['Failed']);
  });

  it('keeps transitions whose two endpoints match', () => {
    // Tag every SyncState state that takes part in the Idle↔Syncing loop.
    const tagged: DomainCore = {
      ...syncCore,
      machines: syncCore.machines.map((machine) => ({
        ...machine,
        states: machine.states.map((state) =>
          state.name === 'Idle' || state.name === 'Syncing'
            ? { ...state, tags: [...state.tags, 'hot'] }
            : state,
        ),
      })),
    };
    const focus = focusFor(tagged, { tagQuery: 'hot', undocumentedOnly: false })!;
    expect(names(tagged, focus.stateIds)).toEqual(['Idle', 'Syncing']);
    const events = tagged.machines[0].transitions
      .filter((transition) => focus.transitionIds.includes(transition.id))
      .map((transition) => `${transition.fromName}->${transition.toName}`);
    expect(events).toEqual(['Idle->Syncing']);
  });

  it('reads a tag on the state enum as covering the whole region', () => {
    const tagged: DomainCore = {
      ...recorderCore,
      machines: recorderCore.machines.map((machine) =>
        machine.name === 'InputState' ? { ...machine, tags: ['audio-io'] } : machine,
      ),
    };
    const focus = focusFor(tagged, { tagQuery: 'audio-io', undocumentedOnly: false })!;
    expect(names(tagged, focus.stateIds)).toContain('Ready');
    expect(names(tagged, focus.stateIds)).toContain('Switching');
    // ...and only that region: RecorderState stays dimmed.
    expect(names(tagged, focus.stateIds)).not.toContain('Recording');
  });

  it('returns an empty focus (not null) when nothing carries the tag', () => {
    const focus = focusFor(authCore, { tagQuery: 'no-such-tag', undocumentedOnly: false });
    expect(focus).toEqual({ stateIds: [], transitionIds: [] });
  });
});

describe('focusFor on undocumented states', () => {
  it('keeps only the states with no authored description', () => {
    const focus = focusFor(syncCore, { tagQuery: '', undocumentedOnly: true })!;
    // "Conflict" is the only documented state in SyncState
    expect(names(syncCore, focus.stateIds)).toEqual(['Done', 'Idle', 'Syncing']);
    const events = syncCore.machines[0].transitions
      .filter((transition) => focus.transitionIds.includes(transition.id))
      .map((transition) => `${transition.fromName}->${transition.toName}`);
    expect(events).toEqual(['Idle->Syncing', 'Syncing->Done', 'Done->Syncing']);
  });

  it('keeps a wildcard edge into a kept state, and the pseudo-node with it', () => {
    const inputMachine = recorderCore.machines.find((m) => m.name === 'InputState')!;
    const wildcard = wildcardStateId(inputMachine.id);
    const focus = focusFor(recorderCore, { tagQuery: '', undocumentedOnly: true })!;
    // nothing in the bundled Recorder core is documented, so everything stays
    expect(focus.stateIds).toContain(wildcard);
    const wildcardEdge = inputMachine.transitions.find((t) => t.from === wildcard)!;
    expect(focus.transitionIds).toContain(wildcardEdge.id);
  });

  it('composes with the tag query as an intersection', () => {
    // "Failed" carries the tag but is documented: both criteria → nothing.
    const focus = focusFor(authCore, { tagQuery: 'retryable', undocumentedOnly: true });
    expect(focus).toEqual({ stateIds: [], transitionIds: [] });
  });
});

describe('focusFor when idle', () => {
  it('returns null when no criterion is active', () => {
    expect(focusFor(authCore, { tagQuery: '', undocumentedOnly: false })).toBeNull();
    expect(focusFor(authCore, { tagQuery: '   ', undocumentedOnly: false })).toBeNull();
  });
});

describe('declaredTags', () => {
  it('collects the tags of a core for suggestions', () => {
    expect(declaredTags(authCore)).toEqual(['retryable']);
    expect(declaredTags(syncCore)).toEqual(['manual-resolution']);
  });

  it('orders by declaration count, ties alphabetical', () => {
    const tagged: DomainCore = {
      ...syncCore,
      machines: syncCore.machines.map((machine) => ({
        ...machine,
        tags: ['zeta'],
        states: machine.states.map((state, index) => ({
          ...state,
          // "hot" on two states, "alpha"/"zeta" once each
          tags: index < 2 ? ['hot'] : index === 2 ? ['alpha'] : [],
        })),
      })),
    };
    expect(declaredTags(tagged)).toEqual(['hot', 'alpha', 'zeta']);
  });

  it('is empty for a core with nothing to filter by', () => {
    expect(declaredTags(recorderCore)).toEqual([]);
  });
});
