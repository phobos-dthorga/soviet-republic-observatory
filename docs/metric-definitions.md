# Metric definitions

This is the initial measurement contract. Names and formulas are provisional
until validated against supported fixtures, but the denominator and evidence
rules are not optional.

## Primary outcomes

### Plan attainment

For a target with scheduled cumulative value `S(t)` and observed cumulative
value `A(t)` at game date `t`:

`plan attainment(t) = A(t) / S(t)`

Display the ratio with the two underlying values, target unit, target direction,
effective game date, and whether the target is cumulative or period-based. A
lower-is-better target uses an explicitly defined status rule rather than
silently inverting the ratio.

### Demographic resilience

For a common observation window:

`net demographic change = births + immigration − deaths − escapes`

`demographic resilience rate = net demographic change / population exposure × 1,000`

Population exposure should be the mean or person-time approximation supported
by the source window. If only a latest population is available, label the rate
as an approximation. Never mix flow windows.

### External dependency

External dependency is a framework, not one magically authoritative number.
The default view reports:

1. import quantity and value for each critical resource;
2. the count and share of critical resources with non-zero imports;
3. import value per capita by currency; and
4. an experimental resource-level reliance ratio only where production coverage
   is acceptable:

`recorded import reliance(r) = imports(r) / (imports(r) + recorded domestic production(r))`

The aggregate experimental index may use a fixed base-price basket so unlike
physical quantities are not directly summed. It must disclose the basket date,
currency treatment, included-resource coverage, and sensitivity to missing
production.

## Trade and solvency

### Net external trade

`net trade(c) = export value(c) − import value(c)`

Compute separately for each currency `c`. Do not add rubles and dollars unless
the user explicitly supplies a conversion assumption, which is stored as a
scenario calculation rather than a save fact.

### Export concentration

For export value shares `sᵢ` within one currency and window:

`HHI = Σ sᵢ²`

`effective export product count = 1 / HHI`

Report the top-product share and included export value alongside HHI. A high
index describes concentration, not necessarily bad policy.

### Debt-service coverage

`coverage(c) = (exports(c) + tourism receipts(c)) / interest due(c)`

If the source does not establish the same period for numerator and denominator,
the ratio is unavailable. Principal repayment and maturity risk require
additional fields.

### Price indices

For a fixed reference basket with quantities `qᵢ,0` and observed prices `pᵢ,t`:

`Laspeyres(t) = Σ pᵢ,t qᵢ,0 / Σ pᵢ,0 qᵢ,0 × 100`

Maintain separate construction, consumer-essential, industrial-input, and
optional player-defined baskets. Publish the basket composition and base date.
An index does not replace the underlying nominal prices.

### Terms of trade

`terms of trade(t) = export price index(t) / import price index(t) × 100`

The two baskets must use compatible currency treatment and stable weights.

## Materials and production

### Accounted sources and uses

For resource `r` over one period:

`sources = recorded production + imports`

`uses = factories + shops + construction + vehicles + exports`

`measurement residual = sources − uses`

Until stock change and production coverage are proven, the residual is labelled
“unaccounted by current sources.” It is not interpreted as inventory movement,
loss, dumping, or parser error without further evidence.

### Theoretical chain requirement

For a recipe consuming `a` units of input to produce `b` units of output, the
input required for target output `T` is:

`required input = T × a / b`

Propagate requirements upstream using versioned game-definition coefficients.
Where a resource has several recipes, the selected route is part of the
scenario and never silently chosen.

### Limiting-input capacity

For available quantity `xᵢ` and requirement coefficient `aᵢ` per unit output:

`supported outputᵢ = xᵢ / aᵢ`

The smallest supported output is the theoretical limiting capacity. “Available”
must identify whether it means inventory, production plan, import allowance, or
a user-entered scenario.

### Actual yield

`actual yield = observed output / theoretical output from observed limiting input`

This metric remains unavailable until the parser establishes compatible actual
input, output, stock change, and window coverage. Theoretical conversion alone
is not actual yield.

## Population and cities

### Rate normalisation

Deaths, escapes, births, and similar counts should be offered as:

`rate per 1,000 = count / population exposure × 1,000`

City comparisons use population weighting where the question concerns the
resident experience. National totals and an unweighted “average city” answer
different questions and must not share a label.

### Between-city dispersion

The default dispersion view reports weighted median, interquartile interval,
and selected percentiles. Coefficient of variation is allowed for positive ratio
measures whose mean is meaningful. Gini is optional and must state its weight
and interpretation.

### Intervention estimate

An interrupted time-series model may estimate a level change and slope change
around a player annotation. The result must expose:

- intervention game date;
- observations and duration before and after;
- model form and version;
- interval, residual diagnostics, and missing periods; and
- the phrase “estimated association” unless a stronger design is justified.

## Broadcast and receiver metrics

### Stable receiver-class identifiers

Host API 1 publishes four normalised save-fact metrics:

- `core.citizens.electronics.none`
- `core.citizens.electronics.radio`
- `core.citizens.electronics.television`
- `core.citizens.electronics.computer`

They map versioned source spellings into stable public names. Each value retains
branch, observation date, geographic scope, source field, parser version, and
coverage.

The implemented receiver parser also retains record ID and source line for each
class. Its time coordinate is `year × 365 + day`, while the interface continues
to display the original game year/day pair. This creates an ordering and spacing
coordinate; it does not claim a Gregorian calendar date.

The same four class identifiers may appear in a save-sampled republic snapshot
when `$STAT_CURRENT` supplies all values. Other captured current/city scalar
fields use internal `source.stats.*` fact identifiers. Those identifiers are not
published Analysis Pack inputs until their precise game meaning, accumulation
window, and compatibility range are validated.

### Electronics-classified population

For aligned class values:

`classified population = none + radio + television + computer`

This denominator is not assumed to equal the total population. If any required
class is missing or non-finite, the total and dependent shares are unavailable.

### Receiver-class share

For receiver class `i`:

`receiver class share(i) = class(i) / classified population × 100`

A zero, missing, or non-finite denominator yields unavailable. The four shares
may form a 100% stacked view only when all inputs share branch, observation
date, and geographic scope.

### Audience utilisation

After binary station telemetry is independently validated:

`audience utilisation = current audience / potential audience × 100`

Potential and current audience are shown beside the ratio. A zero or missing
potential audience yields unavailable; it never implies zero utilisation.

### Broadcast intervention estimate

A player programme annotation may anchor a lagged outcome comparison. Results
retain the pre/post windows, candidate lag, station, programme values,
contemporaneous annotations, observation count, model version, and uncertainty.
The default description is “estimated association,” not programme effect.

## Definition and overlay values

An installed definition fact is `game_definition` evidence and retains its
source-qualified entity identity, revision hash, directive, bounded arguments,
line number, unit, parser version, and catalogue generation. An unresolved
automatic construction coefficient is a rule input, not a material-demand
quantity.

A definition fact produced through a compatibility profile additionally retains
the stable mapping ID, optional catalogue-scope ID, reviewed/player mapping
classification, update policy, acknowledged supported-definition hash, current
supported-definition hash, and scope state. These are interpretation
provenance; they do not change the installed value.

An overlay operation is `player_override`; a supplemental entity is
`player_definition`. For every affected scalar or repeatable value the public
shape is `original → override → effective`. A failed revision or value
precondition makes the override unavailable and leaves the original effective.
It is never silently rebased.

Planning and model results pin the catalogue generation, compatibility profile
and resolved hash, mapping classification, active planning-overlay profile and
revision, SQLite observation watermark, warehouse schema, and projector
version. Results from different snapshots cannot be joined as though they were
one coherent model run.

## Analysis Pack calculations

An Analysis Pack result uses evidence kind `extension_calculation`. It records
pack ID, pack version, content hash, host API, exact operation rule, and source
observations. Packs cannot self-declare complete or trusted evidence.

Version 1 supports `sum`, `difference`, `product`, `safe_ratio`, and `scale`.
Inputs align within one branch, observation date, and geographic scope. Derived
metrics can reference only earlier declarations, so forward references and
cycles are invalid. `safe_ratio` is unavailable for a missing, non-finite, or
zero denominator.

## Statistical signals

### Robust standardised deviation

For baseline median `m` and median absolute deviation `MAD`:

`robust z = 0.6745 × (x − m) / MAD`

When `MAD = 0`, the signal rule uses an explicit zero-variation fallback rather
than dividing by zero. Alerts combine magnitude, persistence, practical
threshold, and data coverage; a z-score alone is not an emergency.

### Change points and control charts

EWMA, CUSUM, or change-point results are calculations over a declared baseline.
The interface displays the baseline dates, rate denominator, tuning parameters,
and whether multiple testing is controlled. Signals invite investigation; they
do not explain their own cause.

## Forecasts and scenarios

- A forecast requires enough historical points for the selected model and must
  be evaluated through rolling-origin backtesting before it is promoted beyond
  experimental status.
- The default forecast visual is a median or central estimate with prediction
  intervals.
- A scenario is a deterministic or simulated “what if” under player-specified
  assumptions. It is not labelled a forecast.
- Monte Carlo outputs report iteration count, input distributions, random seed
  policy, target probability, and percentile intervals.
- Optimisation outputs report objective, constraints, bounds, units, infeasible
  cases, and sensitivity. The “optimal” result is optimal only for that declared
  mathematical problem.
