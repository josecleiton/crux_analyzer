# Parser

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

- direct: `*.field = Enum::Variant` (any construction form), or
- via reset: `*.x = T::default()` where struct `T` has a field `field: Enum`.

No naming convention is required. Assignment is the discriminating signal:
ViewModel mirror enums are only ever *constructed* into view structs, never
assigned to a model field, so they never become machines.

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

Each event arm's requested operations attach to the transitions it produces:

- constructions of effect-closure enums (`AudioOperation::Start`), labeled
  `Enum::Variant`;
- a call to crux's bare `render()` → `Render`.

An arm that produces several transitions shares its effect set among them
(an over-approximation for arms with internal branching).

## Warnings reference

The honesty rule: what cannot be inferred statically is surfaced, never
silently dropped, never guessed. All warnings carry `file:line`.

| Warning | Meaning |
| --- | --- |
| `could not infer the triggering event` | a state assignment was reached with no event label in scope (e.g. under a catch-all arm with unknown context) |
| `source-state condition could not be resolved statically` | the guard references the state but defeats analysis (e.g. an unresolvable predicate) |
| `target state is dynamic (assigned from a runtime value)` | the assigned value has no payload typing and no resolvable constraints |
| `core X: no update method found` | an `impl App` block without an `update` fn |

A clean corpus run (the Corpus test) extracts with **zero** warnings.

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
