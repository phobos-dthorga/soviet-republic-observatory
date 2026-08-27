# ADR-0009: compact shared history before automatic observation

- Status: accepted
- Date: 2026-08-27

## Context

Every supported save embeds the complete receiver-history prefix. Storing every
record and four metric rows again for every automatically observed save would
make database growth depend on `save count × history length`, even when each
save adds only one historical record. Current and city snapshots are different:
they are save-sampled facts that do not exist as an embedded time series and
must remain one bounded set per distinct state.

The automatic observer must also distinguish an incomplete file from a
permanent parser failure, avoid repeatedly reading a known unchanged file, and
retain every new candidate noticed while the desktop program is open.

## Decision

Receiver history is stored as a content-addressed prefix chain. Each node keeps
one receiver record, its parent, its depth, and the cumulative SHA-256
fingerprint of the prefix ending at that node. A distinct save stores one tip
reference. Exact successors therefore add only their suffix; rollback and
divergent branches reuse their verified common prefix.

Migration 3 backfills compact nodes and latest-line evidence from the released
version-one tables. Those legacy tables remain in the schema for migration
compatibility, but new observations do not add rows to them.

Current and city blocks are stored separately as save-sampled snapshot scopes.
The first supported snapshot vocabulary captures:

- the four receiver classes for the republic;
- selected plain-text citizen counts for the republic; and
- births, deaths, escapes, and the two immigration counts for each numeric city
  source identifier.

The snapshot date is the latest supported historical game date in the same
save. Zero-valued dates inside `$STAT_CURRENT` and `$STAT_CITY` are never used as
the observation date. The `source.stats.*` fact identifiers are internal source
facts, not published extension metrics, until their game meaning and window are
documented.

Automatic observation is opt-in. A Rust-owned state machine, called by a small
desktop heartbeat, waits for the newest candidate to retain identical size and
modification metadata for at least 1.5 seconds. It retries transient incomplete
archive states up to five times, records only sanitised status codes and file
names in the presentation model, and queues every additional candidate noticed
during the desktop session. Existing older files form the initial baseline;
the newest file is considered when observation begins.

## Benchmark evidence

The ignored, repeatable Rust growth test models 128 distinct saves beginning
with 1,900 receiver-history records and adding one record per save. Every save
also contains 18 republic snapshot facts and five facts for each of 139 cities.
In the 2026-08-27 development run it produced:

- 2,027 shared receiver-history nodes, equal to the longest history rather than
  the sum of all histories;
- an 18,870,272-byte SQLite database after a WAL checkpoint; and
- 7.39 seconds total import time in an unoptimised test build.

These numbers are regression evidence for the synthetic workload, not a
performance promise for every machine or republic.

## Consequences

- Continuous observation no longer multiplies the embedded receiver history.
- Snapshot storage still grows linearly with distinct saves and discovered city
  scopes because those facts are the reason to observe each save.
- Automatic observation runs only while the desktop program is open; it is not
  an operating-system service.
- A terminal failure cannot damage earlier observations. A changed or newer
  candidate is considered independently.
- The interface can compare two distinct states only when both belong to the
  same resolved non-unassigned branch.
- Future pruning, retention, or down-sampling requires a separate explicit
  player-facing policy; this decision does not silently discard evidence.
