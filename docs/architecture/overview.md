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
          │
          ▼
  Read-only observer ──► archive validation and stable-file gate
          │
          ▼
  stats.ini parser ────► application-owned facts + coverage report
          │
          ▼
  content dedupe ──────► neutral observation identity
          │
          ▼
  storage boundary ────► SQLite facts + lineage + query projections
          │
          ▼
  analytics services ──► built-in + Analysis Pack calculations
          │
          ▼
  thin Tauri commands ─► Svelte presentation ─► ObservatoryChart ─► ECharts
                                ▲
             UI catalogue + game-vocabulary resolver
```

## Layer responsibilities

### Observer

Inspects only a player-configured directory, validates bounded archive and entry
sizes, compares file metadata before and after reading, and streams `stats.ini`
read-only. Manual observation selects the newest candidate. The opt-in automatic
observer runs while the desktop application is open, uses a Rust-owned
stable-file and retry state machine, and queues every newly noticed candidate.
It never extracts beside the save, mutates timestamps, or manages save
retention.

### Parser

Translates supported source fields into stable facts. It emits coverage and
unsupported-field evidence rather than leaking raw parser maps throughout the
application. Sanitised fixtures establish compatibility.

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
and privacy-preserving directory identities. New successors share exact prefix
nodes rather than duplicating full histories. Configured paths remain private
settings and never enter the presentation model. Later migrations add
annotations, targets, and analytical results. Raw archives remain outside the
database.

SQLite is app-local and deliberately unencrypted: the current database contains
no credentials or secrets that justify application key management. Ordinary OS
file permissions are the protection boundary. Future credentials belong in an
OS credential vault; they do not automatically justify encrypting observations.

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
localisation service. Community language packs are validated inert catalogues,
layered over canonical `en-AU`. Installed-game display vocabulary is a separate
future resolver over stable source IDs; neither translated UI nor game labels
become database keys. Analysis Pack prose carries its own declared locale.

## Technology direction

- **Rust** for bounded archive access, parsing, timeline logic, SQLite, and
  analytical services where correctness and performance matter.
- **Tauri 2** for the desktop shell and a small command boundary.
- **Svelte 5 + TypeScript** for the interface.
- **Apache ECharts** behind an application-owned declarative adapter.
- **SQLite** for local observations and plans behind a modular persistence
  boundary; the database is unencrypted by explicit decision.

MapLibre, Three.js, an executable plugin runtime, a hosted service, and a general
notebook runtime are not foundation dependencies. The Analysis Pack schema is a
foundation contract, while actual local import waits for branch-aware storage.
Each later dependency requires a demonstrated current use case and an explicit
replacement boundary.

## Initial domain concepts

- `SaveObservation`
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
- `AnalysisPackDeclaration`
- `ExtensionContentIdentity`
- `LanguagePackManifest` and `LanguageSelection`
- `GameVocabularyCatalogue`

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
- The interface remains useful offline and when the game is not running.
