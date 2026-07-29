import type { DomainCore, DomainMachine } from '../../domain/types';
import { machineOf } from '../../domain/fromParserJson';
import type { Selection } from '../../state/selection';

interface InspectorProps {
  core: DomainCore | null;
  selection: Selection;
}

export function Inspector({ core, selection }: InspectorProps) {
  return (
    <aside className="inspector">
      <h2 className="panel-title">Inspector</h2>
      <InspectorBody core={core} selection={selection} />
    </aside>
  );
}

function InspectorBody({ core, selection }: InspectorProps) {
  if (!core || !selection) {
    return <p className="inspector-empty">Select a state or a transition.</p>;
  }
  const machine = machineOf(core, selection.id);
  if (!machine) return null;

  if (selection.kind === 'state') {
    const state = machine.states.find((s) => s.id === selection.id);
    if (!state) return null;
    return (
      <div>
        <h3 className="inspector-name">{state.name}</h3>
        <MachineTag machine={machine} core={core} />
        <h4>Incoming</h4>
        <EventList events={state.incoming.map((t) => t.event)} />
        <h4>Outgoing</h4>
        <EventList events={state.outgoing.map((t) => t.event)} />
      </div>
    );
  }

  const transition = machine.transitions.find((t) => t.id === selection.id);
  if (!transition) return null;
  return (
    <div>
      <h3 className="inspector-name">{transition.event}</h3>
      <MachineTag machine={machine} core={core} />
      <div className="transition-flow">
        <span>{transition.fromName === '*' ? 'any state' : transition.fromName}</span>
        <span className="transition-arrow">↓</span>
        <span>{transition.toName === '*' ? 'any state (runtime)' : transition.toName}</span>
      </div>
      {transition.effects.length > 0 ? (
        <>
          <h4>Effects</h4>
          <EventList events={transition.effects} />
        </>
      ) : null}
    </div>
  );
}

function MachineTag({ machine, core }: { machine: DomainMachine; core: DomainCore }) {
  if (core.machines.length < 2) return null;
  return <p className="inspector-machine">{machine.name}</p>;
}

function EventList({ events }: { events: string[] }) {
  if (events.length === 0) return <p className="inspector-empty">—</p>;
  return (
    <ul className="event-list">
      {events.map((event, i) => (
        <li key={`${event}-${i}`}>{event}</li>
      ))}
    </ul>
  );
}
