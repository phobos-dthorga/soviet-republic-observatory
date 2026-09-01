# ADR-0024: offer bounded actionable recovery by default

## Status

Accepted.

## Context

An error message that already knows one safe remediation should not require a
player to translate an internal code into manual maintenance. Conversely, a
generic **Fix** button can conceal destructive work, guess at an ambiguous
cause, or turn presentation code into a second lifecycle authority.

Republic Observatory already owns retry, fallback, migration, projection, and
last-known-good behaviours in native services. Those behaviours need one
consistent player-facing contract.

## Decision

Actionable recovery is the default for an application state when the host can
name exactly one remediation that is:

- bounded and allowlisted;
- deterministic and idempotent or safely repeatable;
- local to Observatory-owned state;
- non-destructive to observations, lineage, revisions, and save archives; and
- accurately explainable before execution.

The ordinary notification states what stopped and exposes **Review recovery**.
A first-class recovery dialog then names the proposed action, what it changes,
what remains unchanged, and offers an explicit action and cancellation. The
dialog owns busy and failure presentation. The native host remains the sole
lifecycle and storage authority; the renderer may invoke only a typed,
purpose-specific command through `desktopClient.ts`.

Errors remain available in Diagnostics. Dismissing a proposal changes no data,
and an unsuccessful recovery returns to a safe inspectable state.

No one-click recovery is offered when the remedy is destructive, ambiguous,
requires credentials or elevated permissions, changes an external
installation, downloads an update, chooses among conflicting histories, or
could hide low disk space or filesystem damage. Those cases receive precise
guidance and, where useful, a navigation action rather than guessed repair.

## Initial application and audit

Markets indexing is the first complete use. A temporary storage lock can be
retried. A known app-local schema or projection contract can be verified,
derived warehouse work can be requeued, and indexing can then resume. The
operation does not rewrite existing observations or access game-save contents
beyond the already player-invoked exact-save indexing task.

New and existing failure states must be assessed against this decision. The
current recovery candidates are:

| State                                          | Default proposal                                                      |
| ---------------------------------------------- | --------------------------------------------------------------------- |
| SQLite critical task busy                      | Retry the same bounded action                                         |
| DuckDB projection failed or stale              | Requeue/rebuild derived projections                                   |
| Definition catalogue refresh failed safely     | Retain the published generation and retry refresh                     |
| Compatibility override became invalid          | Retain/reload the last valid profile, with local edits left untouched |
| Selected theme or language revision is invalid | Select the safe built-in fallback without deleting the local revision |
| Optional research prerequisites changed        | Re-run bounded prerequisite inspection                                |

Storage unavailability, permissions, disk exhaustion, corrupted evidence,
external application updates, and any operation that would discard data are
explicit non-candidates until a narrower safe contract is proven.

## Consequences

Failures with known remedies become understandable and recoverable without
technical maintenance steps. The reusable surface also prevents each workspace
from inventing different confirmation language, focus behaviour, and error
handling. Each new recovery still requires a bounded native operation and
tests; the presence of a generic dialog does not authorise a generic command,
arbitrary script, SQL, filesystem access, or silent retry loop.
