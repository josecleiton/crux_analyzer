import type { DomainCore } from '../../domain/types';
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

  if (selection.kind === 'state') {
    const state = core.states.find((s) => s.id === selection.id);
    if (!state) return null;
    return (
      <div>
        <h3 className="inspector-name">{state.name}</h3>
        <h4>Incoming</h4>
        <EventList events={state.incoming.map((t) => t.event)} />
        <h4>Outgoing</h4>
        <EventList events={state.outgoing.map((t) => t.event)} />
      </div>
    );
  }

  const transition = core.transitions.find((t) => t.id === selection.id);
  if (!transition) return null;
  return (
    <div>
      <h3 className="inspector-name">{transition.event}</h3>
      <div className="transition-flow">
        <span>{transition.fromName}</span>
        <span className="transition-arrow">↓</span>
        <span>{transition.toName}</span>
      </div>
    </div>
  );
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
