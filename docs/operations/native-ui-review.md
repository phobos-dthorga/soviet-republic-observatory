# Native UI review

Republic Observatory has two complementary interface gates:

- the browser audit is the fast renderer-independent check; and
- the native review opens the packaged Tauri window through Windows WebDriver.

The native route does not move the global mouse, inject screen coordinates, or
attach to an already-running Observatory. It launches a separate review
process through `tauri-driver` and Microsoft Edge WebDriver, exercises fixed
allowlisted scenarios with keyboard and WebDriver element operations, and then
closes only the process it created.

## WyrmGrid findings

The available OnAir WyrmGrid repository and its accessible history do not
contain a general-purpose UI-driving CLI. The useful precedents are narrower:
strict startup-option parsing, deterministic galleries that use real renderers,
background-work suppression during visual review, and repeatable local commands
with bounded output. The Observatory retains those principles and adds Tauri's
supported external Windows WebDriver route.

No automation server, remote-control endpoint, arbitrary selector interface,
or test plugin is compiled into the production application. The review-only
frontend controller appears only after the native host validates a marked
temporary review root.

## Setup and commands

Install the pinned native test tools explicitly:

```powershell
npm run ui:review:setup
```

This installs `tauri-driver` 2.0.6 and downloads the Edge WebDriver matching the
installed WebView2 runtime beneath ignored `.tools/native-ui-review/`. Ordinary
builds never download or update these tools.

```powershell
npm run ui:review -- list
npm run ui:review -- run --suite smoke
npm run ui:review -- run --suite full
```

The smoke suite covers representative workspaces, warehouse attention, missing
probe evidence, failure progress, dialogs, notifications, contextual help,
Attention Cues, keyboard focus, and a native select interaction. The full suite
crosses every registered scenario with seven viewport/text-scale cases,
Classic, High Contrast, and a Rust-generated validator-boundary theme. It is a
deliberately exceptional laboratory gate for releases and major theme,
accessibility, native-shell, responsive-layout, or review-infrastructure work;
it does not run on every pull request or push. Trigger the Windows workflow
manually when that depth is warranted.

`npm run desktop:build` is the final release gate. It runs fast contracts, Rust
tests and Clippy, the browser interface audit, one packaged binary build, and
then native smoke. The binary build reuses the audited web artifact. A missing
or incompatible toolchain fails with the exact `npm run ui:review:setup`
remediation.

When only this review harness or its scenarios changed, and no application,
native, configuration, or bundled asset changed, `npm run
desktop:smoke:existing` reuses the already-built binary. This is not a release
substitute. See the [development-gate guide](development-gates.md) for the
authoritative tier boundary.

## Fixture and live review

Fixture mode is the default and authoritative build gate. Rust suppresses save
watchers, compatibility watchers, catalogue refreshes, warehouse workers, and
other autonomous background work. Scenarios feed typed synthetic host models
into the production workspaces and components; they do not render visual
imitations.

An explicit developer-only live clone is available:

```powershell
npm run ui:review -- live --acknowledge-live-data
```

Live review refuses to start while an ordinary Observatory process is open and
never terminates it. It copies only app-local SQLite/DuckDB files and the one
local compatibility profile into a marked temporary review root. It does not
open configured save or game-definition directories. Source database hashes
are checked before and after the run. The original state is never passed to the
review process.

Live artifacts may contain private republic information. They are marked
potentially sensitive, remain local, and must be reviewed before sharing.

## Outputs and privacy

Each run writes ignored local evidence beneath
`artifacts/native-ui-review/<safe-run-id>/`:

- screenshots for each reviewed state;
- `findings.json` and `summary.md`;
- Axe, computed-style contrast, and geometry findings;
- bounded driver and application diagnostics; and
- a startup-state screenshot if the controller never becomes ready.

Reports redact the repository, temporary review, and user-profile paths. Logs
are bounded to 250 KiB. Normal completion and ordinary failures remove the
temporary data root. Marked abandoned roots older than 24 hours are recovered
at the next run; unmarked, malformed, linked, or unexpected directories are
never removed.

## Safety boundary and troubleshooting

Review options are strictly parsed. The root must be a direct marked child of
the Observatory's dedicated operating-system temporary directory; traversal,
unsafe IDs, mismatched markers, symlinks, and Windows reparse points fail
closed. The frontend controller exposes only scenario, validated theme,
100–200% text scale, and readiness operations. It exposes no arbitrary
JavaScript, Tauri invocation, SQL, filesystem operation, or selector API.

If a run fails:

1. read `summary.md` and `findings.json`;
2. inspect the named screenshot;
3. check `startup-state.json`, `native-review.log`, and the bounded app log;
4. rerun `npm run ui:review:setup` only when the message reports missing or
   incompatible tools; and
5. close the ordinary app before an explicit live review.

The native driver uses an ephemeral loopback port and is independent of Vite's
port 1420. Port races, driver startup failure, and an app that exits before its
WebView is ready are reported as failures rather than retried indefinitely.
