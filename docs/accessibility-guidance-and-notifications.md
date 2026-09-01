# Accessibility, contextual guidance, and notifications

Republic Observatory treats accessibility and explanation as application
infrastructure. They are not workspace-by-workspace decoration to be added
after the analytical surface has grown.

## Attention cues

Important new or newly enabled actions use the shared `AttentionCue` contract
instead of workspace-specific glow effects. A cue combines visible localised
guidance with a bounded three-cycle outline. Reduced-motion users receive the
same persistent outline and message without animation, and forced-colour mode
uses the operating-system highlight colour. Dismissal is stored by stable cue
ID and content revision in SQLite; revised guidance reappears automatically and
users can explicitly replay existing guidance.

The default cue layout is compact: its persistent explanation is bounded and
its pulse traces the actual interactive target, not the full width of whatever
panel happens to contain it. A deliberate wide layout remains available for a
real full-width target. This distinction must be checked at desktop and narrow
viewports whenever a cue is introduced.

Cues never carry validation or business policy. They render only when the
owning service has already established that the highlighted action is
available. Essential warnings remain visible outside the cue.

## Readable typography

The interface uses named type tokens. Ordinary captions and controls have a
12-pixel-equivalent floor at the default browser scale; body copy is larger.
New components must use the tokens rather than introduce 7–11 px microtext.
Browser and operating-system text scaling must remain usable, and dense layouts
must reflow or scroll rather than shrink important text to make it fit.

Text size does not replace the other interface requirements: sufficient
contrast, semantic structure, visible keyboard focus, logical properties for
right-to-left layouts, reduced motion, textual chart summaries, and non-colour
state distinctions remain required.

## Automated interface-scale assurance

The production build runs one app-wide Playwright gate rather than relying on
workspace-specific visual memory. Every enabled workspace is measured at
narrow, laptop, FHD, QHD, ultrawide, and UHD-equivalent logical resolutions,
including text/UI scales from 100% to 200%. The geometry assay rejects global
horizontal overflow, landmarks or dialogs escaping the viewport, overlapping
dialog regions, and enabled controls below a 24 CSS-pixel target floor.

Repeated peer cards with equivalent actions use the aligned-action geometry
contract. Cards in the same visual row reserve flexible space for differing
descriptions and translations so their action controls share one lower edge;
stacked responsive rows remain independent. New peer-card action groups must
declare the contract rather than relying on equal English copy. The shared DOM
auditor verifies it in both browser and native review at every supported
viewport and text scale, and a deliberately drifting fixture proves that the
gate fails when alignment regresses.

Transient native-only states must remain testable through a real host-owned
component state, a bounded native-command mock, or an application-owned assay;
tests must not draw a separate visual imitation. Critical running/failed task
indicators and successful research-build results have deterministic screenshot
baselines. The completed research assistant deliberately snaps its scroll
position to a whole prerequisite row or result boundary, never a half-visible
heading. Contrast, Axe, reduced-motion, and Windows-native popup checks remain
separate mandatory layers because geometry alone cannot establish usability.

## Context-aware help

`ContextHelp.svelte` is the application-owned help primitive. It provides a
keyboard-focusable and pointer-accessible explanation using a real tooltip
relationship. Each help point has a stable `data-help-topic` identifier. Those
identifiers are the future tutorial anchors; a guided tour can locate existing
help without inventing a second explanation system.

Contextual help is suitable when a control has a consequential lifecycle,
special evidence meaning, unfamiliar statistical term, or safety boundary. A
tooltip must not contain an action, conceal a required warning, or be the only
way to learn information necessary to complete a task. Essential wording stays
in the page or dialog.

### Metric context standard

Analytical values use the host-published Metric Context contract whenever a
reader could reasonably mistake the counted basis, time window, denominator,
scope, or comparison rule. Cards and consequential form fields render that
contract through `MetricContextHelp.svelte`; charts pass the same content to
the application-owned chart frame. All use the same question-mark trigger,
keyboard/focus behaviour, topic identity, guidance surface, and ordered detail
ledger.

Use the complete context for a KPI, plan target, derived share, source counter,
or chart. A short plain-language hint remains appropriate for a familiar
control whose meaning has no statistical boundary. A metric tooltip is never
used to conceal the displayed unit, evidence class, required scope, warning,
or unavailable state. If the contract does not describe a composite chart or
city-specific value honestly, omit the generic tooltip and provide a dedicated
visible explanation instead.

### Guidance surfaces

Inert previews, context tooltips, and tutorial/help messages use one
host-owned guidance-surface convention. The host derives a translucent tint
from the theme's validated `success` role, composites it over permitted
surfaces, and validates both ordinary and muted text against the result. This
green-family tint is a visual affordance for explanatory material in the
built-in themes; it does not claim that the subject succeeded.

`GuidanceSurface.svelte` is the reusable markup primitive for ordinary
workspace guidance. Context tooltips and Attention Cues retain their specialised
accessible behaviour while consuming the same `.guidance-surface` presentation
contract. Every consumer declares one bounded semantic kind through
`data-guidance-surface`: `help`, `instruction`, `preview`, or `boundary`.
This inventory lets the production interface audit confirm that the convention
has not drifted into another collection of workspace-specific callouts.

Use the contract for explanatory side notes, evidence and interpretation
boundaries, read-only setup instructions, and visibly inert previews. A preview
must contain no enabled control. Do not use it for an active analytical context,
validation state, live task progress, success outcome, warning, error, or legal
disclaimer merely because a tinted panel would be convenient.

Guidance remains distinguishable without colour: tooltips retain semantic
relationships, previews say **not interactive** and contain no focusable
controls, and tutorial messages remain visibly separate from their real action
buttons. Warning and error surfaces do not adopt the guidance treatment.
Colour-harmony or complementary-colour rules are not admission criteria because
they are subjective and culturally variable; measurable contrast, state
distinction, colour-vision resilience, and non-colour labels remain authoritative.

## First-class notifications

Transient cross-workspace feedback goes through the application notification
service and the shell-owned notification centre. The service:

- accepts host-localised title and message text plus `info`, `success`,
  `warning`, or `error` tone;
- keeps a bounded five-item visible queue;
- automatically retires informational and successful notices;
- leaves errors available until the user dismisses them;
- exposes text, a glyph, and an accessible live-region role instead of relying
  on colour; and
- never contains domain calculations or changes application state beyond the
  notification queue.

Repeated outcomes from one operation use a stable deduplication key, updating
the existing notice instead of flooding the queue. This is presentation
coalescing only: every durable failure attempt may still appear separately in
the bounded native diagnostic record.

Inline validation remains beside the affected field or operation. Critical
task progress remains in the shared task-progress system. Notifications report
an outcome; they do not replace durable diagnostics, evidence, or progress.

Failure messages use the same player-first order everywhere:

1. what happened;
2. what remained safe;
3. what the player can do next; and
4. expandable technical details when a code or operation name helps with
   troubleshooting.

Raw `snake_case` codes never lead an ordinary notification, task panel, dialog,
or inspector. The shared `TechnicalDetails` component keeps those values
keyboard-accessible without making them look like controls. Diagnostics uses a
plain summary for each entry and places its exact code, operation, and original
message inside the same disclosure.

Analysis Pack actions and language-pack installation/selection are the first
consumers. New workspaces should reuse this service when an outcome must remain
visible after attention moves away from the initiating control.

## Language packs

The community-localisation design follows the useful WyrmGrid boundary:
canonical `en-AU`, Project Fluent message patterns, inert versioned
`.rolanguage.json` files, protected safety namespaces, bounded validation,
partial-pack coverage, and per-message English fallback. End users can author
packs from the public schema and example, install them through **Language**, and
select them independently.

Republic Observatory validates and retains community packs through its
authoritative Rust-and-SQLite boundary. The frontend requests bounded lifecycle
operations and renders returned models; it does not decide whether a pack is
safe, compatible, installed, or selected.

## When to make a facility first-class

Ask during implementation and review: **does this function need to be
genericised or made first-class?** Promote it when at least one concrete signal
exists:

- the same behaviour has two real consumers;
- state must survive navigation or coordinate multiple workspaces;
- correctness, accessibility, security, provenance, or failure isolation must
  be uniform;
- a host-owned policy boundary would otherwise be reimplemented in
  presentation code; or
- a long-running or high-cardinality operation needs shared progress,
  cancellation, limits, or diagnostics.

Do not build a framework for hypothetical reuse. Extract the smallest stable
contract demonstrated by current behaviour. Existing examples include
localisation and formatting, chart rendering, modal focus, critical-task
progress, notifications, contextual help, and governed DuckDB writes.

## Debugging tools

Contributors and coding agents are explicitly authorised to use appropriate
debuggers, profilers, browser developer tools, database inspection tools, trace
captures, and other diagnostic facilities whenever they are the preferable way
to understand a defect or performance problem. Their use does not require a
separate request.

Debugging remains subject to the project's boundaries: never mutate saves,
never commit personal paths or captured game data, do not disclose private
local content, and keep user-visible diagnostic exports bounded and reviewable.
