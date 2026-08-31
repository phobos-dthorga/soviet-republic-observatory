# ADR-0021: External native UI review with bounded fixture scenarios

## Status

Accepted.

## Context

Browser audits found colour, overflow, and broad responsive defects but did not
catch a collapsed guidance surface or Windows-owned native-control behaviour.
The player does not want development automation to take over the global mouse.
The available WyrmGrid repository provides deterministic-gallery and strict-CLI
precedents, but no general UI-driving command line that can be reused.

## Decision

Keep Playwright as the fast interface gate and add an external Windows native
review using Tauri's supported `tauri-driver` route, pinned WebdriverIO tooling,
and Microsoft Edge WebDriver. Automation locates production elements through
the WebView and sends element/keyboard operations. It does not use global
screen coordinates or mouse injection.

The production binary contains no automation server or control plugin. Review
mode is entered only through strict startup options naming a safe run ID, one
fixture/live state, and a CLI-marked root beneath a dedicated OS-temporary
directory. Rust suppresses autonomous services and returns a bounded
`UiReviewContext`. Only then does the frontend register an allowlisted
scenario/theme/scale/readiness controller.

Fixture scenarios use typed synthetic host models with real workspaces,
dialogs, charts, notifications, guidance, and task components. Browser and
native review serialize the same geometry and computed-style contrast
auditors. Screenshots are diagnostic evidence; measurable geometry, Axe,
contrast, and state-contract failures block delivery.

The explicit live mode operates only on a temporary clone while the ordinary
application is closed. It never attaches to a running process, opens configured
save/game directories, mutates source databases, or terminates the player's
application.

## Consequences

- Native visual review remains mouse-free and cannot become an end-user remote
  control surface.
- Release builds require explicit pinned developer-tool setup; they never
  download test tools silently.
- Windows is the first native gate, while scenario and audit contracts remain
  portable.
- Native screenshots may vary slightly and are retained as diagnostics rather
  than brittle pixel-perfect approval baselines.
- Live artifacts are potentially sensitive, local-only evidence.
- The browser audit remains necessary because it is faster and isolates
  presentation regressions from native-driver failures.
