# Web UI

`apps/web` — React + TypeScript + React Flow + ELKJS. Start it with
`just dev` (or `pnpm --filter web dev`).

## What it shows

Three areas, LangGraph-Studio style:

- **Sidebar** — the project's Cores. Selecting one renders its machines.
- **Canvas** — the state machines. A core with several machines (orthogonal
  regions) renders each as a **titled section**; a single-machine core
  renders flat. Every state is a node, every transition an edge labeled with
  its event. Wildcard transitions (`from`/`to` = `"*"`) connect to a dashed
  **any state** pseudo-node. Composite leaves show as `Parent / Child`.
- **Inspector** (right panel) — selecting a state shows its incoming/outgoing
  events; selecting a transition shows `event: from ↓ to` plus the
  **effects** it requests. The owning machine is tagged when the core has
  more than one.

## Data source

On load the app fetches `/model.json` (put a generated model at
`apps/web/public/model.json` — `just model <src> <name>`, or `model-watch`
to keep it fresh). Without one — or with a stale/invalid one — it falls back
to the bundled example (`shared/schema/examples/audio-recorder.json`) and
logs a console warning. The artifact is gitignored.

## Simulation

Select a state (optional) and hit **Simulate**:

- the right panel switches to the simulation: current state, the events that
  can fire from it (wildcard-sourced ones are always available;
  runtime-target `to: "*"` transitions are excluded from replay), and the
  trail of what already fired;
- the canvas highlights the current state and the last transition taken in
  green;
- **Restart** goes back to the machine's first state; **Stop simulation**
  returns to the inspector.

The engine (`src/simulation/engine.ts`) is pure domain logic; it drives the
Graph exclusively through highlight props.

## Theming

The toolbar's theme toggle switches between light and dark. The active theme
is the `data-theme` attribute on `<html>`; every color is a CSS custom
property defined per theme in `src/index.css`, so adding a theme is one
`:root[data-theme='...']` block. The choice persists in `localStorage`
(a pre-paint script in `index.html` applies it before first render — no
flash), and with no explicit choice the app follows the OS preference live.
SVG-only colors (edge arrowheads) are read back from the same tokens
(`src/theme/theme.ts`), keeping CSS the single source of truth.

## Layout

All geometry comes from the `LayoutEngine` interface
(`src/layout/LayoutEngine.ts`): node positions, orthogonal edge routes with
rounded corners, and the label box each edge label occupies — ELK computes
them (`ElkLayoutEngine`, `elk.algorithm: layered` with inline edge labels),
so edges never cross nodes and labels never overlap. Machine sections use
ELK's hierarchical layout (React Flow group nodes with relative child
positions). Nodes are not draggable — routes are engine-owned; use
**Re-layout** to recompute.

## Extending

- New visualization of the same data: consume the domain model
  (`src/domain/types.ts`), not React Flow types.
- New layout algorithm: implement `LayoutEngine`, swap it in `App.tsx`.
- New highlight-driven feature (à la simulation): compute ids, pass them via
  the Graph's `highlight` prop — do not modify the Graph.
