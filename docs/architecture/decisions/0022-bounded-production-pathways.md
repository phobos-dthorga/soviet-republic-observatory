# ADR-0022: Expand production pathways through a bounded host model

Status: Accepted

## Context

The single-recipe Production Route Laboratory proves one proportional
definition-coefficient step. Republic planning also needs to answer which
upstream recipes and materials are required to supply that target. Recursively
joining catalogue relationships can otherwise create cycles, silently choose
between modded alternatives, mix incompatible units, or turn a renderer into
an unbounded graph-query interface.

## Decision

- Rust owns recursive expansion over the active DuckDB catalogue and returns a
  bounded `ProductionPathwayModel` v1. Presentation code receives neither SQL
  nor catalogue table identities.
- Every request pins one root recipe, output resource, positive target, depth
  from two through six, and no more than 32 explicit resource-to-recipe
  selections.
- A unique compatible upstream recipe may expand automatically. Two or more
  candidates create a visible player choice; declaration or package order
  never selects one implicitly.
- Quantities propagate only through finite, positive coefficients bearing the
  root route's exact unit. Different or missing bases remain separate auxiliary
  requirements and never receive Sankey widths.
- Cycles and depth, candidate, node, and link limits terminate expansion with a
  named diagnostic and terminal requirement. The model is limited to 16
  candidates per resource, 128 nodes, and 256 links.
- The result pins catalogue generation, compatibility mapping, planning-overlay
  revision, observation watermark, warehouse schema, and projector version.
  Every link retains directive, source line, and mapping provenance.
- The chart adapter may render the host model as an application-owned Sankey,
  but the exact terminal, auxiliary, diagnostic, and evidence ledgers remain
  authoritative and keyboard-readable.

## Consequences

Players can inspect and scale a genuine multi-stage definition pathway without
the Observatory inventing mod precedence, converting units, or claiming
observed throughput. The bounded model is deliberately not capacity planning,
transport routing, inventory balance, limiting-input analysis, cost
optimisation, or a physical mass-balance solver. Those require independently
verified evidence and contracts.
