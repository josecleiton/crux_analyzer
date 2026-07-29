# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Status

The original roadmap is fully implemented: syn-based parser (predicate guards, `==`/closure guards, `Default` resets, wildcard sources AND targets, value-flow analysis for runtime-assigned targets, hierarchical/composite states as `Parent/Child` paths, per-transition effects, multiple state machines per core), CLI (`crux-analyzer generate | docs`, both with `--watch`), doc generators (Mermaid with composite blocks, Markdown), and the web UI (machine sections, inspector with effects, Simulation Engine). The corpus test against a real app is gated on `CORPUS_SRC` and extracts it warning-free. `init.md` is the original project spec (in Portuguese).

## Conventions

- **English everywhere**: all git commit messages and all code — identifiers, comments, doc comments, error messages, UI strings, schema descriptions — must be written in English.
- **Parser honesty rule**: what cannot be inferred statically is surfaced as a `Warning` (never silently dropped, never guessed). An assignment with *no* state evidence legitimately fires from any state (`"*"`).

## Commands

```sh
pnpm install && pnpm dev   # web UI (Vite, apps/web)
pnpm test                  # vitest: mapping layers + simulation engine
cargo check && cargo test  # Rust crates (parser unit + fixture + docgen tests)
CORPUS_SRC=<corpus>/shared/src cargo test          # + real-app corpus test
cargo run -p crux-analyzer-cli -- generate \
  --src <app>/src --out apps/web/public/model.json   # feed the UI a real model
cargo run -p crux-analyzer-cli -- docs --src <app>/src --format mermaid
```

## What Crux Analyzer Is

The project is named **crux_analyzer** (the spec in `init.md` uses the older working name "Crux Studio").

A **semantic analyzer** (not a diagram generator) that turns Rust + Crux applications into living documentation. It parses Rust source via the `syn` AST and builds an intermediate semantic model. The React web app is just one client of that model — the CLI doc generators are another; a VS Code extension and more formats (PlantUML, HTML) are planned future clients.

The project must **not** depend on Crux itself — it only analyzes Rust code statically.

## Architecture (non-negotiable constraints)

Monorepo layout:

```
crux_analyzer/
  apps/
    web/          # React + TypeScript + React Flow + ELKJS — visualization + simulation
  crates/
    parser/       # Rust lib: walks syn AST, identifies Core/State/Event/Effect/transitions, emits the model. Never knows about React.
    docgen/       # Rust lib: Mermaid/Markdown generators. Consume only the model.
    cli/          # `crux-analyzer` binary: generate | docs, --watch. Reuses parser + model + docgen.
    model/        # Rust lib: semantic structs only (Project, Core, Machine, State, Event, Effect, Transition). No parsing logic, no UI logic.
  shared/
    schema/       # JSON Schema contract. Every client depends ONLY on this.
```

Hard rules:
- The UI never accesses the AST or the parser's original format. It consumes only the intermediate model via the schema contract (`apps/web/src/schema/` is the only layer that knows the raw format).
- Layered UI data flow: `Parser JSON → Domain Model → React Flow Model → Components`. Swapping the parser must never require UI changes.
- Graph geometry (node positions AND edge routes/labels) is owned by the `LayoutEngine`; swapping ELK only touches `apps/web/src/layout/`. The `Graph` component is a pure renderer driven by props — the Simulation Engine highlights through props, never by modifying the graph.
- Cores contain `machines[]` (statechart-style orthogonal regions); each machine is one state enum detected by assignment analysis, no naming convention required.

## Known future work

- PlantUML/HTML generators; VS Code extension.
- Visual nesting of composite states in the web graph (they currently render as flat `Parent / Child` nodes; Mermaid already nests them).
