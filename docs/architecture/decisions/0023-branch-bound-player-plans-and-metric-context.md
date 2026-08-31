# ADR-0023: branch-bound player plans and one metric-context contract

## Status

Accepted and implemented for the first Five-Year Plan vertical slice.

## Decision

Republic Observatory stores player plans as operational intent in SQLite. A
plan belongs to one timeline branch, begins at one exact immutable save
interpretation, and consists of immutable revisions. Selecting, revising,
rolling back, or removing a plan never mutates a save observation or abandons
the revision ledger.

Rust owns plan validation, baseline recovery, schedule calculation, directional
variance, guardrails, attainment, compatibility-profile continuity, and branch
truncation. Svelte receives a bounded `RepublicPlanWorkspace` and renders it; it
does not reproduce the calculation rules. Plans are not projected into DuckDB
in this slice because they are small operational state and every implemented
calculation uses the existing bounded SQLite observation history.

The first schedule catalogue is deliberately small:

- linear interpolation from baseline to target;
- quarterly step milestones; and
- hold for the first half, then change linearly.

Targets may use only host-published count metrics with an exact baseline in the
anchor save. The first revision fixes the anchor for every later revision. A
changed compatibility-profile hash makes evaluation unavailable instead of
comparing differently interpreted values. Historical inspection truncates the
series at the exact analytical head, while a continuation receives its own
branch-scoped active-plan selection.

Every published metric also has one host-owned `MetricContext`: counted basis,
time basis, geographic scope, denominator, comparison rule, and limitations.
The same model supplies Briefing, Population, Broadcast, Monitor, Plan, chart
help, and evidence inspectors. `MetricContextHelp.svelte` is the standard card
and form affordance; `ObservatoryChart` consumes the same help content for
charts. Required scope remains visible in ordinary page text and is never
available only through a tooltip.

## Evidence classes

- actual points are parsed save facts;
- the schedule and target are player definitions;
- variance, guardrail state, and attainment are deterministic calculations.

A plan is not a forecast, recommendation, causal model, or game-definition
fact. The current implementation does not invent missing observations,
interpolate observation gaps, or mix branches.

## Consequences

The Briefing may report plan attainment only when an active plan can be
evaluated at the selected head. Otherwise it continues to show the capability
as unavailable and links to Plan. Later forecasting and optimisation must use
separate versioned model contracts rather than expanding the meaning of the
deterministic schedule.
