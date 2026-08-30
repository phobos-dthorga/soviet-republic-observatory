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
- **Monitor** — native recorder health, candidate lifecycle, observation
  cadence, branch warnings, and latest same-branch movement.
- **Broadcast** — receiver ladder, audience research, programme formulation,
  influence assay, outcomes, Notebook, Bulletin, and station inspector.
- **Extensions** — local Analysis Pack inspection, immutable installed
  revisions, explicit enablement, host-resolved charts, denied capabilities,
  and the future Model Plugin boundary.
- **Plan** — targets, milestones, schedule variance, forecasts, and scenarios.
- **Materials** — Material Periodic Table, source/use views, production network,
  and limiting-input laboratory.
- **Population** — demographic flows, welfare trends, city comparison, and
  intervention studies.
- **Markets** — trade, prices, currencies, tourism, debt, and break-even models.
- **Archive** — branch-aware save history, annotations, comparisons, and data
  coverage.

The current application enables Briefing, Monitor, Broadcast, Extensions,
Materials, and Archive in primary navigation. The other buttons remain visibly disabled until their
analytical vertical slices exist. The shell owns only global navigation and
observation context; each enabled destination is a presentational workspace
component.

### Observation archive

The Archive separates files observed from distinct statistical states. Its
branch list changes the analytical context without rewriting history. The state
ledger exposes game date, exact lineage relationship, shared-prefix evidence,
coverage, content identity, repeated-file count, and bounded snapshot coverage.
The comparison assay accepts two distinct states only from the same resolved
branch and reports exact receiver-class changes over the actual elapsed game
days. An unrelated or tied history remains visibly unassigned. File names are
display evidence only and never establish ancestry.

### Save observer

The command-bar status opens one focus-managed dialog. In the desktop host it
shows the selected save and game-folder names, candidate count, observed payload
count, game-vocabulary source identities, and the opt-in automatic-observer
state. Directory selection, manual **Observe newest save**, and enabling
automatic observation are distinct actions. Automatic status distinguishes
watching, waiting for stability, retrying, observed, and terminal failure. In a
normal browser the same dialog explains that native observation is unavailable
and retains the synthetic preview. Closing the dialog returns focus to the
command-bar control.

The same dialog contains a Compatibility section with detected build evidence,
active reviewed/local profile, semantic version, short hash, exact base,
validation state, mapping coverage, and the app-local override location.
**Create starter override**, **Reload override**, and **Reinterpret newest save**
are separate keyboard-operable actions. Invalid edits preserve the last valid
profile and show a warning. The application does not embed a JSON editor or
hide `player_mapped` evidence behind ordinary save-fact styling.
Configured mod scopes appear as matched, dormant, updated-unreviewed, or exact
conflicts with package identity, short acknowledged/current hashes, policy,
mapping count, and remediation guidance. Materials dossiers identify the exact
mapping and scope that produced each affected fact.

### Observer Health and Republic Pulse

Monitor is source-backed and contains no synthetic fallback values. Observer
Health leads with recorder phase, queue depth, retained terminal failures, latest
processing latency, last folder scan, and last native file event. The lifecycle
ledger exposes recent candidates without full paths and separates discovery,
stabilisation, read attempts, imported/duplicate outcomes, retryable failure,
terminal failure, and supersession.

Republic Pulse follows with exact elapsed in-game days between retained
same-branch states, signed receiver-class movement between the latest two
distinct states, current classified receiver population, bounded republic/city
snapshot coverage, and branch/unassigned warnings. A long observation interval
is not automatically called a recorder outage: it may mean the game did not
save. Captured demographic scalars remain withheld from rate charts until their
window and denominator are validated.

### Broadcast composition

The Broadcast canvas proceeds from receiver adoption to unavailable station
telemetry, programme intent, expected influence, and observed outcomes. A
connected save replaces only the receiver ladder; a mixed-evidence notice keeps
the remaining concepts visibly synthetic. The Notebook records hypotheses and
interventions. The Evening Bulletin applies deterministic eligibility,
ranking, wording, and caveat rules. The inspector switches between receiver
evidence and the synthetic radio/television concept while keeping nominal
capacity separate from station state.

### Extensions workspace

The Extensions canvas can inspect a local `.roanalysis.json` file without
installing it, or inspect the included Receiver Adoption Laboratory through the
same public contract. A valid inspected pack may then be imported disabled.
Installed revisions can be enabled, disabled, rolled back, exported, or removed.
Enabled contributions are evaluated over the currently selected observation
branch and rendered through the application-owned chart adapter. Model Plugin
controls remain labelled planned and unavailable.

### Industrial catalogue

Materials now opens a source-backed catalogue workspace. Its health strip shows
the active DuckDB generation, source/file/entity counts, database size, pending
projection jobs, failures, and observation watermark. Search spans Resources,
Buildings, and Vehicles and preserves source/package identity. The dossier
separates typed facts, repeatable production/construction/capability relations,
unknown-directive diagnostics, and unresolved automatic-cost coefficients.

The planning-overlay laboratory keeps inspect, validate, import, activate,
rollback, deactivate, export, and remove distinct. Affected fields display
`original → override → effective`; conflicts never silently replace installed
facts. Guided supplemental definitions are explicitly player-authored.

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
- optional stack identities, fixed value domain, reference lines, and coverage
  gaps;
- optional per-series provenance, inheriting chart provenance when absent;
- provenance kind, source, observation date, and coverage; and
- an accessible textual summary.

Only the chart adapter imports Apache ECharts. Dashboard components do not pass
raw ECharts options, callbacks, HTML formatters, or executable configuration.
This keeps rendering replaceable and prevents chart-library details from
becoming the analytical model.

`extension_calculation` is a distinct evidence kind. Analysis Pack chart
templates reference metrics only; the host resolves observations into concrete
points and provenance. Schema version 1 supports line, area, and bar families.

## Chart map

| Workspace   | Question                              | Preferred form                                 | Fallback when sparse       |
| ----------- | ------------------------------------- | ---------------------------------------------- | -------------------------- |
| Briefing    | Are we on schedule?                   | actual-versus-plan line                        | KPI and period bars        |
| Briefing    | What changed?                         | waterfall when additive; otherwise ranked bars | exact change list          |
| Monitor     | Is recording healthy?                 | lifecycle ledger and recorder-health cards     | explicit unavailable state |
| Monitor     | How far apart are observed saves?     | elapsed-game-day interval bars                 | exact interval text        |
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
| Broadcast   | How is receiver adoption changing?    | 100% stacked area                              | latest composition bars    |
| Broadcast   | Is potential reach being used?        | potential/current lines                        | exact audience cards       |
| Broadcast   | What direction is influence expected? | signed horizontal bars around zero             | signed effect list         |
| Broadcast   | What followed a programme change?     | annotated aligned outcome lines                | pre/post status cards      |
| Extensions  | What does this enabled pack show?     | host-resolved contribution chart               | exact declaration list     |

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
- Ordinary captions and controls use shared tokens with a
  12-pixel-equivalent default floor; zoom and text scaling cause reflow rather
  than a return to microtext.
- Consequential or unfamiliar concepts may expose keyboard-accessible
  contextual help with stable tutorial topic IDs. Required warnings and actions
  never exist only inside a tooltip.
- Transient cross-workspace outcomes use the bounded shell notification centre,
  while field validation remains inline and long-running work remains in the
  critical-task progress system.
- Chart layouts are visually checked at wide desktop, compact desktop, and
  narrow stacked widths.
- Synthetic preview data is permanently labelled so it cannot be mistaken for
  a connected save.

## Localisation boundary

The interface is fully catalogue-backed rather than partially migrated: shell
controls, all four enabled workspaces, chart titles and series, provenance,
textual summaries, observer/language dialogs, and number formatting react to
the selected locale. Document direction uses the manifest's explicit LTR/RTL
value, and layout uses logical edges where direction affects meaning.

Community `.rolanguage.json` files are inspected and installed without becoming
active. Selection is a separate keyboard-accessible operation. Missing messages
fall back to canonical `en-AU`; malformed packs never leave the interface in a
half-translated state. Expanded and RTL pseudo catalogues exercise overflow,
variable preservation, and directional assumptions.

Raw observation IDs and source fields are not interface prose. Installed-game
resource/building labels will use a separate versioned vocabulary resolver.
Analysis Pack names and analytical claims remain author-owned content tagged
with the pack's `default_locale`, or `en-AU` for an older v1 file that predates
the compatible field.
