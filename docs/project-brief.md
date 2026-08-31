# Project brief

## Vision

Republic Observatory is a local-first companion for administering a _Workers &
Resources: Soviet Republic_ republic as a measurable, evolving system. It
observes newly written saves, retains their statistical history, and provides a
Ministry-of-Planning workspace for explanation, experimentation, forecasting,
and celebration.

It should deepen the game rather than play it for the user. A recommendation is
valuable only when the player can inspect the evidence, understand the rule,
and decide whether the proposed intervention belongs in their republic.

## Current delivery

The native foundation observes stable saves manually or automatically while the
desktop program is open. Native folder events and periodic reconciliation feed a
crash-recoverable candidate ledger rather than relying on the displayed
workspace. The host reads receiver-class history and bounded current/city facts
directly from `stats.ini`, compacts shared history prefixes, separates files from
distinct states, resolves supported ancestry, compares two states within one
branch, and renders Observer Health, Republic Pulse, and an observed Receiver
Ladder. The Republic Briefing now reports exact-head population, education, and
receiver facts, proven preceding-observation changes, source provenance, and
deterministic operational findings. Its host-owned Metric Context keeps the
counted population, time and geography, denominator, comparison rule, and
known limitations attached to every displayed measure. Unsupported Briefing
and Broadcast claims remain visibly unavailable rather than synthetic. The
Industrial Catalogue now retains local definition generations in DuckDB,
projects save observations through a durable SQLite outbox, and supports strict
inert planning overlays. The first local Analysis Pack lifecycle now validates,
stores, enables, evaluates, and renders declarative packs without giving them
code or database access. Binary station telemetry is not yet implemented.
Version-sensitive game keys are supplied by a reviewed inert W&R
1.1.1.9 compatibility profile; one local exact-base repair may activate as
visibly `player_mapped` evidence without rewriting an earlier observation.
Definition aliases for unusual mods may be source-scoped to an exact Workshop
or WIP identity and explicit update policy; the mechanism does not configure
mods, infer load order, or claim that an installed mod appears in a save.

## Player loop

The product organises every feature around five questions:

1. **Observe:** What changed since the previous distinct save?
2. **Distinguish:** Is that change ordinary variation, a structural shift, or a
   data-coverage problem?
3. **Diagnose:** Which resources, cities, population measures, or external
   markets contributed to it?
4. **Plan:** What target or intervention should the player consider, and what
   range of outcomes is plausible?
5. **Review:** After the next saves, did the republic move as intended?

## Primary outcomes

The default Republic Briefing is deliberately narrow:

- **Plan attainment** — progress against explicit player targets at the current
  point in the plan.
- **External dependency** — exposure to imported critical resources, currencies,
  and concentrated export income.
- **Demographic resilience** — births and immigration relative to deaths and
  escapes, with citizen welfare and productivity as guardrails.

The full dashboard exists to explain these outcomes. It must not flatten every
available statistic into an equally prominent card.

## Experience areas

### Republic Briefing

The first view reports the latest distinct observation, the most consequential
save-to-save changes, emerging statistical signals, plan variance, and a short
Ministry Dispatch. It answers the player's most urgent questions before any
filter is touched.

### Republic Monitor

Observer Health answers whether the desktop recorder is noticing and safely
retaining completed saves. It separates queue state, retries, terminal failures,
duplicate payloads, and normal absence of a new game save. Republic Pulse then
shows actual observation spacing, latest same-branch receiver movement, snapshot
coverage, and branch warnings. “Near-live” means after a stable save—not process
memory, injection, or frame-live telemetry.

### Five-Year Plan

Players define bounded count targets, dates, guardrails, and one of three
deterministic schedules. Plans are anchored to an exact save, scoped to one
branch, and retained as immutable revisions in SQLite. Actual and scheduled
progress remain separate; historical previews stop at their exact head and
missed observations are never silently filled. Forecasts remain a later,
separate model family with intervals and explicit model versions.

### Material Periodic Table

Resources are arranged into stable families: raw, intermediate, construction,
consumer, fuel, utility, vehicle, and waste. Each cell provides a current value,
a small trend, and a selected lens such as import reliance, price movement,
volatility, or use. The table is both an analytical index and a playful visual
signature.

### Industrial Laboratory

Game-definition recipes form a directed material network. Desired output can be
propagated upstream to show theoretical requirements, limiting inputs, worker
and energy constraints, price sensitivity, and eventually optimisation. Actual
yield is offered only when the save evidence covers both input and output.

The chemistry language is intentional play: pseudo-elements, reaction routes,
limiting inputs, yield, titration-style sensitivity, and shock response make
administration memorable. The Observatory clearly distinguishes those
metaphors from actual chemical and thermodynamic laws.

The first source-backed Materials slice precedes those models: a searchable
catalogue of installed resources, buildings, vehicles, construction phases,
production relations, and capabilities. Player overlays preserve installed
originals and present every assumption as `original → override → effective`.

### Broadcast Desk

Radio and television become a first-class administrative workspace. Receiver
adoption supplies the supported starting point; station audience, programme
mix, influence profiles, staffing, cost, and lagged citizen outcomes expand as
evidence becomes available. A Broadcast Notebook records interventions, while
an earnest-with-a-wink Evening Bulletin narrates only deterministic,
evidence-linked findings.

### Population, Welfare, and Cities

The familiar in-game population trends are retained as a reference layer, then
extended through small multiples, rate normalisation, demographic
decomposition, control limits, city comparisons, and save-annotated
interventions. National averages must not conceal a failing settlement.

The first implemented Population slice is deliberately narrower: direct
republic status and education counts, recorded movement counters, and one
numeric city-source assay at the exact selected save. It does not infer source
windows, denominators, city names, or individual lives. Citizen and family
histories remain behind a documented stable-identity evidence gate rather than
being approximated from row position or names.

### Trade, Markets, Debt, and Tourism

The Observatory separates quantity, value, currency, price, concentration, and
volatility. It supports price baskets, terms of trade, break-even calculations,
tourism yield, and debt-service stress without presenting unlike currencies or
physical units as naturally additive.

### Community Extensions

Players can add locally obtained Analysis Packs that declare bounded
calculations and chart templates over published normalised metrics. Inspection,
import, enablement, rollback, export, and removal are separate local operations;
imported packs begin disabled. The host retains calculation, evidence,
accessibility, and rendering authority. Advanced
executable Model Plugins remain out of process and unavailable until a real
model and security review justify a public protocol.

## Fun as a design requirement

Administrative depth and delight are compatible. The product should include:

- Ministry Dispatches that narrate only statistically supported changes;
- Evening Bulletins that balance sincere administration with a restrained
  newsroom wink;
- plan medals and player-defined milestones;
- import-free and stability streaks;
- republic records and then-versus-now congress reports;
- an animated material and demographic history; and
- an explainable “economic weather” summary whose underlying measures are one
  interaction away.

These features may celebrate or invite investigation. They must never conceal
bad news, invent a causal story, or turn opaque scores into authority.

## Product promises

1. Saves remain untouched.
2. Ordinary use remains local and offline.
3. Rollbacks and forks remain separate timelines.
4. Facts, calculations, estimates, and recommendations remain distinguishable.
5. Unknown and incomplete data remain visible rather than imputed silently.
6. Every chart answers a player question and exposes its units and time window.
7. Advanced statistics provide uncertainty and assumptions, not false precision.
8. Useful in-game graphs may be reproduced when they strengthen the complete
   decision workflow.
9. First-party and community extensions use the same bounded public contracts.
10. The interface can be translated without changing save identity, metric
    meaning, evidence classification, or extension authorship.
11. Game-version mappings remain inspectable, inert, versioned evidence; local
    repairs never masquerade as reviewed first-party facts.

## Non-goals for the first releases

- Modifying game state or saves
- Real-time memory inspection or process injection
- Claiming complete factory, route, worker, or inventory telemetry before the
  relevant binary payloads are documented
- Hosted accounts, social comparison, or global leaderboards
- Executing community code or exposing database connections, SQL, private paths,
  raw saves, or renderer configuration to extensions
- An executable Model Plugin runtime before a demonstrated model and security
  review justify its contracts
- Automated play or prescriptive “optimal republic” judgement
