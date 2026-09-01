# ADR-0020: First-class attention cues and bounded research setup

## Status

Accepted.

## Context

Occasional new or safety-sensitive controls benefit from visual guidance, but
workspace-local glow effects would create inconsistent motion, dismissal,
localisation, persistence, and accessibility behaviour. The optional Tesmio
research probe also previously required a command-line build, leaving its exact
source and safety preconditions difficult to discover.

## Decision

`AttentionCue` is an application-owned guidance primitive. A cue has a stable
lower-case ID and content revision. SQLite records a dismissal for that exact
pair; revised guidance appears again without erasing prior state. The host
provides bounded three-cycle emphasis, a persistent non-motion outline,
localised text, keyboard controls, reduced-motion and forced-colour handling,
and an explicit replay operation. A cue may draw attention to an action but may
not decide whether the action is valid.

The Experimental Research Setup assistant uses bounded Rust commands and the
shared critical-task progress contract. Rust validates the exact reviewed
TesmioLoader header identities, detects the local compiler, invokes one
repository-owned build recipe, bounds and hashes the resulting DLL, and
sanitises its build log. SQLite owns notice acceptance, checkout selection,
artifact identity, build time, and cue dismissal state.

The assistant may download source only after the player accepts the current
research notice and confirms the GitHub connection. The request is fixed to one
reviewed commit, redirects are disabled, and the host retains only the exact
reviewed headers, licence, and provenance. The assistant does not download a
loader binary, install tools, elevate, configure TesmioLoader, inject code,
launch W&R, run the probe, or import telemetry. The normal save recorder remains
independent.

## Consequences

- Guidance can be reused without copying animation or persistence logic.
- Every build rejection ends in a visible failed progress state.
- The reviewed checkout can be supplied manually or created by the one
  explicitly confirmed, allowlisted source download.
- Offline and rejected downloads preserve the manual workflow and every
  previously built artifact.
- Building a DLL is not evidence that it was installed, run, or safe on an
  unreviewed game executable.
