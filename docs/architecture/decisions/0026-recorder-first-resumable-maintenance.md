# ADR-0026: coordinate recorder-first resumable maintenance

## Status

Accepted.

## Context

Market reinterpretation and analytical projection can revisit many retained
save archives and write large amounts of derived evidence. A fixed SQLite busy
timeout alone cannot distinguish an ordinary short collision from a background
task that should yield to a newly recorded save. Repeating completed archive
work also wastes storage bandwidth and extends the contention window.

## Decision

One Rust-owned bulk-work coordinator admits critical SQLite work. Save recording
has unconditional first priority. Background indexers and warehouse projection
acknowledgements acquire bounded leases, retry short storage attempts within the
selected patience budget, and yield at archive or storage checkpoints according
to the selected background-work priority. An active transaction is never
interrupted.

When a Markets indexing patience budget expires, the durable job remains
resumable and is presented as **Paused — storage occupied**. Completed archive
checkpoints and immutable interpretations remain complete. Restart recovery
loads the latest durable job and resumes from the first unfinished archive.

Market persistence is content-addressed by the raw `stats.ini` hash, parser
engine, resolved compatibility profile, and a host-owned storage-contract
version. Shared record rows are written once; subsequent interpretations and
branch memberships reuse them. File size and modification time are discovery
hints only. The bounded raw payload hash remains authoritative, and access time
is ignored.

The ordinary maintenance action revalidates changed data and uses an exact
raw-payload cache hit to avoid parsing and insertion. A full warehouse rebuild
is separate, explicit, confirmed, and rebuilds only derived DuckDB state.
DuckDB stores shared historical market records once and joins them to bounded
interpretation memberships; exact save snapshots remain interpretation-owned.

Ordinary desktop launches use Tauri's single-instance boundary so a second
process cannot become a competing owner of the same app-local stores. Marked,
isolated UI-review roots remain exempt and never use ordinary application data.

## Consequences

Playing while Markets indexing is active may make indexing pause, but it does
not delay save recording or discard completed work. Cache reuse reduces both
row insertion and future warehouse projection. Parser, compatibility-profile,
or storage-contract changes deliberately create new interpretations instead of
silently reusing incompatible evidence.

GPU acceleration is not part of this contract. The observed bottleneck is
storage coordination and repeated database work rather than a parallel numeric
kernel; the decision can be revisited only with profiling evidence of a
substantial compute-bound workload.
