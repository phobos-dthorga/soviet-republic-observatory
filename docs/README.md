# Documentation

Republic Observatory is documented from player decision to implementation
boundary. Read the product and data documents before treating a visualisation as
an engineering requirement.

## Product and analysis

- [Project brief](project-brief.md) — the player experience and product promises
- [Analytical catalogue](analytical-catalogue.md) — proposed graphs, statistics,
  models, decisions, and delivery phases
- [Metric definitions](metric-definitions.md) — formulas, denominators, and
  guardrails
- [Branch-bound player plans and metric context](architecture/decisions/0023-branch-bound-player-plans-and-metric-context.md)
  — immutable plan revisions, exact-head evaluation, evidence classes, and the
  shared metric-tooltip contract
- [Material Periodic Table and Industrial Laboratory](material-periodic-table.md)
  — pseudo-elements, material dossiers, reaction pathways, yield, sensitivity,
  and shock experiments
- [Broadcast Desk](broadcast-desk.md) — receiver fields, station evidence,
  programme experiments, and deterministic bulletins
- [Dashboard and interface](dashboard-and-interface.md) — workspace hierarchy,
  interactions, chart contracts, and visual language
- [Advanced Visual Grammar](advanced-visual-grammar.md) — the exceptional-visual
  admission test, Sankey flow contract, accessible fallback, and extension
  boundary
- [Data sources and limitations](data-sources-and-limitations.md) — observed save
  structure, coverage, and claims the product must not make
- [Citizen Lives and Family Trajectories feasibility](citizen-lives-feasibility.md)
  — worker-table research, stable-identity gate, aggregate vertical slice,
  future data contract, scale, and privacy boundary
- [TesmioLoader reverse-engineering assessment](research/tesmioloader-reverse-engineering.md)
  — live instrumentation, binary-serializer research, upstream citizen-field
  candidates, safety boundaries, and the proposed read-only research bridge
- [Dynamic resource catalogue and live reconciliation](research/dynamic-resource-catalogue.md)
  — exact-token discovery, installed labels, optional session readings, and the
  boundary between live prices and recorded history
- [Broadcast telemetry research findings](research/broadcast-telemetry-findings.md)
  — positive and negative station-field findings, experiment protocol, and the
  promotion gate for audience and programme data
- [Legal and third-party notices](legal-and-third-party-notices.md) — project
  independence, local data, licensing, read-only native risk, and evidence limits
- [Dependency decisions](dependencies.md) — current libraries, licences, and
  complexity budget
- [Definition Catalogue and Planning Warehouse](definition-catalogue-and-warehouse.md)
  — dual-engine ownership, projection recovery, generations, and overlays
- [W&R compatibility profiles](compatibility-profiles.md) — reviewed mappings,
  one local repair file, immutable interpretation identity, bounded binary
  layouts, evidence consequences, and contribution workflow
- [Markets and external economy](markets-and-external-economy.md) — exact-save
  indexing, currency-separated evidence, analytical formulas, player baskets
  and scenarios, warehouse recovery, and limitations
- [Local diagnostics and long-running work](operations/diagnostics.md) — shared
  critical-task progress, startup recovery, stall visibility, bounded local
  logging, and privacy
- [Native UI review](operations/native-ui-review.md) — mouse-free packaged-app
  automation, fixture/live boundaries, commands, artifacts, and troubleshooting
- [Development quality gates](operations/development-gates.md) — staged build
  costs, one-package final validation, timing evidence, and deferred optimisation
  options
- [Roadmap](roadmap.md) — vertical slices from synthetic preview to advanced
  industrial analysis
- [Community Extensions](extensions/overview.md) — Analysis Packs now and the
  future Model Plugin boundary
- [Analysis Pack authoring](extensions/analysis-pack-authoring.md)
- [Extension threat model](extensions/threat-model.md)
- [Localisation and language-pack authoring](localization/README.md) — canonical
  catalogue, Fluent patterns, validation, fallback, UI/game vocabulary split,
  and extension-text ownership
- [Accessibility, contextual guidance, and notifications](accessibility-guidance-and-notifications.md)
  — readable typography, tutorial-ready help, app-wide feedback, shared-service
  criteria, and debugging policy
- [Safe community themes and contrast assurance](themes.md) — inert semantic
  colour roles, native validation, immutable revisions, fallback, authoring,
  and every-build interface audits
- [Historical analytical heads and continuations](architecture/decisions/0017-historical-heads-and-continuations.md)
  — exact save previews, durable forks, many-to-many memberships, and ancestry
  integrity

## Architecture

- [Architecture overview](architecture/overview.md)
- [ADR-0001: local-first read-only save observation](architecture/decisions/0001-local-first-save-observation.md)
- [ADR-0002: WyrmGrid-derived interface methodology](architecture/decisions/0002-wyrmgrid-interface-methodology.md)
- [ADR-0003: declarative ECharts boundary](architecture/decisions/0003-declarative-echarts-boundary.md)
- [ADR-0004: branch-aware observation timeline](architecture/decisions/0004-branch-aware-observation-timeline.md)
- [ADR-0005: declarative Analysis Packs before executable Model Plugins](architecture/decisions/0005-declarative-analysis-packs-and-model-plugins.md)
- [ADR-0006: versioned community localisation before save parsing](architecture/decisions/0006-versioned-community-localisation.md)
- [ADR-0007: stream receiver history read-only before adding a watcher](architecture/decisions/0007-streaming-receiver-observation.md)
- [ADR-0008: application-owned SQLite persistence boundary](architecture/decisions/0008-sqlite-persistence-boundary.md)
- [ADR-0009: compact history before automatic observation](architecture/decisions/0009-compact-history-before-automatic-observation.md)
- [ADR-0010: native recorder and durable candidate ledger](architecture/decisions/0010-native-recorder-and-durable-candidate-ledger.md)
- [ADR-0011: dual-engine definition catalogue and planning warehouse](architecture/decisions/0011-dual-engine-catalogue-and-warehouse.md)
- [ADR-0012: versioned inert game compatibility profiles](architecture/decisions/0012-versioned-game-compatibility-profiles.md)
- [ADR-0013: governed DuckDB write boundary](architecture/decisions/0013-governed-duckdb-write-boundary.md)
- [ADR-0014: exceptional visuals through explicit analytical contracts](architecture/decisions/0014-exceptional-visuals-doctrine.md)
- [ADR-0015: inert community themes with native validation](architecture/decisions/0015-inert-community-themes.md)
- [ADR-0016: enforce the domain and presentation boundary](architecture/decisions/0016-domain-presentation-boundary.md)
- [ADR-0017: exact analytical heads and evidence-backed continuations](architecture/decisions/0017-historical-heads-and-continuations.md)
- [ADR-0018: evidence-gate individual citizen histories](architecture/decisions/0018-evidence-gated-citizen-histories.md)
- [ADR-0019: optional read-only native research bridge](architecture/decisions/0019-optional-read-only-native-research-bridge.md)
  — fail-closed same-process observation without a product dependency
- [ADR-0020: first-class attention cues and bounded research setup](architecture/decisions/0020-first-class-attention-and-research-setup.md)
  — persistent accessible guidance and an exact-source local build assistant
- [ADR-0021: external native UI review with bounded fixture scenarios](architecture/decisions/0021-native-ui-review-boundary.md)
  — mouse-free native automation without a production control surface
- [ADR-0022: bounded host-owned production pathways](architecture/decisions/0022-bounded-production-pathways.md)
  — exact multi-stage coefficients, explicit alternatives, and visible stops
- [ADR-0023: branch-bound player plans and one metric-context contract](architecture/decisions/0023-branch-bound-player-plans-and-metric-context.md)
  — SQLite-owned intent, deterministic schedules, exact-head integrity, and
  standard metric explanations
- [ADR-0024: bounded actionable recovery by default](architecture/decisions/0024-actionable-recovery-by-default.md)
  — explicit, non-destructive in-app remedies when one host-owned safe action
  is known
- [ADR-0025: bounded application settings](architecture/decisions/0025-bounded-application-settings.md)
  — one typed preference authority without exposing evidence, security, or
  storage-integrity policy as presentation choices
- [ADR-0026: recorder-first resumable maintenance](architecture/decisions/0026-recorder-first-resumable-maintenance.md)
  — bounded background leases, durable checkpoints, content-addressed market
  evidence, and ordinary single-instance ownership
- [ADR-0027: player-first language with optional technical wording](architecture/decisions/0027-player-first-language.md)
  — plain default copy, optional technical English, and automated readability
  enforcement
- [ADR-0028: first-class related-data navigation](architecture/decisions/0028-first-class-related-data-navigation.md)
  — allowlisted drill-downs, exact-save history, and accessible reversible paths

## Evidence status vocabulary

Every public finding and visual should carry one of these statuses:

| Status                | Meaning                                                          |
| --------------------- | ---------------------------------------------------------------- |
| Save fact             | Directly parsed from a save payload                              |
| Game-definition fact  | Read from an installed game definition or reviewed documentation |
| Calculation           | Deterministic result from identified inputs and a versioned rule |
| Extension calculation | Host-evaluated result from an identified Analysis Pack           |
| Estimate              | Model result with uncertainty and stated assumptions             |
| Recommendation        | Suggested player action, never presented as a fact               |
| Unavailable           | The source did not establish the value                           |
| Player override       | Player replacement layered over an installed definition fact     |
| Player definition     | Player supplemental planning entity                              |
| Reviewed mapping      | Fact interpreted through a repository-reviewed compatibility map |
| Player mapped         | Fact interpreted through the active app-local compatibility map  |
