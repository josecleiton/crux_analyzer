# Development

> 🌐 **English** · [Português (Brasil)](pt-BR/development.md)

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
| Static doc site | `just site <src> <name> [base]` | `CRUX_BASE=<base> pnpm --filter web build` |
| Rust tests | `just rust-test` | `cargo test --workspace` |
| Corpus tests | `just corpus` | `QUIPU_SRC=<path> cargo test --workspace` |
| Corpus coverage ratchet | `just quipu-coverage [floor]` | `cargo run -p crux-analyzer-cli -- coverage ... --min <floor>` |
| Clippy | `just clippy` | `cargo clippy --workspace` |
| Everything | `just check` | — |
| Model for the UI | `just model <src> <name>` | `cargo run -p crux-analyzer-cli -- generate ...` |
| Docs | `just docs <src> <name> [format] [locale]` | `cargo run -p crux-analyzer-cli -- docs ...` |
| Documentation coverage | `just coverage <src> <name> [min]` | `cargo run -p crux-analyzer-cli -- coverage ...` |
| Example docs refresh (all locales) | `just example-docs` | — |
| Committed examples are current | `just docs-current` | — |
| Fixture extracts cleanly | `just fixture-guard` | — |

## Test layers

1. **Parser unit tests** (`crates/parser/src/tests.rs`) — one inline-source
   test per extraction pattern (guards, predicates, composites, value-flow,
   wildcards, ...). Start here when adding a pattern.
2. **Fixture integration** (`crates/parser/fixtures/mini_recorder/` +
   `crates/parser/tests/mini_recorder.rs`) — a minimal Crux-shaped app
   exercising delegation, nested events, multi-region extraction. Plain
   sources, not a compiled crate.
3. **Corpus test** (`crates/parser/tests/quipu.rs`) — runs against a real
   production app, gated on the `QUIPU_SRC` env var (skips with a message
   when unset). Asserts the full expected transition sets and **zero
   warnings**. This is the ground truth for extraction quality.
4. **Docgen tests** (`crates/docgen`) — generator output assertions, per
   locale: that prose is translated *and* that identifiers and Mermaid node
   ids are not.
5. **Web tests** (vitest) — the mapping layers (`schema → domain → flow`),
   the simulation engine, and the message catalogs (key parity, no empty or
   untranslated entries). UI components are deliberately not unit-tested; the
   layers around them are. Catalog parity is also enforced by `tsc`, so
   `just web-build` is part of that guarantee.

## Validation pipeline for an increment

Every increment lands only after:

1. `just corpus` (includes all Rust tests) and `just clippy` — clean;
2. `just web-test` and `just web-build` — green;
3. a live UI check: `just quipu && just dev`, drive the browser, and **look**
   at the result (states, transitions, inspector, simulation);
4. logical commits in English, pushed.

Changes that touch user-facing text add two steps: regenerate the committed
examples (`just example-docs` must leave no diff for `en`) and check the UI in
**both** locales — a longer translation changes node widths, so the graph is
re-laid out, not just re-rendered.

For parser changes that alter extraction semantics, add an adversarial
cross-check: independently derive the expected transitions from the corpus
source and compare against the CLI output before trusting the tests.

### What CI enforces

`.github/workflows/ci.yml` runs `just check` (which now includes
`fixture-guard`) plus `just docs-current`. Between them they cover the three
ways this project can silently rot:

| Guard | Catches |
| --- | --- |
| `just check` | broken tests, clippy, a missing message-catalog key (`tsc`) |
| `fixture-guard` | the fixture starting to warn, or its documentation regressing below the floor |
| `docs-current` | a committed generated example that no longer matches the generator |

The corpus test gates itself on `QUIPU_SRC`, and that source is not public — so
CI proves the fixture path and the corpus stays a local gate. Keep it that way
when adding guards: anything CI cannot run is not a guard.

The corpus has a coverage ratchet of its own: `just quipu-coverage` (part of
`just check`) fails when the Quipu documentation total drops below the floor
baked into the recipe. Like the corpus test it skips itself when the source is
absent, so in CI the fixture-guard floor is the public stand-in. When coverage
rises, raise the floor in the `justfile` — that is the ratchet clicking.

A guard that cannot fail is decoration. When you add one, break it on purpose
once and watch it go red before trusting it.

## Conventions

- **English is the source language** — commits, code, comments, schema
  descriptions. User-facing text (UI, CLI output, warnings, generated labels)
  is localized: it lives in the locale catalogs, never as an inline literal.
  See [i18n.md](i18n.md).
- **Honesty rule** — the parser warns about anything it cannot infer; it
  never guesses and never drops silently. New inference features must keep
  the corpus warning-free or explain each remaining warning. Reading what the
  source *declares* is fair game — annotations are data the parser may report —
  but inference stays banned, and guesses stay in the clients.
- **Evidence over shape** — detection heuristics (machines, composites,
  nested event enums) key on how the code *uses* a type, not on what it
  looks like. Follow that principle when extending detection.
- Schema changes ship with: schema + `crates/model` (+ round-trip test) +
  bundled example + docgen + web schema/domain layers + tests, in one commit.

## Repository docs map

- `README.md` — front door; quick start.
- `docs/` — this documentation set (English, the source); `docs/pt-BR/` is its
  Portuguese mirror.
- `CLAUDE.md` — working agreements for AI-assisted development (kept in sync
  with the architecture rules).
- `docs/roadmap.md` — the single source for planned work. Add to it rather than
  starting a list elsewhere, and record what you decide *not* to do too.
- `init.md` — the original project spec (Portuguese, historical).
