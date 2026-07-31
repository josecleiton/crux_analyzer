import { useState } from 'react';
import type { DomainCore, DomainEffect, DomainMachine } from '../../domain/types';
import { machineOf } from '../../domain/fromParserJson';
import { entryEffects } from '../../domain/effects';
import { stateRole } from '../../domain/stateRole';
import type { Selection } from '../../state/selection';
import { ANY_STATE_NAME } from '../../domain/types';
import { useTranslate } from '../../i18n/useI18n';
import type { ProposalOp } from '../../proposal/types';
import { EffectEditorForm, NewTransitionForm } from './EffectEditor';
import { StateBadges, StateName } from './StateBadges';
import { DocText, MachineDoc, StateTags } from './StateDoc';

interface InspectorProps {
  core: DomainCore | null;
  selection: Selection;
  isProposing?: boolean;
  onAddOp?: (op: ProposalOp) => void;
}

export function Inspector({ core, selection, isProposing, onAddOp }: InspectorProps) {
  const t = useTranslate();
  return (
    <aside className="inspector">
      <h2 className="panel-title">
        {t('inspector.title')} {isProposing ? <span className="proposal-badge">{t('proposal.badge')}</span> : null}
      </h2>
      <InspectorBody core={core} selection={selection} isProposing={isProposing} onAddOp={onAddOp} />
    </aside>
  );
}

function InspectorBody({ core, selection, isProposing, onAddOp }: InspectorProps) {
  const t = useTranslate();
  const [editingDoc, setEditingDoc] = useState(false);
  const [docText, setDocText] = useState('');
  const [editingEffectForTransition, setEditingEffectForTransition] = useState<{
    transitionId: string;
    index?: number;
    effect?: DomainEffect;
  } | null>(null);
  const [addingTransition, setAddingTransition] = useState(false);

  if (!core || !selection) {
    return <p className="inspector-empty">{t('inspector.empty')}</p>;
  }
  const machine = machineOf(core, selection.id);
  if (!machine) return null;

  if (selection.kind === 'state') {
    const state = machine.states.find((s) => s.id === selection.id);
    if (!state) return null;
    const role = stateRole(machine, state);
    const onEntry = entryEffects(state);

    const handleSaveDoc = () => {
      onAddOp?.({
        kind: 'edit-state-doc',
        stateId: state.id,
        doc: docText.trim() || undefined,
      });
      setEditingDoc(false);
    };

    return (
      <div>
        <StateName name={state.name} role={role} />
        <MachineTag machine={machine} core={core} />
        <StateBadges role={role} />

        {isProposing ? (
          <div className="proposal-edit-box">
            {editingDoc ? (
              <div className="edit-doc-form">
                <textarea
                  value={docText}
                  onChange={(e) => setDocText(e.target.value)}
                  placeholder={t('proposal.docPlaceholder')}
                  rows={3}
                />
                <div className="form-actions">
                  <button type="button" className="button-primary button-sm" onClick={handleSaveDoc}>
                    {t('proposal.save')}
                  </button>
                  <button type="button" className="button-secondary button-sm" onClick={() => setEditingDoc(false)}>
                    {t('proposal.cancel')}
                  </button>
                </div>
              </div>
            ) : (
              <div className="doc-section">
                {state.doc ? <DocText doc={state.doc} /> : <p className="text-dim">{t('proposal.noDoc')}</p>}
                <button
                  type="button"
                  className="button-link button-sm"
                  onClick={() => {
                    setDocText(state.doc || '');
                    setEditingDoc(true);
                  }}
                >
                  ✏️ {t('proposal.editDoc')}
                </button>
              </div>
            )}
          </div>
        ) : (
          state.doc ? <DocText doc={state.doc} /> : null
        )}

        <StateTags tags={state.tags} />

        <h4>{t('inspector.incoming')}</h4>
        <EventList events={state.incoming.map((transition) => transition.event)} docs={core.eventDocs} />

        <h4>{t('inspector.outgoing')}</h4>
        <EventList events={state.outgoing.map((transition) => transition.event)} docs={core.eventDocs} />

        {isProposing ? (
          <div className="proposal-section">
            {!addingTransition ? (
              <button
                type="button"
                className="button-secondary button-sm button-full"
                onClick={() => setAddingTransition(true)}
              >
                + {t('proposal.addTransition')}
              </button>
            ) : (
              <NewTransitionForm
                fromStateId={state.id}
                availableStates={machine.states.map((s) => ({ id: s.id, name: s.name }))}
                availableEvents={Object.keys(core.eventDocs)}
                onSave={(tDraft) => {
                  onAddOp?.({ kind: 'add-transition', transition: tDraft });
                  setAddingTransition(false);
                }}
                onCancel={() => setAddingTransition(false)}
              />
            )}
          </div>
        ) : null}

        {onEntry.length > 0 ? (
          <>
            <h4>{t('inspector.entryEffects')}</h4>
            <EffectList effects={onEntry} core={core} />
          </>
        ) : null}

        <MachineDoc machine={machine} />
      </div>
    );
  }

  // Selection is transition
  const transition = machine.transitions.find((candidate) => candidate.id === selection.id);
  if (!transition) return null;
  const eventDoc = core.eventDocs[transition.event];
  const isWildcard = transition.fromName === ANY_STATE_NAME || transition.toName === ANY_STATE_NAME;

  return (
    <div>
      <h3 className="inspector-name">{transition.event}</h3>
      <MachineTag machine={machine} core={core} />
      {eventDoc ? <DocText doc={eventDoc} /> : null}

      <div className="transition-flow">
        <span>
          {transition.fromName === ANY_STATE_NAME ? t('state.anyState') : transition.fromName}
        </span>
        <span className="transition-arrow">↓</span>
        <span>
          {transition.toName === ANY_STATE_NAME ? t('state.anyStateRuntime') : transition.toName}
        </span>
      </div>

      {isWildcard && isProposing ? (
        <p className="text-dim-warning">{t('proposal.wildcardReadOnly')}</p>
      ) : null}

      <div className="effects-header">
        <h4>{t('inspector.effects')}</h4>
        {isProposing && !isWildcard && !editingEffectForTransition ? (
          <button
            type="button"
            className="button-link button-sm"
            onClick={() => setEditingEffectForTransition({ transitionId: transition.id })}
          >
            + {t('proposal.addEffect')}
          </button>
        ) : null}
      </div>

      {editingEffectForTransition?.transitionId === transition.id ? (
        <EffectEditorForm
          initialEffect={editingEffectForTransition.effect}
          onSave={(eDraft) => {
            if (editingEffectForTransition.index !== undefined) {
              onAddOp?.({
                kind: 'edit-effect',
                transitionId: transition.id,
                index: editingEffectForTransition.index,
                effect: eDraft,
              });
            } else {
              onAddOp?.({
                kind: 'add-effect',
                transitionId: transition.id,
                effect: eDraft,
              });
            }
            setEditingEffectForTransition(null);
          }}
          onCancel={() => setEditingEffectForTransition(null)}
        />
      ) : null}

      {transition.effects.length > 0 ? (
        <EffectList
          effects={transition.effects}
          core={core}
          isProposing={isProposing && !isWildcard}
          onRemoveEffect={(index) => {
            onAddOp?.({ kind: 'remove-effect', transitionId: transition.id, index });
          }}
          onEditEffect={(index, effect) => {
            setEditingEffectForTransition({ transitionId: transition.id, index, effect });
          }}
        />
      ) : null}

      {isProposing && !isWildcard ? (
        <div className="proposal-section">
          <button
            type="button"
            className="button-danger button-sm button-full"
            onClick={() => {
              onAddOp?.({ kind: 'remove-transition', transitionId: transition.id });
            }}
          >
            🗑️ {t('proposal.removeTransition')}
          </button>
        </div>
      ) : null}

      <MachineDoc machine={machine} />
    </div>
  );
}

function EffectList({
  effects,
  core,
  isProposing,
  onRemoveEffect,
  onEditEffect,
}: {
  effects: DomainEffect[];
  core: DomainCore;
  isProposing?: boolean;
  onRemoveEffect?: (index: number) => void;
  onEditEffect?: (index: number, effect: DomainEffect) => void;
}) {
  const t = useTranslate();
  return (
    <ul className="event-list">
      {effects.map((effect, idx) => {
        const doc = core.effectDocs[effect.name];
        return (
          <li key={`${effect.name}-${idx}`} title={doc} className="effect-item">
            <div className="effect-main">
              <span className="effect-name">
                {effect.name}
                {doc ? <span className="state-doc-mark" aria-hidden="true" /> : null}
              </span>
              {effect.capability ? <span className="effect-capability">{effect.capability}</span> : null}
              {effect.conditional ? <span className="effect-conditional">{t('inspector.conditional')}</span> : null}
              {effect.answers.length > 0 ? (
                <span className="effect-answers">
                  {t('inspector.answersWith')}{' '}
                  {effect.answers.map((event, i) => (
                    <span key={event} title={core.eventDocs[event]}>
                      {i > 0 ? ', ' : ''}
                      {event}
                    </span>
                  ))}
                </span>
              ) : null}
            </div>

            {isProposing ? (
              <div className="effect-actions">
                <button
                  type="button"
                  className="icon-button"
                  title="Edit effect"
                  onClick={() => onEditEffect?.(idx, effect)}
                >
                  ✏️
                </button>
                <button
                  type="button"
                  className="icon-button"
                  title="Remove effect"
                  onClick={() => onRemoveEffect?.(idx)}
                >
                  🗑️
                </button>
              </div>
            ) : null}
          </li>
        );
      })}
    </ul>
  );
}

function MachineTag({ machine, core }: { machine: DomainMachine; core: DomainCore }) {
  if (core.machines.length < 2) return null;
  return <p className="inspector-machine">{machine.name}</p>;
}

function EventList({ events, docs }: { events: string[]; docs?: Record<string, string> }) {
  const t = useTranslate();
  if (events.length === 0) return <p className="inspector-empty">{t('inspector.none')}</p>;
  return (
    <ul className="event-list">
      {events.map((event, i) => (
        <li key={`${event}-${i}`} title={docs?.[event]}>
          {event}
          {docs?.[event] ? <span className="state-doc-mark" aria-hidden="true" /> : null}
        </li>
      ))}
    </ul>
  );
}
