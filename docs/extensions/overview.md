# Community Extensions overview

## Design goal

Republic Observatory should let players and statisticians contribute new
questions without granting a JSON file the powers of a desktop application.
The architecture therefore has two deliberately different tiers.

## Analysis Packs

Analysis Packs are inert, human-readable `.roanalysis.json` files. Version 1 can
declare bounded derived metrics and line, area, or bar chart templates. The
host:

- supplies stable normalised metrics;
- aligns inputs within one branch, observation date, and geographic scope;
- evaluates the small operation catalogue;
- resolves chart points and evidence;
- owns rendering, themes, motion, accessibility, settings, and failure states;
  and
- assigns provenance from pack ID, version, content hash, calculation rule, and
  source observations.

A pack cannot provide JavaScript, Svelte, HTML, CSS, callbacks, SQL, URLs,
database or save paths, ECharts options, trust assertions, or executable
expressions. Unknown fields fail schema validation. Plain strings are displayed
as text, never interpreted as markup.

New pack v1 files declare `default_locale` for their author-owned prose; earlier
v1 files without it remain compatible and default to `en-AU`. The host
translates its own controls but does not let an Observatory language pack
rewrite a pack's analytical claims. Multilingual extension content waits for an
explicit public package contract rather than borrowing host translation
namespaces.

The desktop application now provides a local lifecycle for schema version 1.
The Rust host authoritatively inspects and validates content, SQLite retains
immutable revisions and enablement state, and enabled packs are evaluated over
the selected branch's normalised observations. Pack failures are isolated.
The included example travels through the same import path as a locally obtained
community file; there is no privileged first-party evaluator.

## Model Plugins

Model Plugins are a future tier for an evidenced statistical model that cannot
fit the declarative vocabulary. They will be independently installable,
out-of-process programs communicating through a bounded, language-neutral
protocol.

The phrase **out of process is not an operating-system sandbox**. Process
separation limits coupling and failure propagation; a security boundary still
requires explicit platform controls, packaging rules, capabilities, and review.

No executable manifest, package format, runtime version, or plugin protocol is
published yet. Publishing those contracts before a real model exists would
freeze assumptions and create unsafe compatibility promises.

Any later protocol will provide only bounded normalised observations and
versioned game-definition models. Model Plugins will not receive raw saves,
binary payloads, SQLite access, parser internals, private paths, credentials,
or direct rendering access.

## Equal public contracts

First-party and community extensions use the same contracts. An extension
promoted into primary navigation gains no private API, hidden metric, rendering
hook, or wider capability. The host may curate placement, but not privilege.

This follows lessons from
[OnAir WyrmGrid](https://github.com/phobos-dthorga/onair-wyrmgrid): the host
keeps stable domain models and rendering authority, external contributors use a
narrow versioned boundary, and one extension failure cannot block core work.

## Lifecycle

These operations remain distinct:

1. obtain a local package;
2. inspect identity and contents;
3. validate format, semantics, compatibility, and content identity;
4. import an immutable revision;
5. enable one selected revision;
6. import a later revision without silently changing the enabled one;
7. update or roll back by explicitly selecting a revision;
8. disable and remove; and
9. start, only for a future executable plugin.

Import never implies enabling or starting. Analysis Pack v1 requests no
permissions because its vocabulary has no capability fields. Any future
permissions remain deny-by-default and bind to the exact extension ID, version,
content identity, and requested scope. A changed payload cannot reuse an earlier
decision merely by claiming the same name or version.

Local, offline installation is the baseline. A catalogue may later improve
discovery, but cannot become required for a locally obtained package.

## Compatibility axes

Package format, Analysis Pack schema, chart schema, host API, future plugin
protocol, future plugin runtime, application version, and database version are
separate compatibility decisions. They must not be collapsed into one “plugin
version.”

## Failure containment

An invalid or failed extension becomes unavailable with an actionable reason.
It cannot stop save observation, parsing, storage, core dashboards, or another
extension. Last-known extension output must not masquerade as current evidence
after its inputs or implementation change.

See the [authoring guide](analysis-pack-authoring.md),
[threat model](threat-model.md), and
[ADR-0005](../architecture/decisions/0005-declarative-analysis-packs-and-model-plugins.md).
