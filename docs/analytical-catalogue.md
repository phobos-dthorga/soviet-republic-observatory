# Analytical catalogue

The catalogue starts from a player decision, not a preferred graph. “Ready”
means the known source can support the analytical question; it does not mean the
feature is implemented.

## Readiness classes

| Class                  | Required evidence                                        |
| ---------------------- | -------------------------------------------------------- |
| A — embedded history   | Supported `$STAT_RECORD` fields from one distinct save   |
| B — observed snapshots | Repeated `$STAT_CURRENT` or `$STAT_CITY` observations    |
| C — game definitions   | Versioned resource and production-recipe definitions     |
| D — binary research    | A separately validated binary payload or topology source |

## Republic Briefing

| Player question                                   | Visual or statistic                                     | Readiness   | Notes                                                            |
| ------------------------------------------------- | ------------------------------------------------------- | ----------- | ---------------------------------------------------------------- |
| Are we on plan?                                   | Plan-attainment card and actual-versus-scheduled line   | A + targets | Target definition remains player-owned                           |
| What changed since I last saved?                  | Additive delta waterfall plus ranked exceptions         | A/B         | Waterfall only for components that genuinely reconcile           |
| What deserves attention?                          | Robust anomaly queue with duration and affected scope   | A/B         | Separate magnitude, persistence, and coverage                    |
| Is the republic becoming more externally fragile? | Critical-import exposure and export-concentration cards | A           | Never add rubles and dollars without an explicit conversion rule |
| Is the population replacing itself?               | Demographic-resilience rate and decomposition           | A/B         | Express as rate per 1,000 as well as absolute count              |
| What happened in plain language?                  | Ministry Dispatch                                       | A/B         | Every sentence links to evidence; no invented causes             |

## Five-Year Plan

| Player question                        | Visual or model                                  | Readiness   | Notes                                        |
| -------------------------------------- | ------------------------------------------------ | ----------- | -------------------------------------------- |
| How far ahead or behind are we?        | Actual-versus-plan cumulative line               | A + targets | Use actual in-game dates                     |
| Which measures explain the variance?   | Sorted variance bars or additive bridge          | A + targets | Do not force unrelated units into one bridge |
| Are we likely to finish?               | Forecast fan chart and probability of attainment | A + targets | Requires enough observations and backtesting |
| Which target is at risk first?         | Milestone calendar with confidence intervals     | A + targets | Show model and data coverage                 |
| What if prices or productivity change? | Scenario controls and sensitivity matrix         | A/C         | Scenarios are calculations, not forecasts    |

## Materials and industrial laboratory

| Player question                                        | Visual or model                                         | Readiness   | Notes                                                            |
| ------------------------------------------------------ | ------------------------------------------------------- | ----------- | ---------------------------------------------------------------- |
| Which resources are strategically exposed?             | Material Periodic Table                                 | A           | Lens can show trade, price, volatility, use, or coverage         |
| Where are resources going?                             | Resource-by-use heatmap                                 | A           | Quantity comparisons stay within compatible units                |
| What dominates state material demand?                  | Pareto chart by resource and use category               | A           | Optional base-price weighting must disclose its reference basket |
| How does a final product depend on upstream materials? | Directed production network                             | C           | Nodes are resources; edges carry recipe coefficients             |
| Which input limits a target output?                    | Limiting-reagent bars and theoretical yield             | C, then A/C | “Actual yield” waits for verified observed inputs and output     |
| Which constraint is binding?                           | Linear programme with constraint and shadow-price table | C, later D  | Explain objective, bounds, and infeasibility                     |
| How sensitive is unit cost?                            | Tornado or two-variable contour                         | A/C         | Keep currencies separate unless a user supplies parity           |
| Does the accounted system reconcile?                   | Sources/uses with explicit measurement residual         | A, later D  | Never call the residual waste or stock change without evidence   |

The full family, dossier, and analytical-chemistry metaphor is specified in the
[Material Periodic Table and Industrial Laboratory](material-periodic-table.md).

## Broadcast Desk

| Player question                                     | Visual or model                                | Readiness  | Notes                                                              |
| --------------------------------------------------- | ---------------------------------------------- | ---------- | ------------------------------------------------------------------ |
| How is receiver adoption changing?                  | 100% stacked receiver ladder                   | B          | Denominator is the four electronics classes, not total population  |
| How much reachable audience is actually listening?  | Potential/current reach lines and utilisation  | D          | Plainly label station telemetry as binary research until validated |
| What does each station intend to influence?         | Six-part programme formulation                 | D/C        | Game-facing settings; do not assume a linear dose model            |
| What directional outcomes does the programme imply? | Diverging influence assay                      | D, later B | Separate game-displayed expectation from calibrated estimate       |
| Is staffing constraining reach?                     | Staffing fill, reach-per-position, rating/cost | C + D      | Nominal capacity is a definition; actual staffing is separate      |
| What changed after a schedule intervention?         | Annotated event-time outcome trends            | B + notes  | Association only, with lag, contemporaneous changes, and gaps      |
| Which cities or channels are exposed?               | Exposure matrix and concentration measures     | D          | National averages must not hide an unreached settlement            |
| Would the system tolerate a station outage?         | Channel resilience scenario                    | D          | Scenario, not forecast; retain power and staffing assumptions      |
| What deserves tonight's bulletin?                   | Evidence-linked deterministic Evening Bulletin | B, later D | Rules may be witty; claims and caveats remain deterministic        |

The [Broadcast Desk specification](broadcast-desk.md) records the exact
receiver-field and citizen-status compatibility evidence.

## Trade and markets

| Player question                              | Visual or model                                         | Readiness | Notes                                                         |
| -------------------------------------------- | ------------------------------------------------------- | --------- | ------------------------------------------------------------- |
| What are we buying and selling?              | Quantity/value small multiples by resource and currency | A         | Quantity and value remain distinct                            |
| Are exports dangerously concentrated?        | Pareto curve, HHI, and effective product count          | A         | Offer quantity- and value-based views                         |
| Which prices are structurally moving?        | Indexed price lines and change-point markers            | A         | Retain nominal price view alongside indices                   |
| How volatile is a resource?                  | Rolling dispersion and control chart                    | A         | Use robust measures when spikes dominate                      |
| Is player trade moving the market?           | Lagged price-response regression                        | A         | Exploratory association; global events confound causality     |
| Is domestic production preferable to import? | Break-even and sensitivity view                         | A/C       | Include delivery and assumed operating efficiency where known |
| Is foreign income covering debt service?     | Currency-specific coverage ratio and stress fan         | A         | No cross-currency aggregation by default                      |
| Is tourism becoming more valuable?           | Visitor, score, spending, and yield small multiples     | A         | Yield uses reported visitors as denominator when compatible   |

## Population and welfare

| Player question                             | Visual or model                                     | Readiness | Notes                                                              |
| ------------------------------------------- | --------------------------------------------------- | --------- | ------------------------------------------------------------------ |
| Why did population change?                  | Birth + immigration − death − escape waterfall      | A         | Components reconcile to the reported flow window                   |
| Is a death or escape wave emerging?         | EWMA/CUSUM control view                             | A         | Use rates and expose baseline period                               |
| Are conditions improving together?          | Welfare small multiples                             | B         | Familiar in-game measures, one common time axis                    |
| Which condition moves before productivity?  | Lag explorer and scatter                            | B         | Association only; common grain and denominator required            |
| Is the future workforce under pressure?     | Age-band pipeline and forecast interval             | B         | Coarse age bands imply coarse forecasts                            |
| Is national progress hiding local distress? | Weighted city heatmap and ranked dot plots          | B         | Filter empty city slots and expose population weight               |
| Are cities diverging?                       | Weighted dispersion, percentile band, optional Gini | B         | A composite score never replaces metric-specific views             |
| Are there recurring city types?             | Explainable clustering and profile cards            | B         | Exploratory; show features and stability, not authoritative labels |

## Construction, waste, vehicles, and environment

| Player question                                   | Visual or model                                   | Readiness | Notes                                                     |
| ------------------------------------------------- | ------------------------------------------------- | --------- | --------------------------------------------------------- |
| Are we in a construction boom or pause?           | Construction-material pulse and change-point view | A         | Physical demand, not monetary GDP                         |
| What consumes the republic's materials?           | Construction/factory/shop/vehicle stacked bars    | A         | Choose composition or trajectory based on the question    |
| Where is waste being generated?                   | Source-by-type heatmap and per-capita trend       | A/B       | Treatment and diversion wait for verified output coverage |
| Are vehicle imports or exports changing exposure? | Currency-specific value trends and mix            | A         | Fleet condition requires later evidence                   |
| Which route or factory is failing?                | Facility and network diagnostics                  | D         | Explicitly outside the first parser                       |

## Intervention and experimental views

Players can attach an annotation to an observation: a refinery opened, a city
received new heating, a tariff-like self-rule began, or a university expanded.

| Question                                     | Method                          | Minimum evidence                                   |
| -------------------------------------------- | ------------------------------- | -------------------------------------------------- |
| Did the level change after the intervention? | Interrupted time series         | Sufficient observations before and after           |
| Did one city change differently from others? | Treated-versus-comparison trend | Comparable city snapshots at common dates          |
| Was the effect immediate or delayed?         | Event-time plot                 | Stable annotation date and enough post-event saves |
| Did volatility change?                       | Pre/post robust dispersion      | Comparable windows and no hidden branch splice     |

These views report estimated association, interval, sample coverage, and
plausible alternative explanations. They do not use causal language by default.

## Fun and narrative layer

- **Material Periodic Table:** a resource atlas and navigation surface, not a
  decorative novelty.
- **Economic weather:** a short, explainable classification based on named
  signals such as price volatility, demographic pressure, and import exposure.
- **Republic records:** highest, lowest, longest streak, and greatest recovery,
  always scoped to the current branch.
- **Plan medals:** player-defined achievements with visible rules.
- **Congress report:** a selected-save comparison with a structured narrative
  and exact charts.
- **Animated history:** replay material, demographic, and trade states over
  actual game dates, respecting missing intervals.
- **Ministry Dispatch:** evidence-linked prose generated deterministically from
  ranked, thresholded findings before any optional language-generation layer is
  considered.
- **Evening Bulletin:** receiver and station findings written with a restrained
  newsroom wink; it links its claims and declines causal medals when evidence
  is incomplete.

## Chart-family discipline

- Lines show sufficiently dense continuous movement.
- Bars or dots show category comparison and ranking.
- Stacked bars or areas show meaningful part-to-whole composition.
- Heatmaps show city-by-metric or resource-by-use matrices.
- Waterfalls are reserved for components that add to a reported result.
- Pareto combines ranking with cumulative concentration.
- Scatter plots require enough observations at one common grain.
- Fan charts show forecast uncertainty; a lone future line is prohibited.
- Tables support exact lookup after the visual summary, not as a substitute for
  a visible pattern.
