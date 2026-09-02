# ADR-0029: Automate external changes only after informed consent

## Status

Accepted.

## Context

Manual native-tool setup is confusing and easy to perform incorrectly. Safe
automation can remove that burden, but Observatory must not silently change the
game, its installation, a running process, Workshop content, or save data.

## Decision

Observatory automates a supported external change only when all of these rules
are met:

1. The interface first names the exact target, the intended changes, and what
   remains untouched.
2. The player confirms that specific class of change immediately before it
   begins. An earlier notice or a different confirmation cannot be reused.
3. The native command repeats the consent requirement through a typed argument.
   Calling the command without it fails before any write or launch.
4. The operation is allowlisted, bounded, checked after completion, and reports
   progress in the foreground. Failure leaves earlier valid data intact.
5. A game-directory write and a running-memory change are different consent
   thresholds. Preparing a tool never implies permission to launch it.
6. Observatory-owned files use a dedicated marked directory. The app will not
   replace an unmarked directory or another mod installation.
7. Save-data modification is outside the current product boundary. A future
   proposal needs its own architecture decision, recovery design, functional
   review, and unmistakable per-operation consent.

The checked Tesmio flow applies the rule as follows:

- source download stores reviewed source in private application data after a
  network confirmation;
- session preparation builds locally and writes only
  `W&R/tesmioloader/observatory` after a game-directory confirmation;
- launch starts W&R through that checked folder only after a separate
  running-memory confirmation;
- the Observatory probe requests no game-state or save-data writes.

## Consequences

The player receives a guided path instead of a manual installation recipe.
Every meaningful threshold remains visible and revocable by declining the next
action. The extra confirmations are intentional because the actions have
different effects and recovery boundaries.

This rule applies to future automation across the application. Ordinary
app-local preferences and derived caches retain their existing scoped controls;
they do not grant authority over external game or save data.
