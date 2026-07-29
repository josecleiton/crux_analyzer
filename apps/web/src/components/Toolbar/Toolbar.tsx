interface ToolbarProps {
  projectName: string;
  coreName: string | null;
  simulating: boolean;
  onToggleSimulation: () => void;
  onRelayout: () => void;
}

export function Toolbar({
  projectName,
  coreName,
  simulating,
  onToggleSimulation,
  onRelayout,
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
      </div>
    </header>
  );
}
