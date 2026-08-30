# Local diagnostics and long-running work

Republic Observatory follows the useful local-diagnostics pattern established
by WyrmGrid: the player can see a small, structured record of important
operations without enabling telemetry or locating a developer console.

## Critical-task progress contract

Work that can delay an otherwise usable workspace, rebuild durable derived
state, or run long enough to look stalled uses one application-owned progress
contract. The reusable presenter supports an indeterminate start, named stages,
an overall percentage, independently measured sub-tasks, a bounded current
item, aggregate counters, elapsed time, completion, warning, and failure. A
critical task must remain usable without opening the workspace that started it:
an active global indicator links to the detailed ledger.

Native producers retain their latest snapshot. The interface registers its
event listener before reading that snapshot and compares run start and update
timestamps before accepting either value. A late startup response therefore
cannot replace newer live progress, and a task that began before Svelte mounted
still becomes visible. Future save indexing, warehouse rebuild, import, and
model-run producers should use this same listener-first hand-off rather than
inventing workspace-local loaders.

Current-item evidence is optional and privacy bounded. It may contain a safe
source-relative file or application-owned item identity, never an absolute
path, definition contents, save contents, SQL, or a database location. Fast
native producers coalesce presentation updates to about ten per second while
retaining exact aggregate counters; this keeps the interface responsive without
turning “per-file progress” into thousands of queued webview events.

## Catalogue progress and stall visibility

Catalogue refresh is reported as a sequence of application-owned phases:

1. source discovery;
2. definition scanning and classification;
3. batched warehouse publication;
4. generation finalisation; and
5. completion or failure.

The Materials workspace is the first producer using the shared contract. It
shows the trigger, stage ledger, elapsed time, current source-relative file,
source and file counts, unchanged revisions reused, changed files parsed,
entities prepared, file and warehouse sub-progress, and warehouse rows staged.
Discovery is indeterminate until the file count is known. Later phases report a
bounded overall percentage. Traversal reports while a source is being walked,
not only after the source completes. A global Catalogue indicator links to this
detail while work is active.

No update for 15 seconds is labelled as a possible storage stall. This warning
does not invent a failure or cancel the transaction; it directs the player to
the diagnostic record. Concurrent manual and automatic refresh requests never
wait behind the active refresh. The active run remains authoritative and a
filesystem reconciliation can schedule the next necessary pass.

## Local diagnostic log

The desktop host retains at most 300 structured English entries in
`republic-observatory-diagnostics.jsonl` under the app-local data directory.
The **Diagnostics** control in the application header can read, refresh, and
clear it. The file is never uploaded, attached, or shared automatically.

Diagnostic wording deliberately remains controlled English rather than
community-localised source data. Stable operation and error codes make support
evidence comparable, while the surrounding interface explains the workflow in
the active language.

Allowed fields are:

- millisecond timestamp;
- severity;
- stable code;
- bounded operation name; and
- bounded application-owned message containing safe aggregate counts and
  elapsed time.

The writer rejects control characters and bounds every text field. Messages do
not contain save payloads, definition text, database or game paths, usernames,
machine names, Workshop contents, player annotations, or arbitrary extension
output. Even with this narrow vocabulary, players should review entries before
sharing them.

The first implemented producers are application startup, warehouse startup
degradation, and catalogue refresh start, scan completion, success, and
failure. Future background services should use the same boundary and add only
stable, low-cardinality events that answer a concrete support question.

## Performance boundary

DuckDB is a bulk analytical engine. Catalogue publication therefore stages
rows with DuckDB appenders and performs set-oriented insertion of previously
unseen revisions. A query and prepared statement per entity is prohibited: it
creates long low-CPU runs and weak progress evidence. Publication remains one
transaction, so interruption preserves the previously active generation.
