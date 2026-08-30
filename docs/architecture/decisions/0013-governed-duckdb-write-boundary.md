# ADR-0013: governed DuckDB write boundary

Status: Accepted

## Context

DuckDB is efficient when Republic Observatory transfers analytical rows in
bulk. Per-row prepared statements previously made unchanged catalogue refreshes
and receiver projections take minutes with little CPU use, increasing memory
while the sole warehouse connection prevented startup status from completing.
Fixing individual projectors does not prevent a later feature from recreating
the same failure mode.

SQLite save ingestion must remain independent from analytical maintenance. A
large or failing warehouse job must therefore create visible lag, not exert
backpressure on observation recording or make the application shell wait.

## Decision

All variable-cardinality DuckDB writes pass through the application-owned
warehouse write governor, informally called the **Governator**.

- Variable-cardinality facts use DuckDB appenders into staging tables followed
  by set-oriented merges. A query or statement per fact is prohibited.
- Fixed-cardinality metadata, receipts, watermarks, pointer publication, and
  transactional deletion may use direct statements.
- The governor rejects a write before DuckDB work when its declared row count
  exceeds the host-owned class limit: 6,000,000 catalogue rows, 5,000,000
  observation metric rows, or the schema maximum of 4,608 overlay records.
- The one writer connection remains serial. SQLite's durable outbox supplies
  idempotency and recovery; queue pressure never causes an observation to be
  dropped.
- Each write publishes its kind, stage, bounded row counts, and timestamps in a
  non-blocking health snapshot. Presentation status never waits for the writer.
- Successful writes close the circuit. Consecutive failures apply exponential
  projector backoff from 500 milliseconds to a maximum of 30 seconds. Failed
  jobs remain explicit rather than being retried in a hot loop.
- Catalogue publication, receiver projection, overlay projection, and rebuild
  are governed. New write classes require an explicit budget, progress stages,
  maximum-size regression coverage, and this ADR to be reconsidered when the
  workload cannot fit the bulk contract.

The governor is not a security sandbox, memory allocator, job scheduler, or
licence to truncate analytical evidence. A workload that exceeds its contract
must be partitioned into immutable resumable batches with an atomic publication
step; it must not silently discard rows or raise the global limit casually.

## Consequences

The warehouse API has a small amount of additional lifecycle state, and public
health models gain active-write and retry evidence. In return, variable-sized
writes have one performance contract, startup remains responsive during a
writer transaction, support logs explain backoff, and realistic maximum-size
tests catch regressions before release.

Planning-overlay projection now stages all operations and supplements in bulk
and resolves revision/value conflicts in one set operation. This preserves the
existing `target_missing`, `revision_changed`, and `value_changed` meanings
without thousands of queries and inserts.
