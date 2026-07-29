# CLI — `crux-analyzer`

Built by `crates/cli`. Run it from the workspace with
`cargo run -p crux-analyzer-cli --` (or install the binary with
`cargo install --path crates/cli`).

## `generate` — emit the model JSON

```sh
crux-analyzer generate --src <dir> [--name <project>] [--out <file>] [--watch]
```

| Flag | Meaning |
| --- | --- |
| `--src` | Directory with the Rust sources to analyze (e.g. `path/to/app/shared/src`). |
| `--name` | Project name in the model. Defaults to the `--src` directory name. |
| `--out` | Output file. Defaults to stdout. |
| `--watch` | Keep watching `--src` and regenerate on every `.rs` change (debounced). |

Warnings (see the [warnings reference](parser.md#warnings-reference)) go to
stderr; the JSON goes to `--out`/stdout. Exit code is non-zero when parsing
fails (e.g. no `impl App` found).

Feed the web UI:

```sh
crux-analyzer generate --src path/to/app/src --name MyApp \
  --out apps/web/public/model.json --watch
```

## `docs` — emit documentation

```sh
crux-analyzer docs --src <dir> [--name <project>] [--format markdown|mermaid] [--out <file>] [--watch]
```

### Markdown (default)

One document: per machine, a ` ```mermaid ` block plus a transition table
(From / Event / To / Effects). GitHub, GitLab and most Markdown viewers
render the embedded diagrams natively — commit the file and the docs are
readable in the repo:

```sh
crux-analyzer docs --src path/to/app/src --name MyApp --out STATE_MACHINES.md
```

### Mermaid (raw)

Raw `stateDiagram-v2` sources, one diagram per machine, separated by
`%% Core / Machine` comment headers:

```sh
crux-analyzer docs --src path/to/app/src --format mermaid --out machines.mmd
```

```
%% Recorder / RecorderState
stateDiagram-v2
    Idle --> Recording: RecordPressed
    ...

%% Recorder / InputState
stateDiagram-v2
    state "any state" as any_state
    ...
```

To view: paste a single diagram into [mermaid.live](https://mermaid.live),
or split the file on the `%%` headers for embedding. Composite states render
as nested blocks (`state Active { ... }`); wildcard sources/targets render as
an `any state` pseudo-state.

### Living documentation

Both commands accept `--watch`: combined with a committed `--out` file or the
web UI's `model.json`, the documentation regenerates on every save.

## Example (run against the test fixture)

```sh
cargo run -p crux-analyzer-cli -- docs \
  --src crates/parser/fixtures/mini_recorder --name "Mini Recorder"
```

The committed output of exactly that command lives at
[docs/examples/mini-recorder.md](examples/mini-recorder.md).
