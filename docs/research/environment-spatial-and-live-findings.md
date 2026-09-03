# Environment spatial and live-source findings

## Current result

No spatial pollution decoder or ordinary live facility reading is published
yet. Probe contract 4 now provides a separate comparison study for candidate
facility fields on the exact reviewed W&R 1.1.1.9 build.

Observed `pollution.bin` files have a structure consistent with a small header
and repeated fixed-width records, but sample size and naive numeric decoding do
not establish dimensions, coordinate mapping, field meaning, or agreement with
the in-game pollution view. Observatory therefore records this as a negative
finding rather than displaying an attractive but unsupported map.

The candidate reader follows the reviewed main building collection. It copies
at most 128 facilities or one millisecond of work per rendered frame into a
bounded buffer. It currently exposes only candidate production, residential
pollution exposure, and water or sewage storage values whose structure can be
derived from the reviewed upstream findings. Position and radiation remain
absent because their fields have not been proven. None of these candidates are
included in ordinary Environment totals or histories.

The guided **Compare a live reading with W&R** study stores the candidate value,
the value entered by the player, the test kind, build identity, probe version,
and result. It supports positive, zero, disconnected, stability, save/reload,
and restart checks. Candidate indices last only for one snapshot. They are not
durable facility identities.

## Admission tests

A future spatial decoder must prove dimensions, coordinate mapping, bounds,
version identity, and agreement with controlled changes visible in the game.
A future live facility contract must also prove:

- a stable build-specific derivation without hard-coded process pointers;
- complete-snapshot rejection when the world changes during capture;
- bounded facility count, report size, and per-frame work;
- field ranges and agreement with controlled game-interface readings;
- snapshot-local facility identity; and
- no game, save, or simulation writes.

Until promotion, Environment explains why the corresponding sections are
unavailable and links to this study. It does not offer a recording switch that
cannot produce reviewed readings. A previously remembered consent can be turned
off, but it cannot create evidence. No nearest-date reconciliation with save
history is allowed.
