# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Status

The MVP is scaffolded: pnpm + Cargo workspaces, `shared/schema` contract, stub crates (`model`, `parser`), and the working web UI in `apps/web` reading the fake JSON. `init.md` is the original project spec (in Portuguese).

## Conventions

- **English everywhere**: all git commit messages and all code — identifiers, comments, doc comments, error messages, UI strings, schema descriptions — must be written in English.

## Commands

```sh
pnpm install && pnpm dev   # web UI (Vite, apps/web)
pnpm test                  # vitest on the mapping layers
cargo check && cargo test  # Rust crates
```

## What Crux Analyzer Is

The project is named **crux_analyzer** (the spec in `init.md` uses the older working name "Crux Studio").

A **semantic analyzer** (not a diagram generator) that turns Rust + Crux applications into living documentation. It parses Rust source via the `syn` AST and builds an intermediate semantic model. The React web app is just one client of that model — a CLI, VS Code extension, and doc generators (Markdown, Mermaid, PlantUML, HTML) are planned future clients.

The project must **not** depend on Crux itself — it only analyzes Rust code statically.

## Architecture (non-negotiable constraints)

Monorepo layout:

```
crux_analyzer/
  apps/
    web/          # React + TypeScript + React Flow + ELKJS — visualization only
  crates/
    parser/       # Rust lib: reads files, walks syn AST, identifies Core/State/Event/Effect/transitions, emits the model. Never knows about React.
    model/        # Rust lib: semantic structs only (Project, Core, State, Event, Effect, Transition, Capability). No parsing logic, no UI logic.
  shared/
    schema/       # Serialized contract (preferably JSON Schema). The UI depends ONLY on this.
```

Hard rules:
- The UI never accesses the AST or the parser's original format. It consumes only the intermediate model via the schema contract.
- Layered UI data flow: `Parser JSON → Domain Model → React Flow Model → Components`. Swapping the parser must never require UI changes.
- UI components are independent: `Graph`, `Sidebar`, `Inspector`, `Toolbar`, `LayoutEngine`. Swapping ELK for another layout algorithm must only touch `LayoutEngine`.
- The architecture must accommodate a future "Simulation Engine" (replaying events through states) without modifying the graph component.

## MVP Scope

- Do **not** parse Rust code yet — the UI reads a fake JSON model (see the example in `init.md` with the "Audio Recorder" project: cores, states, transitions with `from`/`event`/`to`).
- Web UI only, three areas:
  - **Sidebar**: list of Cores
  - **Main area**: React Flow diagram with automatic ELK layout — each state is a node, each transition an edge labeled with its event
  - **Right panel (Inspector)**: selecting a state shows incoming/outgoing events; selecting a transition shows `event: from → to`
- Layout inspired by LangGraph Studio / Trigger.dev.
- Priorities: clean architecture, parser/visualization separation, low coupling, small components, evolvable code. Do not spend time on visual identity — the MVP validates architecture and navigation, not a final visual tool.
