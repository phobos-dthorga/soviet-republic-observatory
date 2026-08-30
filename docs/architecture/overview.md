# Architecture overview

Republic Observatory is a local-first desktop application assembled through
narrow vertical slices. The desktop host observes stable saves manually or
automatically, parses receiver-class history plus bounded current/city facts,
stores distinct states and file observations separately, resolves supported
prefix ancestry, and renders the selected branch through the declarative chart
contract.

## Proposed components

```text
Configured save directory
          │ native event + periodic reconciliation
          ▼
  Native recorder ─────► durable candidate ledger
          │
          ▼
  Read-only observer ──► archive validation and stable-file gate
          │
          ▼
  compatibility map ──► reviewed or player-mapped source vocabulary
          │
          ▼
  stats/binary parser ─► application-owned facts + coverage report
          │
          ▼
  content dedupe ──────► neutral observation identity
          │
          ▼
  operational store ───► SQLite facts + lineage + projection outbox
          │ versioned idempotent models
          ▼
  write governor ──────► bulk budgets + progress + failure backoff
          │
          ▼
  DuckDB warehouse ────► catalogue generations + analytical projections
          │
          ▼
  analytics services ──► pinned snapshots + Analysis Pack calculations
          │
          ▼
  Tauri events/commands ► Svelte presentation ─► ObservatoryChart ─► ECharts
                                ▲
             UI catalogue + game-vocabulary resolver
```

## Layer responsibilities

### Observer

Inspects only a player-configured directory, validates bounded archive and entry
sizes, compares file metadata before and after reading, and streams `stats.ini`
read-only. Manual observation selects the newest candidate. The opt-in automatic
observer runs while the desktop application is open. A native Rust service uses
folder events as wake-up hints, performs a full reconciliation at least every 15
seconds, and feeds a stable-file and bounded-retry state machine. It queues every
newly noticed candidate without relying on the Svelte event loop.
It never extracts beside the save, mutates timestamps, or manages save
retention.

### Recording ledger

The fourth SQLite migration records candidate discovery, stabilisation, read
attempts, imported or duplicate outcomes, retryable or terminal failures, and
superseded file identities. Interrupted work returns to `discovered` on restart.
The ledger stores a privacy-preserving directory identity and bounded file
evidence, never a configured full path or archive payload. First observation of
a directory baselines older files and considers the newest candidate; later
candidates are processed in deterministic modification-time and filename order.

### Parser

Translates supported source fields into stable facts. Version-sensitive archive
aliases, stats markers/fields, definition directives, and reviewed fixed binary
layouts come from a strict inert compatibility profile. Stable meanings,
operations, limits, and scope rules remain compiled host contracts. It emits
coverage and unsupported-field evidence rather than leaking raw parser maps
throughout the application. Sanitised fixtures establish compatibility.
Definition mappings may additionally reference a source-qualified Workshop/WIP
scope. Exact pins stop publication after definition changes; tracked scopes
remain active with an unreviewed-update warning. Save mappings cannot inherit
catalogue scopes.

### Timeline service

The implemented timeline service hashes statistical payloads, compares exact
supported receiver-history records, continues strict prefixes, and records the
nearest observed parent. Extending an older state after an incompatible tip,
re-observing a shorter prefix, or finding a unique partial divergence creates a
separate fork. Evidence tied across branches or sharing no supported prefix
remains `unassigned`. Filename and modification order never establish ancestry.

### Storage

The first append-only SQLite migration stores observation sources, embedded
receiver records, and normalised metric observations. The second separates
observed files from distinct states and adds timeline branches, lineage
evidence, compact cumulative history signatures, and branch selection. Prefix
resolution scans one signature per state and loads full records only for branch
tips when divergence evidence requires them. Connection, migration,
observation, branch, and settings responsibilities are separate modules behind
one application-owned storage API. The third migration adds content-addressed
receiver-history nodes, latest-line evidence, save-sampled republic/city scopes,
and privacy-preserving directory identities. The fourth adds the durable
recorder ledger and runtime scan/event timestamps; the fifth records which save
directories have completed their initial baseline. New successors share exact prefix
nodes rather than duplicating full histories. Configured paths remain private
settings and never enter the presentation model. Later migrations add
annotations, targets, and analytical results. Raw archives remain outside the
database.

SQLite is app-local and deliberately unencrypted: the current database contains
no credentials or secrets that justify application key management. Ordinary OS
file permissions are the protection boundary. Future credentials belong in an
OS credential vault; they do not automatically justify encrypting observations.

The sixth SQLite migration adds the projection outbox, immutable planning-overlay
revisions, global overlay state, and catalogue refresh metadata. App-local
DuckDB has independent append-only migrations and stores retained definition
generations, normalised properties and relations, effective overlay projections,
and large observation matrices. A receipt written with each DuckDB projection
closes the crash gap before SQLite acknowledgement. DuckDB failure is visible
analytical lag and cannot block SQLite ingestion.

The governed write boundary rejects oversized jobs before warehouse mutation,
requires append-and-merge transfer for variable-cardinality rows, reports the
active write without waiting for its connection, and applies bounded
exponential backoff after consecutive failures. Fixed metadata statements
remain direct and transactional. The governor never drops or delays SQLite
observation commits.

The seventh SQLite migration adds Analysis Pack identities, immutable validated
revisions, and per-pack enabled-revision state. Pack JSON is bounded and
canonicalised before storage. It is operational extension state, not a third
analytical store; the host parses it again before evaluation and isolates a
failed revision.

The eighth SQLite migration separates raw payload identity from immutable
interpretation identity and records exact compatibility-profile provenance.
One app-local override can activate immediately as visibly `player_mapped`
evidence; invalid edits retain the last valid profile. Legacy observations are
backfilled to the reviewed W&R 1.1.1.9 mapping without changing values or branch
labels, then re-projected through an idempotent warehouse rebuild.

### Analytics

Deterministic metrics, anomaly models, forecasts, experiments, scenarios, and
optimisation live outside the interface. Every result records input observation
range, rule/model version, assumptions, and evidence status.

Analysis Packs are validated inert declarations evaluated by the host over
normalised observations. Future Model Plugins sit outside the process and
receive only bounded public models. Neither tier gains raw-save, SQLite, parser,
private-path, or rendering access.

### Presentation

Svelte owns layout, formatting, interaction state, accessibility, and calls into
application services. It does not parse files, calculate business metrics, or
decide recommendations.

All host-owned prose and locale-sensitive formatting pass through the versioned
localisation service. In desktop builds, Rust validates community language
packs and SQLite owns their manifests and selected identity. Svelte receives
only bounded status and manifest models. The browser preview has an explicitly
non-authoritative local fallback. Community packs are inert catalogues layered
over canonical `en-AU`. Installed-game display vocabulary is a separate
future resolver over stable source IDs; neither translated UI nor game labels
become database keys. Analysis Pack prose carries its own declared locale.

The shell owns a bounded notification centre for transient outcomes. Workspace
services submit localised text and severity through one presentation contract;
they do not create independent toast stacks. Contextual explanations use one
keyboard-accessible help primitive with stable topic IDs so a later tutorial
can compose existing guidance. Inline validation and critical-task progress
remain distinct contracts.

## Technology direction

- **Rust** for bounded archive access, parsing, timeline logic, SQLite, DuckDB, and
  analytical services where correctness and performance matter.
- **Tauri 2** for the desktop shell and a small command boundary.
- **Svelte 5 + TypeScript** for the interface.
- **Apache ECharts** behind an application-owned declarative adapter.
- **SQLite** for local observations, plans, recorder and extension lifecycle
  state behind a modular persistence boundary; the database is unencrypted by
  explicit decision.
- **DuckDB** for source-qualified catalogue history and model-ready analytical
  projections, with bundled operation and extension loading disabled.

MapLibre, Three.js, an executable plugin runtime, a hosted service, and a general
notebook runtime are not foundation dependencies. The Analysis Pack schema is a
foundation contract, while actual local import waits for branch-aware storage.
Each later dependency requires a demonstrated current use case and an explicit
replacement boundary.

## Initial domain concepts

- `SaveObservation`
- `RecorderCandidate` and `RecorderHealth`
- `StatisticalPayloadIdentity`
- `TimelineBranch`
- `GlobalHistoryRecord`
- `RepublicSnapshot`
- `CitySnapshot`
- `ResourceMeasure`
- `EvidenceCoverage`
- `Plan` and `PlanTarget`
- `InterventionAnnotation`
- `AnalyticalResult`
- `ChartSpec`
- `SankeyChartSpec` (application-owned advanced visual contract)
- `AnalysisPackDeclaration`
- `ExtensionContentIdentity`
- `LanguagePackManifest` and `LanguageSelection`
- `GameVocabularyCatalogue`
- `CompatibilityProfile`, `CompatibilityProvenance`, and `InterpretationIdentity`
- `CatalogueGeneration`, `DefinitionDossier`, and `PlanningOverlayRevision`
- `ProjectionJob` and `WarehouseSnapshot`
- `WarehouseWriteActivity` and governed write budget
- `AppNotification` and contextual `HelpTopic`

Dates retain game year/day and a derived display date only when the conversion
is verified. Resource identifiers remain source identifiers plus a versioned
display catalogue; display text is not the database key.

## Failure behaviour

- A save that changes during inspection is rejected without affecting
  previously stored data; the automatic observer waits and retries boundedly.
- A corrupt archive is recorded as a bounded observation failure, not repaired.
- An unsupported field is reported in coverage and does not abort unrelated
  supported metrics.
- An identical payload updates scanner evidence without duplicating history.
- A rollback creates or selects a branch; it does not delete later history.
- A model failure removes the estimate, not the underlying facts.
- An extension failure removes that extension's result, not save observation,
  core dashboards, or another extension.
- A catalogue refresh failure preserves the last published generation.
- A warehouse write failure creates visible lag and leaves the recorder,
  Archive, and SQLite-backed charts usable.
- An invalid compatibility edit leaves the last valid mapping active; a valid
  change creates a new interpretation and never rewrites earlier evidence.
- The interface remains useful offline and when the game is not running.
