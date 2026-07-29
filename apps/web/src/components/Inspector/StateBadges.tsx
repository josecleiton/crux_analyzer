/**
 * A state's name and role badges, mirroring the graph colors: blue for the
 * initial state, red for failures, violet for finals (domain/stateRole.ts).
 */

import type { StateRole } from '../../domain/stateRole';

export function StateName({ name, role }: { name: string; role: StateRole }) {
  const tone = role.failure ? ' failure' : role.final ? ' final' : '';
  return <h3 className={`inspector-name${tone}`}>{name}</h3>;
}

export function StateBadges({ role }: { role: StateRole }) {
  if (!role.initial && !role.failure && !role.final) return null;
  return (
    <div className="state-badges">
      {role.initial ? <span className="state-badge initial">initial</span> : null}
      {role.failure ? <span className="state-badge failure">failure</span> : null}
      {role.final ? <span className="state-badge final">final</span> : null}
    </div>
  );
}
