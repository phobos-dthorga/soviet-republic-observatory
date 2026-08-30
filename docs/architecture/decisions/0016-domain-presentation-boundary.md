# ADR-0016: enforce the domain and presentation boundary

- Status: accepted
- Date: 2026-08-31
- Supersedes: no prior decision

## Context

The Observatory now has parsing, persistence, lineage, extension validation,
analytics, theming, notifications, charts, and several workspaces. Leaving
ownership implicit would allow native calls, database assumptions, renderer
configuration, or analytical decisions to leak into Svelte components and make
later accessibility and historical-context work substantially harder.

## Decision

Rust and domain services own parsing, lineage, persistence, validation,
evidence classification, analytical rules, recommendations, and lifecycle
decisions. Native commands expose bounded models and explicit results; they do
not expose connections, SQL, database paths, parser internals, or renderers.

Feature `desktopClient.ts` modules are the only frontend code that imports
Tauri. Typed TypeScript presentation adapters map bounded host models and
synthetic design fixtures into chart and view models. They do not invoke
services, persist state, resolve lineage, select evidence classes, or decide a
lifecycle outcome. Svelte components own rendering, localisation,
accessibility, navigation, ephemeral interaction state, and calls to their
feature service. Co-located Svelte styles remain permitted.

The chart adapter is the only ECharts boundary. The theme runtime is the only
code that mutates validated theme variables. Browser storage remains confined
to the documented one-time legacy language-pack handover; operational state is
native-owned.

`npm run check` executes a source-boundary audit which fails on direct Tauri,
renderer, storage, theme-mutation, service-from-adapter, or domain-policy-from-
component bypasses. The audit complements type checking and review; it does not
claim to prove semantic correctness.

## Consequences

- New host behaviour starts with a Rust/domain contract, not a Svelte helper.
- View-only mapping code lives under `src/lib/presentation`, making its limited
  role visible in review.
- Shared UI primitives remain presentation code; shared policy becomes a
  service or native model instead of CSS or component conditionals.
- CSS may consume semantic state and validated theme roles but never establish
  what a status, fact, recommendation, or lifecycle state means.
- If a feature cannot fit the boundary, its contract must be redesigned or an
  ADR must explain a narrowly bounded replacement. Broad exceptions are not
  accepted.
