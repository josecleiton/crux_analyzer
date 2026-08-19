# Roadmap

> 🌐 **English** · [Português (Brasil)](pt-BR/roadmap.md)

The original spec (`init.md`) is fully implemented, so most of what follows is
about **adoption** and about **keeping the documentation from rotting** rather
than about more parsing. One exception, and it took a real app to find it: §6 was
a state machine the parser read enough of to know it existed and still did not
extract — the first genuine parsing gap since the spec was met, now closed.

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
would be the natural next step and is listed in §7.

---

## 4c. Escaping that over-reaches

Found by reading a real app's generated document rather than by a test: §4b's
encoding pass is correct about *leaving* Markdown and slightly wrong about
*staying* in it. Both halves are the same mistake — escaping applied where the
author's markup was supposed to survive — and the contract they violate is
already written down: author Markdown is a feature, only the ability to leave it
is removed.

- **Backticks in a table cell ✅ done.** `table_cell` escaped them, so a
  documented `` `field` `` reached the reader as a visible `` \`field\` `` — 13
  cells in one target app. The stated reason ("one stray backtick spills code
  formatting across the rest of the row") does not hold: a table row is split on
  its unescaped pipes *before* its cells are parsed as inline content, so a
  backtick cannot cross a column, and an unpaired one is already literal. The
  escape is gone; the pipe escape, which does have to survive inside a code
  span, is pinned by its own test.
- **Entities inside a code span — open.** `<`, `>` and `&` are escaped over the
  whole string, code spans included, and CommonMark does not decode entity
  references inside a code span. So a doc comment reading `` `Option<String>` ``
  publishes as a literal `Option&lt;String&gt;`. It affects `prose_block` and
  `table_cell` equally, and it is not hypothetical for a Rust codebase — it just
  needs an app that documents a generic type, which no fixture and no target app
  does yet. The fix is to escape *around* code spans instead of through them,
  which means `prose_block` has to recognize a span the way it already
  recognizes a fence-shaped line: scan for backtick runs, leave what is between
  a matched pair alone. Cheap, but it is real inline parsing, so it wants
  hostile-input tests of its own (unmatched runs, runs of different lengths, a
  span holding a literal `<script>`) before it replaces a blanket `replace`.

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

### 5.5 Third-party license compliance ✅ **done**

Two obligations that were unmet on every push to `main`, closed together. The
generated `THIRD-PARTY-NOTICES.md` now ships in `apps/web/dist/` (so on Pages and
inside `media/web`), at the VSIX root, and at the repository root as the union of
both artifacts' notices; `just notices-current` inside `just check` keeps it
honest. The rules are `docs/security.md` §10.

Investigating it corrected two things this section used to assert:

- **The binding EPL-2.0 clause is §3.1(a), not §3.2.** §3.2 ("a copy of this
  Agreement must be included with each copy") is scoped to *"When the Program is
  Distributed as **Source Code**"*. We distribute object code, where §3.1(a)
  applies: accompany it with a statement that the source is available under the
  Agreement, and say how to obtain it. The notices file does both, plus the full
  text.
- **Nothing was "stripping" the elkjs notice** — `elk.bundled.js` ships with no
  copyright header at all, so §3.3 was never the live issue. What *was* being
  stripped, by the minifier, was **React's** `@license` header and its
  `Copyright (c) Meta Platforms` line: the bundle carried zero copyright notices
  of any kind. `comments: { legal: true }` restored them for the 1,687 bytes it
  costs, and the notices file covers the packages that ship no inline header.

Scope was also larger than "every VSIX ships MIT code": 68 packages contribute
code to the bundle, and MIT, ISC and BSD-3-Clause all carry notice-retention
terms. The generator is driven by the chunks the bundler emitted rather than by
the installed tree — which is both the correct scope (no `@types/*`, which ship
nothing) and the only one that works, since `pnpm licenses list` reports store
paths that do not resolve in this install layout.

elkjs also became its own chunk. Two payoffs: no output file mixes EPL-2.0 code
with ours, so EPL-2.0's "any new file that contains any contents of the Program"
never has to be argued — and since elkjs was 82% of the bundle, it is also the
answer to the 500 kB chunk warning.

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

## 5b. Effects become the other half of the loop ✅ **done**

Prompted by the question "we map the events in and out well — what about the
effects?" The answer was that an effect was a *string*: a label on a transition,
collected per event arm, with no capability, no return leg, and no honesty about
arms that branch. Events had a whole vocabulary; effects had a name.

Four things landed together, because they are one reading of the same source:

- **The capability.** `Effect::Audio(AudioOperation)` says every
  `AudioOperation` request goes through `Audio`. Structure, not a name-shaped
  guess — and it answers a question the transition tables answered badly: what
  does this core talk to? The Markdown generator gained a per-core
  **Capabilities** table off the back of it.
- **The answer (`resolvesWith`).** Crux's loop is
  `Event → Effect → shell → Event`, and the callback event is written *at the
  request site*, so reading it is evidence, not inference. All three real shapes
  are read — an event passed alongside the operation, `then_send(Event::X)`, and
  a result-mapping closure — plus one call deep into a request helper, which is
  how the real target app writes it. A **set**, because one request has one
  answer per outcome; the target app's shared audio helper legitimately answers
  with thirteen. A `then_send` naming something unreadable is a new warning
  (`unresolved-effect-callback`); a request with *no* callback is not, because
  fire-and-forget is a legitimate shape.
- **Branch scoping, and `conditional`.** Effects were shared across every
  transition of an arm, an over-approximation the model never admitted to. Now
  the chain of alternatives entered to reach a request is compared with the
  assignment's: a sibling branch's request no longer lands on this transition,
  and one found *deeper* travels with it marked conditional — "arriving here
  *may* request this". The honesty rule applied to attribution rather than to
  extraction.
- **Effects on the diagram, and in the replay.** Mermaid transition labels are
  `event / effect` (the statechart convention; the diagram had been hiding
  effects entirely), and the simulation now models the return leg: a request
  with a declared answer waits under *Waiting for the shell*, the event that
  answers it is badged `from the shell` in the fireable list, and an answer no
  transition handles is listed inert instead of hidden.

`Effect` widened from a bare string to "string or object" the way `states[]` did
(§4), so an app whose requests show neither a capability nor a callback still
emits byte-identical JSON.

**Deliberately left out:** `@failure` / `@tag` annotations on effect *variants*.
[parser.md](parser.md#documentation-and-annotations) used to say there was
nothing for a marker to mean on an effect; with capabilities and answers there
now would be (a request that can fail, a capability worth filtering by), so this
is a real next increment rather than a refusal — it just wants a use case from
adoption first, like tags got.

---

## 6. Value-flow-only state machines ✅ **done**

Found by running against a target app, and reproducible from the shape alone. Two
status enums of deliberately identical shape, both held per-entity in a
collection the model owns, both documented as state machines by the app. One is
extracted; the other is invisible.

The whole difference is a single line. Detection requires *literal* assignment
evidence — `*.field = Enum::Variant`, or a `T::default()` reset
([state_enum.rs](../crates/parser/src/state_enum.rs)). The extracted one has
exactly one such line, because the core is what *initiates* that piece of work
and so is what writes the in-progress variant; value-flow analysis then picks up
the remaining `= status` payload assignments for free. The invisible one has
none: the shell owns initiating the work there, so the core only ever *stores*
what the shell reports — via a payload assignment and a field-to-field `.clone()`.
Neither is a literal variant path.

That asymmetry in the app is honest: it reflects which side owns the transition,
and no rewrite earns the diagram without inventing an assignment that misstates
that ownership. The gap is ours.

**What makes it a gap rather than a limitation:** the parser *reads* the enum it
fails to extract. Guards comparing it — an `==` against one variant, a `matches!`
over two more — already put it in `dispatched_enums`. So the parser knows the
enum exists, knows its variants, and emits no machine **and no warning**: silence
where the honesty rule requires a diagnostic.

A generic fixture reproducing this belongs in `crates/parser/tests` alongside
`mini_recorder`, so the case is covered by a tracked test rather than only by a
private one.

### The evidence rule: model-reachable fields

Widening detection to accept value-flow assignment cannot be as simple as "any
field whose declared type is a dispatched crate enum" — that readmits the
ViewModel mirror enums the literal-assignment rule exists to exclude
([state_enum.rs](../crates/parser/src/state_enum.rs) opens by saying so). The
decision is to require the assigned field be **reachable from the `Model`
associated type**: mirror enums are constructed into view structs, never held by
the model, so reachability separates them without a naming heuristic and without
weakening the honesty rule.

Two prerequisites, both discovered while sizing this and both larger than the
detection change itself. Both are now done; kept here because each encodes a
distinction that a later refactor could easily flatten back:

- **Struct field types were recorded without looking through collections.** The
  index stores a field's type as the last path segment, so `items: Vec<Entry>`
  indexes as `("items", "Vec")` and a reachability walk breaks at the `Vec` —
  which is exactly where a per-entity status lives (`Model` → … → some substate
  → `Vec<Entry>` → the status field). `variant_fields` has an unwrapper, but it
  looks through `Box`/`Rc`/`Arc` only. This wants a *separate* unwrapper for
  struct fields rather than widening the shared one, for two reasons: the shared
  one also feeds composite-state detection, where teaching it about `Vec` changes
  what can be read as a sub-state; and the `T::default()` reset path needs the
  *declared* type, since `default()` on an `Option<E>` field yields `None` rather
  than any variant of `E` — unwrapping there would invent an assignment. So a
  field carries both types: declared (drives resets) and reachable (drives the
  walk).
- **The `Model` associated type was never resolved.** The core finder read the
  `Event` and `Effect` associated types and ignored `Model`. Cheap — the existing
  `associated_type` helper covered it — but nothing had needed it before.

Also added while here: a depth cap on both type walkers. Generic arguments nest
without limit and the loader's bracket pre-check counts `(`, `[` and `{` but
never `<`, so `Box<Box<…>>` deep enough would have overflowed the stack. The
pre-existing `Box`/`Rc`/`Arc` unwrapper had the same exposure.

### What landed ✅

Both prerequisites and the widening itself, plus the tracked
`value_flow_status` fixture the section above asked for. The evidence rule is in
[parser.md](parser.md#machine-detection); no new `WarningKind` was needed, so the
locale catalogs are untouched.

Two things came out differently than planned. The `untracked-state-enum`
diagnostic was never written: with detection widened there is no longer a silent
absence to report, and inventing a warning for an enum the parser now extracts
would be noise. And a *different* silent drop turned up in its place, which is
what the rest of this section is now about.

### Source constraints now carry a subject ✅

Widening detection exposed a second gap, pre-existing and worse than the first
because it dropped a transition from a machine the reader can *see*. Source
evidence was keyed by field name while the value mirror that resolves targets is
keyed by exact path. So a guard on `other.status` constrained a machine on
`field: status` even when `other` was a different record, and in a carry-over
write the two conjuncts intersected to nothing:

```rust
if this.status == Pending && matches!(other.status, Done | Deferred) {
    this.status = other.status.clone();   // {Pending} ∩ {Done, Deferred} = ∅
}
```

An empty source set made the emit loop iterate zero times — no transition, no
warning at all.

Fixed by giving every source evaluation a **subject**: the object whose state
field the assignment writes. A guard counts as evidence only when its receiver can
be that object, so the first conjunct above resolves the source and the second is
correctly left to the target mirror. Contradiction is still reported rather than
dropped, as a safety net for constraints that genuinely cannot hold.

What kept this from being a one-liner, and is worth remembering before
"simplifying" it:

- **Receiver comparison must stay permissive.** Field-name keying was load-bearing
  because one object is reachable by several paths — a helper writes
  `session.state` under a guard the caller wrote on
  `model.recording.session.state`. Equality plus dotted-suffix matching covers
  that; an unresolvable receiver is accepted rather than rejected. Tightening this
  to strict equality silently narrows every guard written through a local alias.
- **The subject is not derivable from the assignment alone.** Writing the field
  directly makes it the receiver; resetting the struct that *holds* the field
  (`model.session = T::default()`) makes it the whole left-hand side. Collapsing
  the two — the first thing that looks like duplication here — turns every
  `default()` reset back into a wildcard source. That regression is caught by
  `default_reset_lands_on_default_variant`.

Verified additive rather than merely green: over a real target app the three
machines that already extracted kept byte-identical transition sets (7, 6 and 30),
including the alias case, and the newly detected one went from absent to three
transitions. `crates/parser/tests/value_flow_status.rs` covers both halves on a
tracked fixture.

**The other half of that investigation, and explicitly not parser work.** The same
app documented a second machine that the analyzer also missed — but there the code
holds no enum at all, just several correlated fields (an optional id, a boolean,
a progress float) reset together by one method. Nothing exists for assignment
analysis to find, and the parser is right not to guess: inferring a machine from
a boolean and an `Option` is precisely the name-shaped inference that stays in the
clients ([architecture.md](architecture.md#hard-rules)). A machine like that wants
an enum in the application first — which is also what would make its impossible
combinations unrepresentable. Recorded so the distinction is on file: **a missing
diagram is a parser gap only when the source actually declares the states.**

---

## 6b. The entry and the dead end, in every output ✅ **done**

Two gaps found by reading a generated document next to the app: the analyzer
painted `initial` and `final` on the canvas and mentioned neither in any generated
document — `[*]` appeared **nowhere** in the repository — and the `initial`
derivation did not do what the model's own doc comment promised.

**The outputs disagreed.** `Marker` is a closed vocabulary (`failure`,
`deprecated`) and correctly so, but that left the Markdown states table with no
column for the two derived roles and the Mermaid with no start or end pseudo-state.
"There is no final state in the documentation" was true of *every* machine,
including the ones that have one.

**The derivation over-promised.** `Marker`'s doc comment and the schema both said
`initial` was derived from graph shape *and* `#[default]`, while `StateDecl`
carried no `#[default]` at all — the parser knew the default variant (it resolves
`T::default()` resets with it) and never passed it on. So a cyclic machine fell
back to declaration order, and a `#[default]` on a later variant painted the wrong
entry. It happened to be right whenever the default was also the first variant,
which is the common case and is why it went unnoticed.

Closed as one increment, because they have one cause: the evidence was missing from
the model. `StateDecl.is_default` (wire `default`) now carries what the source
declares; the clients derive the role from it. Both derivations —
`crates/docgen/src/roles.rs` for the generators, `apps/web/src/domain/stateRole.ts`
for the UI — read the same three steps: declared default, then a state nothing
transitions into, then the first declared state. Markdown gained a `Role` column
next to `Markers` (declared and derived stay separate columns on purpose) and the
Mermaid gained `[*] --> Entry` / `Dead end --> [*]`.

Invariants that are easy to flatten by accident:

- **`default` is evidence, not documentation.** `is_documented()` deliberately
  ignores it, so deriving `Default` never improves a coverage score and never makes
  the states table appear for an undocumented machine. Folding it back into
  `is_bare()`-as-`!is_documented()` — they were the same function before this —
  silently inflates coverage.
- **Only a leaf is ever marked.** `#[derive(Default)]` accepts `#[default]` on a
  unit variant only, so a composite can never be the declared default. The parser
  still checks membership before marking, so hostile or non-compiling input marks
  nothing instead of naming an `Active` the model does not declare.
- **The two derivations must stay identical.** They are the same three steps in two
  languages; a change to one that skips the other puts the canvas and the generated
  document in disagreement, which is the gap this closed.
- **Declaration order stays last.** It is the weakest evidence, not the first
  fallback: in a cycle it means nothing, which is the whole reason `default` is in
  the contract.

---

## 7. Deliberately not doing yet

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

---

## 8. What adoption found — a 13-machine production core

Running the tool against a real, private Crux application (13 machines, 197
transitions, 63 states, ~711 effect mentions) produced
[plans/adoption-findings.md](plans/adoption-findings.md): thirteen findings, six
of them bugs — and of those six, **two** (P1, P2) contradict behaviour
`docs/parser.md` already documents, which is what makes them the ones worth
distrusting the docs over. That document is **evidence**, not a tracker: its
numbers are not re-measured as fixes land, and status lives here. It was
corrected once after being written — two arithmetic errors in its own effect
counts, and two pieces of evidence added (the `else`-branch sibling gap under P1,
and the per-state measurement under D2) — so it is not frozen at `cf4f914` in the
literal sense, only in the sense that it describes that commit's behaviour.

Two rules apply to the whole front. Every finding gets a **tracked fixture
written before its fix**, with committed expected output — the fixtures use
invented names, so none of them fall under the untracked-`_hidden` rule, and P1
is precisely the class of bug that regresses in silence. And the work ships
**grouped by cause, not by finding**: `{P3a+P3b}`, `{D1+P6}`, `{P1+P2}`,
`{P4}`, `{D2}`, `{D3+D4}`, `{M1–M3}`, `{D5 provenance}` — each an increment with
one story a commit message can tell.

The order is cheapest-first as the plan proposes, with **P5 promoted to first**,
for the reason that it is the one finding with no reproduction: reducing the real
tree to a fixture is the step that decays, because it depends on a checkout
somebody else controls. Not because it unblocks anything — the earlier reading,
that it was "the warning keeping one adopter's CI red", does not survive
checking. Two warnings fail that build, and the other one is `dynamic-target`
from P4's site, which P4's fix deliberately keeps: that branch really does assign
a runtime value. No sequence of fixes here turns that build green; a recipe that
generates plus a separate gate that checks is the adopter's move.

D2 and D3 are re-measured after P1 rather than tuned against today's numbers,
since much of what degenerates them is a guard clause that should have
narrowed.

### 8.1 Parser

- **P5 — a callback that resolves in isolation but not in the tree** 🔍. Three
  fixtures failed to reproduce it, so the real tree is reduced to a fixture in
  invented names and only the fixture is committed. Start from the observation
  that the resolution names the *wrapper* variant instead of the wrapped events.
- **P1 — a guard clause narrows nothing** 🐞. `if <negated condition> { return … }`
  before the assignment publishes no constraint, so the transition is emitted as
  wildcard `"*"`: 100 of 197 transitions in that core are wildcard-sourced and 6
  of 13 machines are entirely so — and adopters were already contorting handlers
  to satisfy a tool that then dropped the guard anyway. `Ctx.conditions` grows a
  polarity flag (`{expr, negated}`) and `eval_condition` inverts its `GuardEval`
  when negated. The negation is published in **two** places: by a diverging
  then-block, to the rest of the enclosing block (the lifetime the let-else row
  already promises), and by the `else` of any `if` — the second is a sibling gap
  the findings do not name, since `Expr::If` pushes the condition into the then
  branch only.
- **P2 — a closure parameter is compared by name** 🐞. Guard evidence inside
  `find(|d| …)` counts only when the parameter is spelled like the binding the
  result is written through, which makes the analysis rename-sensitive. The
  parameter of a closure passed to the call whose result is bound *is* the element
  that binding receives, so the two are unified **positionally, whatever either is
  called**. Structural identity, so §6's permissive-receiver rule is untouched
  elsewhere. Decided with P1: shape 6 of the plan's fixture fails under both rules
  at once.
- **P3a + P3b ✅ done.** `is_effect_request_enum` and `declares_variant`
  (`crates/parser/src/core_finder.rs`) narrow what `record_effect_path` may record;
  the closure keeps its full membership, since `emit` reads it for the doc comment
  authored on a variant. Fixture first, as the rule says:
  `crates/parser/fixtures/effect_requests/` with `tests/effect_requests.rs`, whose
  four cases are the two shapes that must go (a payload variant at depth 2 and at
  depth 3, an associated function on an operation enum) and the two that must
  survive (a root-carried operation, a bare `render()`). Re-measured against the
  core the findings came from: **711 → 441 mentions, exactly the 270 predicted**,
  23 names dropped, none newly recorded, no machine lost. The document went from
  131 KB to 106 KB, its largest diagram from 10.6 KB to 5.9 KB, and its longest
  edge label from 473 to 302 characters — which is what D3's cap is still for.
- **P3a — the effect closure has no depth bound** 🐞. Payload enums reached
  transitively become `effect_enums`, so any later mention of one of their
  variants is recorded as an effect request — **270 of 711** mentions in that core
  (the 228 data-enum and associated-function mentions plus `TelemetrySignal`'s 42,
  which is payload of one request rather than a sibling of it). The recording
  predicate becomes `name == effect_root || capability_of(name).is_some()`. The
  first clause is load-bearing because `capability_of` returns `None` for two
  different things — a payload enum nothing wraps, and the root itself — so
  without it an app whose root carries operations as its own variants
  (`Effect::StartAudio { .. }`) loses every effect it has. Not, as first written
  here, because of `Render`: a bare `render()` never reaches `record_effect_path`
  at all, being recorded by `record_effect` at `transitions.rs:603` with a literal
  label. The fixture that discriminates is a root with an operation variant of its
  own, and one asserting "`Render` survives" would pass either way. Deeper enums
  simply stop being recorded; documenting them as payload types is a separate
  decision, and removing a false positive owes no `Warning`.
- **P3b — associated functions recorded as variants** 🐞. `record_effect_path`
  takes whatever `enum_variant_path` returns, so `FailureDomain::of` and
  `ApiFailure::from` are reported as things the shell is asked to perform.
  Comparing the last segment against `decl.variants` fixes it with no possible
  false negative. Cheap rather than high-yield, which is the correction to the
  plan's ordering argument: all five names it catches in that core are payload
  enums at depth ≥ 2, so P3a already removes every one of the 122 — its marginal
  contribution there is zero. What it catches alone is an associated function on a
  *depth-1* operation enum (`AudioOperation::of(..)`), which is exactly as wrong
  and simply does not occur in this sample.
- **P4 — one dynamic branch drops the whole machine** 🐞. `model.filter = if cond
  { Filter::All } else { filter }` loses both branches, and a machine with no
  transition never reaches the model: 14 machines in the source, 13 in the output.
  The literal branch is emitted as a real transition, the unresolvable sibling
  gets the existing wildcard-target note, and a **new warning kind** fires when a
  machine ends up with no transitions — today's diagnostic names a transition
  while the thing lost is a machine.
- **P6 — warnings emitted more than once** 🐞. Deduplicated on
  `(file, line, kind)` before reporting: three identical lines read as three
  problems.

### 8.2 Docgen

- **D1 — byte-identical duplicate edges** 🐞. Transitions identical but for
  `resolves_with`, which the label does not render, draw two arrows on top of each
  other. Fixed at both levels: merged in the model (union the answers — two call
  paths to the same helper are not two transitions) and skipped in
  `machine_diagram`, because an identical rendered line carries no information
  whatever the model says. While there, check whether a shared helper reached by
  two call paths is *walked* twice — the same suspicion behind P6, and if it holds
  the effect counts are inflated too.
- **D2 — the `final` role is degenerate on wildcard-driven machines** ⚖️
  **reopened**. 34 states marked final, four machines marking all of theirs, a
  state named `Downloading` among them. The first decision here — no `final` role
  for a machine whose transitions are all wildcard-sourced, others keep today's
  behaviour — was measured against the core afterwards and keeps 11 of the 34
  marks, **8 of them in one machine**: the one marking 8 of its 9 states final has
  a single wildcard transition out of 7, so the rule does not apply to it, and
  that transition is `* -- InsightsUpdated -> *`, wildcard in source *and* target.
  Every state leaves through it. The rule is per machine and the degeneracy is per
  state. The strict per-state reading (no role for any state a wildcard can leave)
  keeps 0 of 34 here, since every machine in that core has at least one
  wildcard-sourced transition, so it empties the feature instead of fixing it.
  Which points at the role not being binary: "nothing leaves this state by name"
  is a real fact and a different one from "terminal", so it belongs in the states
  table under a word that says so, while `X --> [*]` is drawn only where no
  wildcard can leave. One coupling to fix with it — the diagram draws roles
  unconditionally while the states table is gated on `has_documented_states`, so
  the machine holding 7 of these marks asserts them in the only place a reader
  cannot check them. Still re-measured after P1.

  **Decided: a two-tier vocabulary.** The states table keeps the fact under a word
  that states it — nothing leaves this state *by name* — and `X --> [*]` is drawn
  only where nothing leaves it at all, wildcards included. Both readings are true
  and they are different readings, which is why one word could not carry them: the
  fact is evidence the shape actually supplies, and "terminal" is an inference the
  shape contradicts the moment a wildcard exists. It keeps all 34 marks, as true
  statements, and draws none of the false arrows. The strict rule was refused for
  emptying the feature (0 of 34 here) and the per-machine rule for missing the
  machine that needs it most. Costs a label in both locale catalogs and the
  matching change in `apps/web/src/domain/stateRole.ts` — `roles.rs` says change
  one, change both, and this is the change that proves why.
- **D3 — edge labels have no cap** ⚖️. Effects in an edge label are capped at 3
  with a `+n more` suffix, mirroring `ANSWERS_IN_A_CELL`; the table stays complete
  by contract. Dropping `Render` was **refused**: eliding it by name is a naming
  convention in a project that refuses those, and eliding it structurally (the
  root-carried operation) would erase every effect of an app whose root holds
  operations directly. P3 already removes a third of the noise.
- **D4 — `\n` as the label line break is renderer-dependent** ⚖️. Emitted as
  `<br/>` instead, which works on both of mermaid's label paths; the label is
  built from escaped parts joined by the raw tag, so author prose stays escaped
  and only the separator is markup.
- **D5 — a 1027-line document with no index and no provenance** ⚖️. Split, since
  half of it depends on nothing: the table of contents and per-machine anchors
  ship on their own, and provenance links plus the prose de-duplication (a state's
  first paragraph is rendered three times) land on top of M2.

### 8.3 Model — one increment, last

The three fields move the contract **once**, after P1 and P3, so they are designed
against corrected data rather than today's 51%-wildcard output. All three are
additive and optional, so no client breaks.

- **M1 — `Transition` has no guard** ⚖️. `guard: Option<String>` carrying the
  condition as written, rendered as `Event [guard] / effects`. It is untrusted
  prose: escaped at every boundary, and capped in length with a `Warning` when the
  cap fires. This is what makes P1 *visible* where one source state fans out on
  one event.
- **M2 — no source span** ⚖️. `source: { file, line }`, with `file` relative to
  the analyzed `src` root — never absolute. Paths come out of an untrusted tree,
  and a relative path also keeps `model.json` reproducible between machines.
- **M3 — no path to the machine** ⚖️. A singleton at `flags.identity` and one
  instance per record at `drafts[].submission.status` render identically today.
  The parser emits the field path **and** the cardinality it derived from that
  path crossing a collection: §6b's lesson was that the same derivation written
  twice in two languages drifts apart.
- **The web's share.** `apps/web/src/schema/` and the domain model accept all
  three in the same increment; the Inspector renders **the guard only** — the one
  a reader misses when three arrows share an event. Span and cardinality wait for
  a UI decision.

### 8.4 One thing that is only documentation

That run reported 79% documentation coverage with the biggest machine at 0 of 7
states described, and nothing failed on it: `coverage --min` was never wired into
the adopter's `check` recipe. `docs/cli.md` and its pt-BR twin gain the wiring
guidance beside `docs --deny-warnings`, because adoption evidently does not find
it unaided.
