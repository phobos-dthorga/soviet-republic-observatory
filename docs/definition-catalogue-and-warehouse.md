# Definition Catalogue and Planning Warehouse

Republic Observatory uses two embedded databases because recorder state and
large planning queries have different failure and access patterns. Both files
are app-local, offline, unencrypted, and owned by the Rust application.

## Ownership

| Concern                                                       | Authority | Reason                                               |
| ------------------------------------------------------------- | --------- | ---------------------------------------------------- |
| settings, save ingestion, recorder ledger, branches           | SQLite    | small transactional changes and crash recovery       |
| overlay installation and active lifecycle                     | SQLite    | one operational authority for user intent            |
| Analysis Pack revisions and enablement                        | SQLite    | immutable local lifecycle and explicit activation    |
| projection outbox and rebuild requests                        | SQLite    | save commits never depend on analytical availability |
| catalogue generations and definition facts                    | DuckDB    | retained, column-oriented analytical history         |
| effective planning projections and large observation matrices | DuckDB    | filtered aggregates and model inputs                 |

JSON is the portable overlay interchange format, not a third database. No
presentation component, Analysis Pack, or future Model Plugin receives a
connection, SQL, table name, complete definition source, or database path.

## Projection protocol and recovery

Every SQLite outbox job has a content-derived identity, source identity, kind,
attempt count, and `pending`, `running`, `applied`, or `failed` state. The
projector claims one job, writes an immutable DuckDB partition in a transaction,
records a DuckDB receipt and watermark, then acknowledges SQLite. A restart
returns interrupted `running` jobs to `pending`.

If DuckDB commits and SQLite acknowledgement is interrupted, the next delivery
finds the receipt and acknowledges without inserting duplicate rows. If DuckDB
is unavailable, the job becomes a visible failure. SQLite save observation,
Archive, and core receiver charts continue to work. Rebuild clears only derived
observation projections and redelivers retained SQLite observations.

Observation matrices cross the boundary through a temporary DuckDB staging
table populated by one bulk appender, followed by one set-oriented idempotent
merge. Row-at-a-time metric inserts are prohibited. Presentation-facing health
and catalogue summaries never wait for the single writer connection: while a
projection owns it, the host returns the durable SQLite queue state and a
lagging warehouse snapshot so startup and the shared task indicator remain
responsive.

The application-wide warehouse write governor bounds each declared workload,
publishes active stage and row progress without taking the DuckDB connection,
and applies exponential backoff after consecutive failures. Catalogue,
observation, and planning-overlay facts all use appenders plus set-oriented
merges; only fixed metadata, receipts, publication pointers, and transactional
deletion use direct statements. Overlay conflict detection is one bulk merge,
not a query per operation. Work exceeding a class limit must be split into
immutable resumable partitions rather than truncated or admitted by casually
raising the limit.

Every model request must obtain one `WarehouseSnapshot`: catalogue generation,
compatibility profile ID/version/resolved hash and mapping classification,
active planning-overlay profile and revision, observation watermark, warehouse
schema, and projector version. A model may not combine rows from different
snapshots.

The bounded `ProductionRouteModel` v2 is the first presentation-facing model over
catalogue relationships. The Rust host accepts one current recipe identity, an
optional output resource, and an optional finite positive target. It returns at
most 63 production-input, waste-input, and production-output relations with
their exact quantities, basis, directive, line, mapping provenance, and pinned
snapshot. Presentation code receives neither SQL nor table identities.

The selected output establishes the primary route basis before geometry is
considered. Relations sharing that basis may enter the Sankey; relations using
a different or unavailable basis are classified as auxiliary requirements and
remain outside ribbon widths. A route can therefore be `ready_with_auxiliary`
without comparing electricity, material, time, or unknown coefficients as if
they shared one unit. Missing or invalid primary quantities, absent inputs or
outputs, no comparable input, duplicate primary endpoints, and excessive
relation counts remain explicit unavailable states.

One dimensionless recipe scale factor is applied to every well-formed recorded
coefficient, including auxiliary requirements, so the exact ledger remains
useful at the selected output target. This is proportional
definition-coefficient scaling, not unit conversion, rated capacity, observed
throughput, inventory movement, or a mass-balance claim. Every returned row
states whether it entered the primary geometry and why it was excluded.

The route coverage query reports catalogue-wide route, diagrammable,
auxiliary-requirement, unresolved-basis, and unquantified-relation counts from
the same host-owned rules. It is a bounded summary over the current generation,
not a promise that every supported directive has a verified physical unit.

## Catalogue generations

A refresh discovers the configured base game, `dlc*`/`elc*` packages,
`media_soviet/workshop_subscribed`, `workshop_wip`, and the Steam Workshop
content root derived from the app `784150` manifest. Source-qualified identities
remain distinct. The catalogue does not invent or apply undocumented mod
precedence.

Files are bounded, symlinks are skipped, paths are stored relative to a source,
and content hashes make unchanged entity revisions reusable. Publication is one
DuckDB transaction; malformed input leaves the previous generation active.
Rows are staged through DuckDB bulk appenders and unseen revisions are selected
as sets; a query or statement per entity is not an acceptable publication path.
Native file events are hints collected into five-second batches. Startup and
manual refresh perform a manifest/fingerprint reconciliation so missed events
do not become silent staleness.

The interface reports discovery, scanning, publication, and finalisation
through the application-wide critical-task progress contract. It exposes
aggregate counts, the current source, and a bounded source-relative current
file, never an absolute path or source contents. Traversal and parsing updates
are time-coalesced to protect the webview while exact totals continue to
advance. A listener-first durable-snapshot hand-off prevents startup races. A
15-second absence of progress is labelled as a possible storage stall and
linked conceptually to the local Diagnostics log; it is not silently treated as
success or failure. Concurrent refresh requests return the active status
instead of forming an invisible queue behind a long operation.

The parser retains typed fields, repeatable relations, units, bounded directive
arguments, line numbers, coverage, and unknown-directive counts. It does not
copy full definitions, absolute paths, assets, meshes, or binaries. Explicit
`$COST_RESOURCE` quantities remain separate from `$COST_RESOURCE_AUTO`
coefficients, construction nodes, keywords, and phases. Automatic coefficients
are unresolved rules until their game conversion is verified. Repair and
maintenance enter the catalogue only when an explicit directive or reviewed
rule exists.

Original source identifiers are immutable evidence. A source token such as
`resource::eletric` remains visible in the route ledger and is never silently
corrected in storage, joins, exports, or provenance. The interface may attach a
translated presentation alias such as “Electricity” to an exact reviewed token,
but that alias changes neither identity nor meaning. Unknown mod vocabulary
falls back to a bounded human-readable rendering of the original token. This
keeps misspellings and legacy identifiers available to mod authors while
allowing language packs to improve the player-facing label.

An optional compatibility mapping may target one exact `workshop.*` or `wip.*`
source. The warehouse records the mapping ID, scope ID, reviewed/player origin,
acknowledged definition hash, observed definition hash, update policy, and
scope state beside the affected facts. An `exact` hash conflict prevents the
new generation transaction from beginning; `track_updates` publishes with an
`updated_unreviewed` warning. Missing sources are dormant. Scope hashes cover
only supported definition files, so models and textures do not create false
mapping conflicts.

Historical generations remain authoritative for reproducing old model runs.
The current installed catalogue can be rebuilt; retained historical generations
cannot be reconstructed safely after mods or the game change.

## Planning overlays

`.rooverlay.json` uses strict Draft 2020-12 schema version 1. A document has a
reverse-domain ID, semantic version, author, locale, description, optional game
build, bounded operations, required reasons, revision/value preconditions, and
bounded supplemental entities. It cannot contain code, expressions, SQL, paths,
URLs, markup, renderer options, or executable callbacks.

`set`, `unset`, and repeatable-field `add` never rewrite installed facts. The UI
presents every affected field as `original → override → effective` and marks the
evidence `player_override`. Supplemental resources, buildings, vehicles, and
recipes are `player_definition`, not claims about installed assets.

Overlay schema v1 changes typed properties but does not change definition
relationships. Production-route results therefore pin and display the active
overlay revision for reproducibility while making clear that it did not alter
the recipe coefficients.

Inspect, validate, import, activate, update, roll back, deactivate, export, and
remove remain distinct operations. Named profiles have immutable revisions and
one profile may be globally active. A catalogue refresh checks preconditions
again. Missing entities, changed revisions, or changed values become conflicts;
the effective value falls back to the installed original until explicit rebase.

## Backups and compatibility

Back up SQLite and DuckDB together while Republic Observatory is closed. A lone
SQLite backup preserves operational truth and can rebuild current projections,
but cannot reproduce historical catalogue generations whose installed sources
no longer exist. A lone DuckDB file does not preserve settings, overlay or
Analysis Pack lifecycle, branches, or the projection outbox.

SQLite schema, DuckDB schema, projector, parser, overlay schema, application,
Analysis Pack, and future plugin protocol versions are separate compatibility
decisions. Both migration series are append-only. A warehouse created by a
newer unsupported schema is refused rather than guessed at.

The optional app-local compatibility file should be backed up with the two
databases when reproducible player mappings matter. A profile change schedules
a new catalogue generation; it never updates a retained generation in place.

The bundled DuckDB client is pinned. Extension autoload, automatic installation,
and external access are disabled on connection; the application does not use
DuckDB's SQLite extension, Parquet support, or network installation. Shared data
crosses the engine boundary only through versioned Rust models and prepared bulk
writes.

## Remaining research boundary

Actual in-republic building and vehicle instances, condition, wear, repair
state, location, and utilisation remain binary-research candidates. When a
supported decoder exists, those facts will enter SQLite transactionally and
project to DuckDB in batches. Definition capacity must never masquerade as an
observed instance state.

## Reference validation

Ignored reference-machine tests keep performance and growth claims explicit
without making them ordinary CI requirements. An optimised Windows run on
2026-08-27 recorded:

- 5,484 supported installed definition files and 6,010 catalogue entities in
  1.56 seconds;
- the same 5,484-file catalogue scanned and published as 150,642 staged rows to
  a new temporary DuckDB warehouse in 6.60 seconds after the batched-publication
  change;
- a 5,000-file Workshop batch with 100 changed definitions re-indexed in 1.41
  seconds; and
- 100,000 entities, 2,000,000 typed properties, and 5,000,000 observation rows
  plus 200,000 planning relations loaded in 21.29 seconds, producing a
  431,239,168-byte DuckDB file. The filtered observation, material-demand,
  production-chain, and fleet-capability queries completed in 1.63–34.61
  milliseconds.

These figures are regression evidence from one machine, not performance
guarantees. The checked-in ignored tests retain the stated 30-second full local
catalogue, 90-second synthetic load, five-second incremental update, and
500-millisecond representative-query targets.
