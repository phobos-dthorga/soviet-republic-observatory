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
degradation, warehouse projection start/completion/failure, and catalogue
refresh start, scan completion, success, and failure. Projection diagnostics
record only the job kind, stable outcome code, and elapsed time; content
identities and database paths remain private. Compatibility adds controlled
codes for valid/invalid local profiles, exact mod-scope conflicts, and tracked
unreviewed definition updates; it never logs package contents, paths, or profile
JSON. Future background services should use the same boundary and add only
stable, low-cardinality events that answer a concrete support question.

The Experimental Research Setup records selection validation plus build start,
safe failure, and verified completion. Failure entries identify the stopped
stage, a stable remediation code, an available compiler exit code, and at most
two bounded lines of already sanitised compiler output. The setup dialog shows
the same stage and next action and links directly to **Diagnostics**. Checkout
and build-output paths are never placed in the public status model or diagnostic
record; the interface uses only a folder leaf name and repository-relative
artifact label.

Reviewed-source acquisition uses the same foreground task contract. Its
confirmation closes before work begins, so connection, transfer, archive
inspection, local storage, and header verification remain visible in the
Research dialog. The actual upstream ZIP may contain larger unrelated assets;
whole-archive limits still apply, while the stricter retained-file limit applies
only to the two reviewed headers and licence that Observatory keeps.

## Performance boundary

DuckDB is a bulk analytical engine. Catalogue publication and observation
projection therefore stage rows with DuckDB appenders and perform set-oriented
insertion. A query or prepared statement per entity, observation, or metric is
prohibited: it creates long low-CPU runs, excessive memory growth, and weak
progress evidence. Publication remains one transaction, so interruption
preserves the previously active generation. Status snapshots use non-blocking
connection access, keeping setup and progress indicators responsive while a
writer owns the warehouse.

The non-blocking warehouse health snapshot exposes only the controlled write
kind, phase, and aggregate row progress. Completion and failure diagnostics add
elapsed time, the failure code, and bounded retry delay. Success closes the
failure circuit. Consecutive failures delay the next projector claim
exponentially up to 30 seconds, preventing a damaged or unavailable warehouse
from producing a hot failure loop. The diagnostic record does not include
projection identities, definition contents, save values, SQL, or paths.

## Resumable maintenance and cache evidence

Long-running Markets and warehouse work is coordinated behind save recording.
The per-connection SQLite wait remains a short host-owned safety interval;
Settings controls only the total patience budget for resumable background
retries. A patience expiry is recorded as a pause, not a failed interpretation.
The durable job can resume from its first unfinished archive.

Broadcast and Environment indexing use the same recorder-first checkpoints.
Environment progress reports archives, preserved history records, source rows,
cache reuse, contention, and resume count. It never reports resource values,
factor contents, save paths, or live facility positions. Live environmental
recording status reports only consent state, the checked-contract state,
snapshot identity, game date, and bounded facility count.

The Settings maintenance assay and controlled diagnostic events report shared
market records, shared fact rows, interpretation memberships, cache records and
rows reused, contention duration, retry count, pause/resume state, and the
application-owned task class holding the coordinator. These are aggregate
operational measurements. They never include archive names, paths, raw source
fields, save contents, SQL, or database locations.

**Refresh changed data** performs the ordinary content-addressed validation
pass. File size and modification time may avoid unnecessary archive work only
as hints; raw `stats.ini` hashing decides identity, and access time is never
used. **Rebuild analytical warehouse** is reserved for explicit recovery. Its
diagnostic event records that a rebuild was queued, while SQLite evidence and
save observation remain available.

Settings also provides a separate last-resort **Erase Observatory databases**
action. It requires an exact typed phrase and restarts before deleting an exact
allowlist of app-local SQLite and DuckDB files. It never accepts a path, scans a
directory, or touches configured game, save, and Workshop folders. The action
removes settings and recorded Observatory history as well as derived data, so
it is not a routine rebuild or preference reset.
