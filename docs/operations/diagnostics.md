# Local diagnostics and long-running work

Republic Observatory follows the useful local-diagnostics pattern established
by WyrmGrid: the player can see a small, structured record of important
operations without enabling telemetry or locating a developer console.

## Progress and stall visibility

Catalogue refresh is reported as a sequence of application-owned phases:

1. source discovery;
2. definition scanning and classification;
3. batched warehouse publication;
4. generation finalisation; and
5. completion or failure.

The Materials workspace shows the trigger, elapsed time, current source,
source and file counts, unchanged revisions reused, changed files parsed,
entities prepared, and warehouse rows staged. Discovery is indeterminate until
the file count is known. Later phases report a bounded overall percentage. A
global Catalogue indicator links to this detail while work is active.

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
