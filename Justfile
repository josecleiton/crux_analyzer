# crux_analyzer — task runner (https://just.systems)
# `just` with no arguments lists the recipes.

default:
    @just --list

# --- Web -------------------------------------------------------------------

# Start the web UI (Vite dev server)
dev:
    pnpm --filter web dev

# Web unit tests (mapping layers + simulation engine)
web-test:
    pnpm --filter web test

# Type-check + production build of the web UI
web-build:
    pnpm --filter web build

# --- VS Code extension -------------------------------------------------------

# Unit tests of the extension's pure modules (webview HTML, source resolution)
ext-test:
    pnpm --filter crux-analyzer-vscode test

# Compile the extension and embed the built web bundle as its webview UI.
# The copied dist must not carry a baked model.json: the webview injects the
# freshly analyzed model instead, and a stale artifact would shadow nothing
# but confuse a reader of the package.
ext-build: web-build
    pnpm --filter crux-analyzer-vscode build
    rm -rf apps/vscode/media/web
    mkdir -p apps/vscode/media
    cp -R apps/web/dist apps/vscode/media/web
    rm -f apps/vscode/media/web/model.json
    # Every VSIX must carry its own license text and the notices of the code it
    # embeds. Copied rather than committed, so the repository root stays the one
    # source of truth (both are gitignored).
    cp LICENSE apps/vscode/LICENSE
    cp apps/web/dist/THIRD-PARTY-NOTICES.md apps/vscode/THIRD-PARTY-NOTICES.md

# Package the extension into a .vsix (installable via code --install-extension)
ext-package: ext-build
    cd apps/vscode && pnpm dlx @vscode/vsce package --no-dependencies

# --- Rust ------------------------------------------------------------------

# All Rust tests (parser unit + fixtures + docgen).
#
# `--locked` so a test run can never quietly rewrite Cargo.lock: the committed
# lockfile is what was reviewed, and it is what must be built.
rust-test:
    cargo test --workspace --locked

# Clippy across the workspace
clippy:
    cargo clippy --workspace

# --- Security --------------------------------------------------------------

# Supply-chain gate: advisories, license policy, banned and git dependencies
# (policy in `deny.toml`), plus the npm side. Blocking, as part of `check` —
# the reasoning is in `docs/security.md`.
#
# cargo-deny is installed on demand rather than being a documented prerequisite:
# a gate people skip because it needs setup is not a gate.
security:
    @command -v cargo-deny >/dev/null || cargo install --locked cargo-deny
    cargo deny check
    pnpm audit --audit-level high

# --- Licenses --------------------------------------------------------------

# Regenerate THIRD-PARTY-NOTICES.md: the union of what the two artifacts ship.
#
# Two generators because the two artifacts are different: the web half comes
# from the chunks the bundler actually emitted (`apps/web/notices.ts`), the Rust
# half from the crates linked into the binary (`about.toml` + `about.hbs`).
# Neither is derived from what merely happens to be installed.
notices: web-build
    @command -v cargo-about >/dev/null || cargo install --locked cargo-about --features cli
    @cp apps/web/dist/THIRD-PARTY-NOTICES.md THIRD-PARTY-NOTICES.md
    @printf '\n---\n\n' >> THIRD-PARTY-NOTICES.md
    @cargo about generate --target x86_64-unknown-linux-gnu about.hbs >> THIRD-PARTY-NOTICES.md
    @echo "THIRD-PARTY-NOTICES.md regenerated"

# The committed notices must be what the generators produce right now — the same
# ratchet shape as `docs-current`. A new dependency either appears here or turns
# the build red; there is no third outcome, which is how this debt stays paid.
notices-current: notices
    @git diff --exit-code -- THIRD-PARTY-NOTICES.md \
      || { echo "THIRD-PARTY-NOTICES.md is stale — commit the regenerated file"; exit 1; }

# --- Everything ------------------------------------------------------------

# Full validation: Rust tests + clippy + web tests + extension tests + builds
# (ext-build includes web-build) + fixture guard + the supply-chain and license
# gates
check: rust-test clippy security notices-current web-test ext-test ext-build fixture-guard

# The fixture is the public stand-in for a real app: it must extract with zero
# warnings, and the documentation it declares must not regress. The floor sits
# below today's 67% on purpose — `UploadState`'s variants are deliberately
# bare, so the point is to catch a drop, not to chase 100%.
fixture-guard:
    cargo run -q -p crux-analyzer-cli -- generate \
      --src crates/parser/fixtures/mini_recorder --name "Mini Recorder" \
      --out /dev/null --deny-warnings
    cargo run -q -p crux-analyzer-cli -- coverage \
      --src crates/parser/fixtures/mini_recorder --name "Mini Recorder" --min 60

# The committed example docs must be what the generator produces right now.
docs-current: example-docs
    @git diff --exit-code -- docs/examples docs/pt-BR/examples \
      || { echo "docs/examples is stale — commit the regenerated files"; exit 1; }

# --- Analyzer --------------------------------------------------------------

# Emit the model JSON for a crate: just generate path/to/app/src MyApp
generate src name out="/dev/stdout":
    cargo run -q -p crux-analyzer-cli -- generate --src {{src}} --name "{{name}}" --out {{out}}

# Feed the web UI with a real model: just model path/to/app/src MyApp
model src name:
    cargo run -q -p crux-analyzer-cli -- generate --src {{src}} --name "{{name}}" \
      --out apps/web/public/model.json

# Same as `model`, but regenerating on every save (living documentation)
model-watch src name:
    cargo run -q -p crux-analyzer-cli -- generate --src {{src}} --name "{{name}}" \
      --out apps/web/public/model.json --watch

# Documentation coverage of a crate: just coverage path/to/app/src MyApp [min]
coverage src name min="0":
    cargo run -q -p crux-analyzer-cli -- coverage --src {{src}} --name "{{name}}" \
      --min {{min}} --list

# Static documentation site: analyze and export the web UI bundle.
site src name out="apps/web/dist": web-build
    cargo run -q -p crux-analyzer-cli -- site --src {{src}} --name "{{name}}" --out {{out}}
    @echo "Static site ready in {{out}} — serve it over HTTP, not file://"

# Generate docs: just docs path/to/app/src MyApp [markdown|mermaid] [en|pt-BR]
docs src name format="markdown" locale="en":
    cargo run -q -p crux-analyzer-cli -- docs --src {{src}} --name "{{name}}" \
      --format {{format}} --locale {{locale}}

# Regenerate the committed example docs, every locale
example-docs: (example-docs-locale "en" "docs/examples/mini-recorder.md") (example-docs-locale "pt-BR" "docs/pt-BR/examples/mini-recorder.md")

# One locale of the committed example docs (see `example-docs`)
[private]
example-docs-locale locale out:
    cargo run -q -p crux-analyzer-cli -- docs \
      --src crates/parser/fixtures/mini_recorder --name "Mini Recorder" \
      --locale {{locale}} --out {{out}}
