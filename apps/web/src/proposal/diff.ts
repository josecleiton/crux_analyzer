import type { DomainCore, DomainEffect } from '../domain/types';
import type { ChangeSet, MachineChange, StateFieldChange, TransitionChange } from './types';

function effectsEqual(a: DomainEffect, b: DomainEffect): boolean {
  return (
    a.name === b.name &&
    a.capability === b.capability &&
    a.conditional === b.conditional &&
    a.answers.length === b.answers.length &&
    a.answers.every((val, idx) => val === b.answers[idx])
  );
}

/**
 * Computes a structural ChangeSet comparing baseDomainCore with projectedDomainCore.
 */
export function computeChangeSet(baseCore: DomainCore, projectedCore: DomainCore): ChangeSet {
  const machineChanges: MachineChange[] = [];
  let totalChanges = 0;

  for (const projMachine of projectedCore.machines) {
    const baseMachine = baseCore.machines.find((m) => m.id === projMachine.id);
    if (!baseMachine) continue;

    const addedTransitions: MachineChange['transitions']['added'] = [];
    const removedTransitions: MachineChange['transitions']['removed'] = [];
    const modifiedTransitions: TransitionChange[] = [];

    // Map base transitions by key (from, event, to)
    const baseTransMap = new Map<string, typeof baseMachine.transitions[0]>();
    for (const t of baseMachine.transitions) {
      baseTransMap.set(`${t.from}|${t.event}|${t.to}`, t);
    }

    const projTransMap = new Map<string, typeof projMachine.transitions[0]>();
    for (const t of projMachine.transitions) {
      projTransMap.set(`${t.from}|${t.event}|${t.to}`, t);
    }

    // Check for added & modified transitions
    for (const [key, projTrans] of projTransMap.entries()) {
      const baseTrans = baseTransMap.get(key);
      if (!baseTrans) {
        addedTransitions.push({
          fromName: projTrans.fromName,
          event: projTrans.event,
          toName: projTrans.toName,
          effects: [...projTrans.effects],
        });
        totalChanges++;
      } else {
        // Compare effects
        const effectsAdded = projTrans.effects.filter(
          (pe) => !baseTrans.effects.some((be) => effectsEqual(be, pe))
        );
        const effectsRemoved = baseTrans.effects.filter(
          (be) => !projTrans.effects.some((pe) => effectsEqual(pe, be))
        );

        if (effectsAdded.length > 0 || effectsRemoved.length > 0) {
          modifiedTransitions.push({
            key: { from: projTrans.from, event: projTrans.event, to: projTrans.to },
            fromName: projTrans.fromName,
            toName: projTrans.toName,
            effectsAdded,
            effectsRemoved,
          });
          totalChanges++;
        }
      }
    }

    // Check for removed transitions
    for (const [key, baseTrans] of baseTransMap.entries()) {
      if (!projTransMap.has(key)) {
        removedTransitions.push({
          fromName: baseTrans.fromName,
          event: baseTrans.event,
          toName: baseTrans.toName,
          effects: [...baseTrans.effects],
        });
        totalChanges++;
      }
    }

    // Check for modified state metadata (doc, markers, tags)
    const stateFieldChanges: StateFieldChange[] = [];
    for (const projState of projMachine.states) {
      const baseState = baseMachine.states.find((s) => s.id === projState.id);
      if (!baseState) continue;

      if (projState.doc !== baseState.doc) {
        stateFieldChanges.push({
          stateId: projState.id,
          stateName: projState.name,
          field: 'doc',
          before: baseState.doc,
          after: projState.doc,
        });
        totalChanges++;
      }

      if (JSON.stringify(projState.markers) !== JSON.stringify(baseState.markers)) {
        stateFieldChanges.push({
          stateId: projState.id,
          stateName: projState.name,
          field: 'markers',
          before: baseState.markers,
          after: projState.markers,
        });
        totalChanges++;
      }

      if (JSON.stringify(projState.tags) !== JSON.stringify(baseState.tags)) {
        stateFieldChanges.push({
          stateId: projState.id,
          stateName: projState.name,
          field: 'tags',
          before: baseState.tags,
          after: projState.tags,
        });
        totalChanges++;
      }
    }

    if (
      addedTransitions.length > 0 ||
      removedTransitions.length > 0 ||
      modifiedTransitions.length > 0 ||
      stateFieldChanges.length > 0
    ) {
      machineChanges.push({
        machineId: projMachine.id,
        machineName: projMachine.name,
        transitions: {
          added: addedTransitions,
          removed: removedTransitions,
          modified: modifiedTransitions,
        },
        states: {
          modified: stateFieldChanges,
        },
      });
    }
  }

  return {
    coreId: baseCore.id,
    machines: machineChanges,
    totalChanges,
  };
}
