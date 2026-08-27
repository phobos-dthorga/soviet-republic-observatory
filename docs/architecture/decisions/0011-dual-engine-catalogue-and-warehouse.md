# ADR-0011: dual-engine definition catalogue and planning warehouse

Status: Accepted

## Context

Save recording needs short, reliable transactions and must remain available
during analytical maintenance. Definition generations, typed properties,
historical matrices, production graphs, and fleet-capability queries are bulk
analytical workloads. Keeping every workload in one SQLite store would couple
recorder reliability to catalogue growth and make reproducible historical
planning queries unnecessarily awkward.

## Decision

- SQLite remains operational authority for settings, ingestion, recorder
  state, branches, overlays, and a durable projection outbox.
- Bundled DuckDB is the primary definition catalogue and analytical warehouse.
- The two engines never attach to or query each other. Versioned Rust domain
  models and prepared bulk writers are the only transfer boundary.
- DuckDB extension autoload, automatic installation, and external access are
  disabled. No runtime extension download or SQLite extension is permitted.
- Warehouse commits precede SQLite outbox acknowledgement. DuckDB projection
  receipts make retries idempotent across the crash gap.
- Warehouse failure is analytical lag and cannot roll back a completed save
  observation.
- Both stores are app-local, offline, and unencrypted. No credentials are
  stored.
- Migrations and compatibility checks remain independent and append-only.

This supersedes ADR-0008's clause naming SQLite the sole local storage engine.
ADR-0008's application-owned boundary, bounded domain models, unencrypted
storage decision, and SQLite operational authority remain in force.

## Consequences

The application has a more explicit synchronisation protocol and backup story.
It gains reproducible retained catalogue generations and a warehouse suited to
large model-ready tables without placing analytical load on the recorder.
Observer Health must report lag, pending and failed jobs, last projection,
watermark, rebuild state, and database size. Models must pin a generation,
overlay revision, observation watermark, warehouse schema, and projector
version before querying.

Process-local locking coordinates the current single application instance.
DuckDB's documented multi-process write limitations mean the database is not a
shared network store and must not be placed on NAS or cloud-synchronised
storage. A future writer topology requires a new ADR.
