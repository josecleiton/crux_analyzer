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
  tagQuery: string;
  tagOptions: string[];
  undocumentedOnly: boolean;
  showEffects: boolean;
  onTagQueryChange: (query: string) => void;
  onToggleUndocumented: () => void;
  onToggleEffects: () => void;
  onToggleSimulation: () => void;
  onRelayout: () => void;
  onToggleTheme: () => void;
  // Proposal mode props
  isProposing?: boolean;
  changeCount?: number;
  canUndo?: boolean;
  canRedo?: boolean;
  isStale?: boolean;
  onTogglePropose?: () => void;
  onOpenReview?: () => void;
  onUndo?: () => void;
  onRedo?: () => void;
  onDiscard?: () => void;
}

export function Toolbar({
  projectName,
  coreName,
  simulating,
  theme,
  tagQuery,
  tagOptions,
  undocumentedOnly,
  showEffects,
  onTagQueryChange,
  onToggleUndocumented,
  onToggleEffects,
  onToggleSimulation,
  onRelayout,
  onToggleTheme,
  isProposing = false,
  changeCount = 0,
  canUndo = false,
  canRedo = false,
  isStale = false,
  onTogglePropose,
  onOpenReview,
  onUndo,
  onRedo,
  onDiscard,
}: ToolbarProps) {
  const t = useTranslate();
  return (
    <header className="toolbar">
      <div className="toolbar-lead">
        <span className="toolbar-title">
          {projectName}
          {coreName ? <span className="toolbar-core"> / {coreName}</span> : null}
        </span>
        {tagOptions.length > 0 ? (
          <TagFilter
            query={tagQuery}
            options={tagOptions}
            disabled={simulating || isProposing}
            onChange={onTagQueryChange}
          />
        ) : null}
      </div>

      <div className="toolbar-actions">
        {/* Propose Changes Toggle */}
        <button
          className={`proposal-toggle${isProposing ? ' active' : ''}`}
          onClick={onTogglePropose}
          disabled={simulating}
          title={simulating ? t('proposal.disabledInSimulation') : t('proposal.proposeChangesHint')}
        >
          ✏️ {t('proposal.proposeChanges')}
        </button>

        {isProposing ? (
          <>
            <button
              className="icon-button-toolbar"
              onClick={onUndo}
              disabled={!canUndo}
              title="Undo (Ctrl+Z)"
            >
              ↩️
            </button>
            <button
              className="icon-button-toolbar"
              onClick={onRedo}
              disabled={!canRedo}
              title="Redo (Ctrl+Y)"
            >
              ↪️
            </button>
            <button
              className="proposal-review-button"
              onClick={onOpenReview}
              disabled={changeCount === 0}
            >
              📋 {t('proposal.review')} ({changeCount})
            </button>
            <button
              className="button-secondary button-sm"
              onClick={onDiscard}
              disabled={changeCount === 0}
              title={t('proposal.discardTitle')}
            >
              🔄 {t('proposal.discard')}
            </button>
            {isStale ? (
              <span className="badge-stale" title={t('proposal.staleHint')}>
                ⚠️ {t('proposal.stale')}
              </span>
            ) : null}
          </>
        ) : null}

        {/* Simulate Toggle */}
        <button
          className={simulating ? 'active' : ''}
          onClick={onToggleSimulation}
          disabled={isProposing}
          title={isProposing ? t('proposal.disabledInProposal') : undefined}
        >
          {simulating ? <StopIcon /> : <PlayIcon />}
          {simulating ? t('toolbar.stopSimulation') : t('toolbar.simulate')}
        </button>

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

        <button
          className={showEffects ? 'active' : ''}
          onClick={onToggleEffects}
          title={t('toolbar.showEffectsHint') || 'Show effects on edges'}
        >
          <svg
            className="toolbar-icon"
            aria-hidden="true"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M8 1L3 9H7.5L7 15L13 6H8L8.5 1Z" />
          </svg>
          {t('toolbar.showEffects') || 'Effects'}
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
