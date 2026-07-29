import type { DomainCore, DomainMachine } from '../../domain/types';
import { machineOf } from '../../domain/fromParserJson';
import { stateRole } from '../../domain/stateRole';
import type { Selection } from '../../state/selection';
import { ANY_STATE_NAME } from '../../domain/types';
import { useTranslate } from '../../i18n/useI18n';
import { StateBadges, StateName } from './StateBadges';

interface InspectorProps {
  core: DomainCore | null;
  selection: Selection;
}

export function Inspector({ core, selection }: InspectorProps) {
  const t = useTranslate();
  return (
    <aside className="inspector">
      <h2 className="panel-title">{t('inspector.title')}</h2>
      <InspectorBody core={core} selection={selection} />
    </aside>
  );
}

function InspectorBody({ core, selection }: InspectorProps) {
  const t = useTranslate();
  if (!core || !selection) {
    return <p className="inspector-empty">{t('inspector.empty')}</p>;
  }
  const machine = machineOf(core, selection.id);
  if (!machine) return null;

  if (selection.kind === 'state') {
    const state = machine.states.find((s) => s.id === selection.id);
    if (!state) return null;
    const role = stateRole(machine, state);
    return (
      <div>
        <StateName name={state.name} role={role} />
        <MachineTag machine={machine} core={core} />
        <StateBadges role={role} />
        <h4>{t('inspector.incoming')}</h4>
        <EventList events={state.incoming.map((transition) => transition.event)} />
        <h4>{t('inspector.outgoing')}</h4>
        <EventList events={state.outgoing.map((transition) => transition.event)} />
      </div>
    );
  }

  const transition = machine.transitions.find((candidate) => candidate.id === selection.id);
  if (!transition) return null;
  return (
    <div>
      <h3 className="inspector-name">{transition.event}</h3>
      <MachineTag machine={machine} core={core} />
      <div className="transition-flow">
        <span>
          {transition.fromName === ANY_STATE_NAME
            ? t('state.anyState')
            : transition.fromName}
        </span>
        <span className="transition-arrow">↓</span>
        <span>
          {transition.toName === ANY_STATE_NAME
            ? t('state.anyStateRuntime')
            : transition.toName}
        </span>
      </div>
      {transition.effects.length > 0 ? (
        <>
          <h4>{t('inspector.effects')}</h4>
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
  const t = useTranslate();
  if (events.length === 0) return <p className="inspector-empty">{t('inspector.none')}</p>;
  return (
    <ul className="event-list">
      {events.map((event, i) => (
        <li key={`${event}-${i}`}>{event}</li>
      ))}
    </ul>
  );
}
