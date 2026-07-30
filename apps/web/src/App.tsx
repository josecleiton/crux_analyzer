import { useEffect, useMemo, useRef, useState } from 'react';
import { loadProject } from './data/loadProject';
import type { DomainProject } from './domain/types';
import { machineOf } from './domain/fromParserJson';
import { declaredTags, focusFor } from './domain/focus';
import { NOTHING_HIDDEN, isOnCanvas, machineStateIds, withHidden } from './domain/visibility';
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

/** A set with one more member — the immutable update React state wants. */
function union(set: ReadonlySet<string>, member: string): ReadonlySet<string> {
  return new Set(set).add(member);
}

/** The same set with `member` added if it was absent, removed if it was there. */
function toggled(set: ReadonlySet<string>, member: string): ReadonlySet<string> {
  const next = new Set(set);
  if (!next.delete(member)) next.add(member);
  return next;
}

export default function App() {
  const [project, setProject] = useState<DomainProject | null>(null);
  const [activeCoreId, setActiveCoreId] = useState<string | null>(null);
  const [selection, setSelection] = useState<Selection>(null);
  const [simulation, setSimulation] = useState<Simulation | null>(null);
  const [tagQuery, setTagQuery] = useState('');
  const [undocumentedOnly, setUndocumentedOnly] = useState(false);
  // Reader-hidden states, by id. Held for the whole project rather than per
  // core: the ids are absolute, so what the reader trimmed in one core is still
  // trimmed when they come back to it.
  const [hiddenStateIds, setHiddenStateIds] = useState<ReadonlySet<string>>(NOTHING_HIDDEN);
  // Cores whose outline is open. Selecting a core opens it, so the panel starts
  // showing the states of the core on screen.
  const [expandedCoreIds, setExpandedCoreIds] = useState<ReadonlySet<string>>(
    () => new Set<string>(),
  );
  // Machines and composite families the reader folded shut. Open is the default,
  // so this holds the exceptions — and it is presentation only: a folded machine
  // keeps every one of its states on the canvas.
  const [collapsedGroupIds, setCollapsedGroupIds] = useState<ReadonlySet<string>>(
    () => new Set<string>(),
  );
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
      const coreId = initial.coreId ?? loaded.cores[0]?.id ?? null;
      setActiveCoreId(coreId);
      setSelection(initial.selection);
      // The core on screen opens its outline; the others stay collapsed, so a
      // project with many cores still shows its list at a glance.
      if (coreId) setExpandedCoreIds(new Set([coreId]));
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
        ? toFlowModel(activeCore, { anyState: t('state.anyState') }, hiddenStateIds)
        : { nodes: [], edges: [] },
    [activeCore, t, hiddenStateIds],
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
      step: simulation.trail.length,
    };
  }, [simulation, simulatedMachine, activeCore, tagQuery, undocumentedOnly]);

  function selectCore(coreId: string) {
    // Its own outline is what the panel shows for the core on screen.
    setExpandedCoreIds((expanded) => (expanded.has(coreId) ? expanded : union(expanded, coreId)));
    if (coreId === activeCoreId) return;
    setActiveCoreId(coreId);
    setSelection(null);
    setSimulation(null);
    // each core declares its own tags, so a filter does not survive the switch
    setTagQuery('');
    setUndocumentedOnly(false);
  }

  function toggleCoreExpanded(coreId: string) {
    setExpandedCoreIds((expanded) => toggled(expanded, coreId));
  }

  /** Folds a machine or a composite family in the outline, or opens it again. */
  function toggleGroup(groupId: string) {
    setCollapsedGroupIds((collapsed) => toggled(collapsed, groupId));
  }

  /**
   * Takes states off the canvas, or puts them back. A selection that loses what
   * it pointed at goes with them: an inspector describing a state nobody can see
   * is worse than an empty one.
   */
  function setStatesHidden(stateIds: string[], hidden: boolean) {
    const next = withHidden(hiddenStateIds, stateIds, hidden);
    if (next === hiddenStateIds) return;
    setHiddenStateIds(next);
    // The graph changes size, so the frame has to follow: this bump makes the
    // viewport re-fit once the new layout lands, the same way Re-layout does.
    setLayoutVersion((version) => version + 1);
    if (activeCore && selection && !isOnCanvas(activeCore, selection.kind, selection.id, next)) {
      setSelection(null);
    }
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
    // A run needs its machine whole: replaying through states the reader trimmed
    // away would highlight nothing. Trimming again mid-run stays allowed.
    setStatesHidden(machineStateIds(machine), false);
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
          expandedCoreIds={expandedCoreIds}
          collapsedGroupIds={collapsedGroupIds}
          hiddenStateIds={hiddenStateIds}
          selection={selection}
          onSelectCore={selectCore}
          onToggleCore={toggleCoreExpanded}
          onToggleGroup={toggleGroup}
          onSelect={setSelection}
          onSetStatesHidden={setStatesHidden}
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
