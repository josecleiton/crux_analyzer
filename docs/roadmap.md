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

## 1. The ratchet — make the documentation have teeth

The highest-leverage work, and the cheapest. There is no CI in the repository at
all today.

### 1.1 CI running `just check`

`just check` already does the right thing (corpus + clippy + web tests + web
build). It just never runs unless a human types it. One workflow closes the
biggest gap in the project.

Needs: Rust toolchain + pnpm + `just`, and a decision about the corpus — the
Quipu test is gated on `QUIPU_SRC` and that source is not public, so CI runs the
fixture tests and skips the corpus (the gate already handles this by design).

### 1.2 `--deny-warnings`

`crux-analyzer` already counts warnings and prints them to stderr; nothing acts
on them. A `--deny-warnings` flag that exits non-zero when the count is above
zero turns "the corpus extracts cleanly" from a note in `parser.md` into
something a pipeline enforces.

Small, self-contained, and the natural companion to 1.1.

### 1.3 `crux-analyzer coverage`

The state-documentation work made this measurable for the first time: `doc` is
in the model, so the model can be asked *how much of it is documented*.

A `coverage` subcommand that reports, per core and machine, the share of states
carrying a description — and fails below a `--min` threshold. That is what turns
the tool from a viewer into a ratchet: a team adopts it, the number goes up, and
CI stops it going down.

It is also the honest counterpart to the feature just shipped. Documentation you
can add is nice; documentation you can *measure* is what actually gets written.

---

## 2. Close the loop on tags

`@tag` exists in the model and renders as chips, but it is **inert**: you can
declare a tag and look at it, not *use* it. With eight states that is fine; with
thirty it is the difference between a diagram and a tool.

- **Filter and search by tag in the web UI.** Type `retryable`, keep the states
  that carry it, dim the rest. The graph already dims nodes during simulation,
  so the visual vocabulary exists.
- **Highlight undocumented states.** The visual counterpart of §1.3 — the
  states a reader should not trust yet. Deliberately opt-in, so the default view
  stays about the machine rather than about our metrics.

---

## 3. Reach — the VS Code extension

The largest audience and the largest build: the state machine beside the code,
without leaving the editor. Every layer it needs already exists — it is another
client of the same JSON contract, which is exactly what the architecture was
shaped for.

`just site` already covers the "share it with the team" path (a static build
with the model baked in), so the extension is about the *authoring* loop rather
than the reading one. That is why it sits after the ratchet: it widens reach, it
does not protect quality.

---

## 4. Smaller gaps worth fixing

Observed while building, in rough order of how visible they are:

- **Composite states render flat in the web graph** (`Parent / Child` nodes)
  while Mermaid already nests them. The most visible inconsistency in the
  product today. Touches `flow/` and `layout/` only — React Flow supports
  parent nodes, which the machine sections already use.
- **No selection state in the URL.** There is no link to "this state of this
  machine". For documentation meant to be shared and referenced in a review,
  that costs more than it looks.
- **Doc comments on events and effects.** States and machines are covered; an
  event with a `///` explaining *when* it fires is the natural next request.
  Needs a richer type for `Transition.event`, so it is a contract change rather
  than an additive one.
- **Effects are only shown per transition**, never aggregated per state. "What
  does entering `Uploading` actually do" needs a union over its incoming
  transitions.
- **The simulation cannot replay wildcard targets** (`to: "*"`), since there is
  nothing static to land on. Fine as-is, but it deserves a visible explanation
  in the panel rather than silence.
- **Markdown inside descriptions is literal in the web UI.** The generated
  document renders it properly; the UI shows the raw syntax. Fixing it means a
  Markdown dependency, which is why it was deferred rather than done.

---

## 5. Deliberately not doing yet

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
