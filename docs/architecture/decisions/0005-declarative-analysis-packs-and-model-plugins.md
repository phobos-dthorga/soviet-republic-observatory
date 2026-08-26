# ADR-0005: declarative Analysis Packs before executable Model Plugins

- **Status:** accepted
- **Date:** 2026-08-27

## Context

Republic Observatory should accept community statistics and visualisations
without exposing saves, private paths, storage, or ECharts internals. A general
in-process plugin API would combine calculation, renderer, application, and
security compatibility before any demonstrated model requires that complexity.

OnAir WyrmGrid established useful boundaries: external extensions, host-owned
models and presentation, independent lifecycle operations, capability grants,
and process-level failure containment. Its experience also shows that declaring
a contract is not the same as wiring a complete runtime.

## Decision

Publish two conceptual tiers:

1. **Analysis Packs** are inert `.roanalysis.json` documents. The host validates
   them, evaluates a deliberately small operation vocabulary over normalised
   observations, resolves chart templates, and owns provenance and rendering.
2. **Model Plugins** are deferred out-of-process programs for a demonstrated
   model that cannot fit the declarative vocabulary.

Schema v1 proves identity, limits, five operations, ordered references, and
line/area/bar templates. It exposes no executable expression, raw save, path,
URL, SQL, markup, or renderer configuration.

First-party and community extensions use identical public contracts. Local
offline delivery is the baseline; a catalogue is optional. Installation,
validation, permission approval, enabling, starting, updating, rollback, and
removal remain distinct.

The Model Plugin manifest, package, protocol, and runtime versions remain
unpublished until a concrete model and security review justify them. When
introduced, they receive only bounded normalised observations and versioned
game-definition models. “Out of process” must never be described as an
operating-system sandbox.

## Consequences

- Common community graphs can be reviewed as data and evaluated consistently.
- The host retains accessibility, themes, provenance, limits, and failure
  behaviour.
- Schema, host API, chart, protocol, runtime, application, and database versions
  evolve independently.
- Advanced models wait rather than escaping through JavaScript or ECharts
  configuration.
- A local Analysis Pack importer is meaningful only after branch-aware storage
  provides trustworthy normalised observations.
- The current foundation contains proofs and concepts, not an extension manager
  or executable runtime.

## Rejected alternatives

- **In-process JavaScript plugins:** excessive authority and tight coupling to
  the webview and renderer.
- **Arbitrary formulas in JSON:** an executable language disguised as data.
- **ECharts option contributions:** renderer lock-in plus callback and markup
  attack surface.
- **Publish an executable protocol now:** compatibility and security promises
  without a demonstrated consumer.
- **Marketplace-only delivery:** conflicts with local-first, offline use and
  independent community distribution.
