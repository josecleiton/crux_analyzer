import type { DomainCore } from '../../domain/types';

interface SidebarProps {
  cores: DomainCore[];
  activeCoreId: string | null;
  onSelectCore: (coreId: string) => void;
}

export function Sidebar({ cores, activeCoreId, onSelectCore }: SidebarProps) {
  return (
    <nav className="sidebar">
      <h2 className="panel-title">Cores</h2>
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
