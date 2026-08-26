# ADR-0002: WyrmGrid-derived interface methodology

Status: Accepted

## Context

[OnAir WyrmGrid](https://github.com/phobos-dthorga/onair-wyrmgrid) already
establishes an effective local-first operational interface: semantic theme
tokens, explicit historical state, compact navigation, a central analytical
surface, contextual inspection, provenance, responsive modes, and ECharts
behind a host-owned contract.

Republic Observatory needs similarly dense operational work, but it has a
different domain, evidence model, brand, and chart vocabulary.

## Decision

Adopt the following methodology:

- Svelte 5 and TypeScript presentation;
- semantic colour and surface tokens;
- command, temporal-state, navigation, canvas, inspector, and status regions;
- summary-to-movement-to-driver hierarchy;
- visible evidence provenance and observed time;
- responsive and reduced-motion behaviour; and
- one host-owned chart adapter around Apache ECharts.

Do not copy WyrmGrid's name, exact palette, aviation vocabulary, Atlas-first
layout, plugin architecture, or broad technical dependency set. Republic
Observatory owns an industrial-planning identity and begins with only the
dependencies required by its current vertical slice.

## Consequences

The two projects feel related in interaction quality without becoming coupled
repositories or forced component libraries. Shared code may be extracted only
after both products demonstrate stable identical semantics and a maintenance
benefit greater than a small local implementation.
