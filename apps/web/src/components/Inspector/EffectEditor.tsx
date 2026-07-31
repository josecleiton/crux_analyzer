import { useState } from 'react';
import type { DomainEffect } from '../../domain/types';
import type { EffectDraft, TransitionDraft } from '../../proposal/types';
import { useTranslate } from '../../i18n/useI18n';

interface EffectEditorFormProps {
  initialEffect?: DomainEffect;
  onSave: (effect: EffectDraft) => void;
  onCancel: () => void;
}

export function EffectEditorForm({ initialEffect, onSave, onCancel }: EffectEditorFormProps) {
  const t = useTranslate();
  const [name, setName] = useState(initialEffect?.name || '');
  const [capability, setCapability] = useState(initialEffect?.capability || '');
  const [conditional, setConditional] = useState(initialEffect?.conditional || false);
  const [answersInput, setAnswersInput] = useState(initialEffect?.answers.join(', ') || '');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    const answers = answersInput
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);

    onSave({
      name: name.trim(),
      capability: capability.trim() || undefined,
      answers,
      conditional,
    });
  };

  return (
    <form className="effect-editor-form" onSubmit={handleSubmit}>
      <div className="form-group">
        <label>{t('proposal.effectName')}</label>
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="e.g. Render or Audio::Start"
          required
        />
      </div>
      <div className="form-group">
        <label>{t('proposal.capability')}</label>
        <input
          type="text"
          value={capability}
          onChange={(e) => setCapability(e.target.value)}
          placeholder="e.g. Render or Audio"
        />
      </div>
      <div className="form-group checkbox-group">
        <label>
          <input
            type="checkbox"
            checked={conditional}
            onChange={(e) => setConditional(e.target.checked)}
          />
          {t('proposal.conditionalLabel')}
        </label>
      </div>
      <div className="form-group">
        <label>{t('proposal.answers')}</label>
        <input
          type="text"
          value={answersInput}
          onChange={(e) => setAnswersInput(e.target.value)}
          placeholder="e.g. DataLoaded, Error"
        />
      </div>
      <div className="form-actions">
        <button type="submit" className="button-primary">
          {t('proposal.save')}
        </button>
        <button type="button" className="button-secondary" onClick={onCancel}>
          {t('proposal.cancel')}
        </button>
      </div>
    </form>
  );
}

interface NewTransitionFormProps {
  fromStateId: string;
  availableStates: Array<{ id: string; name: string }>;
  availableEvents: string[];
  onSave: (transition: TransitionDraft) => void;
  onCancel: () => void;
}

export function NewTransitionForm({
  fromStateId,
  availableStates,
  availableEvents,
  onSave,
  onCancel,
}: NewTransitionFormProps) {
  const t = useTranslate();
  const [eventInput, setEventInput] = useState('');
  const [toStateId, setToStateId] = useState(availableStates[0]?.id || '');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!eventInput.trim() || !toStateId) return;

    onSave({
      from: fromStateId,
      event: eventInput.trim(),
      to: toStateId,
      effects: [],
    });
  };

  return (
    <form className="new-transition-form" onSubmit={handleSubmit}>
      <h4>{t('proposal.addTransition')}</h4>
      <div className="form-group">
        <label>{t('proposal.eventName')}</label>
        <input
          type="text"
          list="event-suggestions"
          value={eventInput}
          onChange={(e) => setEventInput(e.target.value)}
          placeholder="e.g. NextClicked"
          required
        />
        <datalist id="event-suggestions">
          {availableEvents.map((evt) => (
            <option key={evt} value={evt} />
          ))}
        </datalist>
      </div>
      <div className="form-group">
        <label>{t('proposal.targetState')}</label>
        <select value={toStateId} onChange={(e) => setToStateId(e.target.value)}>
          {availableStates.map((s) => (
            <option key={s.id} value={s.id}>
              {s.name}
            </option>
          ))}
        </select>
      </div>
      <div className="form-actions">
        <button type="submit" className="button-primary">
          {t('proposal.createTransition')}
        </button>
        <button type="button" className="button-secondary" onClick={onCancel}>
          {t('proposal.cancel')}
        </button>
      </div>
    </form>
  );
}
