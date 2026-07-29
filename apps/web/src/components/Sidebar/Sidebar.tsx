import type { DomainCore } from '../../domain/types';
import { useTranslate } from '../../i18n/useI18n';

interface SidebarProps {
  cores: DomainCore[];
  activeCoreId: string | null;
  onSelectCore: (coreId: string) => void;
}

export function Sidebar({ cores, activeCoreId, onSelectCore }: SidebarProps) {
  const t = useTranslate();
  return (
    <nav className="sidebar">
      <h2 className="panel-title">{t('sidebar.cores')}</h2>
      <ul>
        {cores.map((core) => (
          <li key={core.id}>
            <button
              className={core.id === activeCoreId ? 'core-item active' : 'core-item'}
              onClick={() => onSelectCore(core.id)}
            >
              {core.name}
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}
