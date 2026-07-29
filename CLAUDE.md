# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Status

The original roadmap is fully implemented: syn-based parser (predicate guards, `==`/closure guards, `Default` resets, wildcard sources AND targets, value-flow analysis for runtime-assigned targets, hierarchical/composite states as `Parent/Child` paths, per-transition effects, multiple state machines per core, plus state/machine documentation and `@failure` / `@deprecated` / `@tag` annotations read from `///` doc comments), CLI (`crux-analyzer generate | docs`, both with `--watch`), doc generators (Mermaid with composite blocks and per-state notes, Markdown with a states table), and the web UI (machine sections, nested composite containers, inspector with effects — per transition and aggregated per state — and authored documentation rendered as Markdown, event/effect doc catalogs, Simulation Engine, tag filter + undocumented-states highlight, selection-in-URL deep links). The VS Code extension (`apps/vscode`) embeds the built web bundle in a webview, spawning the CLI and regenerating on save. The corpus test against a real app is gated on `QUIPU_SRC` and extracts it warning-free. `init.md` is the original project spec (in Portuguese).

## Documentation

The full documentation set lives in `docs/` (index at `docs/README.md`): architecture, parser semantics + warnings reference, schema contract, CLI, web UI, internationalization, development guide, and a CLI-generated example at `docs/examples/mini-recorder.md` (regenerate with `just example-docs`). `docs/pt-BR/` is the Portuguese mirror of that set. Keep it in sync: parser semantics changes update `docs/parser.md`; schema changes update `docs/schema.md`; new commands update `docs/cli.md` and the `Justfile`; new user-facing strings update the locale catalogs and, when the rules change, `docs/i18n.md`. A change to an English doc should update its `docs/pt-BR/` counterpart too.

## Conventions

- **English is the source language**: all git commit messages and all code — identifiers, comments, doc comments, schema descriptions — must be written in English.
- **User-facing text is localized**: UI strings, CLI output, parser diagnostics and generated-document labels live in the locale catalogs (`en` + `pt-BR`), never as inline literals. Add a key instead of a string. Identifiers read out of the analyzed application (core/machine/state/event/effect names) are data and are never translated. See `docs/i18n.md`.
- **Parser honesty rule**: what cannot be inferred statically is surfaced as a `Warning` (never silently dropped, never guessed). An assignment with *no* state evidence legitimately fires from any state (`"*"`). Reading what the source *declares* is not guessing — doc comments and their `@failure` / `@deprecated` / `@tag` annotations are evidence and belong in the model; name-shaped inferences stay in the clients (`apps/web/src/domain/stateRole.ts`).
- **Security rule**: the analyzed source tree is untrusted input, and so is everything read out of it — prose, identifiers, paths. `docs/security.md` is the full contract (threat model, rules, guaranteed properties); the short form is: author prose never becomes markup (no `dangerouslySetInnerHTML`, no `rehype-raw`, escape `<`/`&` in generated Markdown); identifiers never influence a filesystem path; every unbounded input dimension has a cap and every cap that fires emits a `Warning` (the honesty rule, applied to resources); subprocesses take an argv array, never a shell string; new dependencies pass `just security`, which `just check` runs. Removing one of the guaranteed properties in that document is a design change, not a refactor.

## Commands

Tasks are driven by the root `justfile` (`just` lists everything):

```sh
just dev                     # web UI (Vite, apps/web)
just web-test                # vitest: mapping layers + simulation engine
just rust-test               # cargo tests (parser unit + fixture + docgen)
just corpus                  # + real-app corpus test (QUIPU_SRC overrides the path)
just quipu-coverage [floor]  # corpus documentation ratchet (skips if corpus absent)
just clippy                  # lint the workspace
just check                   # full validation: corpus + clippy + web tests + build
just model <src> <name>      # analyze an app into apps/web/public/model.json
just model-watch <src> <name># same, regenerating on every save
just site <src> <name> [base] # static doc site in apps/web/dist (model baked in)
just docs <src> <name> [markdown|mermaid] [en|pt-BR]
just quipu                   # shortcut: analyze the Quipu corpus into the UI
just example-docs            # regenerate the example docs in every locale
just ext-test | ext-build | ext-package   # VS Code extension (tests, build, .vsix)
```

Raw `cargo`/`pnpm` equivalents are documented in `docs/development.md`. The full increment validation pipeline (tests + live UI check with screenshots + English commits + push) is described there too.

## What Crux Analyzer Is

The project is named **crux_analyzer** (the spec in `init.md` uses the older working name "Crux Studio").

A **semantic analyzer** (not a diagram generator) that turns Rust + Crux applications into living documentation. It parses Rust source via the `syn` AST and builds an intermediate semantic model. The React web app is just one client of that model — the CLI doc generators and the VS Code extension are others; more formats (PlantUML, HTML) are possible future clients.

The project must **not** depend on Crux itself — it only analyzes Rust code statically.

## Architecture (non-negotiable constraints)

Monorepo layout:

```
crux_analyzer/
  apps/
    web/          # React + TypeScript + React Flow + ELKJS — visualization + simulation
    vscode/       # VS Code extension: embeds the built web bundle, spawns the CLI
  crates/
    parser/       # Rust lib: walks syn AST, identifies Core/State/Event/Effect/transitions, emits the model. Never knows about React.
    docgen/       # Rust lib: Mermaid/Markdown generators. Consume only the model.
    cli/          # `crux-analyzer` binary: generate | docs, --watch, --locale. Reuses parser + model + docgen.
    i18n/         # Rust lib: the shared `Locale` type + env detection. No message catalogs — each crate owns its own.
    model/        # Rust lib: semantic structs only (Project, Core, Machine, State, Event, Effect, Transition). No parsing logic, no UI logic.
  shared/
    schema/       # JSON Schema contract. Every client depends ONLY on this.
```

Hard rules:
- The UI never accesses the AST or the parser's original format. It consumes only the intermediate model via the schema contract (`apps/web/src/schema/` is the only layer that knows the raw format).
- Layered UI data flow: `Parser JSON → Domain Model → React Flow Model → Components`. Swapping the parser must never require UI changes.
- Graph geometry (node positions AND edge routes/labels) is owned by the `LayoutEngine`; swapping ELK only touches `apps/web/src/layout/`. The `Graph` component is a pure renderer driven by props — the Simulation Engine highlights through props, never by modifying the graph.
- Cores contain `machines[]` (statechart-style orthogonal regions); each machine is one state enum detected by assignment analysis, no naming convention required.
- Localization is a presentation concern: `crates/model`, parser extraction and the web `domain/`/`flow/`/`layout/` layers hold no translated text. Diagnostics travel as data (`WarningKind`), generators take a `Locale`, and translated chrome is injected at the boundary. The model JSON is locale-independent.

## Known future work

`docs/roadmap.md` is the single source — planned work in order, plus what is
deliberately *not* being done and why. Keep it updated instead of starting a
list here; the pt-BR twin is `docs/pt-BR/roadmap.md`.

The short version: everything *built* is done — the parser against `init.md`, the
adoption fronts (ratchet, tag filtering, VS Code extension), and the §4 smaller
gaps (composite nesting, URL selection, event/effect docs, per-state effects,
wildcard-target notes, Markdown rendering). The open front is **§5
distribution**: nothing outside this checkout can install the tool, so the plan
there covers crates.io, prebuilt binaries and the Marketplace, in that order —
plus two license obligations (§5.5) that are already overdue rather than
planned. After that, the deliberate "not yet" list (§6) and whatever adoption
teaches next.
