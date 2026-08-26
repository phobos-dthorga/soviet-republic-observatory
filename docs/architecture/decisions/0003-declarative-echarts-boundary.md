# ADR-0003: Declarative ECharts boundary

Status: Accepted

## Context

The planned analytical catalogue needs trends, rankings, composition,
decomposition, matrices, uncertainty, and production networks. Allowing every
page to assemble raw ECharts options would couple evidence semantics to a
renderer and produce inconsistent accessibility, provenance, themes, and empty
states.

## Decision

- Apache ECharts is the initial renderer.
- `ObservatoryChart.svelte` is the only component that initialises ECharts.
- Other interface code supplies a versioned application-owned `ChartSpec`.
- The host owns themes, sizing, tooltips, source labels, coverage gaps, reduced
  motion, accessible summaries, and resource limits.
- The interface foundation initially implements line, area, and bar charts.
- A new chart family is added only with a documented player question, data
  sufficiency rule, fallback, tests, and visual QA.
- Analytics services return facts or analytical result models, not raw ECharts
  configuration.

## Consequences

The first preview cannot render every catalogue item, which is intentional. The
contract can grow through real vertical slices, and replacing ECharts remains a
presentation-layer change rather than a rewrite of metric or parser services.
