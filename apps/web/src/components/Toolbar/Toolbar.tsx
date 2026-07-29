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
      <span className="toolbar-title">
        {projectName}
        {coreName ? <span className="toolbar-core"> / {coreName}</span> : null}
      </span>
      <div className="toolbar-actions">
        {/* The reading filters step aside while the simulation owns the
            emphasis — disabled rather than hidden, so they don't reflow.
            A core with no declared tags has nothing to filter by, so the
            input does not render at all. */}
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
        <button
          className={undocumentedOnly ? 'active' : ''}
          onClick={onToggleUndocumented}
          title={t('toolbar.undocumentedHint')}
          disabled={simulating}
        >
          {t('toolbar.undocumented')}
        </button>
        <button className={simulating ? 'active' : ''} onClick={onToggleSimulation}>
          {simulating ? t('toolbar.stopSimulation') : t('toolbar.simulate')}
        </button>
        <button onClick={onRelayout}>{t('toolbar.relayout')}</button>
        <LocaleToggle />
        <ThemeToggle theme={theme} onToggle={onToggleTheme} />
      </div>
    </header>
  );
}
