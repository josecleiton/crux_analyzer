# Crux Analyzer for VS Code

The state machines of a Rust + Crux application beside the code: run
**Crux Analyzer: Show State Machines** and the analyzed workspace renders in a
panel — machines, states, transitions, authored documentation, simulation —
regenerating on every save.

This extension is a client of the crux_analyzer JSON contract; it embeds the
project's web UI and spawns the `crux-analyzer` CLI (install it with
`cargo install --path crates/cli`, or point `cruxAnalyzer.binary` at it).

Full documentation: [docs/vscode.md](../../docs/vscode.md) in the repository
(Português: `docs/pt-BR/vscode.md`).
