import { useEffect, useRef } from 'react';
import type { ReactNode } from 'react';
import type { DomainCore, DomainMachine } from '../../domain/types';
import type { TreeEntry } from '../../domain/hierarchy';
import { familyId, machineTree } from '../../domain/hierarchy';
import { groupVisibility, hiddenInCore, machineStateIds } from '../../domain/visibility';
import type { Selection } from '../../state/selection';
import { useTranslate } from '../../i18n/useI18n';
import type { Translate } from '../../i18n/translate';

/**
 * This project's own repository — a hardcoded literal, never read out of the
 * analyzed source. The analyzer's chrome may point at the analyzer; nothing
 * the target app declares is allowed to become a link the UI offers.
 */
const REPOSITORY_URL = 'https://github.com/josecleiton/crux_analyzer';

interface SidebarProps {
  cores: DomainCore[];
  activeCoreId: string | null;
  /** Cores whose outline is open. The active core is opened when selected. */
  expandedCoreIds: ReadonlySet<string>;
  /** Machines and composite families folded shut — open is the default. */
  collapsedGroupIds: ReadonlySet<string>;
  hiddenStateIds: ReadonlySet<string>;
  selection: Selection;
  onSelectCore: (coreId: string) => void;
  onToggleCore: (coreId: string) => void;
  onToggleGroup: (groupId: string) => void;
  onSelect: (selection: Selection) => void;
  onSetStatesHidden: (stateIds: string[], hidden: boolean) => void;
  onIsolateStates: (coreId: string, stateIds: string[]) => void;
}

/**
 * The Cores panel, and inside each one the outline of what it contains:
 * machines, composite families, states. The outline is both a navigator —
 * clicking a state selects it on the canvas — and the visibility control:
 * a deselected checkbox takes the state off the canvas, along with the
 * transitions that can no longer be drawn.
 *
 * Everything is visible by default and the checkbox is what the reader turns
 * *off*: the panel opens showing the whole core, never a partial one.
 *
 * One click, one meaning, and inside the outline the name and the checkbox mean
 * different things: **the name isolates** what its row governs — that state,
 * that family, that machine, and nothing else of the core — while the checkbox
 * next to it turns that one row off and on without touching the others. Folding
 * is the arrow's alone, and presentation only: a folded machine keeps every one
 * of its states on the canvas.
 */
export function Sidebar({
  cores,
  activeCoreId,
  expandedCoreIds,
  collapsedGroupIds,
  hiddenStateIds,
  selection,
  onSelectCore,
  onToggleCore,
  onToggleGroup,
  onSelect,
  onSetStatesHidden,
  onIsolateStates,
}: SidebarProps) {
  const t = useTranslate();
  return (
    <nav className="sidebar">
      <h2 className="panel-title">{t('sidebar.cores')}</h2>
      <ul className="core-list">
        {cores.map((core) => {
          const expanded = expandedCoreIds.has(core.id);
          const hiddenHere = hiddenInCore(core, hiddenStateIds);
          return (
            <li key={core.id} className="core-entry">
              <div className={core.id === activeCoreId ? 'core-row active' : 'core-row'}>
                <button
                  className="core-twisty"
                  aria-expanded={expanded}
                  title={t(expanded ? 'sidebar.collapse' : 'sidebar.expand')}
                  onClick={() => onToggleCore(core.id)}
                >
                  <Chevron open={expanded} />
                </button>
                {/* Selecting a core opens it, so the name folds it back: on the
                    core already on screen the click reads as the arrow's. */}
                <button
                  className="core-item"
                  onClick={() =>
                    core.id === activeCoreId ? onToggleCore(core.id) : onSelectCore(core.id)
                  }
                >
                  {core.name}
                </button>
              </div>
              {expanded ? (
                <div className="core-tree">
                  {core.machines.map((machine) => (
                    <MachineOutline
                      key={machine.id}
                      core={core}
                      machine={machine}
                      collapsedGroupIds={collapsedGroupIds}
                      hiddenStateIds={hiddenStateIds}
                      selection={selection}
                      onSelectCore={onSelectCore}
                      onToggleGroup={onToggleGroup}
                      onSelect={onSelect}
                      onSetStatesHidden={onSetStatesHidden}
                      onIsolateStates={onIsolateStates}
                      t={t}
                    />
                  ))}
                  {hiddenHere.length > 0 ? (
                    <button
                      className="tree-reset"
                      onClick={() => onSetStatesHidden(hiddenHere, false)}
                    >
                      {t('sidebar.showAll')}
                    </button>
                  ) : null}
                </div>
              ) : null}
            </li>
          );
        })}
      </ul>
      {/* Bottom of the sidebar: out of the way of the reading, still reachable
          from every view. Same link hygiene as author prose in the inspector. */}
      <a
        className="sidebar-repo"
        href={REPOSITORY_URL}
        target="_blank"
        rel="noopener noreferrer"
        title={t('sidebar.sourceCode')}
      >
        <GitHubIcon />
        GitHub
      </a>
    </nav>
  );
}

interface MachineOutlineProps {
  core: DomainCore;
  machine: DomainMachine;
  collapsedGroupIds: ReadonlySet<string>;
  hiddenStateIds: ReadonlySet<string>;
  selection: Selection;
  onSelectCore: (coreId: string) => void;
  onToggleGroup: (groupId: string) => void;
  onSelect: (selection: Selection) => void;
  onSetStatesHidden: (stateIds: string[], hidden: boolean) => void;
  onIsolateStates: (coreId: string, stateIds: string[]) => void;
  t: Translate;
}

/** One machine: its own row, then its states in declaration order. */
function MachineOutline({
  core,
  machine,
  collapsedGroupIds,
  hiddenStateIds,
  selection,
  onSelectCore,
  onToggleGroup,
  onSelect,
  onSetStatesHidden,
  onIsolateStates,
  t,
}: MachineOutlineProps) {
  const tree = machineTree(machine);
  const allIds = machineStateIds(machine);

  /**
   * Reading something means reading it in its core, and alone: the rest of the
   * core leaves the canvas, and a row in a core that is not the active one
   * switches to it first — the same jump a deep link makes.
   */
  const isolate = (ids: string[]) => {
    onSelectCore(core.id);
    onIsolateStates(core.id, ids);
  };

  /** A leaf also fills the inspector — it is the row that has one. */
  const readState = (id: string) => {
    isolate([id]);
    onSelect({ kind: 'state', id });
  };

  const rows = (entries: TreeEntry[]) =>
    entries.map((entry) => {
      if (entry.kind === 'state') {
        const selected = selection?.kind === 'state' && selection.id === entry.state.id;
        return (
          <li key={entry.state.id} className="tree-row">
            {/* Leaves have nothing to fold; the gap keeps the checkboxes of a
                level aligned with the rows that do. */}
            <span className="tree-twisty" aria-hidden="true" />
            <VisibilityToggle
              ids={[entry.state.id]}
              name={entry.state.name}
              hiddenStateIds={hiddenStateIds}
              onSetStatesHidden={onSetStatesHidden}
              t={t}
            />
            <button
              className={selected ? 'tree-label selected' : 'tree-label'}
              // The outline is narrow, so a long name is elided in CSS: the
              // tooltip says what the click does *and* spells the name out —
              // the whole name, as declared.
              title={t('sidebar.showOnly', { name: entry.state.name })}
              onClick={() => readState(entry.state.id)}
            >
              {entry.label}
            </button>
          </li>
        );
      }

      const id = familyId(machine.id, entry.name);
      const open = !collapsedGroupIds.has(id);
      const familyIds = entry.children.map((leaf) => leaf.state.id);
      return (
        <li key={id} className="tree-family">
          <FoldableRow
            open={open}
            onToggle={() => onToggleGroup(id)}
            // A composite parent is a container, not a state: it has nothing of
            // its own to select, so its name reads the family — every child of
            // it, and nothing else of the core.
            onIsolate={() => isolate(familyIds)}
            labelClassName="tree-label container"
            name={entry.name}
            t={t}
          >
            <VisibilityToggle
              ids={familyIds}
              name={entry.name}
              hiddenStateIds={hiddenStateIds}
              onSetStatesHidden={onSetStatesHidden}
              t={t}
            />
          </FoldableRow>
          {open ? (
            <ul className="tree-children">
              {rows(entry.children.map((leaf) => ({ kind: 'state' as const, ...leaf })))}
            </ul>
          ) : null}
        </li>
      );
    });

  const open = !collapsedGroupIds.has(machine.id);
  return (
    <div className="tree-machine">
      <FoldableRow
        open={open}
        onToggle={() => onToggleGroup(machine.id)}
        onIsolate={() => isolate(allIds)}
        labelClassName="tree-label machine"
        name={machine.name}
        t={t}
      >
        <VisibilityToggle
          ids={allIds}
          name={machine.name}
          hiddenStateIds={hiddenStateIds}
          onSetStatesHidden={onSetStatesHidden}
          t={t}
        />
      </FoldableRow>
      {open ? <ul className="tree-states">{rows(tree.entries)}</ul> : null}
    </div>
  );
}

interface FoldableRowProps {
  open: boolean;
  onToggle: () => void;
  /** What the name does: read this group alone, the rest of the core off. */
  onIsolate: () => void;
  labelClassName: string;
  /** Name of the machine or family — analyzed-app data, never translated. */
  name: string;
  t: Translate;
  /** The row's visibility checkbox, between the arrow and the name. */
  children: ReactNode;
}

/**
 * A row that has children: the arrow folds it, the name reads it alone. The two
 * are deliberately not the same click — folding says nothing about what is
 * drawn, while the name is the group-sized version of what a leaf's name does.
 */
function FoldableRow({
  open,
  onToggle,
  onIsolate,
  labelClassName,
  name,
  t,
  children,
}: FoldableRowProps) {
  const fold = t(open ? 'sidebar.collapse' : 'sidebar.expand');
  return (
    <div className="tree-row">
      <button className="tree-twisty" aria-expanded={open} title={fold} onClick={onToggle}>
        <Chevron open={open} />
      </button>
      {children}
      <button
        className={labelClassName}
        title={t('sidebar.showOnly', { name })}
        onClick={onIsolate}
      >
        {name}
      </button>
    </div>
  );
}

interface VisibilityToggleProps {
  /** The states this checkbox governs: one, a family, or a whole machine. */
  ids: string[];
  /** Name of what is being toggled — analyzed-app data, never translated. */
  name: string;
  hiddenStateIds: ReadonlySet<string>;
  onSetStatesHidden: (stateIds: string[], hidden: boolean) => void;
  t: Translate;
}

/**
 * A checkbox over a group of states. Partly-hidden groups read as mixed, which
 * only the DOM property expresses — React has no `indeterminate` attribute.
 */
function VisibilityToggle({
  ids,
  name,
  hiddenStateIds,
  onSetStatesHidden,
  t,
}: VisibilityToggleProps) {
  const box = useRef<HTMLInputElement>(null);
  const visibility = groupVisibility(ids, hiddenStateIds);
  useEffect(() => {
    if (box.current) box.current.indeterminate = visibility === 'some';
  }, [visibility]);

  // Mixed counts as on: the next click hides the rest of the group, matching
  // what the checkmark offers to undo.
  const checked = visibility !== 'none';
  const label = t(checked ? 'sidebar.hideFromCanvas' : 'sidebar.showOnCanvas', { name });
  return (
    <input
      ref={box}
      type="checkbox"
      className="tree-check"
      checked={checked}
      title={label}
      aria-label={label}
      onChange={(event) => onSetStatesHidden(ids, !event.target.checked)}
    />
  );
}

/** Disclosure arrow: rotated by CSS, so one shape serves both directions. */
function Chevron({ open }: { open: boolean }) {
  return (
    <svg
      className={open ? 'chevron open' : 'chevron'}
      viewBox="0 0 10 10"
      width="10"
      height="10"
      aria-hidden="true"
    >
      <path d="M3 1.5 L7 5 L3 8.5" fill="none" stroke="currentColor" strokeWidth="1.6" />
    </svg>
  );
}

/** The GitHub mark, inlined: the CSP allows no remote image. */
function GitHubIcon() {
  return (
    <svg viewBox="0 0 16 16" width="15" height="15" fill="currentColor" aria-hidden="true">
      <path d="M8 0a8 8 0 0 0-2.53 15.59c.4.07.55-.17.55-.38v-1.34c-2.22.48-2.69-1.07-2.69-1.07-.36-.93-.89-1.18-.89-1.18-.73-.5.05-.49.05-.49.8.06 1.23.83 1.23.83.71 1.22 1.87.87 2.33.67.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82a7.6 7.6 0 0 1 4 0c1.53-1.03 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.28.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48v2.19c0 .21.15.46.55.38A8 8 0 0 0 8 0Z" />
    </svg>
  );
}
