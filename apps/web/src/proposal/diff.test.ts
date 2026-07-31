import { describe, expect, it } from 'vitest';
import type { DomainCore } from '../domain/types';
import { computeChangeSet } from './diff';

const baseCore: DomainCore = {
  id: 'core-1',
  name: 'TestCore',
  eventDocs: {},
  effectDocs: {},
  machines: [
    {
      id: 'm1',
      name: 'Machine1',
      markers: [],
      tags: [],
      hasWildcard: false,
      states: [
        {
          id: 's1',
          name: 'State1',
          markers: [],
          tags: [],
          isDefault: true,
          incoming: [],
          outgoing: [],
        },
      ],
      transitions: [],
    },
  ],
};

describe('computeChangeSet', () => {
  it('returns empty changeset for identical cores', () => {
    const diff = computeChangeSet(baseCore, baseCore);
    expect(diff.totalChanges).toBe(0);
    expect(diff.machines).toHaveLength(0);
  });

  it('detects doc changes on states', () => {
    const projCore: DomainCore = {
      ...baseCore,
      machines: [
        {
          ...baseCore.machines[0],
          states: [
            {
              ...baseCore.machines[0].states[0],
              doc: 'New state doc',
            },
          ],
        },
      ],
    };
    const diff = computeChangeSet(baseCore, projCore);
    expect(diff.totalChanges).toBe(1);
    expect(diff.machines[0].states.modified).toHaveLength(1);
    expect(diff.machines[0].states.modified[0]).toEqual({
      stateId: 's1',
      stateName: 'State1',
      field: 'doc',
      before: undefined,
      after: 'New state doc',
    });
  });
});
