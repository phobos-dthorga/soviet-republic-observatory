# Republic Observatory contributor instructions

## Product boundaries

- Treat game saves as read-only observations. Never modify, replace, rename, or
  delete a player's save.
- Never commit save archives, extracted save payloads, personally identifying
  paths, or unlicensed game assets.
- Do not claim official support or affiliation with 3Division or Hooded Horse.
- Preserve the distinction between save facts, game-definition facts, derived
  calculations, forecasts, and recommendations using provenance metadata.
- Unsupported or incomplete fields must remain visibly unavailable. Never infer
  a value merely to keep a chart populated.

## Architecture

- Keep Svelte components presentational. Parsing, deduplication, branching,
  metric rules, forecasts, and recommendations belong in testable services.
- Keep direct Apache ECharts usage inside the chart adapter. Other components
  consume a small application-owned chart specification.
- Keep the scanner, parser, storage, analytics, and presentation layers
  replaceable through stable application-owned models.
- Store observations locally. A future network feature must remain optional and
  must not become necessary for ordinary use.
- Extensions receive only bounded normalised observations and versioned public
  game-definition models. Never expose raw saves, binary payloads, SQLite,
  parser internals, private paths, or credentials through an extension API.
- The host owns chart resolution, ECharts options, themes, accessibility,
  provenance, limits, settings, and extension failure states. Extensions never
  submit JavaScript, Svelte, HTML, CSS, callbacks, SQL, or renderer options.
- Preserve the external-delivery invariant: first-party and community
  extensions use the same public contracts and can be obtained and installed
  independently of the application source tree.
- Capabilities are deny-by-default and bind exact extension ID, version,
  content identity, and scope. Installation, approval, enabling, starting,
  updating, rollback, and removal are separate lifecycle actions.
- Out-of-process execution is failure containment, not an operating-system
  sandbox. Do not describe it as one.
- Treat database migrations as append-only once released.
- Reduce duplicate calculations and magic field names as soon as their shared
  meaning is established; do not generalise hypothetical requirements.

## Statistical honesty

- Plot observations against their actual in-game dates; do not assume equal
  spacing between records or saves.
- Keep rollback and forked saves on separate branches. Never splice them into a
  single apparent timeline.
- Hash and deduplicate identical statistical payloads.
- Do not describe `Resources_Produced` as complete production or claim a closed
  material balance until inventory and production coverage are proven.
- Correlation, intervention studies, and forecasts must state their assumptions
  and must not be presented as deterministic or causal findings.
- Normalised measures must expose their denominator, window, and units.

## Interface

- Lead with the republic brief, then movement, drivers, and exact detail.
- Keep provenance, observation time, coverage, and caveats close to the chart
  they qualify.
- Use semantic colour tokens and at least one non-colour distinction for state.
- Respect reduced-motion preferences and provide accessible textual summaries
  for every chart.
- Empty, loading, unavailable, partial, and error states are distinct.

## Quality gates

- Run formatting, Svelte type checking, unit tests, and the production build.
- New calculation rules require successful, boundary, unavailable-data, and
  invalid-input tests.
- Parser changes require sanitised fixtures and compatibility notes.
- Chart-family additions require a documented analytical question and visual QA
  at desktop and narrow widths.
- Analysis Pack changes require strict schema tests, semantic reference tests,
  limit and injection fixtures, and compatibility notes.
