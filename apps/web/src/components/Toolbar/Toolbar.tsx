import type { Theme } from '../../theme/theme';
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
  return (
    <header className="toolbar">
      <span className="toolbar-title">
        {projectName}
        {coreName ? <span className="toolbar-core"> / {coreName}</span> : null}
      </span>
      <div className="toolbar-actions">
        <button className={simulating ? 'active' : ''} onClick={onToggleSimulation}>
          {simulating ? 'Stop simulation' : 'Simulate'}
        </button>
        <button onClick={onRelayout}>Re-layout</button>
        <ThemeToggle theme={theme} onToggle={onToggleTheme} />
      </div>
    </header>
  );
}
