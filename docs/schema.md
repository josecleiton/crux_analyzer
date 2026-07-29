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
          "states": ["Idle", "Recording", "Paused", "Uploading", "Completed"],
          "transitions": [
            {
              "from": "Idle",
              "event": "RecordPressed",
              "to": "Recording",
              "effects": ["AudioOperation::Start"]
            }
          ]
        }
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
| `states[]` | Leaf state names, in declaration order. Children of **composite states** are `Parent/Child` paths (`Active/Loading`). A client that ignores the convention still renders a valid flat machine. |
| `transitions[].from` | Source state, or `"*"` — the transition fires from **any** state (statically unguarded). |
| `transitions[].event` | Leaf event variant name that triggers the transition. |
| `transitions[].to` | Target state, or `"*"` — the target is decided at **runtime** (e.g. carried by the event payload). |
| `transitions[].effects[]` | Optional. Effects requested when the transition fires: `"Render"`, `"AudioOperation::Start"`, ... Omitted when empty. |

## The contract is locale-independent

Every string in the model is an identifier read out of the analyzed
application — no translated text ever enters it. `crux-analyzer generate`
therefore produces byte-identical JSON in every locale, and clients localize
their own chrome (the prose standing in for `"*"`, table headers, panel
titles). Adding a locale must never add a field here. See
[i18n.md](i18n.md).

## Wildcards

`"*"` is a reserved state name on both ends of a transition:

- `from: "*"` — fires from any state. UIs render a pseudo-node ("any state");
  simulation offers these transitions from every state.
- `to: "*"` — lands wherever the runtime value says. Simulation excludes
  these from replay (there is nothing static to land on).

## Evolution guidelines

- Additive fields (like `effects`) are optional with empty defaults, so old
  clients keep working.
- Breaking shape changes (like the `machines[]` introduction) change every
  layer in the same commit: schema, `crates/model` (+ round-trip test), the
  bundled example, `crates/docgen`, `apps/web/src/schema` + domain + tests.
- The web app treats an invalid generated model as absent (falls back to the
  bundled example with a console warning) so stale artifacts never break it.
