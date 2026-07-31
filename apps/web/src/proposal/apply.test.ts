import { describe, expect, it } from 'vitest';
import type { DomainCore } from '../domain/types';
import { applyProposal } from './apply';
import type { Proposal } from './types';

const mockCore: DomainCore = {
  id: 'core-1',
  name: 'TestCore',
  eventDocs: { EventA: 'Event A doc', EventB: 'Event B doc' },
  effectDocs: {},
  machines: [
    {
      id: 'machine-1',
      name: 'CounterMachine',
      markers: [],
      tags: ['ui'],
      hasWildcard: false,
      states: [
        {
          id: 'state-idle',
          name: 'Idle',
          markers: [],
          tags: [],
          isDefault: true,
          incoming: [],
          outgoing: [],
        },
        {
          id: 'state-active',
          name: 'Active',
          markers: [],
          tags: [],
          isDefault: false,
          incoming: [],
          outgoing: [],
        },
      ],
      transitions: [
        {
          id: 'state-idle-EventA-state-active',
          event: 'EventA',
          from: 'state-idle',
          to: 'state-active',
          fromName: 'Idle',
          toName: 'Active',
          effects: [{ name: 'Render', capability: 'Render', answers: [], conditional: false }],
        },
      ],
    },
  ],
};

describe('applyProposal', () => {
  it('returns base core if undoCursor is 0', () => {
    const proposal: Proposal = {
      coreId: 'core-1',
      ops: [{ kind: 'edit-state-doc', stateId: 'state-idle', doc: 'New Idle Doc' }],
      undoCursor: 0,
      baseHash: 'hash',
      note: '',
    };
    const result = applyProposal(mockCore, proposal);
    expect(result).toEqual(mockCore);
  });

  it('applies edit-state-doc op and re-indexes incoming/outgoing', () => {
    const proposal: Proposal = {
      coreId: 'core-1',
      ops: [{ kind: 'edit-state-doc', stateId: 'state-idle', doc: 'Updated Idle Documentation' }],
      undoCursor: 1,
      baseHash: 'hash',
      note: '',
    };
    const result = applyProposal(mockCore, proposal);
    const idleState = result.machines[0].states.find((s) => s.id === 'state-idle')!;
    const activeState = result.machines[0].states.find((s) => s.id === 'state-active')!;

    expect(idleState.doc).toBe('Updated Idle Documentation');
    expect(idleState.outgoing).toHaveLength(1);
    expect(activeState.incoming).toHaveLength(1);
  });

  it('adds and removes effects on transitions', () => {
    const proposal: Proposal = {
      coreId: 'core-1',
      ops: [
        {
          kind: 'add-effect',
          transitionId: 'state-idle-EventA-state-active',
          effect: { name: 'FetchData', capability: 'Http', answers: ['DataLoaded'], conditional: true },
        },
      ],
      undoCursor: 1,
      baseHash: 'hash',
      note: '',
    };
    const result = applyProposal(mockCore, proposal);
    const t = result.machines[0].transitions[0];
    expect(t.effects).toHaveLength(2);
    expect(t.effects[1]).toEqual({
      name: 'FetchData',
      capability: 'Http',
      answers: ['DataLoaded'],
      conditional: true,
    });
  });

  it('adds a new transition between existing states and re-indexes incoming/outgoing', () => {
    const proposal: Proposal = {
      coreId: 'core-1',
      ops: [
        {
          kind: 'add-transition',
          transition: {
            from: 'state-active',
            event: 'EventB',
            to: 'state-idle',
            effects: [],
          },
        },
      ],
      undoCursor: 1,
      baseHash: 'hash',
      note: '',
    };
    const result = applyProposal(mockCore, proposal);
    expect(result.machines[0].transitions).toHaveLength(2);
    const activeState = result.machines[0].states.find((s) => s.id === 'state-active')!;
    const idleState = result.machines[0].states.find((s) => s.id === 'state-idle')!;
    expect(activeState.outgoing).toHaveLength(1);
    expect(idleState.incoming).toHaveLength(1);
  });
});
