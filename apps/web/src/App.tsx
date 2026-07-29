import { useEffect, useMemo, useState } from 'react';
import { loadProject } from './data/loadProject';
import type { DomainProject } from './domain/types';
import { machineOf } from './domain/fromParserJson';
import { toFlowModel } from './flow/toFlowModel';
import type { LayoutEngine, LayoutResult } from './layout/LayoutEngine';
import { ElkLayoutEngine } from './layout/ElkLayoutEngine';
import type { Selection } from './state/selection';
import type { Simulation } from './simulation/engine';
import { fire, lastStep, startSimulation } from './simulation/engine';
import { Graph } from './components/Graph/Graph';
import type { GraphHighlight } from './components/Graph/Graph';
import { Sidebar } from './components/Sidebar/Sidebar';
import { Inspector } from './components/Inspector/Inspector';
import { SimulationPanel } from './components/Simulation/SimulationPanel';
import { Toolbar } from './components/Toolbar/Toolbar';
import { useTheme } from './theme/useTheme';

const layoutEngine: LayoutEngine = new ElkLayoutEngine();

export default function App() {
  const [project, setProject] = useState<DomainProject | null>(null);
  const [activeCoreId, setActiveCoreId] = useState<string | null>(null);
  const [selection, setSelection] = useState<Selection>(null);
  const [simulation, setSimulation] = useState<Simulation | null>(null);
  const [layoutVersion, setLayoutVersion] = useState(0);
  const [layouted, setLayouted] = useState<LayoutResult>({ nodes: [], edges: [] });
  const { theme, toggleTheme } = useTheme();

  useEffect(() => {
    let cancelled = false;
    loadProject().then((loaded) => {
      if (cancelled) return;
      setProject(loaded);
      setActiveCoreId(loaded.cores[0]?.id ?? null);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const activeCore = useMemo(
    () => project?.cores.find((core) => core.id === activeCoreId) ?? null,
    [project, activeCoreId],
  );

  const flowModel = useMemo(
    () => (activeCore ? toFlowModel(activeCore) : { nodes: [], edges: [] }),
    [activeCore],
  );

  useEffect(() => {
    let cancelled = false;
    layoutEngine.layout(flowModel.nodes, flowModel.edges).then((result) => {
      if (!cancelled) setLayouted(result);
    });
    return () => {
      cancelled = true;
    };
  }, [flowModel, layoutVersion]);

  const simulatedMachine = useMemo(
    () =>
      activeCore && simulation
        ? activeCore.machines.find((m) => m.id === simulation.machineId) ?? null
        : null,
    [activeCore, simulation],
  );

  const highlight: GraphHighlight | undefined = useMemo(() => {
    if (!simulation) return undefined;
    const last = lastStep(simulation);
    return {
      nodeIds: [simulation.currentStateId],
      edgeIds: last ? [last.transitionId] : [],
    };
  }, [simulation]);

  function selectCore(coreId: string) {
    setActiveCoreId(coreId);
    setSelection(null);
    setSimulation(null);
  }

  function toggleSimulation() {
    if (simulation) {
      setSimulation(null);
      return;
    }
    if (!activeCore || activeCore.machines.length === 0) return;
    // Start on the machine of the selected state (at that state), or on the
    // core's first machine.
    const machine =
      (selection?.kind === 'state' ? machineOf(activeCore, selection.id) : null) ??
      activeCore.machines[0];
    const initialState = selection?.kind === 'state' ? selection.id : undefined;
    setSelection(null);
    setSimulation(startSimulation(machine, initialState));
  }

  function fireTransition(transitionId: string) {
    if (!simulation || !simulatedMachine) return;
    setSimulation(fire(simulatedMachine, simulation, transitionId));
  }

  if (!project) {
    return <div className="app-loading">Loading…</div>;
  }

  return (
    <div className="app">
      <Toolbar
        projectName={project.name}
        coreName={activeCore?.name ?? null}
        simulating={simulation !== null}
        theme={theme}
        onToggleSimulation={toggleSimulation}
        onRelayout={() => setLayoutVersion((v) => v + 1)}
        onToggleTheme={toggleTheme}
      />
      <div className="app-body">
        <Sidebar
          cores={project.cores}
          activeCoreId={activeCoreId}
          onSelectCore={selectCore}
        />
        <main className="graph-area">
          <Graph
            nodes={layouted.nodes}
            edges={layouted.edges}
            selection={selection}
            onSelect={setSelection}
            highlight={highlight}
            theme={theme}
          />
        </main>
        {simulation && simulatedMachine ? (
          <SimulationPanel
            machine={simulatedMachine}
            simulation={simulation}
            onFire={fireTransition}
            onRestart={() => setSimulation(startSimulation(simulatedMachine))}
          />
        ) : (
          <Inspector core={activeCore} selection={selection} />
        )}
      </div>
    </div>
  );
}
