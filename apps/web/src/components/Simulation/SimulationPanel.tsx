/**
 * Right-panel UI for the Simulation Engine: shows the current state, the
 * events that can fire from it, and the trail of what already fired.
 */

import type { DomainMachine } from '../../domain/types';
import type { Simulation } from '../../simulation/engine';
import { availableTransitions } from '../../simulation/engine';

interface SimulationPanelProps {
  machine: DomainMachine;
  simulation: Simulation;
  onFire: (transitionId: string) => void;
  onRestart: () => void;
}

export function SimulationPanel({ machine, simulation, onFire, onRestart }: SimulationPanelProps) {
  const current = machine.states.find((s) => s.id === simulation.currentStateId);
  const available = availableTransitions(machine, simulation);

  return (
    <aside className="inspector">
      <h2 className="panel-title">Simulation</h2>
      <p className="inspector-machine">{machine.name}</p>
      <h3 className="inspector-name">{current?.name ?? '?'}</h3>

      <h4>Send event</h4>
      {available.length === 0 ? (
        <p className="inspector-empty">No events can fire from here.</p>
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

      <h4>Trail</h4>
      {simulation.trail.length === 0 ? (
        <p className="inspector-empty">Nothing fired yet.</p>
      ) : (
        <ol className="trail-list">
          {simulation.trail.map((step, i) => (
            <li key={`${step.transitionId}-${i}`}>
              <span className="trail-event">{step.event}</span>
              <span className="trail-states">
                {step.fromName} → {step.toName}
              </span>
            </li>
          ))}
        </ol>
      )}

      <button className="restart-button" onClick={onRestart}>
        Restart
      </button>
    </aside>
  );
}
