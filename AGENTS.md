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
- Optional same-process research companions remain separately built and
  installed, fail closed on unreviewed executable identities, and cannot become
  dependencies of normal save observation. “Read-only” describes their data
  behaviour; never call process instrumentation an operating-system sandbox.
- Changes to native research, third-party licensing, affiliation, local-data
  handling, or warranty boundaries must update the first-class Legal & notices
  screen and the repository legal notice in the same slice.

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
- Keep recorder liveness in the native host. Filesystem events are wake-up hints;
  periodic directory reconciliation remains authoritative, and Svelte never
  drives observation with a required heartbeat.
- Keep the app-local SQLite database unencrypted while it contains no secrets.
  Do not add SQLCipher or application key management without a concrete threat
  model; future credentials belong in an operating-system credential vault.
- Reduce duplicate calculations and magic field names as soon as their shared
  meaning is established; do not generalise hypothetical requirements.
- During implementation and review, explicitly ask whether a function should be
  genericised or made first-class. Extract the smallest stable contract when
  there are at least two concrete consumers, cross-workspace state, or a shared
  accessibility, security, provenance, progress, or failure-isolation policy.
- Debuggers, profilers, browser developer tools, database inspection tools, and
  trace captures are authorised whenever they are the preferable way to
  diagnose behaviour. Preserve the save-safety, privacy, and repository-content
  boundaries while using them.

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
- Ordinary captions and controls use the shared readable type tokens and have a
  12-pixel-equivalent floor at default scale. Reflow rather than shrinking
  meaningful text to fit.
- Use the application notification service for transient cross-workspace
  outcomes, the critical-task system for progress, and inline messages for
  field-level validation. Do not create workspace-local toast systems.
- Contextual help uses the shared keyboard-accessible help primitive and stable
  tutorial topic IDs. Essential instructions and warnings must remain visible
  without opening a tooltip.
- New or newly enabled actions that need emphasis use the shared AttentionCue
  primitive. Cues require stable IDs and content revisions, bounded motion,
  reduced-motion and forced-colour behaviour, persistent dismissal, and replay;
  they never decide whether a domain action is valid.

## Localisation

- All user-facing host text, chart prose, textual chart summaries, and
  locale-sensitive values use the canonical localisation runtime and central
  formatting helpers. Do not add a parallel string or formatting path.
- Use literal translation keys or an explicit typed mapping. Constructed keys
  and sentence keys are prohibited.
- Keep raw save fields, source spellings, branch IDs, metric IDs, extension IDs,
  file names, and diagnostics unchanged; accompany them with translated context
  when useful.
- Keep Observatory UI language separate from installed-game vocabulary and
  extension-authored prose. Never use translated display text as stored
  identity or a calculation key.
- Language packs are inert data. They cannot provide markup, callbacks, code,
  renderer options, URLs, paths, capabilities, trust, evidence classification,
  or safety policy.
- Installation and selection are separate. A failed language pack falls back
  per message to canonical `en-AU` and cannot block the application.
- Additive catalogue messages advance the source revision. Removed, renamed, or
  incompatibly changed messages advance the compatibility version.
- New or changed language contracts require schema, semantic, variable-parity,
  fallback, pseudo-language, RTL, keyboard, and narrow-layout checks in
  proportion to the change.

## Quality gates

- Run formatting, Svelte type checking, JavaScript unit tests, the production
  webview build, Rust formatting/check/tests/clippy, and the desktop build when
  the native boundary changes.
- New calculation rules require successful, boundary, unavailable-data, and
  invalid-input tests.
- Parser changes require sanitised fixtures and compatibility notes.
- Chart-family additions require a documented analytical question and visual QA
  at desktop and narrow widths.
- Analysis Pack changes require strict schema tests, semantic reference tests,
  limit and injection fixtures, and compatibility notes.
- `npm run check` includes the localisation audit and must remain a required
  gate.
