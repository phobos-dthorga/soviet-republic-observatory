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

Write ordinary interface copy for W&R players, not for the implementation
team. Lead with what happened or what the player can do, prefer familiar words,
and keep one idea per sentence. Put formulas, diagnostic codes, operation
names, and specialist storage terms in the shared technical-details surface.
Exact game and mod identifiers remain unchanged in labelled source details.
Technical English belongs in the validated partial overlay; community language
packs must never receive English technical wording. The player-language audit
blocks terminology, readability, sentence-length, and raw-code regressions.
See [ADR-0027](docs/architecture/decisions/0027-player-first-language.md).

Rust/domain services own parsing, lineage, persistence, validation, evidence,
analytics, recommendations, and lifecycle decisions. `desktopClient.ts` is the
only Tauri boundary. TypeScript presentation adapters may map bounded host
models into view and chart models but cannot invoke services or decide domain
meaning. Svelte owns rendering, localisation, accessibility, navigation,
ephemeral interaction state, and service invocation. CSS consumes semantic
state and validated theme roles; it never establishes business meaning. The
automated architecture audit enforces these import and mutation seams. See
[ADR-0016](docs/architecture/decisions/0016-domain-presentation-boundary.md).

Every non-obvious metric must receive a host-owned Metric Context covering its
population/entity basis, time and geography, denominator, comparison rule, and
known limitations. Presentation adapters may translate that context into help
and chart copy but may not reconstruct it from labels or values. Keep the
essential scope visible; contextual help supplements rather than replaces it.

Use the shared notification centre for transient outcomes, the critical-task
components for long-running progress, and inline messages for validation tied
to a particular field or operation. Contextual explanations use the shared
help primitive and stable tutorial topic IDs; required instructions never live
only in a tooltip. New ordinary captions and controls must use the readable
type tokens rather than introduce microtext below the 12-pixel-equivalent
default floor.

Actionable recovery is the default whenever the native host knows one bounded,
deterministic, non-destructive remedy. Attach that typed proposal to the shared
notification and explain its exact side effects in the shared recovery dialog;
do not create workspace-specific repair modals or generic execution commands.
Destructive or ambiguous repair, external updates, permission and credential
changes, low-disk conditions, and choices between competing histories remain
guidance-only until a narrower safe contract is proven. Every new failure state
must record whether it has a safe retry, fallback, rebuild, reload, or
prerequisite-recheck path. See
[ADR-0024](docs/architecture/decisions/0024-actionable-recovery-by-default.md).

Admit a new application setting only for a genuine player preference or a
bounded operational trade-off. It needs one Rust-owned type, default, range or
enumeration, effect boundary, reset behaviour, localisation, and tests. Never
make evidence interpretation, provenance, schema or parser identity, save
stability, storage limits, recorder priority, or security policy configurable.
See [ADR-0025](docs/architecture/decisions/0025-bounded-application-settings.md).

Bulk maintenance must use the recorder-first coordinator and checkpoint work at
bounded, resumable units. Never hold a coordinator lease while parsing an
archive or running a DuckDB calculation, never repeat a completed immutable
unit merely to resume a job, and never treat the user-configurable patience
budget as permission to weaken SQLite integrity or save-stability checks. See
[ADR-0026](docs/architecture/decisions/0026-recorder-first-resumable-maintenance.md).

Use the first-class AttentionCue for important new or newly available actions;
do not reproduce pulse or glow CSS in a workspace. A cue needs a stable ID and
content revision, must respect reduced motion, and can explain an already-valid
action but cannot establish domain availability or safety.

Debuggers, profilers, browser developer tools, database inspection tools, and
trace captures may be used whenever they are the preferable diagnostic method.
Keep captures local, bounded, and free of save contents, personal paths, or
other private data.

## Local checks

Use the smallest gate that can answer the current question. During
implementation, run the fast contract gate:

```powershell
npm install
npx playwright install chromium
npm run verify:fast
```

After the interface and domain contracts have settled, run the browser gate
once:

```powershell
npm run verify:browser
```

At the end of the slice, run the final gate once:

```powershell
npm run desktop:build
```

The final gate stops at the first failed phase, records phase timings beneath
`artifacts/release-gate/`, and does not begin the expensive Windows package
until fast contracts, Rust tests and Clippy, and the browser interface audit
have passed. Tauri reuses that freshly audited web artifact rather than
building and auditing it again. `npm run verify:release:plan` lists the exact
order without executing it. See the [development-gate
guide](docs/operations/development-gates.md).

Do not repeatedly package the application while implementation is still
changing. When only the native review scenarios or driver harness changed and
the application binary did not, `npm run desktop:smoke:existing` deliberately
reuses the existing binary. Any application, Rust, configuration, or bundled
asset change requires the final gate again.

The browser gate includes the production interface audit: contrast and Axe
checks, deterministic component-state screenshots, and geometry checks across
narrow, laptop, FHD, QHD, ultrawide, and UHD-equivalent text/UI scales. Shared
defects must be fixed at the semantic theme role or shared component
foundation. A temporary exception may identify only one exact element, must
include a written justification and expiry, and must never exclude a workspace,
state family, or viewport class. Desktop release checks must also open at least
one native select menu on Windows because the operating-system popup is outside
browser automation.

Install the pinned native review toolchain once with `npm run
ui:review:setup`. The final gate runs the packaged-app smoke suite;
`npm run ui:review -- run --suite full` exercises the complete native scenario,
theme, viewport, and text-scale matrix without moving the global mouse. Reserve
the full matrix for release candidates and exceptional changes to themes,
accessibility, responsive layout, native controls, or the review system itself;
ordinary pull requests and pushes run the smoke gate. New
workspaces, dialogs, notifications, guidance states, task states, and native
controls must add or extend a typed fixture scenario. Do not add arbitrary
selector, JavaScript, Tauri-command, SQL, or filesystem facilities to the
developer CLI. See [ADR-0021](docs/architecture/decisions/0021-native-ui-review-boundary.md).

Any new global task state, dialog result, overlay, tutorial cue, form state, or
workspace must add a deterministic audit state when ordinary browser preview
data cannot reach it. The audit must exercise the production component rather
than a look-alike test implementation. Use `npm run audit:ui` for the complete
gate or `npm run audit:contrast` for the colour-and-Axe subset.

Ordinary application workspaces must never substitute synthetic republic
figures when native evidence is absent. Show the unavailable state, explain the
missing source contract, and keep any supported neighbouring facts usable.
Synthetic host models belong only to the typed UI-review fixture registry or to
an explicit, player-invoked authoring example such as an Analysis Pack sample.
They must be visibly identified as fixtures or examples and must travel through
the same production renderer as real host models. The architecture audit
rejects the former production preview-module pattern.

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
