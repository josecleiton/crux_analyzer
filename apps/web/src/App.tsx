import { useEffect, useMemo, useRef, useState } from 'react';
import { loadProject } from './data/loadProject';
import type { DomainProject } from './domain/types';
import { machineOf } from './domain/fromParserJson';
import { declaredTags, focusFor } from './domain/focus';
import { stateRole } from './domain/stateRole';
import { toFlowModel } from './flow/toFlowModel';
import type { LayoutEngine, LayoutResult } from './layout/LayoutEngine';
import { ElkLayoutEngine } from './layout/ElkLayoutEngine';
import type { Selection } from './state/selection';
import { fromHash, resolveUrlState, toHash } from './state/urlSelection';
import type { Simulation } from './simulation/engine';
import {
  availableTransitions,
  fire,
  lastStep,
  goToStep,
  startSimulation,
  traveledPath,
} from './simulation/engine';
import { Graph } from './components/Graph/Graph';
import type { GraphHighlight } from './components/Graph/Graph';
import { Sidebar } from './components/Sidebar/Sidebar';
import { Inspector } from './components/Inspector/Inspector';
import { SimulationPanel } from './components/Simulation/SimulationPanel';
import { Toolbar } from './components/Toolbar/Toolbar';
import { useTranslate } from './i18n/useI18n';
import { useTheme } from './theme/useTheme';

const layoutEngine: LayoutEngine = new ElkLayoutEngine();

export default function App() {
  const [project, setProject] = useState<DomainProject | null>(null);
  const [activeCoreId, setActiveCoreId] = useState<string | null>(null);
  const [selection, setSelection] = useState<Selection>(null);
  const [simulation, setSimulation] = useState<Simulation | null>(null);
  const [tagQuery, setTagQuery] = useState('');
  const [undocumentedOnly, setUndocumentedOnly] = useState(false);
  const [layoutVersion, setLayoutVersion] = useState(0);
  const [layouted, setLayouted] = useState<LayoutResult>({ nodes: [], edges: [] });
  // Bumped when an explicit Re-layout lands, so the Graph re-frames: the
  // layout is deterministic, and recomputing alone would change nothing on
  // screen — re-framing is what the click visibly does.
  const [fitSignal, setFitSignal] = useState(0);
  const appliedLayoutVersion = useRef(0);
  const { theme, toggleTheme } = useTheme();
  const t = useTranslate();

  useEffect(() => {
    let cancelled = false;
    loadProject().then((loaded) => {
      if (cancelled) return;
      setProject(loaded);
      // a deep link (#state=Core/Machine/Name) lands selected; a stale or
      // foreign one falls back to the first core, nothing selected
      const initial = resolveUrlState(loaded, fromHash(window.location.hash));
      setActiveCoreId(initial.coreId ?? loaded.cores[0]?.id ?? null);
      setSelection(initial.selection);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  // The address bar mirrors the selection — replaceState, so casual clicking
  // does not pile up history entries. The default view keeps a clean URL.
  useEffect(() => {
    if (!project) return;
    const defaultCoreId = project.cores[0]?.id ?? null;
    const hash = toHash({
      coreId: activeCoreId === defaultCoreId && !selection ? null : activeCoreId,
      selection,
    });
    const base = window.location.pathname + window.location.search;
    window.history.replaceState(null, '', hash === '' ? base : base + hash);
  }, [project, activeCoreId, selection]);

  // A link pasted into the address bar applies without a reload.
  useEffect(() => {
    if (!project) return;
    const onHashChange = () => {
      const resolved = resolveUrlState(project, fromHash(window.location.hash));
      if (!resolved.coreId) return;
      if (resolved.coreId !== activeCoreId) {
        setActiveCoreId(resolved.coreId);
        setSimulation(null);
        setTagQuery('');
        setUndocumentedOnly(false);
      }
      setSelection(resolved.selection);
    };
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, [project, activeCoreId]);

  const activeCore = useMemo(
    () => project?.cores.find((core) => core.id === activeCoreId) ?? null,
    [project, activeCoreId],
  );

  // Re-mapped when the locale changes: the wildcard node's label feeds its
  // width estimate, so a longer translation has to be re-laid out, not restyled.
  const flowModel = useMemo(
    () =>
      activeCore
        ? toFlowModel(activeCore, { anyState: t('state.anyState') })
        : { nodes: [], edges: [] },
    [activeCore, t],
  );

  useEffect(() => {
    let cancelled = false;
    layoutEngine.layout(flowModel.nodes, flowModel.edges).then((result) => {
      if (cancelled) return;
      setLayouted(result);
      if (layoutVersion !== appliedLayoutVersion.current) {
        appliedLayoutVersion.current = layoutVersion;
        setFitSignal((signal) => signal + 1);
      }
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

  const tagOptions = useMemo(() => (activeCore ? declaredTags(activeCore) : []), [activeCore]);

  const highlight: GraphHighlight | undefined = useMemo(() => {
    // The simulation owns the emphasis while it runs; the reading filters
    // (tag query, undocumented-only) take over when it does not.
    if (!simulation || !simulatedMachine) {
      if (!activeCore) return undefined;
      const focus = focusFor(activeCore, { tagQuery, undocumentedOnly });
      if (!focus) return undefined;
      return {
        nodeIds: [],
        edgeIds: [],
        kept: { nodeIds: focus.stateIds, edgeIds: focus.transitionIds },
        dimOthers: true,
      };
    }
    const last = lastStep(simulation);
    const current = simulatedMachine.states.find((s) => s.id === simulation.currentStateId);
    const traveled = traveledPath(simulatedMachine, simulation);
    const next = availableTransitions(simulatedMachine, simulation);
    return {
      nodeIds: [simulation.currentStateId],
      edgeIds: last ? [last.transitionId] : [],
      visited: { nodeIds: traveled.stateIds, edgeIds: traveled.transitionIds },
      available: {
        // where each fireable transition starts (the current state, or the
        // wildcard pseudo-node) and where it lands
        nodeIds: next.flatMap((transition) => [transition.from, transition.to]),
        edgeIds: next.map((transition) => transition.id),
      },
      dimOthers: true,
      failure: current ? stateRole(simulatedMachine, current).failure : false,
      step: simulation.trail.length,
    };
  }, [simulation, simulatedMachine, activeCore, tagQuery, undocumentedOnly]);

  function selectCore(coreId: string) {
    setActiveCoreId(coreId);
    setSelection(null);
    setSimulation(null);
    // each core declares its own tags, so a filter does not survive the switch
    setTagQuery('');
    setUndocumentedOnly(false);
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

  /** Stand at step `steps` of the recorded run — the trail is navigable. */
  function goTo(steps: number) {
    if (!simulation || !simulatedMachine) return;
    setSimulation(goToStep(simulatedMachine, simulation, steps));
  }

  if (!project) {
    return <div className="app-loading">{t('app.loading')}</div>;
  }

  return (
    <div className="app">
      <Toolbar
        projectName={project.name}
        coreName={activeCore?.name ?? null}
        simulating={simulation !== null}
        theme={theme}
        tagQuery={tagQuery}
        tagOptions={tagOptions}
        undocumentedOnly={undocumentedOnly}
        onTagQueryChange={setTagQuery}
        onToggleUndocumented={() => setUndocumentedOnly((on) => !on)}
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
            fitSignal={fitSignal}
            theme={theme}
          />
        </main>
        {simulation && simulatedMachine ? (
          <SimulationPanel
            machine={simulatedMachine}
            simulation={simulation}
            onFire={fireTransition}
            onGoToStep={goTo}
            onRestart={() => setSimulation(startSimulation(simulatedMachine))}
          />
        ) : (
          <Inspector core={activeCore} selection={selection} />
        )}
      </div>
    </div>
  );
}
