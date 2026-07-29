/**
 * A state's name and role badges, mirroring the graph colors: blue for the
 * initial state, red for failures, amber for deprecated, violet for finals
 * (domain/stateRole.ts).
 */

import type { StateRole } from '../../domain/stateRole';
import { useTranslate } from '../../i18n/useI18n';

export function StateName({ name, role }: { name: string; role: StateRole }) {
  // Color and strikethrough are separate, so a deprecated failure reads as
  // both: red, and struck through.
  const tone = role.failure
    ? ' failure'
    : role.deprecated
      ? ' deprecated'
      : role.final
        ? ' final'
        : '';
  const struck = role.deprecated ? ' struck' : '';
  return <h3 className={`inspector-name${tone}${struck}`}>{name}</h3>;
}

export function StateBadges({ role }: { role: StateRole }) {
  const t = useTranslate();
  if (!role.initial && !role.failure && !role.deprecated && !role.final) return null;
  return (
    <div className="state-badges">
      {role.initial ? <span className="state-badge initial">{t('badge.initial')}</span> : null}
      {role.failure ? <span className="state-badge failure">{t('badge.failure')}</span> : null}
      {role.deprecated ? (
        <span className="state-badge deprecated">{t('badge.deprecated')}</span>
      ) : null}
      {role.final ? <span className="state-badge final">{t('badge.final')}</span> : null}
    </div>
  );
}
