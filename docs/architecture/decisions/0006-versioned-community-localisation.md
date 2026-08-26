# ADR-0006: versioned community localisation before save parsing

- **Status:** accepted
- **Date:** 2026-08-27

## Context

OnAir WyrmGrid established a useful standard: canonical `en-AU`, Fluent
patterns in data-only JSON manifests, partial-catalogue fallback, typed keys,
strict validation, and local persistence. Republic Observatory needs the same
capability before real save fields, game-definition labels, error states, and
extension-authored prose multiply the number of accidental string boundaries.

Repeating WyrmGrid's incremental screen-by-screen migration would leave a
mixed interface and make every later parser feature pay conversion cost.
Observatory also has unusually sensitive labels: facts versus estimates,
coverage, causal claims, synthetic values, save safety, and extension
permissions must not be weakened by an unreviewed translation file.

## Decision

Adopt WyrmGrid's format and runtime principles with these changes:

1. Migrate the complete current interface and chart content at foundation time.
2. Keep one canonical `en-AU` JSON catalogue and Fluent formatting runtime.
3. Separate breaking `source_catalog_version` from additive
   `source_catalog_revision`.
4. Treat import, inspection, selection, and removal as distinct operations.
5. Protect evidence, coverage, causality, synthetic-data, save-safety, security,
   destructive-action, permission, and error namespaces from unreviewed packs.
6. Centralise locale-sensitive formatting and statically audit translation-key
   and formatting usage.
7. Exercise expanded and RTL pseudo catalogues from the first implementation.
8. Keep Observatory UI language, installed-game vocabulary, raw source IDs, and
   extension-authored text as separate domains.
9. Add a compatible Analysis Pack v1 `default_locale` field for author-owned
   prose, defaulting older v1 files to `en-AU`.
10. Make Rust authoritative for validation and persistence when the desktop host
    arrives; the current webview implementation proves the contract and UX.

Community files remain inert data and may be partial. Resolution order is the
selected pack, canonical English, then the explicit caller fallback. No
community file can supply executable code, renderer configuration, markup, or
ambient access.

## Consequences

- New interface work has one required text and formatting path rather than a
  later translation project.
- A partially translated language remains usable and reports honest coverage.
- Some safety and evidence wording remains English until a reviewed built-in
  translation is shipped.
- Changing UI language cannot alter parser identity, metrics, provenance, or
  game-source evidence.
- The current browser persistence is deliberately replaceable and is not a
  desktop trust boundary.
- Multilingual Analysis Packs require a future explicit package contract; host
  language packs cannot silently rewrite extension-authored analytical claims.

## Alternatives rejected

- **Translate after the save reader:** rejected because it creates mixed string
  ownership and makes parser error states expensive to repair.
- **Use JavaScript modules as translations:** rejected because language packs
  are untrusted data, not executable extensions.
- **Let every component format values directly:** rejected because locale,
  rounding, unit, and accessibility behaviour would drift.
- **Reuse game translation files as the UI catalogue:** rejected because game
  vocabulary, licensing, versioning, and Observatory product language have
  different owners and identities.
- **Allow community overrides of every sentence:** rejected because provenance
  and safety classifications are part of the application's trust boundary.
