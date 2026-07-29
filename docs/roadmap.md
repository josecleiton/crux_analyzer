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
  did the right thing; it just never ran unless a human typed it. The corpus
  test gates itself on `QUIPU_SRC` and that source is not public, so CI proves
  the fixture path and the corpus stays a local gate.
- **`--deny-warnings`** — a global flag that exits non-zero when the parser
  reported anything. Turns "the corpus extracts cleanly" from a note in
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

**What is left here:** put a `--min` on the corpus in whatever pipeline analyzes
a real app. `RecordingState` sits at 13% with no description on the enum itself,
which is exactly the kind of number a ratchet is for.

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
