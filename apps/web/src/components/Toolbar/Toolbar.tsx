interface ToolbarProps {
  projectName: string;
  coreName: string | null;
  onRelayout: () => void;
}

export function Toolbar({ projectName, coreName, onRelayout }: ToolbarProps) {
  return (
    <header className="toolbar">
      <span className="toolbar-title">
        {projectName}
        {coreName ? <span className="toolbar-core"> / {coreName}</span> : null}
      </span>
      <button onClick={onRelayout}>Re-layout</button>
    </header>
  );
}
