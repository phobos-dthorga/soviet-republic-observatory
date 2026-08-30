# ADR-0012: versioned inert game compatibility profiles

Status: Accepted

## Context

W&R archive member names, stats directives, definition directives, and verified
binary layouts can change between game builds. Keeping those spellings in Rust
would require an application release for a small compatibility repair and would
mix volatile source vocabulary with stable Observatory meanings. Allowing
scripts or a general binary grammar would instead turn a repair mechanism into
an executable plugin and undermine the read-only parser boundary.

## Decision

- Reviewed `.rocompat.json` profiles contain version-sensitive mappings and are
  embedded for reliable offline startup.
- Stable fact IDs, scopes, units, evidence classifications, parsing operations,
  calculations, limits, persistence, and rendering remain host-owned.
- Draft 2020-12 structure plus authoritative Rust semantic validation fail
  closed on unknown fields, slots, operations, duplicate mappings, unsafe
  scope expansion, or unresolved exact bases.
- Binary schema v1 permits fixed bounded reads only. It forbids expressions,
  code, SQL, URLs, paths, globs, callbacks, pointer graphs, loops, arbitrary
  decompression, and renderer configuration.
- One watched app-local override may inherit one exact reviewed base. Valid
  changes activate immediately; invalid changes leave the last valid resolved
  profile active.
- Raw payload and interpretation identities are separate. A profile change
  creates a new immutable interpretation and never rewrites stored facts.
- Local results are `player_mapped` evidence. They are usable but remain visibly
  distinct from `reviewed_mapping` evidence in presentation and model snapshots.
- Definition mapping changes schedule a new content-addressed catalogue
  generation. Save reinterpretation remains an explicit read-only action.
- Definition mappings may reference an inert source scope using the catalogue's
  exact Workshop/WIP identity and acknowledged supported-definition hash.
  Mapping IDs, rather than operations, are inheritance keys so global reviewed
  and package-specific aliases can coexist.
- Each source scope chooses `exact` publication refusal or `track_updates` with
  a persistent unreviewed-update warning. Missing packages are dormant.
- Source scopes cannot qualify archive, statistics, or binary save mappings;
  installation alone is not evidence that a save used a mod.

Compatibility profiles are not planning overlays: overlays express player
assumptions as original → override → effective values. They are not language
packs: source aliases never become interface prose. They are not Analysis Packs:
they cannot calculate new metrics or declare charts.

## Consequences

Players can repair a renamed source key without waiting for a release, while
the application can state exactly which mapping produced every fact. Storage
and projection models become profile-aware, and old databases need a legacy
provenance backfill plus warehouse rebuild. Comparisons across profile changes
must disclose that boundary.

The narrow schema cannot express variable-length records, pointer graphs,
compression algorithms, or novel semantic operations. Those require a reviewed
engine change, new schema version, fixtures, and security analysis—not a clever
profile workaround. Processed results remain evidence about an interpretation,
not proof that a community mapping is correct.

Normal mod definitions require no profile. Scopes are a compatibility repair
for unusual vocabulary, not a mod database, load-order system, configuration
mechanism, or substitute for planning overlays and Analysis Packs. Exact-scope
conflicts retain the previous catalogue generation; tracked updates remain
usable but visibly unreviewed until their hash is acknowledged.
