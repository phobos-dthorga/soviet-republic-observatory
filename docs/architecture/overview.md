# Architecture overview

Republic Observatory is a local-first desktop application assembled through
narrow vertical slices. The first production slice now observes the newest save
on explicit request, parses the receiver-class global-history subset, stores one
distinct observation, and renders it through the declarative chart contract.
Automatic watching and branch ancestry remain the next slice.

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
  SQLite repository ───► raw-field evidence references + normalised facts
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

Inspects only a player-configured directory after an explicit request, selects
the newest ZIP candidate, validates bounded archive and entry sizes, compares
file metadata before and after reading, and streams `stats.ini` read-only. It
never extracts beside the save, mutates timestamps, or manages save retention.
The automatic watcher is not yet implemented.

### Parser

Translates supported source fields into stable facts. It emits coverage and
unsupported-field evidence rather than leaking raw parser maps throughout the
application. Sanitised fixtures establish compatibility.

### Timeline service — next slice

The current slice hashes statistical payloads and rejects duplicate history.
The next slice compares record prefixes, records parent observation where
supported, and starts a branch where ancestry is ambiguous or divergent. It
will never infer historical order solely from filename.

### Storage

The first append-only SQLite migration stores observation sources, embedded
receiver records, and normalised metric observations. Source fields and lines,
payload hash, parser/profile versions, branch placeholder, scope, and coverage
remain queryable. Configured paths are private settings and never enter the
presentation model. Later migrations add resolved branches, snapshots,
annotations, targets, and analytical results. Raw archives remain outside the
database.

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
- **SQLite** for local observations and plans; encryption is evaluated when the
  first persistent vertical slice defines the actual sensitivity and backup
  model.

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
  previously stored data; automatic wait/retry belongs to the watcher slice.
- A corrupt archive is recorded as a bounded observation failure, not repaired.
- An unsupported field is reported in coverage and does not abort unrelated
  supported metrics.
- An identical payload updates scanner evidence without duplicating history.
- A rollback creates or selects a branch; it does not delete later history.
- A model failure removes the estimate, not the underlying facts.
- An extension failure removes that extension's result, not save observation,
  core dashboards, or another extension.
- The interface remains useful offline and when the game is not running.
