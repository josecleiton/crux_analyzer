# crux_analyzer — task runner (https://just.systems)
# `just` with no arguments lists the recipes.

# Absolute by default: cargo runs integration tests from the crate directory,
# so a relative path would resolve wrongly there.
quipu_src := env_var_or_default("QUIPU_SRC", justfile_directory() / ".." / "quipu_app_crux" / "shared" / "src")

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

# --- Rust ------------------------------------------------------------------

# All Rust tests (parser unit + fixtures + docgen)
rust-test:
    cargo test --workspace

# Rust tests including the real-app corpus (set QUIPU_SRC to override)
corpus:
    QUIPU_SRC={{quipu_src}} cargo test --workspace

# Clippy across the workspace
clippy:
    cargo clippy --workspace

# --- Everything ------------------------------------------------------------

# Full validation: Rust + corpus + clippy + web tests + web build
check: corpus clippy web-test web-build

# --- Analyzer --------------------------------------------------------------

# Emit the model JSON for a crate: just generate path/to/app/src MyApp
generate src name out="/dev/stdout":
    cargo run -q -p crux-analyzer-cli -- generate --src {{src}} --name {{name}} --out {{out}}

# Feed the web UI with a real model: just model path/to/app/src MyApp
model src name:
    cargo run -q -p crux-analyzer-cli -- generate --src {{src}} --name {{name}} \
      --out apps/web/public/model.json

# Same as `model`, but regenerating on every save (living documentation)
model-watch src name:
    cargo run -q -p crux-analyzer-cli -- generate --src {{src}} --name {{name}} \
      --out apps/web/public/model.json --watch

# Static documentation site in apps/web/dist: analyze, then build the UI with
# the model baked in. `base` is the path the site will be served from —
# default root, e.g. `just site ../app/src MyApp /crux-docs/` for Pages.
site src name base="/":
    just model {{src}} {{name}}
    CRUX_BASE={{base}} pnpm --filter web build
    @echo "Static site ready in apps/web/dist (base {{base}}) — serve it over HTTP, not file://"

# Generate docs: just docs path/to/app/src MyApp [markdown|mermaid] [en|pt-BR]
docs src name format="markdown" locale="en":
    cargo run -q -p crux-analyzer-cli -- docs --src {{src}} --name {{name}} \
      --format {{format}} --locale {{locale}}

# Analyze the Quipu corpus into the web UI (QUIPU_SRC to override the path)
quipu:
    just model {{quipu_src}} Quipu

# Regenerate the committed example docs, every locale
example-docs: (example-docs-locale "en" "docs/examples/mini-recorder.md") (example-docs-locale "pt-BR" "docs/pt-BR/examples/mini-recorder.md")

# One locale of the committed example docs (see `example-docs`)
[private]
example-docs-locale locale out:
    cargo run -q -p crux-analyzer-cli -- docs \
      --src crates/parser/fixtures/mini_recorder --name "Mini Recorder" \
      --locale {{locale}} --out {{out}}
