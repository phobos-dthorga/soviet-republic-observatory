# Dashboard and interface specification

## Design intent

The Observatory should feel like an instrument panel for a planning ministry:
dense but calm, historical without pastiche, and capable of moving from a
national signal to its material or regional evidence in one or two actions.

The default view is useful without interaction. Filters refine a question; they
do not rescue an otherwise empty dashboard.

## Workspace anatomy

The desktop foundation uses five persistent bands:

1. **Command bar** — identity, primary workspaces, scanner state, and settings.
2. **Observation bar** — current branch, selected game date, latest distinct
   save, live-versus-historical state, and data-coverage warning.
3. **Navigator** — ministries, saved lenses, plan selection, and global filters.
4. **Canvas** — summary first, then movement, decomposition, and exact detail.
5. **Inspector** — selected resource/city/metric evidence, attention queue,
   provenance, and linked actions.

On narrow displays the navigator and inspector become stacked drawers without
changing their semantic order.

## Primary workspaces

- **Briefing** — three headline outcomes, guardrails, change summary, and
  Ministry Dispatch.
- **Plan** — targets, milestones, schedule variance, forecasts, and scenarios.
- **Materials** — Material Periodic Table, source/use views, production network,
  and limiting-input laboratory.
- **Population** — demographic flows, welfare trends, city comparison, and
  intervention studies.
- **Markets** — trade, prices, currencies, tourism, debt, and break-even models.
- **Archive** — branch-aware save history, annotations, comparisons, and data
  coverage.

## Briefing hierarchy

### Hero outcomes

The first row contains three cards only:

1. Plan attainment
2. External dependency
3. Demographic resilience

Each card includes the current value, effective game date, target or comparison,
direction, coverage badge, and a small trend only when at least eight meaningful
time points exist.

### Movement

The next row shows actual-versus-plan movement and the highest-signal external
or demographic trend. It is followed by a save-to-save decomposition or ranked
exception view.

### Drivers and action

Material dependency, affected cities, prices, waste, debt, and tourism appear
only when they help explain the current status. The inspector contains no opaque
“fix it” button; it links to evidence, a target editor, an annotation, or a
scenario.

## Global controls

Keep controls few and consequential:

- branch;
- game-date range;
- selected plan;
- currency view;
- material family; and
- evidence coverage threshold.

City and resource selection is contextual rather than a permanent wall of
filters. Ruble and dollar values are not merged by a convenience toggle.

## Visual language

### Identity

The synthetic preview establishes “Observatory Classic”:

- near-black blue-charcoal canvas;
- slate-blue operational surfaces;
- parchment-gold highlights;
- desaturated cyan for observed factual series;
- warm coral for risk, used sparingly;
- restrained uppercase labels and serif display titles; and
- fine borders, generous chart interiors, and very little ornamental shadow.

It inherits WyrmGrid's semantic-token methodology, not its literal palette.

### State without colour dependence

Observed, calculated, estimated, recommended, partial, historical, and warning
states use text labels, border or fill treatment, line style, and iconography in
addition to colour. Positive and negative values carry signed labels and a zero
reference rather than relying on green/red semantics.

### Motion

Animation should clarify an observation change or timeline movement. Reduced
motion disables chart and surface transitions. Historical replay stops at gaps
and branches rather than interpolating through them silently.

## Chart contract

Every chart specification contains:

- stable schema version and chart identifier;
- analytical title and short description;
- chart kind and orientation;
- axes, units, denominator, and time grain;
- one or more typed series;
- optional reference lines and coverage gaps;
- provenance kind, source, observation date, and coverage; and
- an accessible textual summary.

Only the chart adapter imports Apache ECharts. Dashboard components do not pass
raw ECharts options, callbacks, HTML formatters, or executable configuration.
This keeps rendering replaceable and prevents chart-library details from
becoming the analytical model.

## Chart map

| Workspace   | Question                              | Preferred form                                 | Fallback when sparse       |
| ----------- | ------------------------------------- | ---------------------------------------------- | -------------------------- |
| Briefing    | Are we on schedule?                   | actual-versus-plan line                        | KPI and period bars        |
| Briefing    | What changed?                         | waterfall when additive; otherwise ranked bars | exact change list          |
| Plan        | How uncertain is completion?          | fan chart                                      | milestone interval table   |
| Materials   | Which resources are exposed?          | periodic-table cells and ranked bars           | sortable resource table    |
| Materials   | Where are resources used?             | heatmap                                        | grouped horizontal bars    |
| Laboratory  | What limits output?                   | required-versus-available bars                 | coefficient table          |
| Population  | Why did population change?            | waterfall                                      | signed component bars      |
| Population  | Are welfare measures moving together? | aligned small multiples                        | latest values with slopes  |
| Cities      | Where is distress concentrated?       | city-by-metric heatmap                         | ranked dot plots           |
| Markets     | How concentrated are exports?         | Pareto                                         | sorted bars plus top share |
| Markets     | Are prices unstable?                  | indexed line with control band                 | discrete period bars       |
| Experiments | What changed after an intervention?   | event-time line and interval                   | pre/post slope chart       |

## Tables and exact detail

Tables sit below the chart or in the inspector when the task is exact lookup,
export, or audit. Chart-backed tables retain context beyond the plotted fields:
source dates, quantities, values, currency, coverage, comparison values, and
calculation version.

## In-game graph continuity

Useful in-game population and economy trends are reproduced as a familiar
reference layer when the save data supports them. The Observatory extends them
with:

- several aligned measures instead of hover-swapping one graph;
- city and national comparisons;
- rates and explicit denominators;
- targets, annotations, control limits, and uncertainty;
- branch-aware history and save-to-save changes; and
- an evidence inspector.

Duplication is justified when it completes the planning workflow, not merely to
claim feature parity.

## Accessibility and QA

- Every chart has an accessible title, description, source, and textual series
  summary.
- Keyboard focus order follows navigator, canvas, then inspector.
- Interactive cells and marks expose the same facts available on hover.
- Colour contrast is validated against theme roles.
- Long resource and city labels receive horizontal space rather than smaller
  unreadable type.
- Chart layouts are visually checked at wide desktop, compact desktop, and
  narrow stacked widths.
- Synthetic preview data is permanently labelled so it cannot be mistaken for
  a connected save.
