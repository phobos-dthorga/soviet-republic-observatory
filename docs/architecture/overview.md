# Architecture overview

Republic Observatory is designed as a local-first desktop application assembled
through narrow vertical slices. The first production slice will observe one
save, parse a supported global-history subset, store one distinct branch-aware
observation, and render it through the existing declarative chart contract.

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
  ancestry/dedupe ─────► branch-aware observation identity
          │
          ▼
  SQLite repository ───► raw-field evidence references + normalised facts
          │
          ▼
  analytics services ──► calculations, estimates, recommendations
          │
          ▼
  thin Tauri commands ─► Svelte presentation ─► ObservatoryChart ─► ECharts
```

## Layer responsibilities

### Observer

Watches only configured directories, waits for a candidate file to stabilise,
validates ZIP structure, and opens entries read-only. It never extracts beside
the save, mutates timestamps, or manages save retention.

### Parser

Translates supported source fields into stable facts. It emits coverage and
unsupported-field evidence rather than leaking raw parser maps throughout the
application. Sanitised fixtures establish compatibility.

### Timeline service

Hashes statistical payloads, identifies duplicates, compares record prefixes,
records parent observation where supported, and starts a branch where ancestry
is ambiguous or divergent. It never orders saves solely by filename.

### Storage

SQLite stores saves, payload identities, branches, records, snapshots, field
coverage, annotations, targets, and versioned analytical results. Migrations
become append-only after release. Raw save archives remain outside the database.

### Analytics

Deterministic metrics, anomaly models, forecasts, experiments, scenarios, and
optimisation live outside the interface. Every result records input observation
range, rule/model version, assumptions, and evidence status.

### Presentation

Svelte owns layout, formatting, interaction state, accessibility, and calls into
application services. It does not parse files, calculate business metrics, or
decide recommendations.

## Technology direction

- **Rust** for bounded archive access, parsing, timeline logic, SQLite, and
  analytical services where correctness and performance matter.
- **Tauri 2** for the desktop shell and a small command boundary.
- **Svelte 5 + TypeScript** for the interface.
- **Apache ECharts** behind an application-owned declarative adapter.
- **SQLite** for local observations and plans; encryption is evaluated when the
  first persistent vertical slice defines the actual sensitivity and backup
  model.

MapLibre, Three.js, a plugin system, a hosted service, and a general notebook
runtime are not foundation dependencies. Each requires a demonstrated current
use case and an explicit replacement boundary.

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

Dates retain game year/day and a derived display date only when the conversion
is verified. Resource identifiers remain source identifiers plus a versioned
display catalogue; display text is not the database key.

## Failure behaviour

- A half-written save waits and retries without blocking previously stored data.
- A corrupt archive is recorded as a bounded observation failure, not repaired.
- An unsupported field is reported in coverage and does not abort unrelated
  supported metrics.
- An identical payload updates scanner evidence without duplicating history.
- A rollback creates or selects a branch; it does not delete later history.
- A model failure removes the estimate, not the underlying facts.
- The interface remains useful offline and when the game is not running.
