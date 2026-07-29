# crux_analyzer

Semantic analyzer for **Rust + Crux** applications: turns the code into living
documentation. The parser (Rust, via `syn`) produces an **intermediate model**;
the web UI (React) is just one client of that model — CLI, VS Code extension
and documentation generators are future clients.

> MVP status: the UI reads a fake JSON (`shared/schema/examples/audio-recorder.json`).
> The Rust crates are compilable stubs; no Rust parsing happens yet.

## Structure

```
apps/web/        React + TypeScript + React Flow + ELKJS — visualization only
crates/parser/   Rust lib: will read files via syn and emit the model (stub)
crates/model/    Rust lib: semantic structs (Project, Core, State, ...)
shared/schema/   JSON Schema contract — the UI depends ONLY on this
```

UI data flow (layers):

```
Parser JSON → Domain Model → React Flow Model → Components
```

## Running

```sh
# UI
pnpm install
pnpm dev            # opens the web app (Vite)
pnpm test           # mapping-layer tests

# Crates
cargo check
cargo test
```
