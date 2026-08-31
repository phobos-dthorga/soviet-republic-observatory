# Safe community themes and contrast assurance

Republic Observatory themes are inert colour-role documents. A theme can alter
the Observatory's validated semantic colours and chart palette; it cannot alter
layout, typography, navigation, behaviour, data, evidence, calculations, or
rendering code.

## Public contract

The `.rotheme.json` contract is defined by
[`schemas/theme-v1.schema.json`](../schemas/theme-v1.schema.json). Version 1
accepts identity metadata, twelve semantic colour roles, and three to eight
chart colours. Colours must be complete six-digit hexadecimal values. Unknown
fields fail validation and the native host rejects documents larger than 32
KiB.

CSS, HTML, JavaScript, selectors, expressions, callbacks, fonts, images, URLs,
paths, opacity values, layout instructions, external resources, renderer
configuration, and ECharts options are outside the contract. Secondary tokens
such as soft fills, dividers, focus treatments, and overlays are derived by the
host.

## Authoritative validation

Rust validates the structure and returns a complete `ThemeValidationReport`.
It tests ordinary and muted text against every permitted surface at 4.5:1,
semantic text colours at 4.5:1, controls and meaningful graphics at 3:1,
decorative dividers at 1.5:1, chart marks against chart and tooltip surfaces at
3:1, and host-derived translucent fills after compositing them over their
permitted surfaces.

Colour-vision simulations produce advisory chart-distinction warnings. A
warning does not make colour a sufficient encoding: charts retain labels,
styles, accessible ledgers, and textual summaries.

The authoring laboratory displays the native report. A structurally valid but
low-contrast draft can be exported for more work, but cannot be imported or
selected.

## Lifecycle and provenance

Theme revisions are immutable in SQLite. Inspection, validation, import,
selection, export, rollback through revision selection, and removal are
separate actions. Selection pins the exact ID, semantic version, and content
hash. Import never silently changes the active theme.

Built-in and local provenance remain visible. Community author metadata is an
unverified presentation claim. The host rejects conflicting content under an
existing ID and version, exact duplicate content, an identical visual role set
under another identity, and use of the reserved `org.republic-observatory`
namespace.

If a selected local revision is missing, corrupt, incompatible, or no longer
valid, startup atomically selects **Republic Observatory Classic**, notifies the
player, and preserves the invalid row for diagnosis. Local themes are offline,
app-local, and unencrypted.

## Build assurance

Every production web build runs the complete Playwright interface audit after
bundling. Its contrast-and-Axe layer exercises every enabled workspace and
representative dialogs, forms, focus, disabled controls, native options,
notifications, charts, empty states, and error/loading surfaces under both
built-ins and a generated validator-boundary theme. Host-owned state assays
also keep otherwise transient running and failed task states reachable by the
same production-bundle audit. Failures retain a screenshot, trace, and JSON
report with the component, selector, foreground/background, measured ratio,
and required threshold.

The geometry layer covers narrow, laptop, FHD, QHD, ultrawide, and
UHD-equivalent layouts from 100% through 200% text/UI scale. It rejects document
overflow, escaping landmarks or dialogs, dialog-region overlap, and interactive
targets below the 24 CSS-pixel floor. Deterministic screenshots protect the
critical-task states and completed research-result layout which motivated this
gate. Browser automation supplements rather than replaces the Windows-native
smoke check below.

The audit does not pretend to see the operating-system-owned open dropdown
popup. Desktop release validation therefore retains a Windows-native smoke
check of the expanded popup. There are no workspace-wide contrast exceptions;
any temporary exception must identify one exact element, state a reason and an
expiry, and cannot weaken the native theme validator.
