# Development

> 🌐 **English** · [Português (Brasil)](pt-BR/development.md)

## Setup

Requirements: Rust (stable), Node 24 (the active LTS) and optionally
[`just`](https://just.systems) for the task runner. Any other Node major is a
hard install error, not a warning (`engines` plus `engineStrict`);
[`.nvmrc`](../.nvmrc) pins the exact version CI runs.

pnpm is not installed separately: `packageManager` in the root `package.json`
pins the version *and* its sha512, and corepack fetches exactly that.

```sh
corepack enable  # once, so `pnpm` resolves to the pinned version
nvm use          # or fnm/asdf — reads .nvmrc
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
| Extension tests | `just ext-test` | `pnpm --filter crux-analyzer-vscode test` |
| Extension build (embeds web dist) | `just ext-build` | — |
| Extension `.vsix` package | `just ext-package` | — |
| Static doc site | `just site <src> <name> [base]` | `CRUX_BASE=<base> pnpm --filter web build` |
| Rust tests | `just rust-test` | `cargo test --workspace` |
| Target-app tests (private, local) | — | `APP_SRC=<path> cargo test --workspace` |
| Target-app coverage (private, local) | — | `just coverage <path> <name> <floor>` |
| Clippy | `just clippy` | `cargo clippy --workspace` |
| Supply-chain gate | `just security` | `cargo deny check` + `pnpm audit --audit-level high` |
| Third-party notices | `just notices` | `cargo about generate about.hbs` (+ the web build) |
| Notices are current | `just notices-current` | — |
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
3. **Target-app test** (`crates/parser/tests/*_hidden.rs`) — runs against a
   real production app, gated on the `APP_SRC` env var (skips with a message
   when unset). Asserts the full expected transition sets and **zero
   warnings**. This is the ground truth for extraction quality — and it is
   *not in this repository*: such a test names a private application, its
   cores and its states, so `crates/parser/.gitignore` keeps
   `tests/*_hidden.rs` untracked. Whoever has the source writes their own and
   points `APP_SRC` at it; a fresh clone simply has none.
4. **Docgen tests** (`crates/docgen`) — generator output assertions, per
   locale: that prose is translated *and* that identifiers and Mermaid node
   ids are not.
5. **Web tests** (vitest) — the mapping layers (`schema → domain → flow`),
   the simulation engine, and the message catalogs (key parity, no empty or
   untranslated entries). UI components are deliberately not unit-tested; the
   layers around them are. Catalog parity is also enforced by `tsc`, so
   `just web-build` is part of that guarantee.
6. **Extension tests** (vitest, `apps/vscode`) — the pure modules: the
   webview HTML transformation and source-directory resolution. The extension
   host pieces are thin plumbing around them and are exercised manually
   (see [vscode.md](vscode.md)).

## Validation pipeline for an increment

Every increment lands only after:

1. `just rust-test` and `just clippy` — clean (with a private target app on the
   machine, `APP_SRC=<path> cargo test --workspace` instead, so that test runs
   too);
2. `just web-test` and `just web-build` — green;
3. a live UI check: `just model <src> <name> && just dev`, drive the browser, and **look**
   at the result (states, transitions, inspector, simulation);
4. logical commits in English, pushed.

Changes that touch user-facing text add two steps: regenerate the committed
examples (`just example-docs` must leave no diff for `en`) and check the UI in
**both** locales — a longer translation changes node widths, so the graph is
re-laid out, not just re-rendered.

For parser changes that alter extraction semantics, add an adversarial
cross-check: independently derive the expected transitions from the analyzed
source and compare against the CLI output before trusting the tests.

### What CI enforces

`.github/workflows/ci.yml` runs `just check` (which now includes
`fixture-guard`) plus `just docs-current`. Between them they cover the three
ways this project can silently rot:

| Guard | Catches |
| --- | --- |
| `just check` | broken tests, clippy, a missing message-catalog key (`tsc`) |
| `security` | a dependency advisory, a license outside the allowed set, a git or wildcard dependency |
| `notices-current` | a dependency added without its notice reaching `THIRD-PARTY-NOTICES.md` |
| `fixture-guard` | the fixture starting to warn, or its documentation regressing below the floor |
| `docs-current` | a committed generated example that no longer matches the generator |

`just security` installs `cargo-deny` on first use, and `just notices` installs
`cargo-about` the same way — a gate people skip because it needs setup is not a
gate. The policies live in `deny.toml` and `about.toml` (their accepted-license
lists must agree), and [security.md](security.md) explains what they defend.

`THIRD-PARTY-NOTICES.md` is generated from **what each artifact actually ships**,
not from what is installed: the web half from the chunks the bundler emitted
(`apps/web/notices.ts`), the Rust half from the crates linked into the binary. So
adding a dependency changes that file, and `notices-current` is what makes
forgetting it a red build rather than a silent license violation.

A dependency used by **both** `apps/web` and `apps/vscode` — today `typescript`,
`vitest` and `@types/node` — has its version in the `catalog:` of
[`pnpm-workspace.yaml`](../pnpm-workspace.yaml), and the manifests ask for it as
`"typescript": "catalog:"`. The two packages are type-checked and tested by the
same toolchain, so a build that passes under two different TypeScripts proves
less than one that passes under the one people actually run. Bump a shared
version in that file, **not** with `pnpm add` — `pnpm add` writes a literal
version and unlinks the package from the catalog without saying so. Versions used
by only one package stay in that package's manifest.

Installing does **not** run dependency lifecycle scripts: pnpm blocks them
unless the package is allowed in `allowBuilds` (`pnpm-workspace.yaml`), and that
map is deliberately empty. A dependency that wants to run code at install time shows up
as `ERR_PNPM_IGNORED_BUILDS` and has to be approved on purpose — so approving one
is a reviewed decision, not something a `pnpm add` does on its own. That is still
the main reason a new dependency deserves a look: its *code* ends up in the
bundle either way.

A target-app test gates itself on `APP_SRC`, and lives untracked because the
app it names is private — so CI proves the fixture path and a real app stays a
local gate. Keep it that way when adding guards: anything CI cannot run is not
a guard.

On every push to `main`, CI also publishes a **living preview**: the
mini-recorder fixture analyzed by the freshly built analyzer and deployed to
GitHub Pages via `just site` — the same recipe users run, pointed at the
public fixture. If the preview looks wrong, the release would too.

A private target app gets no ratchet in this repository: it is not here to
ratchet. Run `just coverage <path> <name> <floor>` against it locally when you
want that gate — the public counterpart, `fixture-guard`, is the one CI keeps
clicking.

A guard that cannot fail is decoration. When you add one, break it on purpose
once and watch it go red before trusting it.

## Conventions

- **English is the source language** — commits, code, comments, schema
  descriptions. User-facing text (UI, CLI output, warnings, generated labels)
  is localized: it lives in the locale catalogs, never as an inline literal.
  See [i18n.md](i18n.md).
- **Honesty rule** — the parser warns about anything it cannot infer; it
  never guesses and never drops silently. New inference features must keep
  a real target app warning-free or explain each remaining warning. Reading what the
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
