# CLI — `crux-analyzer`

> 🌐 **English** · [Português (Brasil)](pt-BR/cli.md)

Built by `crates/cli`. Run it from the workspace with
`cargo run -p crux-analyzer-cli --` (or install the binary with
`cargo install --path crates/cli`).

## `generate` — emit the model JSON

```sh
crux-analyzer generate --src <dir> [--name <project>] [--out <file>] [--watch] [--locale <locale>]
```

| Flag | Meaning |
| --- | --- |
| `--src` | Directory with the Rust sources to analyze (e.g. `path/to/app/shared/src`). |
| `--name` | Project name in the model. Defaults to the `--src` directory name. |
| `--out` | Output file. Defaults to stdout. |
| `--watch` | Keep watching `--src` and regenerate on every `.rs` change (debounced). |
| `--locale` | `en` or `pt-BR`. Language of the CLI's own output and of generated prose — see below. |
| `--deny-warnings` | Exit non-zero if the parser reported anything. Global: works on every subcommand. |
| `--max-file-size` | Skip `.rs` files larger than this many bytes (default 2 MiB). Global. |
| `--max-total-size` | Stop reading once this many bytes of source have been loaded (default 256 MiB). Global. |
| `--max-steps` | Expression-walking steps allowed per Core (default 2,000,000). Global. |

Warnings (see the [warnings reference](parser.md#warnings-reference)) go to
stderr; the JSON goes to `--out`/stdout. Exit code is non-zero when parsing
fails (e.g. no `impl App` found).

### Resource limits

The `--max-*` flags exist because the analyzer is routinely pointed at source
nobody on your team wrote — a dependency, a fork's pull request, a crate someone
downloaded. The defaults are far above any real Crux application (the test
fixture uses a four-figure step count) and far below what it takes to hang a
machine, so **you should never need to change them**; raise one only for a large
codebase you trust.

Every limit obeys the honesty rule: hitting one emits a warning
([resource warnings](parser.md#resource-warnings)), so `--deny-warnings` makes a
truncated analysis fail the run instead of publishing a quietly partial diagram.
`docs/security.md` explains what each limit bounds and why memoization is not the
alternative.

`--deny-warnings` still writes the output — the exit code is the signal, so a
pipeline fails while a human still gets the artifact to look at. Under
`--watch` it reports without ending the session.

The emitted **model JSON is locale-independent** — everything in it is read out
of the analyzed source, identifiers and the author's own doc-comment prose
alike, so `generate` produces byte-identical output in every locale.
`--locale` only affects the messages on stderr here.

Feed the web UI:

```sh
crux-analyzer generate --src path/to/app/src --name MyApp \
  --out apps/web/public/model.json --watch
```

## `site` — emit static web documentation site

```sh
crux-analyzer site --src <dir> [--name <project>] [--out <dir>] [--watch] [--locale <locale>]
```

| Flag | Meaning |
| --- | --- |
| `--src` | Directory with the Rust sources to analyze. |
| `--name` | Project name in the model. Defaults to the `--src` directory name. |
| `--out` | Output directory. Defaults to `dist`. |
| `--watch` | Keep watching `--src` and regenerate `model.json` on every `.rs` change. |

Exports the interactive web UI bundle and embeds the analyzed `model.json` into `--out`.

```sh
crux-analyzer site --src path/to/app/src --name MyApp --out ./public-docs
```

## `docs` — emit documentation

```sh
crux-analyzer docs --src <dir> [--name <project>] [--format markdown|mermaid|html|site] [--out <file/dir>] [--watch] [--locale <locale>]
```

Here `--locale` also translates the generated document's own prose (section
labels, table headers, marker names, the `any state` pseudo-state). State,
event and effect names stay untouched, **and so does documentation read out of
the analyzed source** — the language of a doc comment is its author's choice.
The Mermaid node id `any_state` stays stable because transition lines refer to
it.

### Markdown (default)

One document: per machine, its description, a ` ```mermaid ` block, a states
table (State / Role / Description / Markers / Tags) and a transition table
(From / Event / To / Effects). GitHub, GitLab and
most Markdown viewers render the embedded diagrams natively — commit the file
and the docs are readable in the repo:

```sh
crux-analyzer docs --src path/to/app/src --name MyApp --out STATE_MACHINES.md
```

The `Role` column carries the two *derived* roles — `initial` for the machine's
entry point, `final` for a state nothing leaves — kept separate from `Markers`,
which is what the author declared. The diagram states the same two as Mermaid's
`[*]` pseudo-state, so it says where a machine starts and ends even for a machine
with no documentation at all. How the entry point is decided is in
[schema.md](schema.md#where-a-machine-starts).

The description and states table appear only when the analyzed source
documents something — see [annotations](parser.md#documentation-and-annotations)
for how to write them. Two consequences of copying author prose verbatim: a
description is flattened into one line in its table cell (a state whose
description runs to several paragraphs gets it back in full, under its own
heading below the table), and Markdown the author wrote is left as Markdown, so
a doc comment starting with `#` renders as a heading.

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
    [*] --> Idle
    Completed --> [*]

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

## `coverage` — how much is documented

```sh
crux-analyzer coverage --src <dir> [--name <project>] [--min <percent>] [--list] [--locale <locale>]
```

| Flag | Meaning |
| --- | --- |
| `--min` | Exit non-zero when the share of described states is below this percentage. |
| `--list` | Also name the states that have no description. |

```
$ crux-analyzer coverage --src crates/parser/fixtures/mini_recorder --name "Mini Recorder"
MiniRecorder / RecorderState                 100%  6 of 6 states described
MiniRecorder / UploadState                     0%  0 of 3 states described
total                                         67%  6 of 9 states described
```

**"Documented" means the state has a description.** A state carrying only a
marker or a tag is classified, not explained, so it does not count — the point
of the measure is prose a reader can learn something from. A machine whose state
enum has no description of its own gets a note under its line.

`--min` compares **exactly**, not against the displayed percentage: 2 of 3
states shows as 67% and does *not* satisfy `--min 67`. A machine with no states
counts as complete, so an empty project never fails.

This is the ratchet: put it in CI with a `--min` at today's number, and the
documentation can go up but not down. `just coverage <src> <name> [min]` wraps
it.

### Both gates, or neither is a ratchet

`docs --deny-warnings` and `coverage --min` fail on different things, and a
project that wires only the first gets no ratchet at all: the first adoption of
this tool ran `docs --deny-warnings` in CI and sat at 79% coverage with its
biggest machine at 0 of 7 states described, failing nothing. Warnings catch what
the parser could not read; coverage catches what nobody wrote. Wire both:

```sh
crux-analyzer docs --src shared/src --name MyApp --deny-warnings --out docs/machines.md
crux-analyzer coverage --src shared/src --name MyApp --min 79 --list
```

Raise the `--min` in the same commit that documents a state — that is the whole
mechanism, and it only works if the number is in the repository rather than in
someone's memory.

## Choosing the locale

Precedence, highest first:

1. `--locale en|pt-BR`;
2. the `CRUX_ANALYZER_LOCALE` environment variable;
3. the POSIX chain `LC_ALL` → `LC_MESSAGES` → `LANG` (so `LANG=pt_BR.UTF-8`
   is enough);
4. `en`.

Unrecognized *environment* values are ignored and the chain continues; an
unrecognized `--locale` is an error, because silently ignoring an explicit
request would be worse. Note that `--help` itself is English-only — see the
[gap documented in i18n.md](i18n.md#known-gap---help-is-english-only).

## Example (run against the test fixture)

```sh
cargo run -p crux-analyzer-cli -- docs \
  --src crates/parser/fixtures/mini_recorder --name "Mini Recorder"
```

The committed output of exactly that command lives at
[docs/examples/mini-recorder.md](examples/mini-recorder.md), with its
Portuguese twin at
[docs/pt-BR/examples/mini-recorder.md](pt-BR/examples/mini-recorder.md).
Both are regenerated by `just example-docs`.
