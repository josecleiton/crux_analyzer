# Development

## Setup

Requirements: Rust (stable), Node + pnpm, and optionally
[`just`](https://just.systems) for the task runner.

```sh
pnpm install
cargo check
just            # lists every recipe
```

## Everyday commands

| Task | just | raw |
| --- | --- | --- |
| Web dev server | `just dev` | `pnpm --filter web dev` |
| Web tests | `just web-test` | `pnpm --filter web test` |
| Web build (tsc + vite) | `just web-build` | `pnpm --filter web build` |
| Rust tests | `just rust-test` | `cargo test --workspace` |
| Corpus tests | `just corpus` | `CORPUS_SRC=<path> cargo test --workspace` |
| Clippy | `just clippy` | `cargo clippy --workspace` |
| Everything | `just check` | — |
| Model for the UI | `just model <src> <name>` | `cargo run -p crux-analyzer-cli -- generate ...` |
| Docs | `just docs <src> <name> [format]` | `cargo run -p crux-analyzer-cli -- docs ...` |
| Example docs refresh | `just example-docs` | — |

## Test layers

1. **Parser unit tests** (`crates/parser/src/tests.rs`) — one inline-source
   test per extraction pattern (guards, predicates, composites, value-flow,
   wildcards, ...). Start here when adding a pattern.
2. **Fixture integration** (`crates/parser/fixtures/mini_recorder/` +
   `crates/parser/tests/mini_recorder.rs`) — a minimal Crux-shaped app
   exercising delegation, nested events, multi-region extraction. Plain
   sources, not a compiled crate.
3. **Corpus test** (`crates/parser/tests/corpus_hidden.rs`) — runs against a real
   production app, gated on the `CORPUS_SRC` env var (skips with a message
   when unset). Asserts the full expected transition sets and **zero
   warnings**. This is the ground truth for extraction quality.
4. **Docgen tests** (`crates/docgen`) — generator output assertions.
5. **Web tests** (vitest) — the mapping layers (`schema → domain → flow`)
   and the simulation engine. UI components are deliberately not unit-tested;
   the layers around them are.

## Validation pipeline for an increment

Every increment lands only after:

1. `just corpus` (includes all Rust tests) and `just clippy` — clean;
2. `just web-test` and `just web-build` — green;
3. a live UI check: `just corpus-model && just dev`, drive the browser, and **look**
   at the result (states, transitions, inspector, simulation);
4. logical commits in English, pushed.

For parser changes that alter extraction semantics, add an adversarial
cross-check: independently derive the expected transitions from the corpus
source and compare against the CLI output before trusting the tests.

## Conventions

- **English everywhere** — commits, code, comments, UI strings, schema
  descriptions.
- **Honesty rule** — the parser warns about anything it cannot infer; it
  never guesses and never drops silently. New inference features must keep
  the corpus warning-free or explain each remaining warning.
- **Evidence over shape** — detection heuristics (machines, composites,
  nested event enums) key on how the code *uses* a type, not on what it
  looks like. Follow that principle when extending detection.
- Schema changes ship with: schema + `crates/model` (+ round-trip test) +
  bundled example + docgen + web schema/domain layers + tests, in one commit.

## Repository docs map

- `README.md` — front door; quick start.
- `docs/` — this documentation set.
- `CLAUDE.md` — working agreements for AI-assisted development (kept in sync
  with the architecture rules).
- `init.md` — the original project spec (Portuguese, historical).
