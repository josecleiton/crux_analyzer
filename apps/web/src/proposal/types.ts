import type { DomainEffect, StateMarker } from '../domain/types';

export type ChangeType = 'added' | 'modified' | 'removed';

export interface EffectDraft {
  name: string;
  capability?: string;
  answers: string[];
  conditional: boolean;
}

export interface TransitionDraft {
  from: string;
  event: string;
  to: string;
  effects: EffectDraft[];
}

export type ProposalOp =
  | { kind: 'add-effect'; transitionId: string; effect: EffectDraft }
  | { kind: 'edit-effect'; transitionId: string; index: number; effect: EffectDraft }
  | { kind: 'remove-effect'; transitionId: string; index: number }
  | { kind: 'add-transition'; transition: TransitionDraft }
  | { kind: 'remove-transition'; transitionId: string }
  | { kind: 'edit-transition'; transitionId: string; fields: Partial<Pick<TransitionDraft, 'from' | 'event' | 'to'>> }
  | { kind: 'edit-state-doc'; stateId: string; doc?: string }
  | { kind: 'edit-state-markers'; stateId: string; markers: StateMarker[] }
  | { kind: 'edit-state-tags'; stateId: string; tags: string[] };

export interface Proposal {
  coreId: string;
  ops: ProposalOp[];
  undoCursor: number;
  baseHash: string;
  note: string;
}

export interface TransitionChange {
  key: { from: string; event: string; to: string };
  fromName: string;
  toName: string;
  effectsAdded: DomainEffect[];
  effectsRemoved: DomainEffect[];
}

export interface StateFieldChange {
  stateId: string;
  stateName: string;
  field: 'doc' | 'markers' | 'tags';
  before: unknown;
  after: unknown;
}

export interface MachineChange {
  machineId: string;
  machineName: string;
  transitions: {
    added: Array<{ fromName: string; event: string; toName: string; effects: DomainEffect[] }>;
    removed: Array<{ fromName: string; event: string; toName: string; effects: DomainEffect[] }>;
    modified: TransitionChange[];
  };
  states: {
    modified: StateFieldChange[];
  };
}

export interface ChangeSet {
  coreId: string;
  machines: MachineChange[];
  totalChanges: number;
}
