# Material Periodic Table and Industrial Laboratory

## Purpose

The Material Periodic Table is the Observatory's playful analytical index for
_Workers & Resources: Soviet Republic_ resources. It borrows the compact visual
grammar of a chemical periodic table—symbols, families, cells, and reaction
language—because the game is fundamentally about transforming material under
constraints.

The metaphor is not a scientific claim. Resource families are designed for
administration, pseudo-symbols are interface identifiers, and production rules
are game-definition facts. Nothing here implies that aggregates, steel,
electricity, or labour obey literal atomic, chemical, or thermodynamic laws.

## Stable material identity

Every resource receives an application-owned dossier identity with:

- a stable source resource identifier and the game-definition version that
  established it;
- a unique, short **pseudo-symbol** used only for display;
- a full display name and aliases;
- one stable administrative family;
- units and currency rules; and
- coverage and provenance for every observation or calculation.

Pseudo-symbols should be memorable, such as `Ch` for chemicals, but are never
database keys. Collisions are resolved in the versioned display catalogue and
old symbols remain aliases so a theme or saved view does not silently change
meaning.

The initial administrative families are raw, intermediate, construction,
consumer, fuel, utility, vehicle, and waste. A selected analytical lens may
reorder or recolour cells, but it does not rewrite family membership.

## Cell anatomy

A cell is an index into a resource dossier, not a miniature dashboard. It may
show:

1. pseudo-symbol and resource name;
2. selected measure and unit;
3. family;
4. signed change over the selected comparison window;
5. compact status treatment; and
6. coverage or unavailable state.

The selected lens supplies the measure. Useful lenses include recorded import
reliance, import cost, export contribution, price movement, price volatility,
recorded production, accounted use, inventory endurance when available, and
plan variance. A status always retains text or shape in addition to colour.

The dossier expands the cell into exact observations, trends, source-and-use
views, known production routes, plan targets, annotations, and provenance. It
must say “recorded production” or “accounted use” where save coverage is not
known to be complete.

## Analytical-chemistry-inspired extensions

### Reaction pathways

Versioned game recipes form a directed graph from inputs to outputs. A pathway
view can propagate a target upstream, compare alternative routes, and identify
where imports enter the chain. Recipe coefficients are game-definition facts;
actual throughput requires republic observations.

The **Production Route Laboratory** selects one current catalogued recipe and
output, applies one transparent scale factor, and shows comparable input/output
coefficients as an open-boundary Sankey. Its bounded multi-stage pathway can
then expand unique upstream recipes and propagate the compatible target through
their recorded coefficients. Alternative producers require an explicit player
choice; external inputs, cycles, safety limits, and differently based auxiliary
requirements remain visible rather than being fabricated into the ribbons. An
exact row ledger remains authoritative. Planning Overlay v1 still cannot alter
recipe relationships.

### Stoichiometry and theoretical yield

For recipe coefficient \(a_i\), available input \(x_i\), and output coefficient
\(b\), the theoretical output constrained by input \(i\) is:

\[
Y_i = b \times \frac{x_i}{a_i}
\]

The smallest supported \(Y_i\) identifies the limiting input. The result is a
scenario calculation, not measured factory yield. Worker, power, water,
transport, storage, and building capacity become additional constraints only
when their definitions and observations are available.

### Capacity kinetics

“Kinetics” is an interface metaphor for how quickly a production system can
respond. Candidate views include rated capacity, observed utilisation, ramp
time after an interruption, queue growth, and recovery time. The Observatory
must not infer a physical rate law from sparse save snapshots.

### Inventory half-life

Inventory endurance can be expressed as days remaining under an explicit,
recent consumption estimate. A half-life presentation may describe how long it
takes a stockpile to fall by half at the selected demand rule. This is a
planning shorthand, not radioactive decay and not valid when inventory or
outflow coverage is unavailable.

### Accounted-flow residuals

For an aligned time window:

\[
R = sources - accounted\ uses - observed\ stock\ change
\]

The residual \(R\) is **unaccounted flow**. It is not automatically waste,
theft, loss, or parser error. The view should lead the player toward missing
coverage rather than offer a false conservation proof.

### Titration-style sensitivity curves

A sensitivity experiment varies one input, price, staffing level, or target at
a time and plots the resulting model output. Breakpoints can reveal when the
limiting constraint changes. The curve records the baseline, varied parameter,
range, step, held-constant assumptions, and model version.

### Operating envelopes

An operating envelope maps feasible and infeasible combinations of two or more
constraints—for example power availability against worker supply. Boundaries
must be explained by named rules. “Phase diagram” may be used as a playful
subtitle, never as a claim of thermodynamic phases.

### Alternative production routes

Route comparison can evaluate theoretical material demand, currencies, worker
and energy requirements, waste, exposure, and resilience. The least-cost route
is not automatically the best route; the player chooses the objective and
guardrails.

### Shock-response experiments

A player annotation can mark a price shock, embargo, outage, recipe change, or
deliberate policy intervention. Event-time views compare aligned observations
before and after the event, show gaps, and preserve contemporaneous changes.
“Le Châtelier response” is a newsroom-friendly metaphor for adaptation, not a
chemical equilibrium law.

## Evidence rules

- Recipe coefficients and building capacities are game-definition facts.
- Stock, price, trade, and population values are save facts only when parsed
  from a supported source.
- Theoretical requirements and limiting inputs are calculations with named
  rules and inputs.
- Actual yield, utilisation, and inventory endurance remain unavailable until
  both sides of the relevant measurement are established.
- Optimisation is a recommendation layer. It never converts assumptions into
  facts or declares one correct republic.

The Material Periodic Table remains useful before every laboratory lens is
implemented: it is first an honest, compact index and only then a chemistry
playground.
