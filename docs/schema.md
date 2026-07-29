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
              "effects": ["AudioOperation::Start"]
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
| `transitions[].from` | Source state, or `"*"` — the transition fires from **any** state (statically unguarded). |
| `transitions[].event` | Leaf event variant name that triggers the transition. |
| `transitions[].to` | Target state, or `"*"` — the target is decided at **runtime** (e.g. carried by the event payload). |
| `transitions[].effects[]` | Optional. Effects requested when the transition fires: `"Render"`, `"AudioOperation::Start"`, ... Omitted when empty. |
| `cores[].events[]` | Optional. `{ name, doc }` pairs: documentation authored on event enum variants, **only** for events that appear in this core's transitions and **only** when documented — the transition tables already enumerate the vocabulary. Omitted when empty, so an undocumented app emits exactly the JSON it emitted before this field existed. |
| `cores[].effects[]` | Optional. Same for effects, keyed by the label transitions carry (`AudioOperation::Start`, `Render`). |

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
derived from graph shape (and `#[default]`), so declaring them would let a source
contradict the transitions it also declares. See
[parser.md](parser.md#documentation-and-annotations) for how a source authors
these.

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
  data is empty (how `states[]` gained documentation). Existing artifacts stay
  valid and unannotated output stays byte-identical, so the change is additive
  in practice — but the clients still move in the same commit, because the
  producer starts emitting objects the moment a source is annotated.
- Breaking shape changes (like the `machines[]` introduction) change every
  layer in the same commit: schema, `crates/model` (+ round-trip test), the
  bundled example, `crates/docgen`, `apps/web/src/schema` + domain + tests.
- The web app treats an invalid generated model as absent (falls back to the
  bundled example with a console warning) so stale artifacts never break it.
