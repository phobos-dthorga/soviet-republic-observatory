# ADR-0017: exact analytical heads and evidence-backed continuations

## Status

Accepted and implemented.

## Decision

An analytical context is not synonymous with the newest imported file. SQLite
owns an explicit selected branch and exact head interpretation. Every bounded
dataset returned to charts, Analysis Packs, summaries, and bulletins is loaded
from that immutable head.

Branch membership is many-to-many. An observed interpretation retains its
original automatically resolved branch, while a player-created continuation
may reuse the same interpretation as an anchor and later include only saves
whose complete supported history proves a strict prefix descent from its
current head. Membership revisions and branch-specific parent evidence are
append-only. Labels are editable presentation claims; branch identities and
evidence are not.

**Inspect this save** selects an exact historical head without creating a fork.
**Return to latest** resolves the proven membership tip of the selected branch.
**Continue from this save** creates a durable branch immediately and never
deletes or rewrites the abandoned future. Multiple continuations may share one
anchor.

Dates, filenames, modification times, and import order may be displayed or used
in a default label. They never establish ancestry or advance a continuation.
Cross-branch comparison remains unavailable. Same-branch comparison is ordered
by membership evidence, although the resulting game-date delta may legitimately
be negative.

SQLite queues content-derived branch-membership projections through the same
crash-recoverable outbox as observations. DuckDB retains immutable membership
generations and exposes only the latest revision through a view. Warehouse
failure may delay analytical projection but cannot block recording, Archive,
historical inspection, or continuation creation.

## Context identity

`AnalysisContext` contains the selected branch, exact head, original branch,
latest/preview mode, automatic/manual origin, tip state, membership revision,
compatibility identity, observation watermark, and the available catalogue and
overlay watermarks. Its stable identity is derived from those branch/head
selection inputs. Recorder results separately report the newly recorded
interpretation and the still-active context identity so an unrelated save
cannot silently replace the player’s view.

## Consequences

- Regressing to an older save is reversible and evidence-honest.
- An observation can belong to an installed-game lineage and one or more
  player continuations without duplicating its facts.
- Historical charts cannot accidentally include later observations.
- DuckDB gains a projection structure, not operational ownership.
- A future alternate-futures comparison must explicitly model a shared
  divergence point; it cannot reuse the ordinary same-branch comparator.
