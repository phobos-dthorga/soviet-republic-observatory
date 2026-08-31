# ADR-0014: Admit exceptional visuals through explicit analytical contracts

Status: Accepted

## Context

Republic Observatory benefits from specialist diagrams when they reveal
planning relationships that ordinary charts obscure. Sankey diagrams are a
strong example for source-to-use volume, but allowing pages or extensions to
assemble raw renderer options would undermine the existing evidence,
accessibility, localisation, and replacement boundaries.

Adding Sankey directly to Analysis Pack chart schema v1 would also broaden a
public contract that currently has time-series and categorical metric semantics
without defining how community metrics become trustworthy flow links.

## Decision

- Adopt the Exceptional Visuals Doctrine in the Advanced Visual Grammar.
- Add specialist forms only through separate, versioned, application-owned data
  contracts with a documented player question, sufficiency rule, limits,
  fallback, tests, and native visual QA.
- Admit Sankey as the first advanced family through
  `sankey-chart-spec-v1.schema.json`.
- Keep Sankey host-rendered by `ObservatoryChart`; application code supplies
  nodes, links, values, roles, and provenance, never ECharts options.
- Require one common unit and boundary, positive acyclic links, explicit
  residuals, semantic balance checks, reduced-motion behaviour, textual
  summaries, and an exact accessible flow ledger.
- Keep Analysis Pack chart contract v1 unchanged. A future public flow contract
  requires a deliberate host-API revision and an authoritative native
  validator.
- Treat the first Materials diagram as synthetic interface proof, not catalogue
  or save evidence.

Implementation note: the initial proof was subsequently removed from ordinary
application mode after source-backed production routes became available.
Synthetic flow states now live only in the bounded UI-review fixture registry;
an unavailable catalogue produces an unavailable laboratory, not example data.

## Consequences

High-value specialist diagrams can enter the product without creating an
unbounded rendering language. The extra contract and fallback work is an
intentional admission cost. Some attractive visuals will be declined when a
bar chart or table answers the player question sufficiently, and community
packs cannot emit Sankey diagrams until flow semantics are proven safe and
useful.
