# Web UI

> 🌐 **English** · [Português (Brasil)](pt-BR/web-ui.md)

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
  Clicking a section (its title or empty area) selects the machine's **entry
  state** and frames that machine in the viewport, so a machine can be
  inspected — and simulated — in one click.
- **Inspector** (right panel) — selecting a state shows its role badges and
  its incoming/outgoing events; selecting a transition shows
  `event: from ↓ to` plus the **effects** it requests. The owning machine is
  tagged when the core has more than one.

## State roles

Roles are painted on the canvas at all times, simulation or not
(`src/domain/stateRole.ts`):

- **initial** (blue, filled dot before the label) — the machine's entry
  point: a state nothing transitions into. In a fully cyclic machine the
  first state carries the role, which is where the simulation starts. The
  first state with this role is the machine's entry state (`entryState`).
- **final** (violet, double border) — a dead end: no outgoing transition of
  its own. A machine-wide wildcard (`from: "*"`) may still leave it; that
  escape stays visible as an edge from the **any state** node.
- **failure** (red) — a naming heuristic, the only guess of the three: the
  state's words include a failure word (`Failed`, `Error`, `Denied`,
  `Rejected`, `Invalid`, `TimedOut`, …). It never reaches the parser, which
  must not invent semantics; a state that is both failure and final keeps the
  double border in red.

The Inspector and the simulation panel repeat the roles as badges.

## Data source

On load the app fetches `model.json` relative to its base (see
[Static deployment](#static-deployment) — `/model.json` in dev). Put a
generated model at `apps/web/public/model.json` — `just model <src> <name>`,
or `model-watch` to keep it fresh. Without one — or with a stale/invalid
one — it falls back to the bundled example
(`shared/schema/examples/audio-recorder.json`) and logs a console warning.
The artifact is gitignored.

## Static deployment

The UI is a static bundle, so publishing it as internal documentation needs
no server-side logic — only a plain HTTP host. One recipe does everything:

```sh
just site ../my-app/shared/src MyApp              # served from the domain root
just site ../my-app/shared/src MyApp /crux-docs/  # served from a subpath
# then publish apps/web/dist/
```

`site` analyzes the crate into `apps/web/public/model.json` and only then
builds, so the model ships *inside* `dist/` — the published page never calls
back to the analyzer, and refreshing the docs means re-running the recipe.
Both steps live in one recipe deliberately: building without generating first
publishes the bundled example, which looks like a working site instead of an
error.

The third argument is Vite's `base` (`CRUX_BASE=<base>` for raw `pnpm build`
invocations, normalized in `vite.config.ts`). It is **required whenever the
site is not at the domain root** — per-project GitHub/GitLab Pages, for
instance: asset URLs and the `model.json` fetch are both resolved from it, and
a root-absolute build under a subpath fails silently into the bundled example.
A full origin (`https://cdn.example.com/docs/`) works too.

Two things not to expect: the bundle must be served over HTTP (`file://`
blocks both the ES module and the model fetch), and no SPA fallback rule is
needed — there is a single page and no router.

## Simulation

Select a state (optional) and hit **Simulate**:

- the right panel switches to the simulation: current state, the events that
  can fire from it (wildcard-sourced ones are always available;
  runtime-target `to: "*"` transitions are excluded from replay), and the
  trail of what already fired;
- the canvas reads as a path, in three tiers of emphasis: everything already
  **traveled** is bold green (states and transitions, starting state
  included), what can **fire from here** keeps a green outline, and everything
  else fades back — including the other machines' sections;
- the current state and the last transition taken are the strongest of all,
  and the step is animated: the transition's stroke flows as dashes with a
  pulse traveling along its route, the state that was just entered pops and
  then breathes, and the new trail entry slides in;
- landing on a **failure** state turns that whole highlight red (edge, label,
  arrowhead, ring), so failure paths stand out from healthy ones;
- the viewport follows the replay: when the state just entered is not fully in
  view, the canvas pans to center it, keeping the zoom untouched — a step that
  lands on screen never moves the canvas;
- **Restart** goes back to the machine's first state; **Stop simulation**
  returns to the inspector.

Every animation is skipped under `prefers-reduced-motion`, viewport moves
included (`src/components/Graph/ViewportFocus.tsx` reads the preference in JS,
since a CSS rule cannot silence a scripted tween).

The engine (`src/simulation/engine.ts`) is pure domain logic; it drives the
Graph exclusively through highlight props — `traveledPath` and
`availableTransitions` are the facts, the Graph only maps them to emphasis
tiers.

## Theming

The toolbar's theme toggle switches between light and dark. The active theme
is the `data-theme` attribute on `<html>`; every color is a CSS custom
property defined per theme in `src/index.css`, so adding a theme is one
`:root[data-theme='...']` block. The choice persists in `localStorage`
(a pre-paint script in `index.html` applies it before first render — no
flash), and with no explicit choice the app follows the OS preference live.
SVG-only colors (edge arrowheads) are read back from the same tokens
(`src/theme/theme.ts`), keeping CSS the single source of truth.

## Localization

The toolbar's language toggle switches between English and Portuguese
(`en` / `pt-BR`); it shows the short code of the locale it will switch *to*.
The module (`src/i18n/`) mirrors the theme deliberately: the active locale is
the `data-locale` attribute on `<html>` (with `lang` set alongside it for
assistive technology), the choice persists in `localStorage`, a pre-paint script
in `index.html` applies it before first render, and with no explicit choice the
app follows `navigator.languages` — any Portuguese resolves to `pt-BR`.

Two differences from theming are worth knowing:

- translations reach components through **context** (`I18nProvider` in
  `main.tsx`), not props — every panel needs `t`, while only two components
  need the theme;
- switching locale **re-runs layout**. Node widths are estimated from the
  label text, so the translated `any state` / `qualquer estado` pseudo-node
  changes geometry; `toFlowModel` receives the label as a `FlowLabels`
  parameter rather than importing the catalog, keeping the mapping layers
  language-free.

State, event, effect, machine and core names are never translated — they are
identifiers from the analyzed app. The monospace/sans-serif split in
`index.css` mirrors that distinction. See [i18n.md](i18n.md).

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
