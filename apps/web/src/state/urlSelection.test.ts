import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from '../schema/parserJson';
import { fromParserJson } from '../domain/fromParserJson';
import { fromHash, resolveUrlState, toHash } from './urlSelection';

const project = fromParserJson(parseProjectJson(rawProject));
const authCore = project.cores[1];
const failed = authCore.machines[0].states.find((s) => s.name === 'Failed')!;
const someTransition = authCore.machines[0].transitions[0];

describe('toHash / fromHash', () => {
  it('round-trips a state selection, keeping the id readable', () => {
    const url = { coreId: authCore.id, selection: { kind: 'state' as const, id: failed.id } };
    const hash = toHash(url);
    expect(hash).toBe('#state=Authentication/AuthState/Failed');
    expect(fromHash(hash)).toEqual(url);
  });

  it('round-trips a transition selection, escaping what needs escaping', () => {
    const url = {
      coreId: authCore.id,
      selection: { kind: 'transition' as const, id: someTransition.id },
    };
    expect(fromHash(toHash(url))).toEqual(url);
  });

  it('carries a bare core when nothing is selected', () => {
    expect(toHash({ coreId: 'Sync', selection: null })).toBe('#core=Sync');
    expect(fromHash('#core=Sync')).toEqual({ coreId: 'Sync', selection: null });
  });

  it('treats anything unrecognized as no state at all', () => {
    for (const hash of ['', '#', '#foo', '#foo=bar', '#state=', '#state=%E0%A4%A']) {
      expect(fromHash(hash).selection).toBeNull();
    }
  });
});

describe('resolveUrlState', () => {
  it('accepts a link to an existing state', () => {
    const resolved = resolveUrlState(project, fromHash(`#state=${failed.id}`));
    expect(resolved).toEqual({
      coreId: authCore.id,
      selection: { kind: 'state', id: failed.id },
    });
  });

  it('keeps the core but drops a selection that no longer exists', () => {
    const resolved = resolveUrlState(
      project,
      fromHash('#state=Authentication/AuthState/Removed'),
    );
    expect(resolved).toEqual({ coreId: authCore.id, selection: null });
  });

  it('falls back entirely for a foreign core', () => {
    expect(resolveUrlState(project, fromHash('#core=NotHere'))).toEqual({
      coreId: null,
      selection: null,
    });
  });
});
