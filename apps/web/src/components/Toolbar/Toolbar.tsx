import { useState } from 'react';
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
          <TagFilter
            query={tagQuery}
            options={tagOptions}
            disabled={simulating}
            onChange={onTagQueryChange}
          />
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

/**
 * The tag filter input with its own suggestion list. Not a `<datalist>` on
 * purpose: native datalist popups are inconsistent across engines (Chrome on
 * macOS does not open one for this shape at all), and a filter whose
 * suggestions may or may not appear reads as broken. Tag names are data from
 * the analyzed app, hence monospace in the list.
 */
function TagFilter({
  query,
  options,
  disabled,
  onChange,
}: {
  query: string;
  options: string[];
  disabled: boolean;
  onChange: (query: string) => void;
}) {
  const t = useTranslate();
  const [open, setOpen] = useState(false);
  const fragment = query.trim().toLowerCase();
  const suggestions = options.filter((tag) => tag.toLowerCase().includes(fragment));

  return (
    <div className="toolbar-filter-wrap">
      <input
        className={`toolbar-filter${query.trim() !== '' ? ' active' : ''}`}
        type="text"
        role="combobox"
        aria-expanded={open && suggestions.length > 0}
        aria-autocomplete="list"
        value={query}
        onChange={(event) => {
          onChange(event.target.value);
          setOpen(true);
        }}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        onKeyDown={(event) => {
          if (event.key === 'Escape') setOpen(false);
          if (event.key === 'ArrowDown') setOpen(true);
        }}
        placeholder={t('toolbar.filterByTag')}
        aria-label={t('toolbar.filterByTag')}
        disabled={disabled}
      />
      {open && suggestions.length > 0 ? (
        <ul className="tag-suggestions" role="listbox">
          {suggestions.map((tag) => (
            <li key={tag}>
              <button
                type="button"
                role="option"
                aria-selected={tag === query}
                // mousedown, so the pick lands before the input's blur closes
                onMouseDown={(event) => {
                  event.preventDefault();
                  onChange(tag);
                  setOpen(false);
                }}
              >
                {tag}
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
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
