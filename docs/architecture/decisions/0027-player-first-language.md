# ADR-0027: Player-first language with optional technical wording

## Status

Accepted.

## Context

Republic Observatory preserves unusually exact save, game-file, timeline, and
calculation details. Earlier interface copy exposed the terminology used to
build those contracts directly to the player. Words such as “analytical head”,
“immutable revision”, and “denominator” were accurate, but they made ordinary
gameplay questions feel like database work.

The intended audience is W&R players across a wide age range, not data
scientists. Mod authors and troubleshooters still need the exact identifiers,
formulas, source spellings, and diagnostic codes.

## Decision

Built-in English uses player-friendly wording by default. It aims for a
15–16-year-old reading level, leads with the player's question or outcome, and
uses short sentences. Ministry-themed feature names remain, but instructions
must be direct.

An application preference selects either `player_friendly` or `technical`
wording. Technical English is a validated partial overlay on the canonical
catalogue. It changes wording only; it does not reveal extra actions or alter a
calculation. Community language packs use their own ordinary wording in both
modes so the app never mixes English technical phrases into another language.

Raw game and mod identifiers remain exact in labelled source details. Errors
first state what happened, what remained safe, and what the player can do.
Codes, operation names, and implementation messages belong in the shared
expandable technical-details surface and diagnostic export.

The build checks the canonical catalogue for discouraged specialist phrases,
implementation names, raw error codes, long sentences, Fluent-variable drift,
and readability. Exceptions name one exact message, explain why it is needed,
and expire. Browser and native review can switch wording modes while rendering
the same production components.

## Consequences

- Existing installations receive player-friendly wording automatically.
- Source IDs, evidence classes, values, calculations, and storage remain
  unchanged.
- Contributors must write the plain explanation first and add a technical
  override only when the formal term helps a specialist.
- Adding a message still advances only the additive catalogue revision unless
  compatibility or variables change.
