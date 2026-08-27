# Contributing

Republic Observatory is at its foundation stage. Contributions that improve
the documented save format, metric definitions, statistical safeguards,
accessibility, or the first vertical slice are welcome.

Before changing code, read [AGENTS.md](AGENTS.md), the
[architecture overview](docs/architecture/overview.md), and the relevant
decision records. Do not attach real saves to an issue. Reduce a reported case
to a sanitised fixture containing only the minimum fields needed to reproduce
it.

## Local checks

```powershell
npm install
npm run format:check
npm run check
npm test
npm run build
npm run rust:check
npm run rust:test
cargo fmt --manifest-path src-tauri\Cargo.toml --check
cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings
```

Maintainers may additionally point `RO_LIVE_SAVE` at a local save ZIP and run
the opt-in conformance path by test name. Never put that path in a
fixture, document, log capture, or commit. A failing case must be reduced to a
minimal sanitised text fixture before it is shared.

Pull requests should explain the player question being improved, the provenance
of any new field, the behaviour when that field is unavailable, and the tests
or visual checks performed.

Recorder changes must keep folder events advisory and periodic reconciliation
authoritative. The native service owns liveness; a Svelte timer, open dialog, or
selected workspace must never be required to notice a save. Candidate lifecycle
changes require restart recovery, duplicate identity, temporary-write, rename,
retry, and terminal-failure tests. Do not expose configured full paths through
events, health projections, charts, logs intended for sharing, or extension
contracts.

All new interface text must enter the canonical `en-AU` catalogue. Use the
central locale-formatting helpers and explicit typed translation-key mappings;
do not construct keys dynamically. Language-pack contributions should follow
the [localisation guide](docs/localization/README.md), retain Fluent variables,
and avoid protected evidence and safety namespaces. New Analysis Pack prose
should declare its own `default_locale` and is not host UI text.
