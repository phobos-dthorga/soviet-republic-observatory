# ADR 0030: Environment evidence and player carbon accounting

## Status

Accepted.

## Decision

Environment is a top-level workspace between Population and Markets. It uses
three evidence lanes that are never silently combined:

1. exact activity and waste rows read from recorded `stats.ini` histories;
2. optional, checked live facility snapshots retained in Observatory's local
   data; and
3. player-owned carbon factors applied only to matching resource and activity
   identities.

The save parser preserves both numeric values from every environmental row,
its exact resource token, source field, line, record order, profile, and date.
Waste quantities are not published until controlled evidence proves what both
numbers and duplicate-looking rows mean. A malformed environmental block is a
partial Environment result and cannot invalidate other save evidence.

Live facility readings are fail-closed. The research companion may emit them
only after a build-specific contract proves facility identity, field layout,
ranges, and agreement with the game interface. Live indices are snapshot-local.
Pollution and radiation retain W&R-native unit labels. They are distributions,
not summed emissions, and they are never renamed carbon or sieverts.

Carbon factor sets are named, immutable revisions classified as player
settings. Each row targets an exact resource token and activity channel and
uses grams CO₂e per recorded W&R unit. The result is labelled “Estimated CO₂e
for covered activity” and always carries coverage. Waste factors remain
unavailable while waste quantity meaning is unresolved.

## Safety and privacy

Environment indexing uses the existing exact-save matching, content addressing,
pause/resume, and recorder-first coordination. It never writes a save. Live
recording is disabled by default, needs explicit consent, and can be withdrawn
immediately. The focused deletion command removes only app-local live
environmental sessions, snapshots, and facility rows. It preserves saves,
ordinary observations, installed definitions, and carbon factor revisions.

CSV import is previewed before it is applied. It rejects unknown channels,
duplicate identities, invalid or negative factors, unsafe formula prefixes,
oversized input, and malformed rows.

## Consequences

The first release is useful from save evidence even when the live facility and
spatial contracts remain unavailable. Adding a live field or `pollution.bin`
decoder requires controlled fixtures and a separate reviewed change. No built-in
emissions-factor library or network lookup is admitted by this decision.
