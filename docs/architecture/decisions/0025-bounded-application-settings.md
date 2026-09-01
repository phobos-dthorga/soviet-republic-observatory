# ADR-0025: admit only bounded user-owned application settings

## Status

Accepted.

## Context

Republic Observatory needs one discoverable home for private source folders,
accessibility preferences, save watching, and safe background-maintenance
trade-offs. Exposing every internal constant would make evidence integrity and
failure behaviour depend on presentation choices.

## Decision

Settings are accepted only when they express a genuine player preference or a
bounded operational trade-off. Every setting has a Rust-owned type, default,
range or enumeration, effect boundary, reset behaviour, localisation, and
tests. Svelte stages and renders preferences but does not decide validity.

The first contract covers source directories, automatic observation, validated
language and theme lifecycle links, 100–200% text scale, system-aware reduced
motion, background-work priority, and storage-contention patience. The visible
patience value is a total retry budget for resumable background work; SQLite's
short per-attempt wait remains host-owned so foreground controls stay
responsive.

Evidence interpretation, provenance, content hashes, parser and schema
versions, save-stability checks, database row and memory limits, recorder
priority, security controls, and destructive maintenance are not settings.
Themes and language packs retain their existing immutable lifecycle authority,
and folder paths remain private native configuration.

## Consequences

Players can tune accessibility and maintenance without learning database error
codes or weakening accuracy. New settings require an explicit admission review;
an internal constant does not become public merely because it might be
editable.
