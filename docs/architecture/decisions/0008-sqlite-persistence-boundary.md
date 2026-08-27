# ADR-0008: application-owned SQLite persistence boundary

Status: Accepted, storage-engine scope superseded by ADR-0011

## Context

Republic Observatory needs durable local observations, branch ancestry,
settings, annotations, and extension lifecycle records. OnAir WyrmGrid proved
SQLite, transactions, append-only migrations, and real in-memory or temporary
database tests useful for this class of desktop application. It also showed the
maintenance cost of allowing one store module to accumulate unrelated record
types and queries.

The Observatory has no account credentials or other secrets requiring an
application-managed encryption-key lifecycle. Database portability to a hosted
server is also not a current product requirement.

## Decision

- Use bundled SQLite as the authoritative operational storage engine. ADR-0011
  supersedes the former “sole storage engine” clause for the analytical
  catalogue and warehouse.
- Place an application-owned persistence API between services and SQLite.
- Keep connection configuration, migrations, settings, observations, branch
  resolution, and query projections in cohesive storage modules.
- Return bounded domain and presentation models; never expose connections, SQL,
  table names, or the database path to Svelte, Analysis Packs, or Model Plugins.
- Import each distinct supported save state, its records, normalised metrics,
  coverage, provenance, lineage, and archive evidence in one transaction.
- Store one compact cumulative supported-history signature per distinct state.
  Prefix resolution scans signatures and loads complete records only for branch
  tips when reverse-prefix or divergence evidence requires them.
- Keep numbered migrations append-only after release.
- Test against real temporary SQLite databases rather than a behaviourally
  different fake backend.
- Keep the database unencrypted. Ordinary operating-system file permissions
  are the current protection boundary.
- Do not introduce an ORM, database-agnostic query layer, or SQLCipher. A second
  engine requires a demonstrated workload and explicit compatibility decision;
  ADR-0011 records that decision for DuckDB.

## Consequences

SQLite remains replaceable behind application-owned models, but replacement is
not treated as a goal by itself. Storage code is slightly more structured than
a single repository file and cannot be called directly from presentation code.

Backups and exports are unencrypted and must be labelled accordingly when they
are implemented. If a future feature introduces credentials, those credentials
should normally use the operating system's credential vault rather than cause
the observation database to acquire its own encryption-key lifecycle. DuckDB is
now the catalogue-history and derived analytical authority described by
ADR-0011; it does not replace SQLite operational truth.

ADR-0009 records the completed growth benchmark and replaces complete
per-payload receiver-history writes with content-addressed shared-prefix nodes
before continuous observation is enabled.
