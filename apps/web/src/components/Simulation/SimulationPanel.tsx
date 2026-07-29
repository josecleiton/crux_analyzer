/**
 * Right-panel UI for the Simulation Engine: shows the current state, the
 * events that can fire from it, and the trail of what already fired.
 */

import type { DomainMachine } from '../../domain/types';
import { stateRole } from '../../domain/stateRole';
import type { Simulation } from '../../simulation/engine';
import { availableTransitions, pendingAnswers, unreplayableTransitions } from '../../simulation/engine';
import { useTranslate } from '../../i18n/useI18n';
import { StateBadges, StateName } from '../Inspector/StateBadges';
import { DocText, StateTags } from '../Inspector/StateDoc';

interface SimulationPanelProps {
  machine: DomainMachine;
  simulation: Simulation;
  onFire: (transitionId: string) => void;
  onRestart: () => void;
}

export function SimulationPanel({ machine, simulation, onFire, onRestart }: SimulationPanelProps) {
  const t = useTranslate();
  const current = machine.states.find((s) => s.id === simulation.currentStateId);
  const available = availableTransitions(machine, simulation);
  const unreplayable = unreplayableTransitions(machine, simulation);
  const answers = pendingAnswers(machine, simulation);
  const answerOf = (transitionId: string) =>
    answers.find((answer) => answer.transitionId === transitionId);
  const role = current
    ? stateRole(machine, current)
    : { initial: false, failure: false, deprecated: false, final: false };

  return (
    <aside className="inspector">
      <h2 className="panel-title">{t('simulation.title')}</h2>
      <p className="inspector-machine">{machine.name}</p>
      <StateName name={current?.name ?? t('simulation.unknownState')} role={role} />
      <StateBadges role={role} />
      {/* Where you are, then what you can do from here. */}
      {current?.doc ? <DocText doc={current.doc} /> : null}
      <StateTags tags={current?.tags ?? []} />

      <h4>{t('simulation.sendEvent')}</h4>
      {available.length === 0 ? (
        <p className="inspector-empty">{t('simulation.noEvents')}</p>
      ) : (
        <ul className="event-list">
          {available.map((transition) => {
            const answer = answerOf(transition.id);
            return (
              <li key={transition.id}>
                <button className="event-button" onClick={() => onFire(transition.id)}>
                  {transition.event}
                  <span className="event-target"> → {transition.toName}</span>
                  {/* The shell owes this one: it can arrive with no user input. */}
                  {answer ? (
                    <span className="event-answer" title={answer.effect}>
                      {t('simulation.fromShell')}
                    </span>
                  ) : null}
                </button>
              </li>
            );
          })}
        </ul>
      )}

      {/* Runtime-target transitions are real behavior the replay cannot
          follow — shown and explained rather than silently hidden. */}
      {unreplayable.length > 0 ? (
        <>
          <ul className="event-list">
            {unreplayable.map((transition) => (
              <li key={transition.id} className="event-unreplayable">
                {transition.event}
                <span className="event-target"> → {t('state.anyStateRuntime')}</span>
              </li>
            ))}
          </ul>
          <p className="event-unreplayable-note">{t('simulation.runtimeTargetNote')}</p>
        </>
      ) : null}

      {/* What the last events asked the shell to do, and what it can answer
          with — the half of the loop the graph itself cannot show. */}
      {simulation.inFlight.length > 0 ? (
        <>
          <h4>{t('simulation.inFlight')}</h4>
          <ul className="event-list">
            {simulation.inFlight.map((pending) => (
              <li key={`${pending.step}-${pending.name}`}>
                {pending.name}
                <span className="event-target"> → {pending.answers.join(', ')}</span>
              </li>
            ))}
          </ul>
          {/* Answers with no transition from here: real, and inert. */}
          {answers.some((answer) => answer.transitionId === null) ? (
            <>
              <ul className="event-list">
                {answers
                  .filter((answer) => answer.transitionId === null)
                  .map((answer) => (
                    <li key={answer.event} className="event-unreplayable">
                      {answer.event}
                    </li>
                  ))}
              </ul>
              <p className="event-unreplayable-note">{t('simulation.inertAnswerNote')}</p>
            </>
          ) : null}
        </>
      ) : null}

      <h4>{t('simulation.trail')}</h4>
      {simulation.trail.length === 0 ? (
        <p className="inspector-empty">{t('simulation.nothingFired')}</p>
      ) : (
        <ol className="trail-list">
          {simulation.trail.map((step, i) => (
            <li
              key={`${step.transitionId}-${i}`}
              className={i === simulation.trail.length - 1 ? 'trail-new' : undefined}
            >
              <span className="trail-event">{step.event}</span>
              <span className="trail-states">
                {step.fromName} → {step.toName}
              </span>
            </li>
          ))}
        </ol>
      )}

      <button className="restart-button" onClick={onRestart}>
        {t('simulation.restart')}
      </button>
    </aside>
  );
}
