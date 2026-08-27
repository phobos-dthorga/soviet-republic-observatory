# ADR-0010: Native recorder and durable candidate ledger

- Status: accepted
- Date: 2026-08-27

## Context

The first automatic observer was deliberately small: a Svelte heartbeat called
a deterministic Rust state machine every 1.5 seconds. That proved stable-file
gating, retry, ordered candidates, and branch-aware persistence, but it coupled
recording liveness to the webview event loop. A busy or suspended interface
could therefore delay observation, and in-flight candidate state disappeared on
restart even though completed observations were durable.

“Real-time” also needs a precise product meaning. Supported telemetry arrives
only when the game finishes writing a save archive. Reading game memory,
injecting code, or triggering game saves is a different research and security
boundary.

## Decision

The desktop host owns one native recorder thread for the application lifetime.
It uses `notify` to receive non-recursive operating-system events for the
configured save directory. Events are wake-up hints, not truth: a full directory
reconciliation runs at least every 15 seconds, and the deterministic stable-file
state machine remains authoritative.

SQLite migration 4 adds a bounded candidate ledger, and migration 5 retains the
initial-baseline state for each privacy-preserving directory identity. Candidate identity is the
privacy-preserving directory identity, file name, size, and modification time.
The ledger separates:

- discovery;
- stabilisation;
- readiness and reading;
- imported and duplicate outcomes;
- retryable and terminal failures; and
- superseded file identities.

Discovery source, timestamps, attempt count, bounded error code, payload identity,
and processing latency are retained. A restart returns interrupted stabilising,
ready, or reading candidates to `discovered` with an `interrupted` diagnostic.
Completed states remain idempotent. The first scan of a directory baselines older
files and considers only the newest candidate, preserving the established
automatic-observation contract.

The service emits versioned application-owned updates to Svelte. Presentation
subscribes to those events and may query a health snapshot, but it does not drive
the recorder. Full configured paths remain private settings and do not enter the
ledger projection, charts, or extension data.

## Benchmark evidence

The ignored repeatable ledger benchmark records and completes 1,000 synthetic
candidate lifecycles. In the 2026-08-27 unoptimised development run it produced
a 622,592-byte SQLite database after a WAL checkpoint in 7.37 seconds. This is a
regression bound for deliberately connection-heavy test code, not a claim about
production event latency. The benchmark fails above 8 MiB.

## Consequences

- Recording continues across workspace changes and ordinary webview stalls while
  the desktop application is open.
- Lost or coalesced filesystem events delay observation by at most the
  reconciliation interval under normal operation.
- Candidate and failure evidence survives application restart without storing
  raw archives.
- The Monitor workspace can distinguish queue state, terminal failure, and the
  absence of a newly saved game state.
- `notify` becomes a narrow replaceable dependency; correctness never depends on
  an event being delivered.
- This is not an operating-system service and does not run after the desktop
  application exits.
- This does not provide frame-live telemetry, read process memory, inject into
  the game, or create saves.
