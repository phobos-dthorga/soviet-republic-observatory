# ADR-0015: inert community themes with native validation

- Status: accepted
- Date: 2026-08-31
- Supersedes: no prior decision

## Context

Republic Observatory needs accessible first-party appearances and a safe route
for community themes. WyrmGrid demonstrated that semantic colour roles,
contrast gates, explicit lifecycle actions, and safe fallback can support
third-party themes without accepting arbitrary CSS. The Observatory also has a
stronger native ownership boundary: lifecycle and accessibility decisions must
not depend on frontend-only validation.

## Decision

Themes are strict `.rotheme.json` data contracts. Rust is the sole validator
and SQLite lifecycle authority. The host owns derived tokens, native control
scheme, charts, typography, layout, focus, accessibility, and failure recovery.
The frontend renders Rust's validation report and maps an accepted manifest to
host-owned semantic variables; it does not decide whether the theme is safe.

Import and activation remain separate. Revisions are immutable and selection
pins identity, version, and content hash. Built-ins and local themes pass the
same validator. A missing or invalid selected local revision falls back to the
Classic built-in without deleting evidence of the failed revision.

Every production build runs a browser contrast audit over all enabled
workspaces and representative component states under every built-in and a
generated boundary theme. Windows release validation separately inspects the
OS-owned open select popup.

## Consequences

- Community themes cannot execute code, access data, or reach renderers.
- A theme author receives exact measured ratios and remediation labels.
- Native and browser validation have different responsibilities: Rust admits
  theme role sets; the build audit detects component-level misuse and inherited
  opacity regressions.
- New semantic states require a reviewed host role and validator update rather
  than an arbitrary new theme key.
- The application may fall back visibly, but a bad theme can never make the
  interface unusable across restarts.
