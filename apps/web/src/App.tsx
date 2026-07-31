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
import { ReviewPanel } from './components/Proposal/ReviewPanel';
import { useProposal } from './proposal/useProposal';
import { annotateFlowModel } from './proposal/annotate';
import { useI18n } from './i18n/useI18n';
import { useTheme } from './theme/useTheme';

const layoutEngine: LayoutEngine = new ElkLayoutEngine();

function union(set: ReadonlySet<string>, member: string): ReadonlySet<string> {
  return new Set(set).add(member);
}

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
  const [hiddenStateIds, setHiddenStateIds] = useState<ReadonlySet<string>>(NOTHING_HIDDEN);
  const [expandedCoreIds, setExpandedCoreIds] = useState<ReadonlySet<string>>(
    () => new Set<string>()
  );
  const [collapsedGroupIds, setCollapsedGroupIds] = useState<ReadonlySet<string>>(
    () => new Set<string>()
  );
  const [layoutVersion, setLayoutVersion] = useState(0);
  const [layouted, setLayouted] = useState<LayoutResult>({ nodes: [], edges: [] });
  const [fitSignal, setFitSignal] = useState(0);
  const [showReviewPanel, setShowReviewPanel] = useState(false);

  const appliedLayoutVersion = useRef(0);
  const { theme, toggleTheme } = useTheme();
  const { locale, t } = useI18n();

  useEffect(() => {
    let cancelled = false;
    loadProject().then((loaded) => {
      if (cancelled) return;
      setProject(loaded);
      const initial = resolveUrlState(loaded, fromHash(window.location.hash));
      const coreId = initial.coreId ?? loaded.cores[0]?.id ?? null;
      setActiveCoreId(coreId);
      setSelection(initial.selection);
      if (coreId) setExpandedCoreIds(new Set([coreId]));
    });
    return () => {
      cancelled = true;
    };
  }, []);

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
    [project, activeCoreId]
  );

  // Integrate proposal hook
  const {
    isProposing,
    proposal,
    projectedCore,
    changeSet,
    isDirty,
    isStale,
    canUndo,
    canRedo,
    toggleProposalMode,
    addOp,
    undo,
    redo,
    setNote,
    discard,
  } = useProposal(activeCore);

  const handleTogglePropose = () => {
    if (isProposing && isDirty) {
      const confirmExit = window.confirm(t('proposal.confirmExit'));
      if (!confirmExit) return;
    }
    toggleProposalMode();
  };

  const handleDiscardProposal = () => {
    if (window.confirm(t('proposal.confirmDiscard'))) {
      discard();
    }
  };

  const displayedCore = useMemo(() => {
    if (isProposing && projectedCore) {
      return projectedCore;
    }
    return activeCore;
  }, [isProposing, projectedCore, activeCore]);

  const flowModel = useMemo(() => {
    if (!displayedCore) return { nodes: [], edges: [] };
    const rawModel = toFlowModel(displayedCore, { anyState: t('state.anyState') }, hiddenStateIds);
    if (isProposing && changeSet) {
      return annotateFlowModel(rawModel, changeSet);
    }
    return rawModel;
  }, [displayedCore, t, hiddenStateIds, isProposing, changeSet]);

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
    [activeCore, simulation]
  );

  const tagOptions = useMemo(() => (activeCore ? declaredTags(activeCore) : []), [activeCore]);

  const highlight: GraphHighlight | undefined = useMemo(() => {
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
        nodeIds: next.flatMap((transition) => [transition.from, transition.to]),
        edgeIds: next.map((transition) => transition.id),
      },
      dimOthers: true,
      step: simulation.trail.length,
    };
  }, [simulation, simulatedMachine, activeCore, tagQuery, undocumentedOnly]);

  function selectCore(coreId: string) {
    setExpandedCoreIds((expanded) => (expanded.has(coreId) ? expanded : union(expanded, coreId)));
    if (coreId === activeCoreId) return;
    if (isProposing && isDirty) {
      const confirmExit = window.confirm(t('proposal.confirmExit'));
      if (!confirmExit) return;
    }
    setActiveCoreId(coreId);
    setSelection(null);
    setSimulation(null);
    setTagQuery('');
    setUndocumentedOnly(false);
  }

  function toggleCoreExpanded(coreId: string) {
    setExpandedCoreIds((expanded) => toggled(expanded, coreId));
  }

  function toggleGroup(groupId: string) {
    setCollapsedGroupIds((collapsed) => toggled(collapsed, groupId));
  }

  function setStatesHidden(stateIds: string[], hidden: boolean) {
    const next = withHidden(hiddenStateIds, stateIds, hidden);
    if (next === hiddenStateIds) return;
    setHiddenStateIds(next);
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
    const machine =
      (selection?.kind === 'state' ? machineOf(activeCore, selection.id) : null) ??
      activeCore.machines[0];
    const initialState = selection?.kind === 'state' ? selection.id : undefined;
    setSelection(null);
    setStatesHidden(machineStateIds(machine), false);
    setSimulation(startSimulation(machine, initialState));
  }

  function fireTransition(transitionId: string) {
    if (!simulation || !simulatedMachine) return;
    setSimulation(fire(simulatedMachine, simulation, transitionId));
  }

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
        isProposing={isProposing}
        changeCount={changeSet?.totalChanges || 0}
        canUndo={canUndo}
        canRedo={canRedo}
        isStale={isStale}
        onTogglePropose={handleTogglePropose}
        onOpenReview={() => setShowReviewPanel(true)}
        onUndo={undo}
        onRedo={redo}
        onDiscard={handleDiscardProposal}
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
          <Inspector
            core={displayedCore}
            selection={selection}
            isProposing={isProposing}
            onAddOp={addOp}
          />
        )}
      </div>

      {showReviewPanel ? (
        <ReviewPanel
          changeSet={changeSet}
          note={proposal?.note || ''}
          onNoteChange={setNote}
          onClose={() => setShowReviewPanel(false)}
          locale={locale === 'pt-BR' ? 'pt-BR' : 'en'}
        />
      ) : null}
    </div>
  );
}
