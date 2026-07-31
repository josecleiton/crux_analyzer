import type { DomainCore, DomainMachine, DomainState, DomainTransition } from '../domain/types';
import type { Proposal, ProposalOp } from './types';

/**
 * Re-indexes incoming and outgoing transitions arrays on all states of a machine.
 */
function reindexMachineStates(machine: DomainMachine): DomainMachine {
  const transitionMap = new Map<string, DomainTransition>();
  for (const t of machine.transitions) {
    transitionMap.set(t.id, t);
  }

  const updatedStates: DomainState[] = machine.states.map((state) => {
    const incoming = machine.transitions.filter((t) => t.to === state.id);
    const outgoing = machine.transitions.filter((t) => t.from === state.id);
    return {
      ...state,
      incoming,
      outgoing,
    };
  });

  return {
    ...machine,
    states: updatedStates,
  };
}

/**
 * Deeply applies a proposal (up to proposal.undoCursor) on a base DomainCore.
 * Returns a new DomainCore with updated machines, states, transitions, and re-indexed state incoming/outgoing lists.
 */
export function applyProposal(baseCore: DomainCore, proposal: Proposal): DomainCore {
  if (baseCore.id !== proposal.coreId || proposal.undoCursor === 0) {
    return baseCore;
  }

  const activeOps: ProposalOp[] = proposal.ops.slice(0, proposal.undoCursor);

  // Clone machines structure
  let currentMachines: DomainMachine[] = baseCore.machines.map((machine) => ({
    ...machine,
    markers: [...machine.markers],
    tags: [...machine.tags],
    states: machine.states.map((state) => ({
      ...state,
      markers: [...state.markers],
      tags: [...state.tags],
      incoming: [...state.incoming],
      outgoing: [...state.outgoing],
    })),
    transitions: machine.transitions.map((t) => ({
      ...t,
      effects: t.effects.map((e) => ({ ...e, answers: [...e.answers] })),
    })),
  }));

  for (const op of activeOps) {
    switch (op.kind) {
      case 'add-effect': {
        if (!op.effect.name.trim()) break;
        for (let i = 0; i < currentMachines.length; i++) {
          const m = currentMachines[i];
          const tIndex = m.transitions.findIndex((t) => t.id === op.transitionId);
          if (tIndex !== -1) {
            const tr = m.transitions[tIndex];
            // Prevent duplicate effect name + capability on same transition
            const isDuplicate = tr.effects.some(
              (e) => e.name === op.effect.name && e.capability === op.effect.capability
            );
            if (!isDuplicate) {
              const updatedTransitions = [...m.transitions];
              updatedTransitions[tIndex] = {
                ...tr,
                effects: [
                  ...tr.effects,
                  {
                    name: op.effect.name.trim(),
                    capability: op.effect.capability?.trim() || undefined,
                    answers: [...op.effect.answers],
                    conditional: op.effect.conditional,
                  },
                ],
              };
              currentMachines[i] = { ...m, transitions: updatedTransitions };
            }
            break;
          }
        }
        break;
      }

      case 'edit-effect': {
        if (!op.effect.name.trim()) break;
        for (let i = 0; i < currentMachines.length; i++) {
          const m = currentMachines[i];
          const tIndex = m.transitions.findIndex((t) => t.id === op.transitionId);
          if (tIndex !== -1) {
            const tr = m.transitions[tIndex];
            if (op.index >= 0 && op.index < tr.effects.length) {
              const updatedEffects = [...tr.effects];
              updatedEffects[op.index] = {
                name: op.effect.name.trim(),
                capability: op.effect.capability?.trim() || undefined,
                answers: [...op.effect.answers],
                conditional: op.effect.conditional,
              };
              const updatedTransitions = [...m.transitions];
              updatedTransitions[tIndex] = { ...tr, effects: updatedEffects };
              currentMachines[i] = { ...m, transitions: updatedTransitions };
            }
            break;
          }
        }
        break;
      }

      case 'remove-effect': {
        for (let i = 0; i < currentMachines.length; i++) {
          const m = currentMachines[i];
          const tIndex = m.transitions.findIndex((t) => t.id === op.transitionId);
          if (tIndex !== -1) {
            const tr = m.transitions[tIndex];
            if (op.index >= 0 && op.index < tr.effects.length) {
              const updatedEffects = tr.effects.filter((_, idx) => idx !== op.index);
              const updatedTransitions = [...m.transitions];
              updatedTransitions[tIndex] = { ...tr, effects: updatedEffects };
              currentMachines[i] = { ...m, transitions: updatedTransitions };
            }
            break;
          }
        }
        break;
      }

      case 'add-transition': {
        const { from, event, to, effects } = op.transition;
        if (!event.trim()) break;

        // Find machine containing `from` state or `to` state
        for (let i = 0; i < currentMachines.length; i++) {
          const m = currentMachines[i];
          const fromState = m.states.find((s) => s.id === from);
          const toState = m.states.find((s) => s.id === to);

          if (fromState && toState) {
            const transitionId = `${from}-${event}-${to}`;
            const exists = m.transitions.some((t) => t.id === transitionId);
            if (!exists) {
              const newTransition: DomainTransition = {
                id: transitionId,
                event: event.trim(),
                from,
                to,
                fromName: fromState.name,
                toName: toState.name,
                effects: effects.map((e) => ({
                  name: e.name.trim(),
                  capability: e.capability?.trim() || undefined,
                  answers: [...e.answers],
                  conditional: e.conditional,
                })),
              };
              currentMachines[i] = {
                ...m,
                transitions: [...m.transitions, newTransition],
              };
            }
            break;
          }
        }
        break;
      }

      case 'remove-transition': {
        for (let i = 0; i < currentMachines.length; i++) {
          const m = currentMachines[i];
          const exists = m.transitions.some((t) => t.id === op.transitionId);
          if (exists) {
            currentMachines[i] = {
              ...m,
              transitions: m.transitions.filter((t) => t.id !== op.transitionId),
            };
            break;
          }
        }
        break;
      }

      case 'edit-transition': {
        for (let i = 0; i < currentMachines.length; i++) {
          const m = currentMachines[i];
          const tIndex = m.transitions.findIndex((t) => t.id === op.transitionId);
          if (tIndex !== -1) {
            const tr = m.transitions[tIndex];
            const newFrom = op.fields.from ?? tr.from;
            const newEvent = op.fields.event ?? tr.event;
            const newTo = op.fields.to ?? tr.to;

            const fromState = m.states.find((s) => s.id === newFrom);
            const toState = m.states.find((s) => s.id === newTo);

            if (fromState && toState) {
              const newId = `${newFrom}-${newEvent}-${newTo}`;
              const updatedTransitions = [...m.transitions];
              updatedTransitions[tIndex] = {
                ...tr,
                id: newId,
                from: newFrom,
                event: newEvent,
                to: newTo,
                fromName: fromState.name,
                toName: toState.name,
              };
              currentMachines[i] = { ...m, transitions: updatedTransitions };
            }
            break;
          }
        }
        break;
      }

      case 'edit-state-doc': {
        for (let i = 0; i < currentMachines.length; i++) {
          const m = currentMachines[i];
          const sIndex = m.states.findIndex((s) => s.id === op.stateId);
          if (sIndex !== -1) {
            const updatedStates = [...m.states];
            updatedStates[sIndex] = {
              ...updatedStates[sIndex],
              doc: op.doc,
            };
            currentMachines[i] = { ...m, states: updatedStates };
            break;
          }
        }
        break;
      }

      case 'edit-state-markers': {
        for (let i = 0; i < currentMachines.length; i++) {
          const m = currentMachines[i];
          const sIndex = m.states.findIndex((s) => s.id === op.stateId);
          if (sIndex !== -1) {
            const updatedStates = [...m.states];
            updatedStates[sIndex] = {
              ...updatedStates[sIndex],
              markers: [...op.markers],
            };
            currentMachines[i] = { ...m, states: updatedStates };
            break;
          }
        }
        break;
      }

      case 'edit-state-tags': {
        for (let i = 0; i < currentMachines.length; i++) {
          const m = currentMachines[i];
          const sIndex = m.states.findIndex((s) => s.id === op.stateId);
          if (sIndex !== -1) {
            const updatedStates = [...m.states];
            updatedStates[sIndex] = {
              ...updatedStates[sIndex],
              tags: [...op.tags],
            };
            currentMachines[i] = { ...m, states: updatedStates };
            break;
          }
        }
        break;
      }
    }
  }

  // Re-index all machine states for incoming/outgoing arrays
  const reindexedMachines = currentMachines.map(reindexMachineStates);

  return {
    ...baseCore,
    machines: reindexedMachines,
  };
}
