import { useEffect, useMemo, useState } from 'react';
import type { Node } from '@xyflow/react';
import { loadProject } from './data/loadProject';
import { toFlowModel } from './flow/toFlowModel';
import type { LayoutEngine } from './layout/LayoutEngine';
import { ElkLayoutEngine } from './layout/ElkLayoutEngine';
import type { Selection } from './state/selection';
import { Graph } from './components/Graph/Graph';
import { Sidebar } from './components/Sidebar/Sidebar';
import { Inspector } from './components/Inspector/Inspector';
import { Toolbar } from './components/Toolbar/Toolbar';

const project = loadProject();
const layoutEngine: LayoutEngine = new ElkLayoutEngine();

export default function App() {
  const [activeCoreId, setActiveCoreId] = useState<string | null>(
    project.cores[0]?.id ?? null,
  );
  const [selection, setSelection] = useState<Selection>(null);
  const [layoutVersion, setLayoutVersion] = useState(0);
  const [positionedNodes, setPositionedNodes] = useState<Node[]>([]);

  const activeCore = useMemo(
    () => project.cores.find((core) => core.id === activeCoreId) ?? null,
    [activeCoreId],
  );

  const flowModel = useMemo(
    () => (activeCore ? toFlowModel(activeCore) : { nodes: [], edges: [] }),
    [activeCore],
  );

  useEffect(() => {
    let cancelled = false;
    layoutEngine.layout(flowModel.nodes, flowModel.edges).then((nodes) => {
      if (!cancelled) setPositionedNodes(nodes);
    });
    return () => {
      cancelled = true;
    };
  }, [flowModel, layoutVersion]);

  function selectCore(coreId: string) {
    setActiveCoreId(coreId);
    setSelection(null);
  }

  return (
    <div className="app">
      <Toolbar
        projectName={project.name}
        coreName={activeCore?.name ?? null}
        onRelayout={() => setLayoutVersion((v) => v + 1)}
      />
      <div className="app-body">
        <Sidebar
          cores={project.cores}
          activeCoreId={activeCoreId}
          onSelectCore={selectCore}
        />
        <main className="graph-area">
          <Graph
            nodes={positionedNodes}
            edges={flowModel.edges}
            selection={selection}
            onSelect={setSelection}
          />
        </main>
        <Inspector core={activeCore} selection={selection} />
      </div>
    </div>
  );
}
