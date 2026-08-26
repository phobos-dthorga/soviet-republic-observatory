# ADR-0004: Branch-aware observation timeline

Status: Accepted

## Context

Players can reload older saves, save under new names, maintain alternatives, or
produce multiple archives with identical statistical content. Sorting archives
by filename or modification time can splice mutually exclusive histories and
duplicate observations.

Observed global record history is append-like across saves and can help
establish ancestry. Current and city blocks add save-sampled state.

## Decision

- Hash the supported statistical payload and deduplicate identical content.
- Retain archive observation metadata separately from the distinct statistical
  observation.
- Compare record identities or validated prefixes to establish ancestry.
- Keep divergent successors as separate named branches.
- Ask the player only when evidence cannot resolve an ambiguity safely.
- Scope streaks, records, forecasts, interventions, and plans to one branch by
  default.
- Plot every record and snapshot on its actual game date.

## Consequences

The data model is more explicit than a flat time-series table, but historical
analysis remains truthful after rollbacks. Identical autosaves do not inflate
sample counts, and the archive can show both “files observed” and “distinct
states retained.”
