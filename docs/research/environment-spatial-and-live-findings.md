# Environment spatial and live-source findings

## Current result

No spatial pollution decoder or live facility contract is published yet.

Observed `pollution.bin` files have a structure consistent with a small header
and repeated fixed-width records, but sample size and naive numeric decoding do
not establish dimensions, coordinate mapping, field meaning, or agreement with
the in-game pollution view. Observatory therefore records this as a negative
finding rather than displaying an attractive but unsupported map.

Installed interfaces name candidate building readings for pollution,
radioactivity, water, and sewage. Names alone are not evidence of runtime layout,
physical units, or whether fields overlap. The current reviewed Tesmio companion
does not publish these readings.

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

Until then, Environment explains why the corresponding sections are unavailable
and links to this research status. It does not offer a recording switch that
cannot produce readings, and the native capture command fails explicitly rather
than reporting an empty success. A previously remembered consent can be turned
off, but it cannot create evidence. No nearest-date reconciliation with save
history is allowed.
