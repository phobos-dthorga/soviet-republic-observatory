# Broadcast Desk specification

## Scope and evidence snapshot

The Broadcast Desk treats radio and television as public institutions that can
be inspected, compared, and studied over time. This specification is grounded
in the locally reviewed game version **1.1.1.9** and the official
[Radio station](https://wiki.hoodedhorse.com/Workers_Resources_Soviet_Republic/Radio_station),
[TV station](https://wiki.hoodedhorse.com/Workers_Resources_Soviet_Republic/TV_station),
and [Citizens](https://wiki.hoodedhorse.com/Workers_Resources_Soviet_Republic/Citizens)
references. Compatibility must be rechecked for later game versions.

The desktop interface can now replace the Receiver Ladder with parsed save
facts. Every other Broadcast panel remains synthetic and is marked accordingly;
the application does not claim that binary station telemetry has been decoded.

## Implemented receiver slice

After the player chooses a save directory and requests observation, the Rust
host streams `stats.ini` from the newest stable ZIP, normalises complete receiver
records, and deduplicates the payload by SHA-256. The chart uses actual numeric
game-day positions, preserves gaps, and exposes a textual series summary.

Its evidence inspector reports source filename, full payload identity, parser
and compatibility profile, branch placeholder, geographic scope, coverage,
stable metric/source-field mappings, and the latest source lines. Full local
paths stay private. The Broadcast stacked shares remain a built-in calculation
mirroring the Receiver Adoption Laboratory contract. A player may now import
and enable that pack separately in Extensions; this does not replace or
privilege the built-in Broadcast view.

## Known plain-text save facts

The following source spellings are retained exactly at the parser boundary,
including the game's inconsistent `Eletronic` / `Eletrinic` spelling:

| Stable core metric                     | Source field                  |
| -------------------------------------- | ----------------------------- |
| `core.citizens.electronics.none`       | `$Citizens_EletronicNone`     |
| `core.citizens.electronics.radio`      | `$Citizens_EletrinicRadio`    |
| `core.citizens.electronics.television` | `$Citizens_EletronicTV`       |
| `core.citizens.electronics.computer`   | `$Citizens_EletronicComputer` |

Source typos never leak into public metric names. These classes support a
receiver-adoption composition once the parser confirms that all four values
refer to the same branch, observation date, and geographic scope.

The reviewed citizen-status ordering is:

| Index | Status             |
| ----: | ------------------ |
|     0 | Happiness          |
|     1 | Satiated           |
|     2 | Health             |
|     3 | Government loyalty |
|     4 | Alcohol addiction  |
|     5 | Culture enjoyment  |
|     6 | Sports enjoyment   |
|     7 | Religion sympathy  |
|     8 | Clothing quality   |

Index order is compatibility evidence, not a public identifier. Stable metric
names and fixture-tested version mappings must sit between it and analytics.

## Game-definition facts

The reviewed 1.1.1.9 building definitions specify these nominal staffing
capacities:

| Station    | Workers | Professors |
| ---------- | ------: | ---------: |
| Radio      |     100 |         50 |
| Television |     120 |         70 |

Game and interface material also establish six intended-influence allocations:
alcohol, sport, culture, education, Soviet propaganda, and anti-religious
propaganda. The interface exposes potential outreach, current listeners or
viewers, rating, recording budget, actors or moderators, staffing warnings, and
expected effects on citizen outcomes. Their presence in the game interface does
not make them supported save facts.

## Evidence taxonomy

Broadcast analysis keeps five sources visibly separate:

1. **Save facts** — supported receiver classes and citizen statuses parsed from
   plain text.
2. **Game-definition facts** — station staffing capacity and versioned
   definitions.
3. **Binary-research candidates** — station identity, staffing state, intended
   influence, potential reach, current audience, rating, and recording budget
   until decoded with fixtures.
4. **Extension calculations** — host-evaluated results from an identified
   Analysis Pack and source observations.
5. **Player annotations** — programme changes, hypotheses, construction,
   outages, reforms, and contextual events.

An annotation is never evidence that an intervention caused an outcome.

## Analytical catalogue

### Receiver adoption

The Receiver Ladder shows a 100% stacked composition over time. Its denominator
is explicit:

\[
C = none + radio + television + computer
\]

\[
share_i = \frac{class_i}{C} \times 100
\]

If \(C\) is missing, non-finite, or zero, all shares are unavailable. The view
must not divide by total population unless a separately defined penetration
metric calls for that denominator.

Useful companions include adoption transitions between observations,
radio-to-television substitution, cohort or city composition when supported,
and receiver concentration.

### Audience utilisation

Once station telemetry is validated:

\[
utilisation = \frac{current\ audience}{potential\ audience}
\]

Potential and current reach should be plotted together, never as two unlabeled
percentages. Missing or zero potential reach yields unavailable utilisation.

### Programme formulation

The six intended-influence settings are displayed as a composition with exact
values. The view supports comparisons between stations and annotations but does
not assume a linear dose-response relationship.

### Influence profile

A diverging bar can show signed expected-effect directions around a visible
zero. It must distinguish game-displayed expected effects from Observatory
models and later empirically calibrated estimates.

### Staffing and cost efficiency

Candidate measures include worker and professor fill rate, audience per staffed
position, rating per recording-budget unit, and current-to-potential reach. All
retain station type, date, staffing denominator, cost unit, and coverage.

### Lagged outcomes

Citizen-status trends can be aligned around a programme annotation with several
candidate lags. The result is exploratory association. Confounding changes,
autocorrelation, seasonality, save cadence, and multiple testing must be shown
before stronger language is considered.

### City exposure and concentration

When geographic station coverage is established, the Desk can compare reachable
and unreached populations, city exposure, channel overlap, and audience
concentration. National averages must not conceal a city with no signal.

### Resilience

Resilience views ask how many citizens depend on one station, one medium, one
power supply, or one staffing pool. Candidate experiments model a station
outage or reduced programme budget without changing the save.

### Intervention studies

The Broadcast Notebook records a hypothesis, exact player change, observation
window, expected lag, contemporaneous events, outcome metrics, and conclusion.
Interrupted time-series or matched comparisons wait for sufficient observations
and remain non-causal unless a defensible design supports more.

### Later calibrated optimisation

Only after validated inputs and a demonstrated response model may the
Observatory explore schedules under player-selected objectives and guardrails.
Optimisation remains a recommendation, includes uncertainty, and cannot claim
to have discovered the game's hidden formula from correlation alone.

## Broadcast Notebook

The Notebook is an audit trail, not a free-form causal-story generator. A note
contains:

- branch, geographic scope, station, and observation window;
- hypothesis and expected direction;
- programme or infrastructure intervention;
- outcomes and candidate lag;
- other known changes;
- evidence coverage and model version; and
- a status such as planned, observing, inconclusive, or reviewed.

Notes link to the observations used by a chart. Editing a note never rewrites
an observation.

## Evening Bulletin

The Bulletin is earnest with a wink and deterministic by construction. It:

1. selects only eligible findings with explicit evidence and coverage;
2. leads with the largest administratively meaningful receiver or audience
   change;
3. may name a programme annotation but uses “coincided with” rather than causal
   language;
4. includes one material caveat when evidence is partial or experimental;
5. links every sentence to its chart or note; and
6. emits no story when the thresholds or evidence requirements are not met.

Template rules and thresholds are versioned. No language model is required for
ordinary bulletins.
