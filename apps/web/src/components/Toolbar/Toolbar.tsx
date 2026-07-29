import type { Theme } from '../../theme/theme';
import { useTranslate } from '../../i18n/useI18n';
import { LocaleToggle } from './LocaleToggle';
import { ThemeToggle } from './ThemeToggle';

interface ToolbarProps {
  projectName: string;
  coreName: string | null;
  simulating: boolean;
  theme: Theme;
  onToggleSimulation: () => void;
  onRelayout: () => void;
  onToggleTheme: () => void;
}

export function Toolbar({
  projectName,
  coreName,
  simulating,
  theme,
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
