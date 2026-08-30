# Accessibility, contextual guidance, and notifications

Republic Observatory treats accessibility and explanation as application
infrastructure. They are not workspace-by-workspace decoration to be added
after the analytical surface has grown.

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

Inline validation remains beside the affected field or operation. Critical
task progress remains in the shared task-progress system. Notifications report
an outcome; they do not replace durable diagnostics, evidence, or progress.

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

Republic Observatory currently validates and retains community packs in its
frontend app-local storage. WyrmGrid's mature Rust-and-SQLite validation and
persistence boundary remains the target for a later native slice. Documentation
and interface wording must not imply that native authority already exists.

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
