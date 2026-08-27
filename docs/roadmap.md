# Roadmap

The roadmap is organised as verifiable vertical slices. Dates and release
numbers are assigned only when implementation begins.

## Foundation — completed

- Public product and analytical specification
- WyrmGrid-informed visual system with independent Observatory identity
- Svelte 5/TypeScript preview
- Declarative ECharts adapter with synthetic, visibly labelled data
- Presentational Briefing, Broadcast, and Extensions workspace concepts
- Draft 2020-12 Analysis Pack, chart-template, and concrete-chart schemas
- Receiver Adoption Laboratory example plus structural, semantic, limit, and
  injection tests
- Complete current-interface localisation through canonical `en-AU`, Fluent
  formatting, strict inert community language packs, per-message fallback,
  explicit install/select/remove lifecycle, and locale-aware chart summaries
- Localisation audit, expanded pseudo-language tests, and RTL contract tests
- Initial calculation utilities and unit tests
- Data limitations, metric contract, architecture decisions, and contributor
  safeguards

## Slice 1 — one save, one trusted chart — completed

- Tauri/Rust desktop shell and player-selected save/game directories
- Bounded ZIP validation, pre/post-read stability check, and streaming read-only
  `stats.ini` access without extraction
- Actual game year/day and the four receiver-class history fields
- App-local unencrypted SQLite source, record, normalised metric, parser,
  coverage, and content identity records through an append-only migration
- Payload deduplication and one observed 100% receiver-composition chart with an
  exact evidence inspector
- Sanitised fixtures for complete, partial, malformed, duplicate, unsupported,
  and missing-payload cases, plus an optional local-save conformance test
- A versioned installed-game vocabulary-source catalogue kept separate from
  parser IDs and Observatory UI language; BTF decoding remains unavailable
- Explicit mixed-evidence presentation: only the receiver ladder becomes a save
  fact while the remaining Broadcast concepts stay synthetic

## Slice 2 — branch-aware archive

Implemented foundation:

- Modular application-owned SQLite persistence boundary
- Separate file-observation and distinct-state counts
- Prefix-based ancestry, parent evidence, rollback forks, partial-divergence
  forks, and conservative unassigned histories
- Branch selection shared by Archive and observed Broadcast data
- Historical-record catalogue with coverage and content identity
- Append-only migration and version-one database backfill
- Content-addressed shared-prefix receiver history, growth benchmark, and
  version-one history backfill
- Opt-in automatic observation with a stable-file window, bounded retry, and a
  queue for every new candidate noticed while the desktop program is open
- Same-branch Archive comparison of two distinct observed states
- Save-sampled current and numeric city snapshot facts with explicit coverage
- Native event-driven recorder with periodic reconciliation fallback
- Crash-recoverable SQLite candidate lifecycle ledger
- Event-driven Observer Health and save-to-save Republic Pulse workspace

Remaining in this slice:

- Authoritative Rust validation and persistence for community language packs

## Slice 3 — definition catalogue and planning warehouse — implemented foundation

- SQLite operational authority plus content-addressed projection outbox
- Pinned bundled DuckDB with independent append-only migrations, receipts,
  watermarks, rebuilds, and visible analytical lag
- Source-qualified base, DLC, subscribed Workshop, and WIP discovery
- Changed-file hashing with unchanged-revision reuse and source-removal handling
- Typed building, vehicle, resource, production, construction, capacity, and
  unknown-directive records with retained catalogue generations
- Five-second native refresh batching, startup fingerprint reconciliation, and
  transactional generation publication
- Materials catalogue search, dossiers, warehouse health, and provenance
- Strict `.rooverlay.json` contract with immutable named revisions, explicit
  lifecycle, conflict fallbacks, and supplemental player definitions
- Model snapshots pin catalogue generation, overlay revision, observation
  watermark, schema, and projector versions

Ignored reference-machine scale and growth benchmarks accompany the automated
suite. Broader verified directive coverage remains ongoing catalogue hardening.

## Slice 4 — local Analysis Packs

- Authoritative Rust validation against Analysis Pack schema and semantic rules
- Local file inspection with ID, version, content hash, inputs, and contributions
- Distinct install, enable, disable, update, rollback, and remove records
- Host evaluation over branch-aware normalised observations only
- Host-resolved charts, provenance, accessibility, settings, and failure states
- Receiver Adoption Laboratory loaded through the same public contract as any
  community pack
- Invalid or failed packs isolated from save observation and core dashboards

This slice follows branch-aware storage and the catalogue warehouse. Packs
remain independent of installed definitions unless they explicitly request
published catalogue metrics.

## Slice 5 — Republic Briefing and Broadcast foundation

- Player plans and targets
- Plan attainment and guardrails
- Demographic decomposition and rates
- Trade exposure and concentration
- Deterministic Ministry Dispatch
- Attention queue with robust baseline signals
- Receiver adoption from the four stable citizen-electronics metrics
- Broadcast Notebook annotations and deterministic Evening Bulletin
- Binary station telemetry shown as unavailable until the research track
  validates it

## Slice 6 — Materials and markets

- Material Periodic Table
- Price baskets and indexed market views
- Resource-use matrix and Pareto
- Currency-specific trade, tourism, debt, and break-even analysis
- Measurement coverage and accounted-flow residual presentation

## Slice 7 — Population and cities

- Welfare small multiples from observed snapshots
- City heatmap, ranking, weighted dispersion, and intervention queue
- Annotations and event-time comparisons
- Experimental control charts with baseline diagnostics

## Slice 8 — Industrial Laboratory

- Expanded definition coverage and verified automatic-cost rules
- Production-chain graph
- Target propagation and limiting-input analysis
- Cost sensitivity and scenario engine
- Linear optimisation after actual capacity evidence is available

## Research track

- Inventory and production coverage
- Building identity and operating state
- Vehicles and route flows
- Network and geographic topology
- City name and coordinate mapping
- Waste treatment and recovery coverage
- Radio and television identity, staffing, programme, reach, rating, and budget
  telemetry

Research findings do not become product claims until they have versioned
fixtures, compatibility limits, and a safe unavailable-data path.

## Deferred until demonstrated

- Executable Model Plugin manifests, packages, protocol, and runtime; these wait
  for a model that exceeds Analysis Pack vocabulary and a completed security
  review
- Hosted accounts or synchronisation
- Community catalogue or marketplace; local offline installation remains the
  baseline
- Map or 3D renderer
- Automated natural-language model dependency
- Binary-save modification of any kind
