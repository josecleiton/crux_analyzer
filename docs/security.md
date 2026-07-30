# Security

> 🌐 **English** · [Português (Brasil)](pt-BR/security.md)

crux_analyzer sits in an unusually exposed spot for a developer tool. It **reads
Rust source it did not write**, **emits documents that get published**, and
**renders prose from that source in a browser and in a VS Code webview**. None of
those is a normal "input" — each is a trust boundary, and this document says
where they are, what the rules are, and what is deliberately guaranteed.

The rules here are development guidelines, not aspirations: most of them have a
test that fails when they are broken, and `just check` runs the supply-chain gate.

## Threat model

**Untrusted inputs** — anything below may be hostile, malformed, or simply
pathological:

| Input | Why it is untrusted | Reaches |
| --- | --- | --- |
| The analyzed source tree | A dependency, a fork's pull request, a crate someone downloaded. Its shape controls the AST. | `crates/parser` |
| Paths inside that tree | Filenames, symlinks, file sizes | `crates/parser/src/loader.rs` |
| Doc-comment prose | Free text written by whoever wrote the code | docgen output, the web UI, the webview |
| Identifiers (state/event/effect/machine/core names) | Any legal Rust identifier, including raw and non-ASCII ones | docgen output, the web UI |
| `model.json` | May be stale, hand-edited, or from another version | `apps/web/src/schema/` |
| A workspace's `.vscode/settings.json` | Ships inside a cloned repository | `apps/vscode` |

**Trusted inputs**: the CLI's own flags and environment (`--src`, `--out`,
`--max-*`, `CRUX_ANALYZER_LOCALE`). Someone who can set those can already run
arbitrary commands.

**Not in the threat model**: the analyzed application's *runtime* behaviour.
crux_analyzer never executes the code it reads — it only parses it — and it does
not depend on Crux itself.

## The rules

### 1. Author prose is untrusted text, everywhere it lands

Doc comments are free text on their way to a browser and to a published
document. They may never become markup.

- **In the web UI**: prose reaches the DOM only as React children, or through
  `react-markdown` with raw HTML disabled. Never `dangerouslySetInnerHTML`,
  never `rehype-raw`, never `skipHtml` gymnastics, and never a `urlTransform`
  that widens the protocol allowlist beyond `http`, `https` and `mailto`.
  Links carry `rel="noopener noreferrer nofollow"`; images are **not fetched at
  all** — an `![](https://host/x.png)` in a doc comment is a read beacon that
  would report every viewer of a published document, so the alt text stands in
  for it. Pinned by
  [`StateDoc.test.tsx`](../apps/web/src/components/Inspector/StateDoc.test.tsx).
- **In generated Markdown**: `&`, `<` and `>` are escaped in prose, so raw HTML
  cannot become an element. Author *Markdown* is deliberately preserved —
  `**bold**`, lists and backticks are a feature — so this is not an escape of
  Markdown syntax, only of the ability to leave it. Fence-shaped lines in prose
  are neutralized, and the fence around a diagram is computed to be longer than
  any backtick run inside it.
- **In generated Mermaid**: labels and notes go through `mermaid_label`, which
  flattens to one line (a statement is line-terminated), drops control
  characters, and replaces `"`, `<`, `>` and `%%` with entity codes. A
  transition label is `event / effect, effect`, so **effect names** are on that
  path too — the whole composed label is escaped, not its parts.
- **In table cells**: the backslash is escaped *before* the pipe, or prose
  containing `\|` re-opens a column. Backticks are *not* escaped — a row is
  split on its unescaped pipes before its cells are parsed as inline content,
  so a backtick cannot spill into the next column, and author code spans are a
  feature here exactly as they are in a prose block.

Pinned by [`hostile_output.rs`](../crates/docgen/tests/hostile_output.rs).

### 2. Identifiers are data, and data does not become structure

An identifier from the analyzed app may never influence a filesystem path, and
may never be emitted where its characters could be read as syntax.

- The only write in the Rust workspace is the user's `--out`. docgen returns
  strings and never touches the filesystem; there are no per-machine output
  files. **Keep it that way** — a core or state name in a path is a traversal.
- Mermaid node ids are generated, collision-checked and keyword-checked
  (`Ids::build`), with the real name carried in a quoted label. A state named
  `end`, a raw identifier (`r#type`) or a composite leaf colliding with a
  sibling would otherwise break or silently merge nodes.

### 3. Every unbounded input dimension gets a cap, and every cap that fires is reported

This is the **parser honesty rule applied to resources**: a truncated analysis
says so, as a `Warning`, so `--deny-warnings` makes truncation fail a pipeline
instead of publishing a quietly partial diagram.

The caps live in [`crates/parser/src/limits.rs`](../crates/parser/src/limits.rs)
and are overridable with `--max-file-size`, `--max-total-size` and `--max-steps`:

| Dimension | Why it is unbounded without a cap | Warning |
| --- | --- | --- |
| File size, total size | Every file's AST is held for the whole run, and an AST is much larger than its source | `file-too-large`, `input-too-large` |
| Bracket nesting | `syn::parse_file` recurses over nesting; a stack overflow **aborts the process** and cannot be caught, so this one is checked on the raw text *before* parsing | `nesting-too-deep` |
| Walk steps | The call-following walker re-walks a helper per distinct path, so a diamond call graph is exponential — forty small functions describe 2⁴⁰ walks | `analysis-truncated` |
| Expression / pattern / call depth | The walkers recurse over the input's own nesting | `analysis-truncated` |
| Callback expression depth | The scan that reads which events answer a request follows closures, blocks and match arms, all of which nest without limit | past the cap the callback simply reads as unresolved (`unresolved-effect-callback` when a `then_send` names nothing readable) |

Memoizing the walker is *not* the alternative: a helper is legitimately
re-walked under a different context and yields different transitions each time.
The total work is what gets bounded — and unlike memoization, a budget is
reportable.

Pinned by [`hostile_input.rs`](../crates/parser/tests/hostile_input.rs), which
asserts termination rather than extraction quality.

### 4. Only regular files are read

`walkdir` does not descend symlinked *directories*, but a symlinked file would
be followed — reading source from outside the tree, hanging forever on a FIFO,
or exhausting memory on `/dev/zero`. One `file_type().is_file()` check closes all
three. Skipped paths are reported (`not-a-regular-file`), never dropped silently.

### 5. No shell, ever

Subprocesses take an argv array. The Rust workspace spawns nothing at all — no
`Command::new`, no shell. The VS Code extension uses `execFile` with an argv
array. A string passed to a shell is a command injection waiting for the first
path with a space in it.

### 6. Settings that choose an executable are machine-scoped

`cruxAnalyzer.binary` is `"scope": "machine"`, so a cloned repository's
`.vscode/settings.json` cannot decide which executable runs — the
ESLint-`nodePath` class of issue. `cruxAnalyzer.src` is workspace-scoped (it is
genuinely per-project) and is therefore *contained*: a value that climbs out of
the workspace root with `..` is refused, because the watcher follows it too. The
extension declares `untrustedWorkspaces.supported: false`.

### 7. Diagnostics are sanitized before they reach a terminal

A doc comment or a path interpolated into a warning is attacker-controlled text
being written to someone's terminal. `WarningKind::message` and
`ParseError::message` strip control characters from the *whole rendered string*,
so a variant added later cannot forget to.

### 8. The webview stays locked down

`default-src 'none'`, no `unsafe-inline` or `unsafe-eval` for scripts,
`localResourceRoots` limited to the bundle directory.

`script-src` is the per-render nonce **plus the webview's own resource origin**.
Both halves are load-bearing: the nonce authorizes the inline scripts (pre-paint
blocks, model injection), and the origin is required because the bundle is
code-split and **a nonce does not extend to modules a nonced script imports**.
With the nonce alone, every split chunk is blocked and the webview renders an
empty page — verified in a browser, not inferred. The origin is not a widening:
`localResourceRoots` confines it to the bundle directory, and arbitrary inline
script still needs the nonce.
The webview policy **replaces** the static-site policy the build bakes into
`index.html` ([§8.1](#81-the-static-site-carries-its-own-policy)) rather than
joining it: CSP composes by intersection, and that policy's `'self'` is the
`vscode-webview://` document origin, not the `vscode-resource` host serving the
bundle. Both present, the entry module and the model injection are blocked while
the hash-allowed pre-paint scripts still run — a styled, empty page. So
`buildWebviewHtml` strips any `Content-Security-Policy` meta tag before adding
its own, and the stripping is pinned by a test whose fixture carries the baked
tag.
The injected model escapes `<` and U+2028/U+2029 so author prose can neither
close the script tag nor break the statement. **There is no webview↔host message
channel** — the model flows one way, by injection. Do not add one without
validating every message.

#### 8.1 The static site carries its own policy

The `just site` build and the GitHub Pages preview have no host to send a CSP
header, so [`apps/web/csp.ts`](../apps/web/csp.ts) injects the meta tag at build
time. `script-src` is `'self'` plus a **computed** hash per inline pre-paint
script — computed from the file being written, because a hand-maintained hash
goes stale on the first edit and a stale hash breaks the page silently.

### 9. Dependencies are reviewed, and actions are pinned

- New dependencies must pass `just security` (`cargo deny check` against
  [`deny.toml`](../deny.toml) + `pnpm audit --audit-level high`). It is part of
  `just check`, so it is blocking.
- Both lockfiles are committed and `cargo` runs `--locked`.
- **No dependency runs code at install time.** pnpm blocks dependency lifecycle
  scripts unless the package is allowed in `allowBuilds`
  ([`pnpm-workspace.yaml`](../pnpm-workspace.yaml)), and that map is **empty** —
  written out rather than left to the default, so allowing one is a reviewable
  diff instead of a local `pnpm approve-builds` nobody sees. Nothing in the tree
  needs building today. A blocked script is reported
  (`ERR_PNPM_IGNORED_BUILDS`), never silent, which is the honesty rule applied to
  install-time code.
- **The package manager is pinned by hash.** `packageManager` carries pnpm's
  version *and* its sha512, so corepack refuses a substituted tarball; a
  compromised release of the tool that installs everything else would otherwise
  be the shortest path into every build.
- GitHub Actions are pinned to **commit SHAs**, not tags — `@stable` and `@v2`
  are mutable refs. Dependabot keeps the pins from rotting.
- Workflows declare `permissions:` explicitly, and untrusted `${{ }}` values
  reach a `run:` block through `env:`, never by string interpolation.

### 10. Every artifact carries its third-party notices

Not a security property, but the same class of obligation and the same failure
mode: something true of the repository that was not true of what shipped.

Permissive is not obligation-free. MIT requires its notice "in all copies or
substantial portions", BSD-3-Clause clause 2 requires binary redistributions to
reproduce it in accompanying materials, and elkjs's EPL-2.0 §3.1(a) requires a
statement of where its source can be obtained. The built bundle used to contain
**zero** copyright notices — the minifier dropped even the `@license` header
React ships — while being published to GitHub Pages on every push and embedded in
every VSIX.

The rules:

- `THIRD-PARTY-NOTICES.md` is **generated from what each artifact ships**, never
  hand-maintained and never derived from the installed tree: the web half from
  the chunks the bundler emitted, the Rust half from the crates linked into the
  binary. `just notices-current` (inside `just check`) makes a missing notice a
  red build.
- **Legal comments stay in the bundle** (`comments: { legal: true }`). Stripping a
  copyright header out of code you redistribute is the plainest version of this
  problem.
- **A package with no determinable license fails the build.** A notices file that
  quietly omits one is worse than none, because it looks complete.
- **elkjs is used under the EPL-2.0**, elected from its `EPL-2.0 OR
  GPL-3.0-or-later` offer, unmodified, and emitted as its own chunk so no output
  file mixes it with this project's code. Its notice states the version, the
  election and where to get the source.
- `about.toml`'s accepted licenses and `deny.toml`'s allow-list must agree — one
  decides what may enter the tree, the other what gets reproduced, and a
  divergence means one of them is wrong.

## What is deliberately guaranteed

These properties are load-bearing. They are cheap to keep and expensive to
recover, so treat removing one as a design change, not a refactor:

- **No `unsafe` anywhere** in the Rust workspace.
- **No subprocess or shell** in the Rust workspace.
- **No attacker-influenced output path**: one `fs::write`, from `--out`.
- **No reachable `unwrap`/`expect`/`panic!`/slice index** on parsed input. The
  remaining `expect`s are provably unreachable and say so.
- **No HTML-injection sink** for model data in the web app: no
  `dangerouslySetInnerHTML`, no `innerHTML`, no `href`/`src` built from model
  data.
- **No `eval`, `new Function`, or dynamic import** of model-derived strings.
- **No webview↔host message channel.**
- **`localStorage` and URL-hash input are allowlist-validated** before use, in
  the pre-paint inline scripts as well as in the app.

## Reporting a vulnerability

Open a [security advisory](https://github.com/josecleiton/crux_analyzer/security/advisories/new)
rather than a public issue, and please include the input that triggers it — a
minimal `.rs` file or `model.json` is worth more than a description. There is no
bounty; there is a changelog entry and thanks.

## See also

- [Parser](parser.md) — the warnings reference, including the resource warnings
- [CLI](cli.md) — the `--max-*` flags and `--deny-warnings`
- [Development](development.md) — `just check` and the validation pipeline
- [Architecture](architecture.md) — why the layers that make these rules
  enforceable exist
