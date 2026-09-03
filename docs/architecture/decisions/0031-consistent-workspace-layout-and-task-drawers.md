# ADR 0031: Consistent workspace layout and task drawers

## Status

Accepted.

## Decision

Every Observatory workspace follows one placement order:

1. the workspace title and maintenance actions;
2. contained section navigation in the left rail;
3. a section heading that states the answer or current status first;
4. filters directly above the result they change;
5. related records and source details after the result; and
6. multi-step creation or calculation work in a side task drawer.

The shared `WorkspaceSectionHeader`, `WorkspaceToolbar`, `ScopedFilterBar`, and
`WorkspaceTaskDrawer` components own this structure. Long workspace pages remain
the normal reading surface. A drawer preserves the page beneath it, keeps its own
scrolling contained, and becomes the full work surface on a narrow display.

The shell owns the typed task route and session-only drawer trail. Escape and
`Alt+Left` close one task layer after any open confirmation or dialog has been
handled. Existing draft guards decide whether closing needs confirmation. A
successful close returns focus and context to the section that opened the task.

Maintenance commands stay in the workspace heading. Viewing filters stay beside
their chart or table. Destructive controls must live in a separate management
task and may not share an action group with routine work.

## Contributor rule

A new workspace must use the shared page heading and expose contained section
links. A multi-step editor, scenario, comparison, or calculation must register a
typed `WorkspaceTaskRoute`, open from its owning section, and have a deterministic
review scenario. Do not position these workflows with one-off fixed panels,
document scrolling, or arbitrary overlays.

Short one-step filters and selection controls are not tasks. Keep them next to
the result they affect. Source details may remain inline below the result when
they are primarily read-only.

## Enforcement

The workspace layout audit checks all ten workspaces, the typed route registry,
task ownership, deterministic review fixtures, shared page headings, and
destructive-action separation. The browser and native DOM auditor checks visible
page headings, current-section identity, contained task scrolling, readable
heading measure, and filter placement at every supported viewport and text scale.

## Consequences

Players can predict where to start work and where to find its result across the
application. The extra shell state is session-only and changes presentation, not
save evidence, calculations, or game files.
