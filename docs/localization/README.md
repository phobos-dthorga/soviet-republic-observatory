# Localisation and language packs

Republic Observatory uses the community-localisation approach proven in OnAir
WyrmGrid, with several debt-reduction changes made before save parsing begins.
English (Australia), `en-AU`, is the canonical source catalogue. Every current
workspace, chart label, textual chart summary, provenance sentence, dialog, and
locale-sensitive number passes through the same host-owned boundary.
The current source contract is compatibility version 1, additive revision 12.

## What is implemented

- a canonical versioned JSON catalogue in [`locales/en-AU.json`](../../locales/en-AU.json);
- Fluent message formatting through `@fluent/bundle`;
- strict `.rolanguage.json` manifests and a Draft 2020-12 schema;
- local import, inspection, explicit selection, fallback, and removal;
- per-message selected-pack → `en-AU` fallback;
- locale-aware number and percentage formatting;
- left-to-right and right-to-left document direction;
- focus-managed keyboard access to the language dialog;
- expanded and RTL pseudo-language generators used by tests; and
- a localisation audit in `npm run check`.

The runnable webview stores installed community catalogues and the selected ID
in versioned webview-local storage. The Tauri/Rust save host now exists, but
language-pack validation and persistence have not yet migrated across that
boundary. Rust becoming their authoritative owner is a separate slice so it can
retain the same manifest contract and distinct install/select/remove lifecycle
without coupling translation work to save parsing.

## Manifest contract

A community language pack is inert UTF-8 JSON. Start with
[`community-fr-example.rolanguage.json`](../../examples/language-packs/community-fr-example.rolanguage.json)
and validate it against
[`language-pack-v1.schema.json`](../../schemas/language-pack-v1.schema.json).

Required identity and compatibility fields are:

- `schema_version` — language-pack structure;
- `id` — stable lower-case package identity; the `observatory-` prefix is reserved;
- `locale` — a BCP 47-style language tag;
- `name` and optional `author`;
- `source_locale: "en-AU"`;
- `source_catalog_version` — breaking catalogue compatibility;
- `source_catalog_revision` — additive catalogue progress;
- `direction` — `left_to_right` or `right_to_left`; and
- `messages` — a partial map of known message IDs to Fluent patterns.

Adding a source message advances the catalogue revision, not its compatibility
version. A pack targeting an earlier compatible revision continues to load and
falls back to English for newer messages. Removing a message, renaming a key, or
changing its variables incompatibly requires a new catalogue compatibility
version.

Installation never activates a pack. The user inspects the result and selects
it separately. Removing the selected pack returns to built-in English.

## Validation and trust boundary

Unknown manifest fields and message IDs fail validation. The current bounds
are 256 KiB per manifest, 2,048 messages, and 2 KiB per pattern. Validation also
checks identifier and locale syntax, source compatibility, Fluent syntax,
exact variable parity with English, and rejects markup, control characters, and
explicit bidi-control characters.

Community language files contain no JavaScript, Svelte, HTML, CSS, callbacks,
URLs, filesystem paths, SQL, renderer configuration, or capabilities. Strings
are rendered only as text.

Some host wording cannot be replaced by an unreviewed community file. Protected
prefixes cover legal, privacy, credentials, save safety, extension permissions,
security, data protection, destructive actions, errors, evidence status,
coverage, causal caveats, and synthetic-data warnings. This prevents a language
file from relabelling an estimate as a save fact, hiding that values are
synthetic, weakening a removal confirmation, or rewriting a security boundary.
Reviewed built-in translations may eventually localise protected messages as
part of the application release process.

## Authoring rules

1. Copy the example and give it a unique, stable ID.
2. Translate any subset of eligible keys from `locales/en-AU.json`.
3. Preserve every Fluent variable exactly. For example, a translation of
   `chart-accessible-label` must retain both `$title` and `$description`.
4. Do not copy protected keys into a community manifest.
5. Set the correct writing direction; do not insert bidi override characters.
6. Run `npm run check` and `npm test` before sharing a pack.

Partial packs are intentional. English fallback is preferable to copied,
out-of-date source text masquerading as translated coverage.

## UI language is not game vocabulary

Two sources must remain separate:

- **Observatory UI language** is authored here and uses the language-pack
  catalogue.
- **Game vocabulary** includes resource names, building names, vehicle names,
  and other terms supplied by an installed game version.

The save/parser boundary retains exact source IDs and misspellings as evidence.
Stable application IDs remain database and metric keys. A later
`GameVocabularyCatalogue` will resolve display names from the locally installed
game language where licensing and file stability permit, with a reviewed
Observatory fallback. It will not copy game translation files into this
repository, and changing display language will never change observation
identity or calculations.

Raw IDs, branch IDs, file names, metric IDs, extension IDs, diagnostic fields,
and original source fragments are not translated. They may be accompanied by a
translated explanation.

## Host text and extension-authored text

The host translates its own controls and labels. Analysis Pack names,
descriptions, metric labels, and chart-template prose belong to the extension
author. New Analysis Pack v1 files declare `default_locale`; older v1 files
remain valid and default to `en-AU`. The interface tags that content with its
source language instead of pretending it came from the host catalogue. A future
multilingual extension package must add an explicit versioned resource contract
rather than allowing host language packs to override another author's content.

## Adding source messages

- use a stable semantic key rather than a sentence as the ID;
- call the translator with a literal key or select from an explicit typed map;
- use the central formatting helpers for numbers, percentages, dates, and
  currencies;
- decide whether the wording belongs to a protected namespace;
- increment `source_catalog_revision` in the source catalogue, schema maximum,
  validator constant, and fixtures; and
- add or update pseudo, fallback, variable-parity, keyboard, and layout tests as
  appropriate.

Constructed keys such as ``translate(`status-${state}`)`` are forbidden. They
hide catalogue coverage from static checking and make later changes brittle.

See [ADR-0006](../architecture/decisions/0006-versioned-community-localisation.md)
for the decision record.
