import type { ReactNode } from 'react';
import type { Theme } from '../../theme/theme';
import { useTranslate } from '../../i18n/useI18n';
import { LocaleToggle } from './LocaleToggle';
import { ThemeToggle } from './ThemeToggle';

interface ToolbarProps {
  projectName: string;
  coreName: string | null;
  simulating: boolean;
  theme: Theme;
  /** Tag filter — `tagOptions` are the analyzed app's own tag names (data). */
  tagQuery: string;
  tagOptions: string[];
  undocumentedOnly: boolean;
  onTagQueryChange: (query: string) => void;
  onToggleUndocumented: () => void;
  onToggleSimulation: () => void;
  onRelayout: () => void;
  onToggleTheme: () => void;
}

export function Toolbar({
  projectName,
  coreName,
  simulating,
  theme,
  tagQuery,
  tagOptions,
  undocumentedOnly,
  onTagQueryChange,
  onToggleUndocumented,
  onToggleSimulation,
  onRelayout,
  onToggleTheme,
}: ToolbarProps) {
  const t = useTranslate();
  return (
    <header className="toolbar">
      <div className="toolbar-lead">
        <span className="toolbar-title">
          {projectName}
          {coreName ? <span className="toolbar-core"> / {coreName}</span> : null}
        </span>
        {/* The tag filter reads, the buttons act — so it sits on the left
            with the title, out of the action cluster, wide enough for its
            whole label. Disabled (not hidden) while a simulation owns the
            emphasis; a core with no declared tags gets no filter at all. */}
        {tagOptions.length > 0 ? (
          <>
            <input
              className={`toolbar-filter${tagQuery.trim() !== '' ? ' active' : ''}`}
              type="search"
              list="toolbar-tag-options"
              value={tagQuery}
              onChange={(event) => onTagQueryChange(event.target.value)}
              placeholder={t('toolbar.filterByTag')}
              aria-label={t('toolbar.filterByTag')}
              disabled={simulating}
            />
            <datalist id="toolbar-tag-options">
              {tagOptions.map((tag) => (
                <option value={tag} key={tag} />
              ))}
            </datalist>
          </>
        ) : null}
      </div>
      <div className="toolbar-actions">
        {/* Simulate leads: it is the primary action of the toolbar. While a
            simulation runs the icon is a stop square, not a pause — stopping
            discards the run, and an icon must not promise a resume. */}
        <button className={simulating ? 'active' : ''} onClick={onToggleSimulation}>
          {simulating ? <StopIcon /> : <PlayIcon />}
          {simulating ? t('toolbar.stopSimulation') : t('toolbar.simulate')}
        </button>
        {/* The warning triangle says what this toggle is about — the states
            a reader should not trust yet; the title explains it on hover. */}
        <button
          className={`undocumented-toggle${undocumentedOnly ? ' active' : ''}`}
          onClick={onToggleUndocumented}
          title={t('toolbar.undocumentedHint')}
          aria-label={t('toolbar.undocumentedHint')}
          disabled={simulating}
        >
          <svg
            className="undocumented-toggle-icon"
            aria-hidden="true"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M8 2.2 14.6 13.4H1.4L8 2.2Z" />
            <line x1="8" y1="6.8" x2="8" y2="9.8" />
            <circle cx="8" cy="11.9" r="0.4" fill="currentColor" stroke="none" />
          </svg>
          {t('toolbar.undocumented')}
        </button>
        <button onClick={onRelayout}>
          <RelayoutIcon />
          {t('toolbar.relayout')}
        </button>
        <LocaleToggle />
        <ThemeToggle theme={theme} onToggle={onToggleTheme} />
      </div>
    </header>
  );
}

/** Shared frame of the small toolbar icons (decorative, 16-unit grid). */
function ToolbarIcon({ children, filled = false }: { children: ReactNode; filled?: boolean }) {
  return (
    <svg
      className="toolbar-icon"
      aria-hidden="true"
      viewBox="0 0 16 16"
      fill={filled ? 'currentColor' : 'none'}
      stroke={filled ? 'none' : 'currentColor'}
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {children}
    </svg>
  );
}

function PlayIcon() {
  return (
    <ToolbarIcon filled>
      <path d="M4.8 2.9v10.2c0 .5.5.7.9.5l8-5.1a.6.6 0 0 0 0-1l-8-5.1a.6.6 0 0 0-.9.5Z" />
    </ToolbarIcon>
  );
}

function StopIcon() {
  return (
    <ToolbarIcon filled>
      <rect x="3.4" y="3.4" width="9.2" height="9.2" rx="1.6" />
    </ToolbarIcon>
  );
}

function RelayoutIcon() {
  return (
    <ToolbarIcon>
      <path d="M13.2 8A5.2 5.2 0 1 1 11 3.7" />
      <polyline points="11.2 1.6 11.2 4.2 8.6 4.2" transform="rotate(28 11.2 4.2)" />
    </ToolbarIcon>
  );
}
