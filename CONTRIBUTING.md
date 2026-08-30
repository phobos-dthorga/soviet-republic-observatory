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
npm run rust:clippy
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

SQLite owns operational truth; DuckDB owns catalogue generations and derived
analytical projections. Never connect the engines through DuckDB extensions or
expose either connection, SQL, table name, database path, complete definition,
or raw save through a command or extension contract. Transfers use bounded,
versioned Rust models and the idempotent SQLite projection outbox. A warehouse
failure must not block save observation or SQLite-backed views.

Every variable-cardinality DuckDB write must use the governed bulk boundary:
declare its workload class and bounded row total, stage through an appender,
merge as a set, report progress, and add a realistic maximum-size regression
test. Direct statements are reserved for fixed-cardinality metadata, receipts,
watermarks, publication pointers, and transactional deletion. Do not execute a
query or insert inside a fact, entity, metric, or overlay loop. Do not raise a
governor limit to accommodate an unpartitioned workload; design immutable,
resumable partitions and an atomic publication step instead. See
[ADR-0013](docs/architecture/decisions/0013-governed-duckdb-write-boundary.md).

Planning overlays and Analysis Packs are inert data. They cannot contain code,
expressions, markup, URLs, paths, renderer configuration, or direct ECharts
options. Future executable plugins remain out of process with deny-by-default
capabilities; process separation must never be described as an operating-system
sandbox.

Compatibility profiles are a separate inert contract. They may map only
version-sensitive source aliases, allowlisted definition operations, and fixed
bounded binary reads onto host-owned slots. Do not add a new stable fact,
calculation, scope, unit, pointer graph, decompressor, path, URL, or executable
behaviour through a profile. A reviewed profile change requires a sanitised
minimum fixture and byte-for-byte equivalent normalised output for every
unchanged fixture. See the [authoring and recovery guide](docs/compatibility-profiles.md).
Mod mappings must reuse an exact source-qualified Workshop/WIP catalogue
identity, declare an acknowledged supported-definition hash and explicit update
policy, and map only to an existing allowlisted host operation. Never use them
as load-order rules, mod configuration, save-level active-mod evidence, or
planning-value overrides.

The bundled DuckDB build produces long intermediate C++ paths. The npm Rust and
Tauri scripts select a short operating-system temporary target directory so the
Windows compiler does not exceed legacy path limits. Do not replace that helper
with a personal absolute path.
