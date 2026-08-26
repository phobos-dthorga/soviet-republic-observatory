# ADR-0007: stream receiver history read-only before adding a watcher

- Status: accepted
- Date: 2026-08-27

## Context

The synthetic Broadcast Desk needed one complete path from a player-owned save
to an evidence-bearing chart. Implementing a background watcher, general save
parser, branch resolver, and broad analytics at once would make failures hard to
attribute and would enlarge the private-data boundary before its behaviour was
proven.

Reviewed saves expose four receiver classes in the plain-text global history of
`stats.ini`. The file is inside a large ZIP and can be read without extracting
the archive. History ends at `$STAT_CURRENT` or `$STAT_CITY`; treating those
blocks as another historical record can silently corrupt the last real date.

## Decision

The first native slice is a player-initiated manual observation:

1. the player selects a save directory through the native dialog;
2. the Rust host chooses its newest ZIP candidate;
3. archive and `stats.ini` sizes are bounded;
4. file metadata is compared before and after inspection;
5. `stats.ini` is streamed directly from the archive and history closes at the
   first current/city marker;
6. the statistical payload is hashed and duplicate content is not reinserted;
7. complete receiver records are normalised into stable metric identifiers and
   stored with source fields, source lines, parser/profile versions, scope,
   coverage, and a neutral branch placeholder; and
8. Svelte receives only the bounded dataset and directory basenames, never raw
   archive access, SQLite access, or full configured paths.

Only the Receiver Ladder becomes observed. Other Broadcast values remain
synthetic until independently supported. The installed-game folder is treated
as a separate vocabulary-source boundary; the current slice catalogues BTF file
identities but does not decode or redistribute their contents.

## Consequences

- A real save can prove parser, storage, provenance, chart, and unavailable-data
  behaviour without a continuously running scanner.
- Save archives remain untouched and unextracted.
- Content identity prevents repeat insertion of the same statistical history.
- Actual game-day positions preserve irregular history spacing and gaps.
- Rollbacks and forks are not yet resolved; records remain on `unassigned` and
  must not be spliced into a claimed continuous branch.
- A save that changes during reading is rejected. Waiting and retrying belongs
  to the watcher slice.
- The parser is deliberately narrow. Unsupported receiver profiles fail or
  report partial coverage rather than guessing.

## Verification

Sanitised fixtures cover complete, partial, malformed, duplicate, unsupported,
missing-payload, and history-boundary cases. A local opt-in conformance test can
inspect a player-specified archive through the same reader without embedding a
path, save, or republic value in source control. SQLite and chart tests verify
normalisation, deduplication, fixed domains, gaps, per-series provenance,
negative values, and actual-date positioning.
