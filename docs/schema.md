# Schema — the model contract

> 🌐 **English** · [Português (Brasil)](pt-BR/schema.md)

The contract lives at [`shared/schema/crux-model.schema.json`](../shared/schema/crux-model.schema.json)
(JSON Schema draft 2020-12). Every client — the web UI, the doc generators,
anything future — depends only on this document. A bundled example is kept at
[`shared/schema/examples/audio-recorder.json`](../shared/schema/examples/audio-recorder.json)
and a round-trip test in `crates/model` keeps the Rust structs aligned with it.

## Shape

```json
{
  "project": "Audio Recorder",
  "cores": [
    {
      "name": "Recorder",
      "machines": [
        {
          "name": "RecorderState",
          "doc": "Where a recording session lives.",
          "states": [
            "Idle",
            "Recording",
            "Paused",
            "Uploading",
            {
              "name": "Failed",
              "doc": "The upload gave up. The session is kept so the user can retry.",
              "markers": ["failure"],
              "tags": ["retryable"]
            }
          ],
          "transitions": [
            {
              "from": "Idle",
              "event": "RecordPressed",
              "to": "Recording",
              "effects": [
                {
                  "name": "AudioOperation::Start",
                  "capability": "Audio",
                  "resolvesWith": ["RecordingStarted", "RecordingFailed"]
                }
              ]
            }
          ]
        }
      ],
      "events": [
        { "name": "RecordPressed", "doc": "The user hit the record button." }
      ],
      "effects": [
        { "name": "AudioOperation::Start", "doc": "Begins capturing audio." }
      ]
    }
  ]
}
```

## Semantics

| Field | Meaning |
| --- | --- |
| `project` | Name of the analyzed project. |
| `cores[]` | One entry per `impl App` block found. |
| `machines[]` | Statechart **orthogonal regions**: one per state enum driven by the core. Name is the enum's name, disambiguated by field when the same enum drives two machines (`State (left)`). |
| `machines[].doc` | Optional. Documentation authored on the state enum itself, annotation lines removed. |
| `machines[].markers[]` | Optional. Markers declared on the state enum — they describe the whole region. |
| `machines[].tags[]` | Optional. Free-form tag names declared on the state enum. |
| `states[]` | Leaf states, in declaration order — **a bare string or an object** (see below). Children of **composite states** are `Parent/Child` paths (`Active/Loading`). A client that ignores the convention still renders a valid flat machine. |
| `states[].name` | The leaf state's name. The bare-string form is exactly this field. |
| `states[].doc` | Optional. Documentation authored on the enum variant, annotation lines removed. |
| `states[].markers[]` | Optional. Declared markers, in first-seen order: `"failure"`, `"deprecated"`. |
| `states[].tags[]` | Optional. Free-form tag names declared with `@tag <name>`, in first-seen order. |
| `states[].default` | Optional (`false`). The source declares this state as its enum's `#[default]` variant. Evidence about where the machine starts, **not** a role — clients derive `initial` from it plus the shape of the transitions. |
| `transitions[].from` | Source state, or `"*"` — the transition fires from **any** state (statically unguarded). |
| `transitions[].event` | Leaf event variant name that triggers the transition. |
| `transitions[].to` | Target state, or `"*"` — the target is decided at **runtime** (e.g. carried by the event payload). |
| `transitions[].effects[]` | Optional. Effects requested when the transition fires — **a bare string or an object** (see below). Omitted when empty. |
| `effects[].name` | The operation as transitions label it: `"AudioOperation::Start"`, or `"Render"` for crux's builtin. The bare-string form is exactly this field. |
| `effects[].capability` | Optional. The variant of the core's root `Effect` enum that wraps this operation (`Effect::Audio(AudioOperation)` → `"Audio"`). Absent when the request goes through none, or when it could not be resolved. |
| `effects[].resolvesWith[]` | Optional. Events the shell can answer this request with, as declared at the request site — several when the callback maps one event per outcome. Absent for fire-and-forget requests. An event here need not appear in any transition: a confirmation the core only renders is real behavior. |
| `effects[].conditional` | Optional (`false`). The request sits on a branch the transition itself does not imply: arriving there *may* request it. |
| `cores[].events[]` | Optional. `{ name, doc }` pairs: documentation authored on event enum variants, **only** for events that appear in this core's transitions and **only** when documented — the transition tables already enumerate the vocabulary. Omitted when empty, so an undocumented app emits exactly the JSON it emitted before this field existed. |
| `cores[].effects[]` | Optional. Same for effects, keyed by the label transitions carry (`AudioOperation::Start`, `Render`). |

## Effects, and the loop they close

A transition's `effects[]` entry is written **either** as a bare operation label
**or** as an object adding what the analyzed source declares around the request.
Same widening as `states[]`, same reason: an app whose requests show neither a
capability nor a callback emits exactly the JSON it emitted before those fields
existed.

```json
"effects": ["Render", { "name": "HttpOperation::Upload", "capability": "Http", "resolvesWith": ["UploadFinished"] }]
```

`resolvesWith` is the return leg of Crux's `Event → Effect → shell → Event`
loop, and the reason it is in the contract: a state graph shows the events going
in, and without this nothing says which of them the *shell* sends back. It is a
set because one request has one answer per outcome, and it is only ever what the
source names at the request site — never inferred from an operation's name. See
[parser.md](parser.md#effects) for what counts as evidence.

`conditional` is the honesty rule applied to attribution. An effect requested on
a branch below the assignment is neither dropped nor stated flatly: it travels
with the transition and says that arriving there *may* request it.

## Documented states

A state is written **either** as a bare name **or** as an object carrying what
the analyzed source documents about it. A producer emits the bare form whenever
there is no documentation, so a model of an unannotated application is identical
to a pre-documentation one:

```json
"states": ["Idle", { "name": "Failed", "markers": ["failure"] }]
```

Both forms may appear in the same array. Clients should normalize on the way in
(`typeof state === 'string' ? { name: state } : state`) so nothing downstream
branches on the authored shape.

`markers` is a **closed vocabulary** — crux_analyzer's own, which is why clients
render a localized label for each value while the value itself stays a stable
identifier. `initial` and `final` are deliberately *not* markers: they are
*derived*, so declaring them would let a source contradict the transitions it
also declares. See [parser.md](parser.md#documentation-and-annotations) for how a
source authors these.

## Where a machine starts

`default` is the one key of a state object that is not documentation: it says the
source wrote `#[default]` on that variant. It is also the only key that makes an
otherwise plain state take the object form, so an app whose state enums derive
`Default` emits one state object per machine that a pre-`default` model wrote as a
bare string.

```json
"states": [{ "name": "Idle", "default": true }, "Recording"]
```

The model stops there, at what the source declares. Turning it into the `initial`
role is the client's job, and every client should read it the same way:

1. the state whose `default` is true;
2. otherwise every state no transition arrives at;
3. otherwise — a fully **cyclic** machine, where neither kind of evidence
   exists — the first state in `states[]`.

Declaration order is last on purpose: in a cycle it carries no meaning, which is
exactly why `default` is in the contract. The two implementations are
`crates/docgen/src/roles.rs` and `apps/web/src/domain/stateRole.ts`; `final` needs
no evidence of its own, being a state no transition leaves (a machine-wide `"*"`
source does not count — that escape belongs to the wildcard pseudo-node).

The schema pins that vocabulary, so a typo in a hand-written model is a
validation error — but **clients should ignore a marker they do not know** rather
than reject the model, so a newer parser never blanks an older UI. That
strict-schema / lenient-client asymmetry is deliberate.

## The contract is locale-independent

Every string in the model is read out of the analyzed application — identifiers
and, since documentation reached the model, the author's own prose (`doc`) and
tag names. None of it is translated: crux_analyzer copies author prose verbatim,
exactly as it copies a state name, so the *language* of that text is the author's
choice and not ours. No text **of crux_analyzer's own** ever enters the model, so
`crux-analyzer generate` still produces byte-identical JSON in every locale, and
clients localize their own chrome (the prose standing in for `"*"`, table
headers, marker labels, panel titles). Adding a locale must never add a field
here. See [i18n.md](i18n.md).

## Wildcards

`"*"` is a reserved state name on both ends of a transition:

- `from: "*"` — fires from any state. UIs render a pseudo-node ("any state");
  simulation offers these transitions from every state.
- `to: "*"` — lands wherever the runtime value says. Simulation excludes
  these from replay (there is nothing static to land on).

## Evolution guidelines

- Additive fields (like `effects`) are optional with empty defaults, so old
  clients keep working.
- A value that was a bare string can be **widened** to "string or object" by
  making the object form optional and emitting the bare form whenever the extra
  data is empty (how `states[]` gained documentation, and then `effects[]`). Existing artifacts stay
  valid and unannotated output stays byte-identical, so the change is additive
  in practice — but the clients still move in the same commit, because the
  producer starts emitting objects the moment a source is annotated.
- Breaking shape changes (like the `machines[]` introduction) change every
  layer in the same commit: schema, `crates/model` (+ round-trip test), the
  bundled example, `crates/docgen`, `apps/web/src/schema` + domain + tests.
- The web app treats an invalid generated model as absent (falls back to the
  bundled example with a console warning) so stale artifacts never break it.
