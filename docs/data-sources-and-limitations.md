# Data sources and limitations

This document records the presently observed evidence boundary. It is not a
promise that every field remains stable across game versions. Parser support
must be fixture-tested and version-aware.

## Save-container observations

Cloud saves observed on 26–27 August 2026 were ordinary ZIP archives. A save can
be inspected without extraction by opening `stats.ini` directly from the
archive. Large binary entries were also present, including building, worker,
rail, and vehicle payloads.

The implemented manual and automatic observers follow this sequence:

1. notice a candidate ZIP;
2. wait until file size and modification time are stable;
3. validate the archive without modifying it;
4. hash the statistical payload and deduplicate it;
5. parse supported records into application-owned models;
6. store source identity, compacted history, bounded snapshot facts, parser
   version, coverage, source fields, source lines, file evidence, and lineage
   atomically in app-local SQLite;
7. resolve exact supported prefixes into a main timeline, successor, fork, or
   visibly unassigned state; and
8. return a bounded dataset for the selected branch to the presentation.

Automatic observation is opt-in and runs only while the desktop program is
open. It retains an initial baseline of existing files, considers the newest
candidate when enabled, queues every later candidate it notices, waits at least
1.5 seconds of unchanged size and modification metadata, and retries transient
incomplete archives up to five times. It is not an operating-system service and
does not backfill every save created while the program was closed.

Prefix ancestry currently uses only the supported receiver-history record
identity. It cannot claim ancestry from current-state, city, or binary fields,
so tied or unrelated evidence remains visibly `unassigned`.

The SQLite database is unencrypted and contains no credentials. Full configured
paths remain inside app-local settings and are never included in presentation or
extension models. Raw save archives remain outside the database.

Completed observations enqueue content-addressed analytical projection jobs in
the same SQLite transaction. App-local DuckDB receives those jobs later. A
warehouse outage delays model matrices but cannot invalidate or block an
observation. Both database files are unencrypted and must remain on local
storage, not a NAS or cloud-synchronised folder.

## `stats.ini` coverage

Observed global historical records contained fields suitable for:

- buy and sell prices in rubles and dollars, plus base prices;
- imports and exports by resource, currency, quantity, and value;
- construction, factory, shop, and vehicle resource use;
- factory, citizen, and demolition waste production;
- vehicle import and export values;
- a `Resources_Produced` collection;
- births, deaths, escapes, and immigration;
- tourism counts, spending, and scores;
- loan balance and interest; and
- several cost scalars.

Observed saves contained long histories: one review found 1,847
`$STAT_RECORD` blocks and the later conformance pass parsed 1,896 complete
receiver records. Adjacent record dates were usually five in-game days apart
but varied roughly between four and seven days. Charts therefore use numeric
game-day positions rather than record position or equally spaced category
labels.

The history appeared append-only across successive saves: a later save retained
the earlier record prefix and added new records. Two distinct save archives also
contained byte-identical `stats.ini` payloads, establishing the need for
content-based deduplication.

### Broadcast receiver fields

The reviewed game version 1.1.1.9 exposes four plain-text citizen receiver
fields. Their source spellings are retained exactly at the parser boundary:

- `$Citizens_EletronicNone`
- `$Citizens_EletrinicRadio`
- `$Citizens_EletronicTV`
- `$Citizens_EletronicComputer`

The inconsistent source spelling is compatibility evidence, not a public API.
It maps to the stable `core.citizens.electronics.*` identifiers documented in
the metric contract. A receiver share requires all inputs to have the same
branch, observation date, and geographic scope.

`$STAT_CURRENT` and `$STAT_CITY` begin after global history. Their date fields
may be zero-valued and must not be allowed to overwrite the last historical
record. The receiver parser closes the history section explicitly at either
marker; a fixture protects this boundary.

## Current and city snapshots

`$STAT_CURRENT` and `$STAT_CITY` blocks describe the latest state rather than a
complete embedded history. The observer now persists bounded scalar facts from
both block types for every distinct save. Republic coverage includes receiver
classes and selected citizen counts. City coverage presently includes births,
deaths, escapes, and the two immigration counts under the numeric city source
identifier.

The republic snapshot recognises the four receiver spellings above plus:

- `$Citizens_Born`, `$Citizens_Dead`, and `$Citizens_Escaped`;
- `$Citizens_ImigrantSoviet` and `$Citizens_ImigrantAfrica`;
- `$Citizens_SmallChilds`, `$Citizens_MediumChilds`,
  `$Citizens_AdultsParent`, and `$Citizens_Adults`;
- `$Citizens_Unemployed`;
- `$Citizens_NoEducation`, `$Citizens_BasicEducationNum`, and
  `$Citizens_HighEducationNum`; and
- `$Citizens_CarOwners`.

City snapshots currently recognise only the five birth, death, escape, and
immigration fields. Repeated `$Citizens_Status` fields and other list-valued
directives remain unsupported until their ordering and semantics are fixture-
tested independently.

The zero-valued date fields found inside these blocks are not treated as the
observation date. Snapshot scopes inherit the latest supported historical game
date from the same save. Repeated observations can therefore support future
save-sampled national and city trends, comparisons, and intervention studies.
The captured `source.stats.*` identifiers remain internal source facts rather
than published Analysis Pack metrics until their meaning and time window are
validated.

City identifiers are not assumed to be display names. Until names and
coordinates are established by a supported source, the interface must use a
stable neutral label and explain the limitation.

## Game definitions

The implemented catalogue indexes locally available base-game, DLC, subscribed
Workshop, and WIP building and vehicle definitions. It retains source-qualified
identities, content-addressed generations, typed properties, repeatable
relations, line evidence, and unknown-directive diagnostics in DuckDB. Installed
game definitions can provide resource catalogues and production
recipes independently of a republic save. These are game-definition facts, not
observed republic activity. They can support theoretical material requirements,
limiting-input calculations, and scenario models, but cannot establish that a
specific building operated at that theoretical rate.

Definitions should be imported into versioned application-owned models. No game
assets are copied or redistributed.

`$COST_RESOURCE` is an explicit construction quantity. `$COST_RESOURCE_AUTO` is
only an automatic-cost coefficient and remains visibly unresolved alongside
construction nodes, keywords, and phases until its conversion is verified.
Definition repair or maintenance values are unavailable unless an explicit
source directive or reviewed game rule establishes them.

Planning overlays preserve installed originals and add player-authored
`player_override` or `player_definition` evidence. A conflict falls back to the
installed original; it never silently rebases after a game or Workshop update.
The strict contract and lifecycle are documented in
[Definition Catalogue and Planning Warehouse](definition-catalogue-and-warehouse.md).

Installed-game translation files are a potential local display-vocabulary
source, not parser truth and not Observatory UI translations. The current host
catalogues the identities of matching local BTF files and reports that their
contents are unreadable; it does not decode, copy, or display their text. A
later versioned `GameVocabularyCatalogue` may resolve labels while retaining
exact source identifiers and reviewed Observatory fallbacks. No game
translation catalogue is committed or redistributed by this repository.
Changing display language cannot change observation identity, metric
references, joins, or calculations.

For version 1.1.1.9, reviewed station definitions provide nominal radio capacity
of 100 workers and 50 professors, and television capacity of 120 workers and 70
professors. These are game-definition facts, not evidence of staffing in a
particular republic.

## Known limitations

### Production coverage

`Resources_Produced` must not be treated as a complete record of all processed
factory output until coverage is verified resource by resource. A chart may
state “recorded production”; it may not silently relabel this as total domestic
production.

### Material conservation

The first parser does not establish all stock changes, production, loss, or
transfer paths. Therefore:

- sources and uses may be displayed side by side;
- an “accounted-flow” diagram may include an explicit unaccounted remainder;
- the remainder is a measurement residual, not waste or loss; and
- a closed mass balance or actual process yield is prohibited until inventory
  and production coverage are demonstrated.

### Save cadence

Global history is already embedded in a save, so saving more frequently does
not create finer historical records. Frequent observation is still useful for
current and city snapshots, branch detection, and before/after comparisons.
The automatic observer supplies that cadence only while the desktop program is
open.

The Monitor uses **near-live** narrowly: the native recorder reacts after W&R
finishes writing a stable save. Operating-system folder events reduce discovery
latency, while a full scan every 15 seconds catches missed or coalesced events.
Neither mechanism can observe a change the game has not written to a save. A
long game-date interval therefore does not prove recorder failure; the game may
simply not have produced another save during that interval.

Candidate lifecycle evidence is retained in SQLite independently of completed
observations. It includes bounded file identity, discovery source, transition
timestamps, attempt count, outcome, diagnostic code, and payload identity. It
does not contain the archive, a full configured path, game memory, or credentials.
Interrupted reads resume from discovery after restart, and terminal failures do
not remove or block earlier observations.

### Binary payloads

Individual factories, routes, vehicles, worker histories, inventories, network
topology, and geographic mapping are later research areas. The presence of a
binary file does not establish a stable or licensed public format.

Broadcast station identity, intended influence, potential reach, current
listeners or viewers, rating, recording budget, and actual staffing remain
binary-research candidates. Synthetic interface values must never be presented
as decoded telemetry.

### Extension data boundary

Analysis Packs reference published normalised metrics and never read saves.
Future executable Model Plugins receive only bounded normalised observations
and versioned game-definition models. Raw archives, binary payloads, SQLite,
parser maps, and paths remain host-private even when a player grants future
extension capabilities.

Analysis Pack prose is also a separate evidence surface. New pack v1 files
declare `default_locale`; older v1 files default to `en-AU`. The host tags
author-owned names, descriptions, metric labels, and chart prose with that
locale. Observatory language packs cannot rewrite an extension author's
analytical claim or make it appear reviewed by the host.

## Data-quality presentation

Every analytical result carries:

- source kind and source identifier;
- observation or effective game date;
- parser and calculation version;
- coverage status: complete, partial, experimental, or unavailable;
- units, currency, denominator, and time window; and
- material caveats close to the visual.

Missing values remain missing. Zero is used only when the source explicitly
reports zero.
