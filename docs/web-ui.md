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
  **any state** pseudo-node. A **composite state** renders as a container
  holding its leaves — the same nesting the Mermaid output shows; the parent
  is never a state of its own, so the container selects nothing (and a
  machine that somehow declares a plain state colliding with a parent's name
  keeps that family flat). Clicking a section (its title or empty area)
  selects the machine's **entry state** and frames that machine in the
  viewport, so a machine can be inspected — and simulated — in one click.
- **Inspector** (right panel) — selecting a state shows its role badges, the
  description and tags authored on it in the analyzed source, its
  incoming/outgoing events (documented ones carry a mark and a tooltip), and
  **Effects on entry**: the union of the effects its incoming transitions
  request — "some of these fire", never "all". Selecting a transition shows
  the event's own authored description, `from ↓ to`, plus the **effects** it
  requests. Either way the machine's own description closes the panel. The
  owning machine is tagged when the core has more than one.

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
- **failure** (red) — declared, then guessed. A `@failure` marker in the
  state's doc comment in the analyzed source is authoritative: it travels in the
  model as data, so it is the *author's* statement and the parser honesty rule
  holds — nothing was invented. When a machine declares no failure of its own,
  the naming heuristic stands in (`Failed`, `Error`, `Denied`, `Rejected`,
  `Invalid`, `TimedOut`, …): the only guess of the four, which is why it lives
  in the UI (`isFailureName`) and never in the parser. One `@failure` anywhere
  in a machine silences the heuristic for that whole machine — from then on an
  unmarked state is unmarked deliberately. A state that is both failure and
  final keeps the double border in red.
- **deprecated** (amber, dashed border) — declared only, from `@deprecated`. No
  heuristic backs it and none should: a name never says a state is on its way
  out. The panels also strike the name through. Dashed rather than dimmed,
  because dimming already means "outside the simulation's reach".

The Inspector and the simulation panel repeat the roles as badges.

## Documentation from the source

Doc comments on the analyzed app's state enum reach the model and are rendered
**as-is** — they are that application's own prose, so they are never translated
(see [i18n.md](i18n.md)). Only the headings around them are.

On the canvas a documented state carries a small three-line mark after its
label and shows its description as a native tooltip; a section box does the
same for the state enum's own description. `title` rather than a hover card on
purpose: React Flow scales its node pane, so a card inside a node blurs and one
outside needs a portal positioned against the transform.

The Inspector and the simulation panel show the full text with paragraph breaks
preserved, plus any free-form `@tag` values as monospace chips — monospace
because a tag is data from the analyzed app, unlike the uppercase role badges,
which are this UI's own vocabulary. A state's description sits directly under
its name with no heading; the machine's own description comes last, under
*About this machine*, together with any markers declared on the region.

Markdown inside a doc comment **renders as Markdown** in the panels
(react-markdown): code spans, lists, emphasis — the same reading the
generated document always gave it. Raw HTML in author prose is deliberately
left inert (shown as text, never executed — react-markdown builds React
elements and injects no HTML), and hard-wrapped `///` lines rejoin naturally,
since Markdown treats a single newline as a soft break. Node and section
tooltips are native `title` attributes, so there the prose stays plain text.

That prose comes from whatever repository was analyzed, so three limits are
spelled out rather than inherited from the library's defaults — see
[security.md](security.md#1-author-prose-is-untrusted-text-everywhere-it-lands),
and `StateDoc.test.tsx` if you are about to change them:

- **link targets** may only be `http`, `https` or `mailto`, and open with
  `rel="noopener noreferrer nofollow"`. A `javascript:` link renders as inert
  text.
- **images are never fetched.** An `![](https://host/pixel.png)` in a doc
  comment would report every reader of a published document to that host, so
  the alt text is shown in its place.
- **raw HTML has no path to the DOM.** No `dangerouslySetInnerHTML`, and
  `rehype-raw` must not be added.

## Filtering the canvas

Two reading filters. Both say "these, not the rest" the same way the
simulation does: the matching states stay at full strength while every other
state and transition fades back.

- **Filter by tag** — the input beside the title (it reads while the toolbar
  buttons act): type a fragment of a declared `@tag` name, case-insensitive.
  It carries its own suggestion list — most-used tags first, opened on focus
  — rather than a native `<datalist>`, whose popup is inconsistent across
  engines. A tag declared on the state enum covers its whole region. The
  input only renders when the core declares any tag — with nothing to filter
  by there is no filter.
- **Undocumented** — an opt-in toggle (amber warning triangle; amber when
  active — green belongs to the simulation) that keeps only the states with
  no authored description: the states a reader should not trust yet. Opt-in
  on purpose, so the default view stays about the machine rather than about
  documentation coverage (the number itself comes from
  `crux-analyzer coverage`, see [cli.md](cli.md)).

Both criteria compose as an intersection. A transition stays readable only
when everything it connects stays; on a wildcard edge the **any state**
pseudo-node counts as matching — "any state" includes the kept ones — so
`* → Ready` survives whenever `Ready` does.

The filters are pure domain logic (`src/domain/focus.ts`) and reach the Graph
through the same highlight prop the simulation drives — a quiet `kept` tier
that only escapes the dimming, so a filter match never borrows the
simulation's colors. While a simulation runs the filters are disabled: the
emphasis belongs to the replay. Switching cores clears them — each core
declares its own tags.

## Deep links

The selection lives in the URL hash — `#state=Core/Machine/Name`,
`#transition=<id>`, `#core=<name>` — so "this state of this machine" is a
link that can be pasted in a review. Clicking mirrors into the address bar via
`replaceState` (no history pile-up), the default view keeps a clean URL, a
pasted hash applies without a reload, and a stale or foreign link falls back
to the core (or to nothing) instead of a broken UI. Hash-based on purpose:
the static deployment has no router and no SPA fallback rule, and a hash
survives any host untouched (`src/state/urlSelection.ts`).

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
  can fire from it (wildcard-sourced ones are always available), and the
  trail of what already fired. Runtime-target `to: "*"` transitions cannot
  be replayed — there is nothing static to land on — so they are listed
  inert under the fireable ones with a note saying exactly that, rather than
  silently hidden;
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
(`en` / `pt-BR`); it shows the short code of the **active** locale, while its
tooltip and accessible name say which language clicking it brings.
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
positions), and grouping is arbitrary-depth: composite containers nest inside
sections, with each machine laid out as one hierarchical run
(`INCLUDE_CHILDREN`) so edges may cross a composite's boundary, and every
edge declared in the lowest common ancestor of its endpoints. Nodes are not
draggable — routes are engine-owned. **Re-layout** recomputes *and*
re-frames the viewport: the layout is deterministic, so recomputing alone
would change nothing visible.

## Extending

- New visualization of the same data: consume the domain model
  (`src/domain/types.ts`), not React Flow types.
- New layout algorithm: implement `LayoutEngine`, swap it in `App.tsx`.
- New highlight-driven feature (à la simulation): compute ids, pass them via
  the Graph's `highlight` prop — do not modify the Graph.
