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
- [Material Periodic Table and Industrial Laboratory](material-periodic-table.md)
  — pseudo-elements, material dossiers, reaction pathways, yield, sensitivity,
  and shock experiments
- [Broadcast Desk](broadcast-desk.md) — receiver fields, station evidence,
  programme experiments, and deterministic bulletins
- [Dashboard and interface](dashboard-and-interface.md) — workspace hierarchy,
  interactions, chart contracts, and visual language
- [Data sources and limitations](data-sources-and-limitations.md) — observed save
  structure, coverage, and claims the product must not make
- [Dependency decisions](dependencies.md) — current libraries, licences, and
  complexity budget
- [Roadmap](roadmap.md) — vertical slices from synthetic preview to advanced
  industrial analysis
- [Community Extensions](extensions/overview.md) — Analysis Packs now and the
  future Model Plugin boundary
- [Analysis Pack authoring](extensions/analysis-pack-authoring.md)
- [Extension threat model](extensions/threat-model.md)
- [Localisation and language-pack authoring](localization/README.md) — canonical
  catalogue, Fluent patterns, validation, fallback, UI/game vocabulary split,
  and extension-text ownership

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
