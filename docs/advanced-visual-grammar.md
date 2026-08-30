# Advanced Visual Grammar

Republic Observatory may use an unusual visual form when the form makes a
material planning relationship easier to understand than a line, bar, area, or
table. Novelty alone is never sufficient. The purpose of this grammar is to
make high-value specialist visuals available without turning the interface into
a collection of decorative experiments.

## Exceptional-visual admission test

Every proposed advanced visual must pass all of these gates:

1. **Player question:** state the decision or relationship the player is trying
   to understand and the expected reading in one sentence each.
2. **Comparative advantage:** show why an ordinary chart or compact table loses
   an important relationship. If it does not, use the ordinary form.
3. **Data sufficiency:** define compatible units, scope, time window, missing
   values, and minimum evidence. Unknown values remain unknown.
4. **Encoding integrity:** a visible mark has one documented meaning. Area,
   width, position, and colour cannot quietly encode conflicting measures.
5. **Evidence:** the visual, and individual elements when needed, inherit exact
   provenance and coverage. A visual cannot promote an estimate into a fact.
6. **Accessible equivalent:** provide a textual summary and an exact table or
   list that answers the same question without relying on colour or pointer
   interaction.
7. **Bounded behaviour:** publish data limits, reject invalid structures, honour
   reduced motion, and keep library configuration inside the host renderer.
8. **Product review:** test keyboard, narrow and wide layouts, localisation,
   legibility, empty states, and the final rendered application—not only an
   isolated chart fixture.

The chart title names the subject. Its description asks the analytical
question, and its reading states the primary takeaway. Units and the comparison
window stay visible. Contextual tooltips explain exact values but are never the
only place where evidence can be recovered.

## Sankey diagrams

A Sankey diagram encodes flow quantity as ribbon width between named nodes. It
is admitted because it can show source, pooling, allocation, and residual volume
at once—relationships that require several disconnected bars or a much larger
matrix to express.

Suitable Republic Observatory questions include:

- which domestic and imported sources supply one material pool;
- which destinations account for a material during one observed window;
- where a verified production chain splits or converges;
- how construction demand is allocated across known project classes; and
- where an accounted-flow residual remains after compatible facts are matched.

Sankey is not suitable for a simple two-step process, rankings, unrelated
measures, mixed units, unknown joins, ordinary time series, or a causal story.
It must not imply that a link is an observed transfer when the evidence only
establishes separate totals.

### Contract v1

[`sankey-chart-spec-v1.schema.json`](../schemas/sankey-chart-spec-v1.schema.json)
is a strict application-owned contract with:

- 2–64 nodes and 1–128 positive links;
- stable node and link identifiers;
- one common unit and one declared boundary;
- source, process, intermediate, sink, and residual roles;
- chart-level provenance with optional per-link provenance;
- either a conserved or explicitly open boundary;
- no cycles, self-links, duplicate endpoints, disconnected nodes, markup, or
  arbitrary renderer configuration.

For a conserved boundary, internal node inflow must equal outflow and total
source flow must equal terminal flow. A residual is an accounting remainder,
not automatically waste, loss, theft, or inefficiency. The interface labels it
accordingly until stronger evidence exists.

The renderer uses position, labels, role colour, and a distinct residual border;
colour is not the sole carrier of meaning. Each diagram has a keyboard-readable
flow ledger containing source, target, exact value, unit, and evidence. Canvas
animation is removed when reduced motion is requested.

## Current proof

The Materials workspace includes a deliberately synthetic steel allocation
laboratory. It asks where a 100-unit illustrative supply originates and where it
is allocated. The diagram reconciles 68 domestic and 32 imported units with 42
construction, 24 mechanical-component, 18 vehicle, 10 export, and 6 unaccounted
units. These values prove the visual and accounting contract; they are not read
from a save, definition catalogue, or personal republic.

## Extension boundary

Analysis Pack schema v1 continues to expose line, area, and bar templates only.
Community content cannot submit Sankey nodes, links, callbacks, ECharts options,
HTML, or styling through that contract. Promoting advanced visuals into a later
host API requires bounded metric-to-flow semantics, an authoritative validator,
compatibility tests, and a demonstrated community model. First-party use grants
no private rendering or data access.

## Candidates, not commitments

Other potentially valuable specialist forms include alluvial transitions,
chord diagrams, network graphs, ridgelines, fan charts, control charts, ternary
plots, and small-multiple cohort trajectories. Each starts outside the contract
and must independently pass the admission test. Similar appearance is not a
reason to reuse Sankey semantics.
