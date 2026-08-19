# Adoption findings: a 13-machine production core

What running `crux-analyzer` against a real, private Crux application
(13 machines, 197 transitions, 63 states, ~711 effect mentions) surfaced. Every
item below was reproduced against a **self-contained fixture in this repository's
own idiom**, so nothing here needs access to that application: the fixtures are
inline and each one is small enough to paste into
`crates/parser/fixtures/` as a regression test.

**Status is not tracked here.** This document is evidence measured against the
commit below; every finding's decision and progress lives in
[`docs/roadmap.md` §8](../roadmap.md#8-what-adoption-found--a-13-machine-production-core).

It has been corrected once since it was written, which is a different thing from
being re-measured as fixes land: two arithmetic errors in its own effect counts
(P3a and P3b overlap almost entirely and their numbers were being added), and two
pieces of evidence added — the `else`-branch sibling gap under P1, and the
per-state measurement under D2 that reopened it. What is *not* updated here is
anything a fix changes. Those numbers describe `cf4f914` and stay that way.

Read this as a triage list, not a design document. Findings are split by where
the fix belongs — parser, docgen, model — and each says what is a **bug against
documented behaviour** versus a **design question that needs a decision**. That
distinction is the whole point of the split: two of the parser items (P1, P2)
claim to work in `docs/parser.md` today.

Two conventions carried from the rest of `docs/`: prose explains the failure a
change prevents, and identifiers from the analyzed application are data. The
fixtures below use invented names for that reason — none of them are the real
application's.

## 0. Reproducing

Each fixture is a directory holding one `lib.rs` (plain sources, parsed and never
compiled — the shape `crates/parser/fixtures/mini_recorder/` already uses):

```sh
mkdir -p /tmp/probe && $EDITOR /tmp/probe/lib.rs      # paste a fixture below
crux-analyzer docs --src /tmp/probe --name Probe      # read the mermaid block
```

The findings were produced with `crux-analyzer 0.1.0` at `cf4f914`.

---

## 1. Parser

### P1 — Narrowing is lost whenever the guard leaves the block early  🐞

**The one that matters most.** A guard that *lexically wraps* the assignment
narrows the source state correctly. The same guard written as a guard clause —
`if <negated condition> { return … }` followed by the assignment — narrows
nothing, and the transition is emitted as wildcard `"*"`.

This contradicts `docs/parser.md:88-106`, which documents negation, `!=` and
predicate methods as evidence, and `docs/parser.md:99` documents let-else
narrowing as holding "for the rest of the block".

```rust
//! Six shapes of the same source-state guard.

pub enum Event { Inline, InlineEq, EarlyNe, EarlyPredicate, EarlyMatches, LetElse }

pub enum AudioOperation { Start }
pub enum Effect { Audio(AudioOperation) }

/// Where the take stands.
#[derive(Default)]
pub enum Status {
    /// Nothing yet.
    #[default]
    Idle,
    /// Queued for sending.
    Queued,
    /// Sent.
    Done,
    /// Gave up.
    Failed,
}

impl Status {
    /// Whether a sweep still has work here.
    pub const fn is_in_flight(&self) -> bool {
        matches!(self, Self::Queued)
    }
}

pub struct Draft { pub status: Status }
pub struct Model { pub status: Status, pub drafts: Vec<Draft> }
pub struct Probe;

impl App for Probe {
    type Event = Event;
    type Effect = Effect;
    type Model = Model;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            // 1. inline `matches!`
            Event::Inline => {
                if matches!(model.status, Status::Queued) {
                    model.status = Status::Done;
                }
                render()
            }
            // 2. inline `==`
            Event::InlineEq => {
                if model.status == Status::Queued {
                    model.status = Status::Failed;
                }
                render()
            }
            // 3. guard clause, `!=`
            Event::EarlyNe => {
                if model.status != Status::Queued {
                    return render();
                }
                model.status = Status::Failed;
                render()
            }
            // 4. guard clause, predicate method
            Event::EarlyPredicate => {
                if !model.status.is_in_flight() {
                    return render();
                }
                model.status = Status::Done;
                render()
            }
            // 5. guard clause, negated `matches!` — same expression as (1)
            Event::EarlyMatches => {
                if !matches!(model.status, Status::Queued) {
                    return Command::done();
                }
                model.status = Status::Done;
                render()
            }
            // 6. let-else narrowing
            Event::LetElse => {
                let Some(draft) = model.drafts.iter_mut().find(|d| d.status == Status::Queued)
                else {
                    return render();
                };
                draft.status = Status::Done;
                render()
            }
        }
    }
}
```

Emitted:

```
    Queued    --> Done:   Inline           ✅
    Queued    --> Failed: InlineEq         ✅
    any_state --> Failed: EarlyNe          ❌ expected Queued
    any_state --> Done:   EarlyPredicate   ❌ expected Queued
    any_state --> Done:   EarlyMatches     ❌ expected Queued
    any_state --> Done:   LetElse          ❌ expected Queued (see P2)
```

Shapes 1 and 5 hold the *same* condition over the *same* subject. Only the
control flow differs, which localizes the gap precisely: the constraint
collected from an `if` is scoped to that `if`'s own block, and a `return` in
that block should instead publish the negation to everything after it.

Why it dominates: in the production core, **100 of 197 transitions are
wildcard-sourced, and 6 of 13 machines are 100% wildcard**. The guard clause is
the idiom that gets used the moment a handler has more than one precondition, so
the machines it erases are the complicated ones — the ones a diagram is read
for. Three examples from that core, all correct in the source and all wildcard
in the output:

- an audio-progress event guarded on `Fetching` renders as arriving from any state;
- an encode-finished event guarded on `Queued` renders as arriving from any state;
- a machine whose only entry is guarded on `Unsent | Refused` (through a
  predicate method) renders as entered from any state.

There is a second cost that is easy to miss. That core carries a comment
explaining that three `if/else` branches assign literals *because* "the state
machine the analyzer draws from this file cannot follow a target computed at
runtime". Adopters contort code to satisfy the tool, and then the contortion
does not even pay: the guard is dropped anyway. Fixing P1 is what lets that
comment be deleted.

Suspected area: source-evidence collection in `crates/parser/src/transitions.rs`
— where an `if` publishes its condition to the statements it encloses. The fix
wants the negation of a guard clause to survive to the end of the enclosing
block, which is the same lifetime the let-else row already promises.

**A sibling gap, same cause.** `Expr::If` pushes the condition into the then
branch only, so an `else` receives no negation either — and unlike the guard
clause, the equivalent spelling *does* work, which is what makes the pair worth
fixing together:

```rust
// then narrows, else does not
if model.status == Status::Queued {
    model.status = Status::Done;
} else {
    model.status = Status::Failed;
}

// the same decision as a match: `_` resolves to the complement, as documented
match model.status {
    Status::Queued => model.status = Status::Done,
    _ => model.status = Status::Failed,
}
```

```
    Queued    --> Done:   IfElse     ✅
    any_state --> Failed: IfElse     ❌ expected {Idle, Done, Failed}
    Queued    --> Done:   MatchArm   ✅
    Idle      --> Failed: MatchArm   ✅
    Done      --> Failed: MatchArm   ✅
    Failed    --> Failed: MatchArm   ✅
```

Two spellings of one decision, and only one of them survives extraction. A
polarity flag on the collected condition serves both: the negation is published
by a diverging then-block to the rest of the enclosing block, and by the `else`
of any `if` to its own branch.

### P2 — A closure-parameter receiver is compared by name  🐞

Guard evidence inside a `find(|…| …)` closure counts only when the closure's
parameter happens to be spelled the same as the binding the result is written
through. A closure parameter is a fresh binding whose name is arbitrary, so this
makes the analysis rename-sensitive.

```rust
pub enum Event { SameName, DifferentName }
pub enum AudioOperation { Start }
pub enum Effect { Audio(AudioOperation) }

/// Where the take stands.
#[derive(Default)]
pub enum Status {
    /// Nothing yet.
    #[default]
    Idle,
    /// Queued.
    Queued,
    /// Sent.
    Done,
    /// Gave up.
    Failed,
}

pub struct Draft { pub status: Status }
pub struct Model { pub drafts: Vec<Draft> }
pub struct Probe;

impl App for Probe {
    type Event = Event;
    type Effect = Effect;
    type Model = Model;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::SameName => {
                if let Some(draft) =
                    model.drafts.iter_mut().find(|draft| draft.status == Status::Queued)
                {
                    draft.status = Status::Done;
                }
                render()
            }
            Event::DifferentName => {
                if let Some(draft) =
                    model.drafts.iter_mut().find(|d| d.status == Status::Queued)
                {
                    draft.status = Status::Failed;
                }
                render()
            }
        }
    }
}
```

Emitted:

```
    Queued    --> Done:   SameName        ✅
    any_state --> Failed: DifferentName   ❌ expected Queued
```

`docs/parser.md:99`'s own example (`let Some(d) = list.find(|d| d.state == X)`)
names both `d`, which is the coincidence that makes the documented case pass.

The subject rule in `docs/parser.md:107-141` is right about *why* receivers are
compared — a guard about one record must not constrain another — and the
roadmap's §6 warns that "receiver comparison must stay permissive … or aliased
guards narrow silently". This is the mirror failure: the receiver is too
*strict* for the one case where identity is known structurally rather than by
name. The closure's parameter *is* the element that the `if let` / `let-else`
binding receives, so the two should be unified when the closure is the argument
of the call being bound, whatever either is called.

Worth deciding together with P1: shape 6 of the P1 fixture fails under both
rules at once, and a fix for either alone leaves it failing.

### P3 — The effect closure has no depth bound, so payload enums become requests  🐞

`enum_closure` (`crates/parser/src/core_finder.rs:118`) walks variant field types
transitively, and for effects it is called with `dispatched: None, delegating:
None` (line 86-92: *"Effect operations are constructed, never dispatched on"*).
Nothing then stops the walk at the operation layer. Given

```
Effect::Telemetry(TelemetryOperation)
  → TelemetryOperation::Signal { signal: TelemetrySignal, … }
      → TelemetrySignal::…  { domain: FailureDomain, threat: SecurityThreat, … }
```

every one of `TelemetrySignal`, `FailureDomain`, `SecurityThreat` joins
`effect_enums`, and from then on **any mention of one of their variants anywhere
in an update body is recorded as an effect request**. In the production core
this accounts for 228 of 711 effect mentions — **32%** — across 16 names that are
plain data enums or associated functions, plus 42 more from `TelemetrySignal`,
which is the payload of one request rather than a sibling of it:

| Recorded as an effect | Names | Mentions | What it actually is |
| --- | --- | --- | --- |
| `FailureDomain::of` | 1 | 88 | associated function |
| `Restriction::*` | 3 | 45 | plain enum |
| `SubmissionStep::*` | 3 | 32 | plain enum |
| `ApiFailure::from` and two more `::from` | 3 | 29 | `From` impls |
| `SecurityThreat::PrivilegedAccess` | 1 | 15 | plain enum |
| `SubmissionFailure::{Unavailable, Refused}` | 2 | 12 | plain enum |
| `PlatformStage::of` | 1 | 5 | associated function |
| two more | 2 | 2 | plain enums |

There are two independent defects here, and the second is much cheaper than the
first:

**P3a — no depth bound.** Only enums the `Effect` root wraps *directly* are
requests; anything deeper is payload. `capability_of` already computes exactly
that distinction (`core_finder.rs:40-51`), so the recording predicate can ask it
rather than asking `is_effect_enum` — but it has to ask for the root as well:

```rust
name == effect_root || capability_of(name).is_some()
```

because `capability_of` returns `None` for two different things — a payload enum
nothing wraps, and the root itself (`core_finder.rs:41-43`). Without the first
clause, an app whose root carries operations as its own variants
(`Effect::StartAudio { .. }`) loses every effect it has.

`Render` is *not* the reason for that clause, and a fixture asserting "`Render`
survives" would pass either way. It never reaches `record_effect_path`: a bare
`render()` is recognized as an unresolved external call and recorded by
`record_effect` (`crates/parser/src/transitions.rs:603`) with a literal label and
no capability. This core declares `Effect::Render(RenderOperation)` and its model
still carries `Render` as a bare, capability-less name, which is that path
showing. The fixture that discriminates is a root with an operation variant of
its own.

Deeper enums should stay in the model as payload types — they are worth
documenting, just not as things the shell is asked to do.

**P3b — associated functions are recorded as variants.** `record_effect_path`
(`crates/parser/src/transitions.rs:828`) takes whatever `enum_variant_path`
returns without checking that the last segment is a variant the enum declares.
`FailureDomain::of` and `ApiFailure::from` are function calls. The parser already
holds `decl.variants`; comparing against it is a few lines and cannot produce a
false negative. Its *yield* is a separate question — see the overlap note
below.

**The two overlap almost entirely — do not add their numbers.** Applying P3a's
predicate to this core's model keeps 441 of 711 mentions and removes **270
(37%)**: the 228 above plus `TelemetrySignal`'s 42, which is payload of one
request rather than a sibling of it. All five names P3b targets are payload enums
at depth ≥ 2 (this core's `Effect` root wraps `RenderOperation`, `AudioOperation`,
`TelemetryOperation`, `ApiRequest` and nine more — none of them), so P3a already
removes every one of P3b's 122. **P3b's marginal contribution after P3a, in this
core, is zero.** It is still worth doing on its own terms — an associated
function on a *depth-1* operation enum (`AudioOperation::of(..)`) passes P3a and
is exactly as wrong — but the 122 is P3a's number, not evidence of P3b's reach.

Both are also honesty-rule issues, not only noise: a document asserting that a
transition asks the shell to perform `FailureDomain::of` is a guess presented as
a fact.

The reader-visible consequence compounds with docgen: the worst diagram in that
core carries a **473-character edge label** and weighs 10.5 KB, which is not a
diagram any more.

### P4 — One dynamic branch drops the whole machine  🐞

```
warning: …/list.rs:23: transition of `DraftFilter` dropped: target state is dynamic
warning: …/list.rs:23: transition of `DraftFilter` dropped: target state is dynamic
```

The source is a chip toggle:

```rust
model.filter = if model.filter == filter { Filter::All } else { filter };
```

The `else` branch assigns the event's own payload and is genuinely
unresolvable — the warning is correct and welcome. But the `if` branch assigns
a literal, and it is dropped along with its sibling. With both transitions gone
the machine has none, and per `docs/parser.md`'s known limits a machine with no
transition never reaches the model: **`DraftFilter` is absent from the output
entirely**, one of 14 machines silently down to 13.

Two things to decide:

- keep the resolvable branch rather than dropping both, and
- give the unresolvable one a target it can be drawn with — the wildcard-target
  note already exists for this class, and a transition to "somewhere the source
  computes" is more honest than an absent machine.

The honesty rule says nothing is silently dropped, and strictly this obeys it:
there is a warning. But the warning names a transition, while the thing lost is
a machine, so the diagnostic under-reports its own consequence. If the branch
cannot be kept, the warning should at least say that the machine went with it.

### P5 — An effect callback that resolves in isolation but not in the tree  🔍

```
warning: …/storage.rs:87: effect callback not resolved: the event this request
is answered with is not named at the call site
```

for

```rust
Command::request_from_shell(operation).then_send(|result| {
    Event::Storage(match result {
        StorageResult::Measured { entries } => StorageEvent::Measured { entries },
        StorageResult::Purged { freed_bytes } => StorageEvent::Purged { freed_bytes },
        StorageResult::Failed(message) => {
            tracing::warn!(target: "core.storage", message, "…");
            StorageEvent::Failed
        }
    })
})
```

**Not reproduced.** Three fixtures failed to trigger it: the variant constructor
wrapping the `match`; the same with struct-variant payloads and a block-bodied
arm; and the same again with the payload enum declared as `Event` in its own
module and imported as `use crate::storage::{Event as StorageEvent}` — the alias
shape the real file uses. All three resolved. Worth noting what they resolved
*to*, since it may be the same defect seen from the other side: the answer came
back as `Storage`, the **outer** variant, not the three `StorageEvent` variants
the closure actually returns.

So this one needs a repro built from the real tree before it can be fixed.
Whoever picks it up should start from the resolution naming the wrapper instead
of the wrapped events.

It is *one of two* warnings failing that project's `docs --deny-warnings`, and
fixing it does not turn that build green: P4's site keeps emitting
`dynamic-target`, which is a `WarningKind` (`crates/parser/src/lib.rs:78`) and
counts like any other — deliberately, since that branch really does assign a
runtime value, so the warning is correct and permanent. Unblocking that CI is the
adopter's move (a recipe that generates and a separate gate that checks), not
this front's. Worth stating because it is easy to sequence lib work against a
red build that lib work cannot turn green.

### P6 — Warnings are emitted more than once  🐞

The `DraftFilter` warning appears twice and the `storage.rs:87` one three times,
same file, same line, same text. Deduplicating on `(file, line, kind)` before
reporting is cosmetic but it changes how a warning list reads: three identical
lines look like three problems.

---

## 2. Docgen

### D1 — Byte-identical duplicate edges  🐞

One diagram in that core emits **19 duplicate edge lines**, and the model has 20
duplicate `(from, event, to)` triples. They arise where a shared helper (`fail`,
`await_network`) is reached by two call paths: the transitions are identical
except for `resolves_with`, which `transition_label`
(`crates/docgen/src/lib.rs:399`) does not render. Mermaid then draws two
identical arrows on top of each other.

Two fixes, worth doing at both levels:

- in the model, merge transitions identical but for `resolves_with` (union the
  answers) — two call paths to the same helper are not two transitions;
- in `machine_diagram` (`crates/docgen/src/lib.rs:250`), skip an edge line
  already emitted. An identical rendered line carries no information by
  construction, whatever the model says.

### D2 — The `final` role is degenerate on wildcard-driven machines  ⚖️

`MachineRoles::of` (`crates/docgen/src/roles.rs:42-80`) excludes wildcard
escapes from "leaves this state" — deliberately, and the reasoning in the module
doc is sound in isolation: counting them "would erase every final state of every
machine that has one". Applied to a real core it inverts:

- **34 states across 13 machines are marked `final`**;
- one machine marks 8 of its 9 states final, and four mark *all* of theirs;
- a state named `Downloading` is marked final;
- six states come out as `initial, final` at once, drawing `[*] --> X` and
  `X --> [*]` in the same diagram.

`Downloading` being terminal is worse than a machine reporting no terminal
state, because the second is a shape a reader can interpret and the first is
simply false.

**The degeneracy is per state, not per machine.** Measured on this core, both
candidate rules and what each keeps:

| Rule | Marks kept |
| --- | --- |
| today — nothing leaves it *by name* | 34 |
| no role for a machine that is entirely wildcard-sourced | 11, of which **8 are one machine** |
| no role for any state a wildcard can leave | 0 |

The middle rule is per machine, so it leaves the worst offender untouched: the
machine marking 8 of 9 states final has only 1 wildcard transition out of 7, so
it is not "entirely wildcard-sourced" — and that one transition is

```
* -- InsightsUpdated -> *
```

wildcard in the source *and* the target. Every state can leave through it, so
none of the 8 is final in any sense, and 8 of the 11 survivors are exactly the
marks worth removing. The strict rule is honest and empties the feature: every
machine in this core has at least one wildcard-sourced transition, so it keeps
nothing anywhere.

Which suggests the role is not binary. "Nothing leaves this state by name" is a
real fact and a different one from "terminal"; keeping it in the states table
under a word that says so, while `X --> [*]` is drawn only where no wildcard can
leave, fixes `Downloading` without emptying the feature.

One coupling to fix with it: the diagram draws roles unconditionally while the
states table is gated on `has_documented_states`. The machine holding 7 of these
34 marks has no states table at all (0 of 7 states described), so its `X --> [*]`
arrows are asserted in the one place a reader cannot check them against a
description.

Interacts with P1: much of the wildcard traffic that degenerates this is a guard
clause that should have narrowed. Worth re-measuring after P1 rather than tuning
against today's numbers.

### D3 — Edge labels carry everything, with no cap  ⚖️

`transition_label` joins every effect of a transition. Two observations from the
real output:

- **`Render` appears in 195 of 711 effect mentions.** It is on nearly every
  transition, so in a diagram it is the one item that distinguishes nothing.
  Dropping it from the edge label (keeping it in the table, which is complete by
  contract) shortens most labels for free.
- The longest label is 473 characters. `answers_cell`
  (`crates/docgen/src/lib.rs:376-395`) already caps answers at
  `ANSWERS_IN_A_CELL = 3` with a `+n more` suffix, for a reason that reads as if
  it were written for this case — "too many for a table cell, and never dropped
  silently". The same decision has not been applied to effects in an edge label,
  where the pressure is higher, because a diagram edge cannot scroll.

P3 removes about a third of this on its own. The cap is still worth having: the
remaining labels in the biggest machines stay long after the noise is gone.

### D4 — `\n` as the label line break is renderer-dependent  ⚖️

`transition_label` emits a literal `\n`. In mermaid 11.6.0 this works: labels go
through

```js
if (fr(me().flowchart.htmlLabels)) { i = i.replace(/\\n|\n/g, "<br />") … }
```

so with `flowchart.htmlLabels` at its default (`true`) the break happens. The
`else` path splits on `lineBreakRegex` (`/<br\s*\/?>/gi`) only, so a renderer
configured with `htmlLabels: false` shows a literal `\n` in every edge label of
every diagram.

`<br/>` works on **both** paths, which makes it strictly more portable. It needs
the separator exempted from `mermaid_label`'s `<` → `#60;` escaping — build the
label from escaped parts joined by the raw tag, so author prose stays escaped
and only the separator is markup.

### D5 — A 1000-line document with no index and no provenance  ⚖️

The generated document for that core is 134 KB / 1027 lines / 13 machines, and:

- there is no table of contents and no anchor list, so a machine is found by
  scrolling;
- nothing says where a transition is written. This is the single most requested
  affordance for a document this size, and the information exists — warnings
  already carry `file:line`. It needs M2 below.

Two smaller ones: a state's first paragraph is rendered three times (truncated
in the mermaid note, in the states table, and again in the per-state section
when the doc has more than one paragraph), and the mermaid note truncated at 72
characters sits directly above the table cell holding the same text in full.

---

## 3. Model

### M1 — `Transition` has no guard  ⚖️

`crates/model/src/lib.rs:417-424` carries `from`, `event`, `to`, `effects`.
Without a condition field, transitions that differ only by a branch are
indistinguishable in every client. One machine in that core emits three
transitions from `Idle` on the same event, to three different targets, with the
condition — which is right there in the source — nowhere in the output. The
reader cannot tell which one they are looking at.

Mermaid has the notation already: `Event [guard] / effects`. This is also a
prerequisite for P1 being *visible*: narrowing the source states of a guard
clause helps, but where a single source state fans out on one event, only the
condition separates the arms.

### M2 — No source span  ⚖️

A `file:line` on `Transition` and `StateDecl` turns the generated document from
a thing you read into a thing you navigate, and it is what D5 needs. The parser
has the spans — every warning carries one.

### M3 — No path to the machine in the `Model`  ⚖️

Nothing in the output says whether a machine is a singleton or one instance per
record. In that core, one machine lives at `flags.identity` (one per app) and
another at `drafts[].submission.status` (one per draft, many at a time, driven by
a sweep over all of them). Both render identically, and the reader defaults to
assuming the first — which is wrong for exactly the machine that is hardest to
reason about. The parser knows the field path it extracted the machine from.

---

## 4. What the numbers were

For calibration, and worth re-measuring after P1 and P3 land:

| | |
| --- | --- |
| machines | 13 (14 in the source — see P4) |
| transitions | 197 |
| wildcard-sourced | 100 (51%) |
| machines 100% wildcard | 6 of 13 |
| states | 63 |
| effect mentions | 711, of which 270 are not requests (P3) |
| `Render` share of mentions | 195 (27%) |
| duplicate `(from, event, to)` | 20 |
| states marked `final` | 34, seven of them in a machine with no states table |
| generated document | 134 KB, 1027 lines, 36 KB of mermaid |
| largest single diagram | 10.5 KB, longest edge label 473 chars |

The same run also reported 79% documentation coverage with the biggest machine
at 0 of 7 states described — which is the tool working. `crux-analyzer coverage
--min` was not wired into that project's `check` recipe, so nothing was failing
on it. Worth a line in `docs/cli.md` about wiring `coverage` into CI beside
`docs --deny-warnings`, since adoption evidently does not find it on its own.

## 5. A suggested order

Cheapest-first, and each one is independently shippable:

1. **P3b** — variant check in `record_effect_path`. A few lines, no possible
   false negative. Cheap rather than high-yield: in this core P3a subsumes all
   122 of its mentions, and what it catches on its own is an associated function
   on a depth-1 operation enum.
2. **D1** — dedupe edges in `machine_diagram`, then merge in the model. Small,
   and the worst diagram improves immediately.
3. **P3a** — depth bound on the effect closure. This is the one that removes the
   noise: 270 of 711 mentions, and it shortens every label.
4. **P1** — guard clauses. The largest correctness win, and the one that stops
   adopters contorting their code for the tool.
5. **P2** — rename-invariant closure receivers. Decide with P1.
6. **D2** — `final` only with evidence, re-measured after P1.
7. **P4**, **P6**, **D3**, **D4** — the remaining diagnostics and output nits.
8. **M1**–**M3**, then **D5** on top of M2.

**P5** is unblocked only by a repro from the real tree, and it is what keeps one
adopter's CI red today, so it deserves a look out of order if that repro can be
obtained.
