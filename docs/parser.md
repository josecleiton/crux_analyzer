# Parser

> 🌐 **English** · [Português (Brasil)](pt-BR/parser.md)

`crates/parser` statically reconstructs a Crux app's state machines from its
Rust sources. It never executes code and never depends on Crux — everything
is derived from the `syn` AST.

## Pipeline

1. **Load** — every `.rs` file under `--src` is parsed and flattened
   (no module-tree resolution). `#[cfg(test)]` modules are skipped so test
   helpers never contribute states or transitions.
2. **Index** — enums (all declarations per name — names may collide across
   modules — plus `use ... as ...` aliases), structs (field name/type),
   functions (by `(self type, name)`, with parameter names). Variant field
   types look through `Box`/`Rc`/`Arc`.
3. **Detect machines** — see below.
4. **Find Cores** — every `impl App for X` block. The `Event` associated
   type seeds the event-enum closure (nested event enums like
   `Event::Recording(RecordingEvent)` are followed); the `Effect` associated
   type seeds the effect closure the same way.
5. **Extract transitions** — walk `update` and every helper it calls
   (cross-file, cycle-safe), carrying context; emit a transition at each
   state assignment.
6. **Emit** — group transitions by `(enum, field)` into one machine per
   region, deduplicate, attach effects.

## Machine detection

A state machine is a pair `(enum, field)` with **assignment evidence**:

- direct: `*.field = Enum::Variant` (any construction form),
- via reset: `*.x = T::default()` where struct `T` has a field `field: Enum`, or
- via **value flow into a model-reachable field**: `*.field = <runtime value>`,
  where the `Model` associated type reaches a field of that name typed as an
  enum the crate dispatches on in patterns.

No naming convention is required. Assignment is the discriminating signal:
ViewModel mirror enums are only ever *constructed* into view structs, never
assigned to a model field, so they never become machines.

The third form exists because which side owns a transition decides how the core
writes it. A status the *shell* drives is only ever stored by the core — from an
event payload, or cloned from another record — so it never names its enum at the
assignment, and the first two forms miss it entirely. Model reachability is what
supplies the missing type: the walk starts at the `Model` associated type and
follows struct fields through the containers that hold them (`Vec<Entry>` →
`Entry`, maps through their value type), so a status held once per entity still
counts. A mirror enum is not reachable, so requiring reachability keeps the
original exclusion intact without a naming heuristic.

Two limits worth naming. Only struct fields are followed, so a struct sitting
behind an enum variant is not reached. And reachability is an *additional* path,
not a filter on the first two: an enum assigned literally still becomes a machine
whether or not the model holds it.

The same enum may drive several machines through different fields (two
sessions of the same type); transitions are attributed by `(enum, field)` and
machine names disambiguate: `State (left)`, `State (right)`.

### Composite states

A variant with exactly one unnamed field whose type is another crate enum
(`State::Active(ActiveState)`) becomes a **composite state** — leaves
`Active/Loading`, `Active/Ready`, ... — but only with **sub-state evidence**:
a nested variant pattern (`State::Active(ActiveState::Loading)`) somewhere in
the crate. Without that evidence the field is payload data
(`State::Failed(ErrorCode)`) and the variant stays a plain leaf, so
`model.state = State::Failed(reason)` resolves to `Failed` like any other
target.

Pattern resolution is deep: `Active(Phase::Ready)` → the exact leaf;
`Active(_)` → every child leaf.

## Events

Event labels are the **leaf** variant names. Wrapper variants that only carry
a nested event enum (`Event::Recording(RecordingEvent)`) delegate: the inner
match resolves the label. An enum only qualifies as a nested event enum when
the code actually **dispatches on it** (its variants appear in patterns) —
payload enums carried by an event (`Event::Boom(ErrorCode)`) stay data, and
state enums are excluded explicitly.

Multi-event arms fan out: `event @ (A | B) => ...` produces one transition
per event.

## Source states (`from`)

Resolved per machine at each assignment, from three kinds of evidence:

| Evidence | Example | Result |
| --- | --- | --- |
| `matches!` guard | `if matches!(state, A \| B)` | `{A, B}` |
| negation | `if !matches!(state, Idle)` | complement |
| `==` / `!=` comparison | `state == State::Idle` (also inside `find(\|d\| ...)` closures) | `{Idle}` / complement |
| `match` on the state | arm patterns; `_` resolves to the complement of earlier arms | per-arm sets |
| predicate method | `state.has_capture_in_flight()` — the method body (on the state enum's impl) is analyzed, negation and predicate-calling-predicate included (depth-capped) | the predicate's set |
| let-else narrowing | `let Some(d) = list.find(\|d\| d.state == X) else { return }` | holds for the rest of the block |
| **no evidence at all** | unguarded assignment | wildcard `"*"` — the transition legitimately fires from any state |

Conditions compose through `&&` (intersection), `||` (union of concrete
sides), `!` (complement). A concrete constraint wins over an unresolvable
conjunct — the emitted set may then be a superset of the truth, which is the
right bias for documentation.

**Source evidence is keyed by field name, not by object.** A guard on
`other.status` therefore constrains a machine on `field: status` even when
`other` is a *different* record of the same type — the value mirror used for
targets is keyed by exact path, but this one is not. Where such a guard is
conjoined with one on the record being written (`this.status == Pending &&
matches!(other.status, Done | Deferred)`), the two sets intersect to nothing.
That contradiction cannot describe a real branch, so rather than dropping the
transition in silence it is reported as `unresolvable-source`. Resolving the
aliasing is planned — [roadmap.md §6](roadmap.md).

## Targets (`to`) and value-flow

- Literal constructions resolve directly (composite children included).
- `*.x = T::default()` implies every state field of `T` lands on its enum's
  `#[default]` variant.
- **Event payload**: `draft.status = status` where `status` is a binding from
  the event pattern typed as the state enum → target `"*"` (the landing
  state is the shell's choice).
- **Constrained values**: `draft.st = known.st.clone()` guarded by
  `is_this_runs_answer(&known.st)` — the predicate (free fn or method) is
  resolved against its parameter and the target fans out to the variants it
  allows. Value constraints are keyed by the **exact expression path**
  (`known.st` never leaks onto `draft.st`), and only identity-preserving
  calls (`clone`, `to_owned`, `as_ref`, ...) are looked through — `.take()`
  or accessors never alias.

## Effects

An effect is a **request with a return leg**, and that is how it is read: the
operation, the capability it travels through, and the events the shell can
answer it with.

What counts as a request:

- constructions of effect-closure enums (`AudioOperation::Start`), labeled
  `Enum::Variant`;
- a call to crux's bare `render()` → `Render`.

### The capability

The Core's `Effect` enum is what names its capabilities: the variant that wraps
an operation's enum *is* the capability the operation goes through.
`Effect::Audio(AudioOperation)` puts every `AudioOperation` request under
`Audio`. Structure, not inference — nothing is read off the shape of a name.
`Render` goes through none.

### The answer

Crux's loop is `Event → Effect → shell → Event`, and the return leg is written
at the request site, so it is evidence like any other. Three shapes are read,
and the events they name become the request's `resolvesWith`:

```rust
// the callback alongside the operation
Self::request_audio(AudioOperation::Start, Event::CaptureStarted);

// crux's Command API
Command::request_from_shell(operation).then_send(Event::Started);

// a callback that maps the shell's result: every event it can build
Command::request_from_shell(operation).then_send(move |result| match result {
    AudioResult::Started { id } => RecordingEvent::RecordingStarted { id },
    AudioResult::Failed(message) => RecordingEvent::RecordingFailed { message },
})
```

A **set**, not one event: a request routinely has one answer per outcome. A
request built by a shared helper (`audio_command(op)` whose body does the
`then_send`) is followed one call deep, so the operation the caller wrote keeps
the answers its helper declares — which also means every request through that
helper carries the union of what its callback can build. That is the honest
reading: as far as the source shows, any of them can come back.

An answer that names an event **no transition carries** is kept. It is real
behavior — a confirmation the core only renders — and the clients say so rather
than hiding it.

A callback whose event cannot be read off the call site (`then_send(f)`) is an
`unresolved-effect-callback` warning: the request is still recorded, only its
answer is unknown. A request that declares no callback at all is *not* a
warning — fire-and-forget is a legitimate shape.

### Which transitions a request belongs to

Effects are scoped by **branch**, not by arm: the chain of alternatives
(`if`/`else` branches, `match` arms) entered to reach a request is compared with
the chain of the assignment that made the transition.

- Same chain → the request belongs to that transition.
- Forked apart → it does not. A request in one branch never lands on the
  transitions of its sibling.
- Deeper than the transition → it belongs, marked **conditional**: arriving
  there *may* request it. Over-approximating is right (the request is real), and
  saying so beats reading as certainty.

```rust
RecorderEvent::RetryPressed => {
    if session.attempts_left() {
        state = Uploading;                     // ← Upload, certain
        Self::request(HttpOperation::Upload).then_send(RecorderEvent::UploadFinished);
    } else {
        state = Idle;                          // ← Render only
        render();
    }
}
```

## Documentation and annotations

Doc comments on a state enum reach the model: the enum's own `///` becomes the
machine's description, and each variant's becomes its state's.

Doc comments on **event and effect enum variants** reach the model too, as
per-core `events` / `effects` catalogs (`{ name, doc }`). Two restrictions
keep the catalogs honest: only names that appear in the core's model enter (a
documented delegating wrapper like `Event::Recorder(RecorderEvent)` is not an
event a transition can carry) — an event named as an effect's answer counts,
since it is in the model and a client showing it should be able to show its
prose — and only documented names enter, because the transition tables already
enumerate the vocabulary. Annotations (`@…` lines) are not read on events or
effects; there is nothing for a marker to mean there yet.

```rust
/// Where a recording session lives.
pub enum RecorderState {
    /// Nothing is being recorded yet.
    Idle,

    /// The upload failed. The session is kept so the user can retry.
    ///
    /// @failure
    /// @tag retryable
    Failed { reason: String },
}
```

`///`, `/** … */` and a hand-written `#[doc = "…"]` all work; the common
indentation is stripped the way rustdoc strips it, and the author's line
wrapping is never reflowed. `#[doc(hidden)]` is ignored — it does not hide a
state.

**Annotations** are `@` lines written inside the doc comment. That is the only
mechanism that needs no dependency in the analyzed crate: crux_analyzer must
never be a dependency of the apps it reads, so a real attribute is out and a
bare unknown one would not compile.

| Annotation | Meaning |
| --- | --- |
| `@failure` | The state stands for a failure the app recognizes as such. |
| `@deprecated` | The state is on its way out. |
| `@tag <name>` | A free-form label (`retryable`, `offline`). Several names may share one line, separated by spaces or commas. |

Markers are a **closed vocabulary**; `@tag` is the open-ended escape hatch.
There is deliberately no `@initial` or `@final`: those are derived from graph
shape and `#[default]`, so declaring them would let a source contradict the
transitions it also declares.

Recognized lines are removed from the description, and runs of blank lines are
then collapsed — so an annotation written between two paragraphs produces
exactly the same prose as one written at the end.

### What is an annotation, and what is prose

The rule is one sentence: **a line is an annotation only when it is complete and
well-formed; anything else is prose.** Keywords match case-insensitively, so a
capitalization slip still works.

| Line | Read as |
| --- | --- |
| `@failure`, `@FAILURE` | the marker |
| `@tag retryable, offline` | two tags |
| ``Apple constrains it — `@Generable` leaves no room`` | prose — `@` is not the first character |
| `Ask support@example.com` | prose |
| `@deprecated` inside a ` ``` ` fence | prose — fenced blocks are samples |
| `\@failure is how you mark one` | prose, with the backslash dropped — the escape hatch |
| `@failur`, `@see`, `@tag`, `@failure because …` | **a warning** (see below) |

An annotation-shaped line that is not recognized is reported rather than left
in the prose, because a silently inert `@failur` is exactly the quiet wrong
answer the honesty rule exists to prevent. Only enums that actually became
machines are inspected, so a doc comment on an unrelated enum never warns.

### Composite states

A composite parent has no node of its own in the model — only its
`Parent/Child` leaves. So each leaf **inherits** the parent variant's
documentation: markers and tags union (parent first), and the parent's prose is
placed above the child's rather than being replaced by it. Nothing the author
wrote is dropped.

## Warnings reference

The honesty rule: what cannot be inferred statically is surfaced, never
silently dropped, never guessed. All warnings carry `file:line`.

A warning is **data**, not a string: `Warning { file, line, kind }` where
`kind` is a `WarningKind`. `kind.code()` is the stable, locale-independent
identifier — key tooling and documentation on that, since the message text is
localized ([i18n.md](i18n.md)). The English rendering is shown below.

| Code | Message (`en`) | Meaning |
| --- | --- | --- |
| `unknown-event` | `could not infer the triggering event` | a state assignment was reached with no event label in scope (e.g. under a catch-all arm with unknown context) |
| `unresolvable-source` | `source-state condition could not be resolved statically` | the guard references the state but defeats analysis: an unresolvable predicate, or guards that intersect to nothing because two objects' same-named fields were read as one (see below) |
| `dynamic-target` | `target state is dynamic (assigned from a runtime value)` | the assigned value has no payload typing and no resolvable constraints |
| `no-update-method` | `core X: no update method found` | an `impl App` block without an `update` fn |
| `unknown-annotation` | `unrecognized annotation X: not one of @failure, @deprecated, @tag <name>` | a doc line looked like an annotation but is not one: a typo, a marker given an argument, or a `@tag` with no usable name |
| `unresolved-effect-callback` | `effect callback not resolved: the event this request is answered with is not named at the call site` | a `then_send` whose argument builds no event (a function, or a value computed elsewhere). The request is recorded; its answer is not |

The `mini_recorder` fixture extracts with **zero** warnings, which `just check`
enforces with `--deny-warnings`. A real target app currently reports the
field-name aliasing described under [Source states](#source-states-from); that is
a known gap with a planned fix, not a property of the app.

### Resource warnings

The same rule applied to resources: the analyzer is pointed at source it does not
control, so it has caps — and a cap that fires is reported rather than truncating
the model in silence. `--deny-warnings` therefore turns a truncated analysis into
a failed run. The caps and their flags are in [security.md](security.md#3-every-unbounded-input-dimension-gets-a-cap-and-every-cap-that-fires-is-reported)
and [cli.md](cli.md#resource-limits).

| Code | Message (`en`) | Meaning |
| --- | --- | --- |
| `analysis-truncated` | `core X: analysis stopped at the Y limit — the model may be incomplete` | the step, depth or call-depth budget ran out; transitions may be missing |
| `file-too-large` | `file skipped: N bytes exceeds the M-byte limit` | over `--max-file-size`, not read |
| `input-too-large` | `remaining files skipped: the run reached the M-byte total source limit` | over `--max-total-size`; the walk stopped |
| `nesting-too-deep` | `file skipped: brackets nest deeper than N levels` | `syn` recurses over nesting and a stack overflow would abort the process, so this is checked on the raw text before parsing |
| `not-a-regular-file` | `path skipped: not a regular file (symlink, device or FIFO)` | a symlinked `.rs` would read outside the tree; a FIFO would hang |
| `source-unreadable` | `path skipped: <reason>` | a walk or metadata error — previously silent, which made a permission problem look like a complete model |

## Known limits

- Bindings do not flow through helper-call parameters (a payload binding
  passed into a helper under another name loses its typing; constraints
  inside the helper still apply).
- Guard evaluation matches state fields by last field name within a scope;
  two same-named fields of the same enum in one scope can cross-narrow.
- Composite children wrapped in generics other than `Box`/`Rc`/`Arc` are not
  followed.
- Name collisions between modules are resolved by same-file preference and
  alias hints (`use path::Event as RecordingEvent`), not full module-tree
  resolution.
- Documenting a state enum does not make it appear: a machine still needs at
  least one extracted transition to reach the model.
- When two declarations share a name, documentation comes from whichever wins
  the collision (the one with most variants), like everything else about it.
- `#[cfg_attr(…, doc = "…")]` is not followed, and rustdoc intra-doc links
  (`` [`Self::Unavailable`] ``) travel verbatim — they only resolve in rustdoc.
- A misspelled annotation is reported, not corrected: there is no "did you
  mean" matching, because guessing at near misses would trade one quiet wrong
  answer for another.
