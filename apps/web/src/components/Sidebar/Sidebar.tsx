import type { DomainCore } from '../../domain/types';
import { useTranslate } from '../../i18n/useI18n';

/**
 * This project's own repository — a hardcoded literal, never read out of the
 * analyzed source. The analyzer's chrome may point at the analyzer; nothing
 * the target app declares is allowed to become a link the UI offers.
 */
const REPOSITORY_URL = 'https://github.com/josecleiton/crux_analyzer';

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
      {/* Bottom of the sidebar: out of the way of the reading, still reachable
          from every view. Same link hygiene as author prose in the inspector. */}
      <a
        className="sidebar-repo"
        href={REPOSITORY_URL}
        target="_blank"
        rel="noopener noreferrer"
        title={t('sidebar.sourceCode')}
      >
        <GitHubIcon />
        GitHub
      </a>
    </nav>
  );
}

/** The GitHub mark, inlined: the CSP allows no remote image. */
function GitHubIcon() {
  return (
    <svg viewBox="0 0 16 16" width="15" height="15" fill="currentColor" aria-hidden="true">
      <path d="M8 0a8 8 0 0 0-2.53 15.59c.4.07.55-.17.55-.38v-1.34c-2.22.48-2.69-1.07-2.69-1.07-.36-.93-.89-1.18-.89-1.18-.73-.5.05-.49.05-.49.8.06 1.23.83 1.23.83.71 1.22 1.87.87 2.33.67.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82a7.6 7.6 0 0 1 4 0c1.53-1.03 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.28.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48v2.19c0 .21.15.46.55.38A8 8 0 0 0 8 0Z" />
    </svg>
  );
}
