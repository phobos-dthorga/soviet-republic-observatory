# Development quality gates

Republic Observatory orders checks by cost so an ordinary feature does not pay
for repeated Windows release links and native review runs while code is still
changing.

## The three tiers

### Fast contracts

```powershell
npm run verify:fast
```

Use this throughout implementation. It checks formatting, the enforced build
workflow, localisation, architecture boundaries, the Tesmio boundary, Svelte
types, frontend unit tests, Rust formatting, and a Rust compile check. It does
not create a release binary or open native review.

### Browser interface

```powershell
npm run verify:browser
```

Run this once the feature's states and layout have settled. It creates the
production web artifact and runs the Playwright geometry, contrast, Axe, state,
theme, viewport, and text-scale matrix. Fix all reported states together before
rerunning it.

### Final desktop gate

```powershell
npm run desktop:build
```

Run this once at the end of a completed slice. The gate is fail-fast and uses
this order:

1. fast contracts;
2. Rust tests;
3. Rust Clippy;
4. browser interface audit;
5. one Windows desktop package; and
6. native smoke review.

The expensive package cannot start unless every earlier phase passes. The
package reuses the web artifact produced by the immediately preceding browser
phase, so Tauri does not rebuild or rerun the browser matrix. A structured
timing report is written to ignored
`artifacts/release-gate/last-run.json`. Inspect the sequence without doing work
with:

```powershell
npm run verify:release:plan
```

The build-workflow audit is part of `npm run check`. It fails if script or Tauri
configuration changes merge these tiers, move packaging before an earlier
gate, or drop native smoke from the final sequence.

## Reusing an existing native binary

```powershell
npm run desktop:smoke:existing
```

This narrow command is appropriate only when the changed files are confined to
the WebDriver review harness or its typed scenarios. It does not rebuild the
application and is therefore invalid after any application, Rust,
configuration, dependency, or bundled-asset change.

The exhaustive native matrix remains a special laboratory tool:

```powershell
npm run ui:review -- run --suite full
```

Use it for major theme, accessibility, responsive-layout, native-control,
native-shell, review-infrastructure, or release-candidate work. Ordinary feature
work ends with native smoke.

## Failure discipline

When a gate fails, stop at that tier. Read all named findings and screenshots,
batch related fixes, rerun the cheapest affected gate, and return to the final
desktop gate only after the implementation is stable. The final gate records
the failed phase and elapsed time and deliberately skips every later expensive
phase.

## Deferred optimisation reserve

Long-running phases are not defects merely because they take time. The first
complete timed run on 31 August 2026 took 291 seconds; desktop release linking
accounted for 178 seconds, principally around the bundled native DuckDB build
and the production release profile. This is reference-machine evidence, not a
duration promise or a current optimisation project.

Keep the following options in reserve if build duration later becomes a
material development or CI constraint:

- retain comparable warm and clean timing histories and flag meaningful phase
  regressions against a rolling baseline;
- benchmark alternative Cargo release settings, including LTO and codegen-unit
  choices, while measuring binary size, startup, and analytical runtime as well
  as link duration;
- investigate safe compiler and dependency-cache reuse before weakening the
  production profile; and
- separate ordinary native review from a stricter release-candidate profile
  only if evidence shows the distinction saves material time without hiding
  release-only defects.

Do not implement this machinery pre-emptively. Revisit it only when comparable
timings show a sustained regression, the release linker materially impedes
delivery, or CI duration becomes an operational problem. Correctness and
diagnostic visibility take priority over making every phase short.
