# ADR-0028: First-class related-data navigation

## Status

Accepted.

## Context

Observatory presents the same recorded subjects at several levels: a headline,
a chart, an exact record, a source explanation, and sometimes a player plan.
Hand-written workspace switches could not preserve filters or selected-save
context, and browser fragment navigation could scroll the desktop shell itself.

## Decision

Svelte owns one typed, allowlisted related-data navigation registry. A link is
admitted only when stable metric, observation, resource, city, catalogue, plan,
or contribution identity proves the relationship. One destination opens
directly; several honest destinations use the shared chooser.

Navigation preserves timeline, scope, units, currency, compatibility profile,
and selected save unless the action names an exact change. Historical chart
points may select only a save Observatory recorded at that exact point. It never
uses a nearest date. A session-only breadcrumb restores the previous typed
location and analysis context; dialogs retain keyboard priority.

The public `ChartSpec` and `SankeyChartSpec` remain inert version-1 data.
Chart-to-navigation bindings are separate, host-owned presentation data.
Analysis Packs cannot provide routes, selectors, URLs, or callbacks. The host
may relate a contribution only through a known published metric identity.

Every pointer action has a keyboard-accessible data-row action. The destination
receives focus and the workspace canvas scrolls without moving the application
shell. A failure preserves the source screen and context.

## Consequences

- Related links express navigation, never causation.
- Unsupported cross-currency, cross-timeline, city/republic, and alternate-future
  comparisons stay unavailable.
- New destinations require registry, localisation, focus-target, context, and
  accessible-equivalent tests.
- External operating-system links remain out of scope.
