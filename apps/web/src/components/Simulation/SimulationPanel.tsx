/**
 * Right-panel UI for the Simulation Engine: shows the current state, the
 * events that can fire from it, and the trail of what already fired.
 */

import type { DomainMachine } from '../../domain/types';
import { stateRole } from '../../domain/stateRole';
import type { Simulation } from '../../simulation/engine';
import { availableTransitions } from '../../simulation/engine';
import { useTranslate } from '../../i18n/useI18n';
import { StateBadges, StateName } from '../Inspector/StateBadges';

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
  const role = current
    ? stateRole(machine, current)
    : { initial: false, failure: false, final: false };

  return (
    <aside className="inspector">
      <h2 className="panel-title">{t('simulation.title')}</h2>
      <p className="inspector-machine">{machine.name}</p>
      <StateName name={current?.name ?? t('simulation.unknownState')} role={role} />
      <StateBadges role={role} />

      <h4>{t('simulation.sendEvent')}</h4>
      {available.length === 0 ? (
        <p className="inspector-empty">{t('simulation.noEvents')}</p>
      ) : (
        <ul className="event-list">
          {available.map((transition) => (
            <li key={transition.id}>
              <button className="event-button" onClick={() => onFire(transition.id)}>
                {transition.event}
                <span className="event-target"> → {transition.toName}</span>
              </button>
            </li>
          ))}
        </ul>
      )}

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
