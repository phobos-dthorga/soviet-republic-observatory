# ADR-0001: Local-first, read-only save observation

Status: Accepted

## Context

The game does not provide a supported scripting surface for the required custom
graphs. Save archives contain useful statistical history and current snapshots,
but they are the player's primary recovery artefacts and may be written while
the game is active.

## Decision

- Observe only user-configured save directories.
- Wait for stable size and modification time before opening a candidate.
- Open ZIP archives and entries read-only; never extract beside, rename, replace,
  or delete a save.
- Store parsed application facts and content hashes locally in SQLite.
- Do not require an account, hosted service, game process injection, or memory
  inspection.
- Keep raw saves outside the application database and source repository.

## Consequences

The application may lag a new save briefly while it becomes stable, but cannot
corrupt it through ordinary operation. Local history and analysis remain useful
when the game and network are unavailable. Any future sharing feature is an
optional export or separate trust decision.
