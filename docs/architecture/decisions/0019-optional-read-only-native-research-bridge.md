# ADR-0019: Optional read-only native research bridge

## Status

Accepted for a bounded experiment.

## Context

Supported saves expose useful aggregate population facts but do not yet prove
stable citizen identity, family relationships, or several live status fields.
TesmioLoader can instrument the running game and upstream research has candidate
`Person` layouts. It is same-process native code, GPL licensed, and version
sensitive. Making it a normal product dependency would weaken the Observatory's
local, save-first reliability boundary.

## Decision

Republic Observatory may consume telemetry from one optional GPL-3.0-only
research companion under these rules:

1. The main application remains functional without TesmioLoader.
2. The companion is built and installed separately; Observatory never silently
   installs or activates a loader or DLL.
3. TesmioLoader is treated as a general modding platform, not as inherently
   read-only. A research launch requires the reviewed observation-only host
   settings, the companion as the sole plugin DLL, and a passing preflight
   verifier. Save manifests, VFS, built-in probes, menu changes, gameplay
   plugins, and version bypass remain off.
4. The companion performs bounded reads only and emits one fixed local JSONL
   file. It has no save, game-file, database, or network write path.
5. One chainable IAT observation hook is permitted for this experiment. Inline
   hooks, executable allocations, and game-state patches are forbidden.
6. Exact reviewed executable identity is a startup precondition. Unsupported
   builds fail closed before the observation hook is installed.
7. Rust derives the telemetry location from the configured game folder, rejects
   links or escapes, imposes independent size/line/schema bounds, and returns
   aggregate status only.
8. Telemetry is not imported into SQLite or DuckDB in this slice.
9. Samples have no public subject identifier. They cannot form biographies,
   family graphs, or causal claims.
10. The interface provides a first-class legal and technical-risk screen. It
    explicitly says that same-process execution is not an OS sandbox.
11. Source acquisition may retain the exact reviewed headers and upstream
    licence after a separate player-confirmed GitHub download. It never obtains
    a loader binary, accepts an arbitrary URL, installs, or activates anything.

## Consequences

Reverse engineering can advance without making saves fragile or making
TesmioLoader a dependency. The public evidence boundary remains conservative.
Live capture still carries native-code risk and manual setup cost. Each game
build requires a reviewed identity/layout update, and source audit is only a
regression guardrail—not a security proof.

The next admissible expansion is evidence-led: validate candidate field
semantics across controlled observations. Stable identity or family history
requires separate proof and cannot be inferred from vector position.
