# ADR-0018: evidence-gate individual citizen histories

- Status: accepted
- Date: 2026-08-31

## Context

W&R visibly preserves meaningful citizen relationships and life states in some
gameplay situations. Citizen histories and family trajectories could support
powerful descriptive cohort analysis and an unusually engaging administrative
experience. A false cross-save identity, however, would manufacture people,
relationships, and events that the save did not prove.

Research found a regular candidate worker table and name-token fields, but row
positions reorder and visible names are not unique identities. No persistent
citizen key or reviewed family/status field contract has been established.

## Decision

Individual history is denied by default behind a stable-identity evidence gate.
Array position, visible name, token tuple, and unverified composite fingerprints
cannot identify a person. Each decoded field and event family requires its own
reviewed compatibility evidence and unavailable path.

The first Population slice uses only existing branch-aware republic and numeric
city snapshot facts. Rust and SQLite own the query, exact analytical-head
boundary, evidence, and limits. TypeScript presentation adapters map the host
model into charts. Svelte selects a city source and renders the result; it does
not infer rates, totals, event windows, identity, or causality.

Future detailed tracking is opt-in and local. Exact facts, derived events,
descriptive cohort comparisons, and narrative summaries remain distinct public
types. Narrative output cannot fill an absent fact. Cohort differences are
descriptive unless a separately reviewed design justifies stronger language.

## Consequences

- The application may honestly provide aggregate Population analysis now.
- Citizen Lives remains visibly unavailable rather than synthetically populated.
- DuckDB failure cannot block SQLite-backed Population evidence.
- Detailed storage and fictional names are not collected before the player has
  a meaningful consent, selection, retention, and removal model.
- Reverse-engineering progress can activate one field family at a time without
  broadening the parser boundary or invalidating older evidence.
- A future failure to find stable identity ends in an aggregate-only design,
  not an approximate identity matcher.
