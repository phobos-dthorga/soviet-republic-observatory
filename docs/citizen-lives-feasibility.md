# Citizen Lives and Family Trajectories feasibility

## Decision

Individual Citizen Lives histories are **not supportable yet**. Republic
Observatory must not identify a citizen by array position, displayed name, or an
unverified composite fingerprint. The implemented first vertical slice is
therefore a branch-aware aggregate Population laboratory over already supported
`$STAT_CURRENT` and `$STAT_CITY` facts.

This is an evidence-quality decision, not a permanent rejection of the idea.
The save clearly contains more citizen structure than the public interface
currently uses. Stable identity and field meaning must be proved before that
structure becomes a biography.

## Research scope and evidence quality

The investigation used representative local saves from one observed republic
and compared adjacent and more widely separated states. The files were read in
place and were not copied into the repository. The motivating screenshot was
treated only as a player-observed example of game behaviour; it was not treated
as machine-readable evidence.

| Question                                                | Result                                                                                                                                                                                                                                            | Confidence and consequence                                                                                                                                                                                 |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Is there a citizen-scale binary payload?                | `workers.bin` begins with a plausible little-endian record count and contains a large region consistent with candidate 1,832-byte records.                                                                                                        | Strong research lead, not a reviewed format contract. Small trailing regions and the meaning of most offsets remain unresolved.                                                                            |
| Are records aligned between immediately adjacent saves? | Almost all candidate fields remain aligned across a short adjacent-save interval.                                                                                                                                                                 | Useful for controlled reverse engineering only. It does not prove persistent identity.                                                                                                                     |
| Is array position a stable citizen identity?            | No. Longer comparisons show substantial row reordering.                                                                                                                                                                                           | Row index is forbidden as a public person key.                                                                                                                                                             |
| Can displayed names identify citizens?                  | The first candidate fields behave like name-token IDs, and local game vocabulary can decode them using the documented BTF container structure. Visible names and token tuples can repeat, and alternate tokens can produce the same visible name. | Names are presentation attributes, not unique identities. The open-source [BTFTool BTF reader](https://github.com/Nargon/BTFTool) is useful format evidence but does not establish worker-record identity. |
| Is there another stable key?                            | High-cardinality candidate tuples exist, but none has been established as a host-owned persistent citizen identifier.                                                                                                                             | A composite fingerprint would risk merging different citizens or splitting one citizen across saves. It is forbidden.                                                                                      |
| Are family links and life states decoded?               | No reviewed offsets or relationship-key semantics have been established.                                                                                                                                                                          | Parent/child, household, residence, orphanage, prison, sentence, workplace, education, partnership, migration, escape, and death events remain unavailable at individual level.                            |

These results are representative of the investigated saves and current game
compatibility profile. They are not a claim that every W&R build or modded save
uses an identical worker layout.

## Facts safe to expose now

The reviewed plain-text parser already stores 18 bounded republic facts and
five facts per numeric city source. Each value retains exact observation,
branch membership, game date, source field, source line, compatibility profile,
mapping classification, and coverage.

The aggregate Population workspace exposes:

- direct republic status counts across included save observations;
- direct education counts at the selected analytical head;
- direct birth, death, escape, and immigration counters across save samples;
- the five supported movement counters for one numeric city source at the exact
  head; and
- the Citizen Lives evidence gate and its unavailable states.

The source counters' accumulation window has not been validated. The interface
does not convert them into interval flows, rates, population replacement,
causal outcomes, or city rankings. City IDs remain neutral numeric source labels
until a supported name mapping exists. Categories are not assumed to be
mutually exclusive or to sum to total population.

The SQLite query is bounded to 256 republic observations and 512 city scopes.
It follows the selected `AnalysisContext`, so historical inspection excludes
later branch states and continuation forks preserve their own head. It does not
require DuckDB and remains available while analytical projection is lagging.
The special unassigned bucket can contain unrelated histories and therefore
exposes only its exact selected head, never a multi-save trend.

## Required contract before individual histories

A future citizen record may enter the public model only after fixtures prove a
stable identity across at least birth/first observation, ordinary movement,
work or education change, detention or orphanage where available, and removal
from the republic. A reviewed compatibility profile must bind every decoded
field to an allowlisted host fact.

The minimum public boundary is:

```text
CitizenIdentityEvidence
  compatibility profile + layout version
  stable source key and validation method
  first and last supporting observations

CitizenFact
  citizen identity + exact observation + branch
  host fact ID + typed value
  source entry + bounded offset/field + compatibility provenance

CitizenEvent
  citizen identity + earlier/later exact observations
  event family + before/after facts
  deterministic detector version + completeness

CohortComparison
  explicit inclusion rule + observation window
  aggregate result + uncertainty/completeness
  descriptive label; never causal by default

NarrativeSummary
  references facts/events/comparisons only
  never creates missing identity, relationship, or event evidence
```

Birth or first observation, family link, move, orphanage, prison, education,
work, unemployment, partnership, migration, escape, and death remain separate
event families. Proving one does not activate the others.

## Storage, scale, and privacy

Citizen-scale tracking can grow much faster than aggregate snapshots. The safe
default is no detailed indexing. A future implementation should be explicit
opt-in and offer bounded choices such as selected citizens, selected cohorts,
or a retention window before republic-wide tracking is considered.

SQLite should remain the transactional authority for exact decoded
observations, consent/settings, branch membership, and detector state. DuckDB
may receive idempotent bounded projections for cohort analysis. Neither engine
should store raw saves, complete binary entries, or personal filesystem paths.
Names belong to fictional game citizens but should still remain local, absent
from diagnostics by default, excluded from public fixtures, and removable
through a documented retention operation if detailed tracking is introduced.

Deduplication must use immutable interpretation and proven citizen identity,
not a name or approximate attribute match. Events are derived records and never
replace their supporting facts. Re-running a detector creates a versioned
result or idempotently reproduces the same event.

## Next research protocol

1. Create controlled private saves around one known change at a time.
2. Establish the worker-table envelope, trailer, primitive types, sentinels, and
   record-count limits for a pinned game build.
3. Search for a persistent source key and disprove collisions and reuse across
   removal/birth cases.
4. Validate candidate relationship keys symmetrically across parent and child.
5. Validate each requested fact independently against in-game inspection and
   repeat it across saves and restarts.
6. Build sanitised synthetic fixtures containing the binary shape without
   redistributing game or player data.
7. Publish only the smallest field family that passes compatibility, storage,
   privacy, and false-link tests.

If stable identity cannot be established, the research concludes with
aggregate/cohort designs based on source-provided aggregates. It must never
degrade into probabilistic biographies presented as fact.
