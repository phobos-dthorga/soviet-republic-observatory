# Contributing

Republic Observatory is at its foundation stage. Contributions that improve
the documented save format, metric definitions, statistical safeguards,
accessibility, or the first vertical slice are welcome.

Before changing code, read [AGENTS.md](AGENTS.md), the
[architecture overview](docs/architecture/overview.md), and the relevant
decision records. Do not attach real saves to an issue. Reduce a reported case
to a sanitised fixture containing only the minimum fields needed to reproduce
it.

Before adding a workspace-local helper, ask whether the behaviour has become a
host-wide contract. Two concrete consumers, cross-workspace state, or a shared
accessibility, security, provenance, progress, or failure-isolation rule are
signals to extract the smallest reusable service. Do not generalise solely for
hypothetical use.

Rust/domain services own parsing, lineage, persistence, validation, evidence,
analytics, recommendations, and lifecycle decisions. `desktopClient.ts` is the
only Tauri boundary. TypeScript presentation adapters may map bounded host
models into view and chart models but cannot invoke services or decide domain
meaning. Svelte owns rendering, localisation, accessibility, navigation,
ephemeral interaction state, and service invocation. CSS consumes semantic
state and validated theme roles; it never establishes business meaning. The
automated architecture audit enforces these import and mutation seams. See
[ADR-0016](docs/architecture/decisions/0016-domain-presentation-boundary.md).

Use the shared notification centre for transient outcomes, the critical-task
components for long-running progress, and inline messages for validation tied
to a particular field or operation. Contextual explanations use the shared
help primitive and stable tutorial topic IDs; required instructions never live
only in a tooltip. New ordinary captions and controls must use the readable
type tokens rather than introduce microtext below the 12-pixel-equivalent
default floor.

Use the first-class AttentionCue for important new or newly available actions;
do not reproduce pulse or glow CSS in a workspace. A cue needs a stable ID and
content revision, must respect reduced motion, and can explain an already-valid
action but cannot establish domain availability or safety.

Debuggers, profilers, browser developer tools, database inspection tools, and
trace captures may be used whenever they are the preferable diagnostic method.
Keep captures local, bounded, and free of save contents, personal paths, or
other private data.

## Local checks

```powershell
npm install
npx playwright install chromium
npm run format:check
npm run check
npm test
npm run build
npm run rust:check
npm run rust:test
cargo fmt --manifest-path src-tauri\Cargo.toml --check
npm run rust:clippy
```

`npm run build` includes the production interface audit: contrast and Axe
checks, deterministic component-state screenshots, and geometry checks across
narrow, laptop, FHD, QHD, ultrawide, and UHD-equivalent text/UI scales. Shared
defects must be fixed at the semantic theme role or shared component
foundation. A temporary exception may identify only one exact element, must
include a written justification and expiry, and must never exclude a workspace,
state family, or viewport class. Desktop release checks must also open at least
one native select menu on Windows because the operating-system popup is outside
browser automation.

Any new global task state, dialog result, overlay, tutorial cue, form state, or
workspace must add a deterministic audit state when ordinary browser preview
data cannot reach it. The audit must exercise the production component rather
than a look-alike test implementation. Use `npm run audit:ui` for the complete
gate or `npm run audit:contrast` for the colour-and-Axe subset.

Inert previews, context tooltips, and tutorial/help copy use the shared
guidance-surface treatment. Do not make an inert example focusable or render it
as a no-op button. Keep real actions on ordinary control surfaces, label
previews as non-interactive, and never rely on the guidance tint alone.

Themes are inert data. Rust owns schema, contrast, duplicate-appearance,
lifecycle, persistence, and fallback decisions. Svelte may render the returned
report and the theme runtime may apply accepted semantic roles, but frontend
code must not admit, repair, or silently activate a theme. See
[ADR-0015](docs/architecture/decisions/0015-inert-community-themes.md).

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

The optional Tesmio research companion is a separately licensed and installed
exception to the ordinary save-only architecture, not a general plugin host.
It may use only the reviewed chainable observation hook, fixed bounded output,
and exact executable gate documented in ADR-0019. Run `npm run
audit:tesmio-probe` after every change. New hook types, write surfaces, emitted
identities, paths, database access, network access, or build identities require
a new security/evidence review and matching Legal & notices update. Never
commit the built DLL or private telemetry. TesmioLoader's normal defaults are
outside the read-only contract: preserve the fail-closed observation-only
configuration and verifier, including its sole-plugin requirement.

The bundled DuckDB build produces long intermediate C++ paths. The npm Rust and
Tauri scripts select a short operating-system temporary target directory so the
Windows compiler does not exceed legacy path limits. Do not replace that helper
with a personal absolute path.
