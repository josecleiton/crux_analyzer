# VS Code extension

> 🌐 **English** · [Português (Brasil)](pt-BR/vscode.md)

`apps/vscode` — the state machines beside the code. One command,
**Crux Analyzer: Show State Machines**, opens a panel rendering the analyzed
workspace: machines, states, transitions, authored documentation, tag filter
and simulation — everything the [web UI](web-ui.md) does, because it *is* the
web UI. The panel regenerates on every `.rs` save, which makes this the
authoring loop: write a doc comment, save, watch the diagram learn it.

## How it works

The extension is a client of the same JSON contract as every other client. It
never parses Rust: it spawns the `crux-analyzer` CLI, reads `generate`'s
stdout, and hands the model to the **built web bundle** embedded in the
extension (`media/web`, produced by `just ext-build`).

A webview differs from the static site the bundle was built for in three ways,
each handled by one rewrite in `src/webviewHtml.ts` (pure, unit-tested):

- root-absolute asset URLs are re-rooted onto `asWebviewUri`;
- every script — the bundle's pre-paint blocks included — runs under a
  nonce-locked CSP;
- there is no HTTP origin to fetch `model.json` from, so the model is injected
  as `window.__CRUX_MODEL__` — the embedding contract `loadProject` honors
  before it ever tries to fetch.

Parser warnings are not dropped (honesty rule): they land in the
**Crux Analyzer** output channel on every regeneration.

## Setup

The extension needs the CLI:

```sh
cargo install --path crates/cli   # from a crux_analyzer checkout
```

Build and install the extension itself:

```sh
just ext-package                  # produces apps/vscode/crux-analyzer-vscode-<version>.vsix
code --install-extension apps/vscode/crux-analyzer-vscode-*.vsix
```

## Settings

| Setting | Default | Meaning |
| --- | --- | --- |
| `cruxAnalyzer.binary` | `crux-analyzer` | Path to the CLI (default: resolved on PATH). |
| `cruxAnalyzer.src` | *(empty)* | Sources to analyze, relative to the workspace root. Empty tries `shared/src` (the conventional Crux layout), then `src`. An explicit value wins even if missing — the analyzer's own error beats silently analyzing somewhere else. |
| `cruxAnalyzer.projectName` | *(empty)* | Name shown in the panel; empty uses the workspace folder name. |
| `cruxAnalyzer.watch` | `true` | Regenerate when a `.rs` file under the analyzed directory changes. |

## Localization

Contribution points (command title, setting descriptions) live in
`package.nls.json` / `package.nls.pt-br.json`; runtime messages go through
`vscode.l10n` with `l10n/bundle.l10n.pt-br.json`. The panel content follows
the web UI's own locale toggle, independent of the editor language — it is the
same bundle with the same rules ([i18n.md](i18n.md)).

## Development

| Task | Recipe |
| --- | --- |
| Unit tests (webview HTML, source resolution) | `just ext-test` |
| Compile + embed the web bundle | `just ext-build` |
| Package a `.vsix` | `just ext-package` |

Both test and build are part of `just check`. The extension host pieces
(panel, watcher, activation) are deliberately thin plumbing around the pure
modules — the mapping and rendering decisions all live in the web bundle,
which has its own test layers.
