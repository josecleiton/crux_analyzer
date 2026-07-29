# crux_analyzer

Semantic analyzer for **Rust + Crux** applications: turns the code into living
documentation. The parser (Rust, via `syn`) produces an **intermediate model**;
the web UI (React) is just one client of that model — CLI, VS Code extension
and documentation generators are future clients.

## Structure

```
apps/web/        React + TypeScript + React Flow + ELKJS — visualization only
crates/parser/   Rust lib: walks the syn AST and extracts states/transitions
crates/cli/      `crux-analyzer` binary: runs the parser, emits model JSON
crates/model/    Rust lib: semantic structs (Project, Core, State, ...)
shared/schema/   JSON Schema contract — the UI depends ONLY on this
```

UI data flow (layers):

```
Parser JSON → Domain Model → React Flow Model → Components
```

## Running

```sh
# Analyze a Crux app and feed the UI
cargo run -p crux-analyzer-cli -- generate \
  --src path/to/app/src --name MyApp --out apps/web/public/model.json

# UI — shows the generated model, or the bundled fake example without one
pnpm install
pnpm dev            # opens the web app (Vite)
pnpm test           # mapping-layer tests

# Crates
cargo check
cargo test          # parser unit + fixture tests
CORPUS_SRC=path/to/corpus_app/shared/src cargo test  # + real-app corpus test
```

## How the parser extracts the model

The parser never depends on Crux — it analyzes sources statically:

1. Every `impl App for X` block becomes a Core; its `Event` associated type
   seeds the event-enum set (nested event enums are followed).
2. State machines are detected by assignment analysis (`*.state = Enum::V`
   plus matches against the same field) — no naming convention required.
3. Transitions come from walking `update` and its helpers (cross-file),
   combining `matches!` guards / `match`-on-state arms (`from`), event arm
   patterns (`event`) and state assignments (`to`).
4. What cannot be inferred statically (e.g. predicate-method guards) is
   dropped and reported as a warning by the CLI.
