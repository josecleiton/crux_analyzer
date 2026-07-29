# Roadmap

> 🌐 **English** · [Português (Brasil)](pt-BR/roadmap.md)

The original spec (`init.md`) is fully implemented. What follows is not "more
parsing" — the parser understands what it set out to understand. The open work
is about **adoption** and about **keeping the documentation from rotting**.

This document is the single source for planned work; `CLAUDE.md` points here
rather than keeping its own list.

## The thesis

crux_analyzer sells itself as living documentation, and today nothing stops that
documentation from lying. A model is regenerated when someone remembers to
regenerate it; a parser warning is printed and scrolls away; a state with no
description looks exactly like one whose description someone deleted.

So the ordering below is deliberate: **make the documentation defensible before
making it reach further.** A VS Code extension multiplies the audience of
whatever quality the tool currently guarantees — which is why it comes after the
guarantees, not before.

---

## 1. The ratchet — make the documentation have teeth ✅ **done**

The highest-leverage work, and the cheapest. Shipped as three independent
increments; see [cli.md](cli.md) and
[development.md](development.md#what-ci-enforces).

- **CI running `just check`** — `.github/workflows/ci.yml`. `just check` already
  did the right thing; it just never ran unless a human typed it. A test
  against a private target app gates itself on `APP_SRC` and never enters this
  repository, so CI proves the fixture path and a real app stays a local gate.
- **`--deny-warnings`** — a global flag that exits non-zero when the parser
  reported anything. Turns "a real app extracts cleanly" from a note in
  `parser.md` into something a pipeline enforces. Output is still written: the
  exit code is the signal.
- **`crux-analyzer coverage`** — the share of states carrying a *description*,
  per machine and in total, failing below `--min`. State documentation made this
  measurable for the first time. Documentation you can add is nice;
  documentation you can *measure* is what actually gets written.

Two guards came out of it that are worth keeping honest: `just fixture-guard`
(the fixture must extract with zero warnings and not lose documentation) and
`just docs-current` (a committed generated example must match the generator).
Both were broken on purpose once and watched go red — a guard that cannot fail
is decoration.

**Closed out:** the private target app got a coverage ratchet of its own, a
recipe that failed when its documentation total dropped below a floor baked
into the `justfile`. That recipe is gone: a gate nobody outside one machine can
run does not belong in a shared task runner, and the floor named an app this
repository must not name. Run `just coverage <path> <name> <floor>` against a
private app locally instead; `fixture-guard` is the public ratchet CI keeps
clicking.

---

## 2. Close the loop on tags ✅ **done**

`@tag` existed in the model and rendered as chips, but it was **inert**: you
could declare a tag and look at it, not *use* it. With eight states that is
fine; with thirty it is the difference between a diagram and a tool. Both
halves shipped as one increment; see
[web-ui.md](web-ui.md#filtering-the-canvas).

- **Filter and search by tag in the web UI** — type `retryable` (or a
  fragment; the input suggests the core's own tags), keep the states that
  carry it, dim the rest. The dimming *is* the simulation's, reached through
  the same highlight prop via a quiet `kept` tier, so the Graph stayed a pure
  renderer and the matching logic is a tested domain module
  (`src/domain/focus.ts`).
- **Highlight undocumented states** — an opt-in **Undocumented** toggle keeps
  only the states with no authored description. Opt-in as planned: the default
  view stays about the machine, and the *number* stays with
  `crux-analyzer coverage`.

---

## 3. Reach — the VS Code extension ✅ **done**

The largest audience: the state machine beside the code, without leaving the
editor. It landed exactly as the architecture predicted — another client of
the same JSON contract, and a small one, because every layer it needed already
existed. See [vscode.md](vscode.md).

`apps/vscode` embeds the built web bundle in a webview and spawns the CLI; the
model is injected as `window.__CRUX_MODEL__` (the embedding contract
`loadProject` honors), a watcher regenerates on every `.rs` save — the
*authoring* loop, complementing the `just site` reading one — and parser
warnings land in an output channel instead of being dropped. The webview
adaptation (asset re-rooting, nonce CSP, model injection) is one pure,
unit-tested module; the extension host part is thin plumbing.

---

## 4. Smaller gaps worth fixing ✅ **done**

Observed while building; all six closed, in the order they were listed:

- **Composite states nest in the web graph** — a composite parent is a
  container holding its leaves, the nesting Mermaid always had. The layout
  engine generalized to arbitrary-depth grouping (one hierarchical ELK run
  per machine, edges declared in the lowest common ancestor).
- **The selection is a URL** — `#state=Core/Machine/Name`; pasted links
  apply without a reload and stale ones fall back cleanly.
- **Doc comments on events and effects** — landed *additively* instead of
  the predicted contract break: per-core `events` / `effects` catalogs of
  `{ name, doc }`, only documented-and-used names, so an undocumented app
  emits byte-identical JSON. Rendered by the Markdown generator (per-core
  tables) and the Inspector (event doc under the transition, marks +
  tooltips on lists).
- **Effects aggregate per state** — the Inspector's *Effects on entry*: the
  union over incoming transitions, presented as a union.
- **Runtime-target transitions are explained in the simulation panel** —
  listed inert with a note, instead of silence.
- **Markdown renders in the web panels** (react-markdown — the dependency
  the deferral was about). Raw HTML in author prose stays inert text,
  verified with a hostile model; native tooltips stay plain.

---

## 4b. Hardening for public use ✅ **done**

Prompted by the question "do we have security problems?" ahead of putting this in
front of an audience. An audit of both sides found four classes, all now closed
and all pinned by tests. [security.md](security.md) is the standing contract —
threat model, rules, and the properties that must not be traded away — and
`CLAUDE.md` carries the short form as a peer to the parser honesty rule.

- **Resource limits in the parser.** The call-following walker broke recursion
  *cycles* but never bounded fan-out, so a diamond call graph of ~40 tiny
  functions described 2⁴⁰ walks — a hang and an OOM from a 60-line file. Now a
  step budget, depth caps and a call-depth cap, plus per-file and total size
  caps, and a bracket-nesting pre-check (`syn::parse_file` recurses over nesting
  and its stack overflow *aborts the process*, so that one has to run before
  parsing). Every cap reports a `Warning`: the honesty rule, applied to
  resources, which makes `--deny-warnings` cover truncation for free.
- **Output encoding in docgen.** Doc-comment prose reached published Markdown
  verbatim, so raw HTML became a real element and a fence-shaped line hijacked
  the diagram's fence. Now `<`/`&`/`>` are escaped in prose while author
  *Markdown* is preserved, fences are computed to outgrow their content, table
  cells escape the backslash before the pipe, and Mermaid ids are generated,
  collision-checked and keyword-checked with the real name in a quoted label.
- **The web app's Markdown posture is now explicit and tested.** react-markdown's
  defaults were already safe, but that was a property of the dependency; the
  protocol allowlist, link `rel`, and never fetching images are stated in
  `StateDoc.tsx` and pinned by `StateDoc.test.tsx`. Plus a CSP on the static site
  (hashes computed at build, not pasted), an error boundary, and a fix for a
  prototype-chain lookup that let an event variant named `constructor` blank the
  app.
- **The extension and the supply chain.** `cruxAnalyzer.binary` is machine-scoped
  so a cloned repo cannot choose the executable; `cruxAnalyzer.src` is contained
  to the workspace root. CI declares `permissions:`, passes `github.event.*`
  through `env:`, and pins third-party actions to commit SHAs. `just security`
  (`cargo deny` + `pnpm audit`) is blocking inside `just check`, with dependabot
  keeping the pins fresh.

Deliberately *not* done: fuzzing the parser (`cargo-fuzz` over `parse_project`)
would be the natural next step and is listed in §6.

---

## 5. Distribution — getting it into other people's hands

Nobody outside this checkout can install the tool. `cargo run` and `just` are a
contributor's interface, and the VS Code extension, when it cannot find the
binary, tells the user to run `cargo install --path crates/cli` — a command that
means nothing to someone who never cloned the repo. This is the last unaddressed
front, and it belongs here by the thesis above: distribution is the ultimate
"reach further", so it comes after the guarantees.

### The order, and why it is not a hedge

1. **`cargo install crux-analyzer` from crates.io** — the primary channel. The
   audience is Rust/Crux developers; every one of them already has a toolchain,
   and this sidesteps the two worst parts of shipping binaries (macOS Gatekeeper
   quarantining an unsigned download, and "which archive do I want"). No CI, no
   secrets, no signing.
2. **Prebuilt binaries on GitHub Releases** — not primarily for humans, but
   because §5.4 needs them: a Marketplace extension that says "go install Rust"
   has no audience.
3. **VS Code Marketplace + Open VSX** — riding on (2).

Clone-and-build stays documented, demoted to the contributor path. Note what
already works today with no repo change at all:

```sh
cargo install --git https://github.com/josecleiton/crux_analyzer crux-analyzer-cli --locked
```

### 5.1 crates.io

It is **all five crates or none**: a published crate cannot depend on an
unpublished path dependency, and collapsing the libs into the binary would
violate the [hard rules](architecture.md#hard-rules). The mitigation is already
in place — every name is `crux-analyzer`-prefixed, so they are self-namespaced.
All five names are currently free.

The hard blocker is mechanical: inter-crate dependencies are path-only with no
`version` key, which `cargo publish` rejects outright rather than warning about.
`[workspace.package]` also lacks `repository`, `keywords` and `categories`. Worth
recording because it removes a whole category of tooling: **`cargo publish
--workspace`** (cargo ≥ 1.90) topologically sorts the DAG *and* waits for index
propagation between crates, so "publish the leaves first, sleep for the index" is
a flag, not a problem. Publish from a laptop, not from CI — one fewer secret, and
cutting a release stays a deliberate human act.

Rename `crux-analyzer-cli` → `crux-analyzer` **before** publishing anything, so
the documented command never changes: the crate that *is* the product should own
the bare name, and `cargo install crux-analyzer-cli` installing a binary called
`crux-analyzer` is a papercut you would explain forever. It touches ~20 sites
(`Justfile`, `README.md`, `cli.md` and `development.md` with their twins) and
**zero** in CI, which only ever calls `just check`.

### 5.2 Prebuilt binaries — a hand-written workflow, not `cargo-dist`

`dist init` generates and then *owns* `release.yml` and knows nothing about the
pnpm half of the monorepo, so the VSIX matrix would end up in a second workflow
anyway — at which point both a generated and a hand-written one need
maintaining. A tag-triggered `release.yml` reusing the `just` recipes matches how
everything else here works: a human can run each step locally.

Targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-musl`, `x86_64-pc-windows-msvc`. **musl over gnu
deliberately** — a `-gnu` binary built on `ubuntu-latest` links against that
image's glibc and dies with `GLIBC_2.xx not found` on an older distro, the single
most common "your release binary doesn't work" report. The dependency set is pure
Rust with no `build.rs` and no C linkage, so musl builds clean and gives one
static artifact that runs everywhere.

### 5.3 Versions in lockstep

The extension talks to the CLI over the JSON contract, so the Rust workspace, the
root `package.json` and `apps/vscode` ship one number: "extension 0.4.x needs CLI
0.4.x" is a sentence that fits in a head, and a compatibility matrix is not.
`apps/web` stays at `0.0.0` on purpose — a build artifact, never published.
Guarded by a `version-check` recipe inside `just check`, not by a bot. One gotcha
worth writing down: bumping the workspace version invalidates `Cargo.lock`, and
release builds use `--locked`, so the regenerated lock is part of the bump commit.

### 5.4 The extension's real blocker

`apps/vscode/src/panel.ts` shells out to a binary on `PATH` and, on failure,
prints the `cargo install --path crates/cli` message described above. The path
forward is a pure resolution module in the style of the already-tested
`sourceDir.ts` — explicit configuration wins, then a binary bundled at `bin/`,
then `PATH` — plus `vsce package --target` per platform with one target-less VSIX
as the fallback. *Downloading on activation* is rejected: a downloaded Mach-O
gets `com.apple.quarantine` and Gatekeeper refuses to run it, so all that network
and checksum code buys a worse outcome than passing `--target`.

One trap to remember when that message is rewritten: in `vscode.l10n` **the
English string is the catalog key**, so rewording it orphans the pt-BR entry in
`apps/vscode/l10n/bundle.l10n.pt-br.json` and the pt-BR user silently gets
English with no test failing. A parity test would make that a red build; the web
side already has the pattern.

### 5.5 Already overdue, not future work

Two license obligations are unmet **today**, so they are bugs rather than plans:

- **The elkjs EPL-2.0 notice is absent from the built bundle.** Vite strips it
  and `apps/web/dist/` carries no NOTICE, but EPL-2.0 §3.1/§3.2 require that
  recipients of the object code get the license text. `README.md` attributes
  elkjs correctly, and the README does not travel with the artifact — so the
  Pages preview has been redistributing it uncovered on every push to `main`. A
  committed `THIRD-PARTY-NOTICES.md` copied into `dist/` by `web-build` covers
  Pages, every VSIX (`media/web` is already allow-listed) and any release archive
  at once.
- **Every VSIX ships MIT code with no license text.** `.vscodeignore` already
  allow-lists `!LICENSE*`, but `apps/vscode/LICENSE` does not exist. Fix it the
  way the web bundle is already handled — copied at build time by `ext-build`, so
  there is one source of truth and nothing to drift.

### 5.6 Refused, with a revisit trigger

- **Homebrew tap.** A second repo and a formula needing a SHA bump every release,
  for an audience that has cargo. *Revisit if a non-Rust user asks.*
- **npm package for the schema.** Publishing it creates a versioning obligation
  on the contract that git satisfies for free; the raw URL at a tag is the whole
  answer. *Revisit if a third-party client appears.*
- **Code signing / notarization.** An Apple Developer account and a Windows
  certificate to avoid one `xattr -d com.apple.quarantine` line in the docs —
  one more argument for channel (1). *Revisit if Gatekeeper becomes a real
  support burden.*
- **`cargo-dist`.** *Revisit if the target matrix outgrows a readable YAML.*
- **`release-plz` / `cargo-release`.** Their headline feature is now
  `cargo publish --workspace`, and neither knows about `apps/vscode/package.json`
  — so they would break the lockstep in §5.3 rather than enforce it. *Revisit on
  external contributors or a fixed cadence.*

---

## 6. Deliberately not doing yet

- **PlantUML generator.** Listed in `init.md`, but Mermaid already renders
  natively on GitHub/GitLab and `just site` covers the rest. A whole new
  generator for very little reach — last, if ever.
- **Marker styling in Mermaid** (`classDef`). A hardcoded fill breaks in a
  dark-mode reader and renderer support is uneven. If it ever lands it belongs
  behind an explicit generator option, not in the default output.
- **`#[doc(hidden)]` as "hide this state".** Tempting and wrong: the state
  exists in the machine, and hiding it would make the diagram lie by omission.
- **Inferring markers from names in the parser.** The naming heuristic stays in
  the clients. See the honesty rule in
  [architecture.md](architecture.md#hard-rules).
- **Fuzzing the parser.** `cargo-fuzz` over `parse_project` is the natural
  successor to §4b: the resource limits and the nesting pre-check were found by
  writing hostile inputs *by hand*, and a fuzzer finds the ones nobody thought
  of. Deferred rather than refused — it wants a CI budget (a fuzz job is not a
  60-second gate) and a seed corpus to be worth anything, so it belongs after
  distribution rather than squeezed into `just check`.
