# W&R compatibility profiles

Republic Observatory keeps version-sensitive W&R spellings and verified fixed
binary layouts in inert `.rocompat.json` profiles. A profile explains how a
particular game version is interpreted; it does not change the game, plan the
republic, translate interface prose, or add an Analysis Pack calculation.

The reviewed W&R 1.1.1.9 profile is version controlled at
[`compatibility/wrsr-1.1.1.9.rocompat.json`](../compatibility/wrsr-1.1.1.9.rocompat.json).
The public Draft 2020-12 contract is
[`schemas/compatibility-profile-v1.schema.json`](../schemas/compatibility-profile-v1.schema.json).
The packaged application embeds its reviewed default, so startup remains
offline and does not depend on a catalogue service.

## Ownership boundary

Profiles may declare only:

- exact save-archive entry aliases;
- stats format, record, state, city, and date marker aliases;
- bounded source indices for reviewed repeated history fields such as
  `$Citizens_Status`;
- source-field aliases assigned to allowlisted stable host fact slots;
- definition directives assigned to allowlisted host parsing operations; and
- fixed bounded binary layouts with an exact entry name, byte order, base,
  count, stride, primitive fields, masks, scales, missing-value sentinels, and
  magic-byte checks.

The host still owns metric IDs, meanings, scopes, units, evidence kinds,
calculation rules, parsing operations, archive and allocation limits, database
rules, rendering, and failure behaviour. A profile cannot introduce a new host
fact or operation. It cannot contain code, expressions, SQL, callbacks, URLs,
absolute paths, globs, pointer chasing, loops, arbitrary decompression, markup,
or renderer/ECharts configuration.

## Mod-scoped definition mappings

Normal Workshop and WIP buildings and vehicles do not need compatibility
overrides. The catalogue reads their ordinary W&R definitions under
source-qualified identities. A scope is only for unusual mod vocabulary that
means something already represented by an allowlisted host parsing operation:
for example a total conversion renaming a construction directive, a legacy mod
using an old spelling, or a community-researched alias awaiting upstream
review.

`catalogue_scopes` reuse the catalogue's exact `workshop.<item-id>` or
`wip.<item-id>` source identity. Each scope records the SHA-256 identity of the
supported definition content—not models, textures, previews, or complete mod
files—and one explicit update policy:

- `exact` refuses to publish a new catalogue generation when installed
  definition content changes. The previous generation remains active until the
  player reviews the change and updates the acknowledged hash.
- `track_updates` keeps applying the mapping to the exact package identity, but
  exposes `updated_unreviewed` until the new definition hash is acknowledged.

An absent source is `dormant`, not an error. Workshop and WIP identities remain
distinct even when their numeric item component matches.

A `mappings` fragment looks like this:

```json
{
  "catalogue_scopes": [
    {
      "id": "local.example.factory-pack",
      "source_id": "workshop.1234567890",
      "acknowledged_content_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "update_policy": "exact"
    }
  ],
  "definition_directives": [
    {
      "id": "local.example.factory-pack.workers",
      "operation": "building.workers_required",
      "matches": [{ "kind": "exact", "value": "$MOD_WORKERS" }],
      "catalogue_scope": "local.example.factory-pack"
    }
  ]
}
```

Mapping IDs are the inheritance key, so a local scoped mapping can coexist with
a reviewed global mapping for the same operation. Scoped mappings outrank
global mappings; exact directive matches outrank prefix and contains matches;
longer matches outrank shorter ones. A remaining tie is an error and leaves the
previous catalogue active rather than depending on JSON order.

Package scopes are forbidden on archive, statistics-field, and fixed-binary
save mappings. An installed mod does not prove that a particular save used it.
A future save-level mod scope requires trustworthy active-mod evidence inside
the save itself.

Fixed binary layout v1 is deliberately not a general binary-description
language. Reads are bounded to one named archive member and fixed offsets. A
missing optional member produces no facts. A present member that violates its
magic, count, stride, bounds, primitive, or finite-number rules fails that save
interpretation without disturbing earlier observations.

## Reviewed profile and local repair

The application watches one app-local file:

```text
compatibility/local.rocompat.json
```

Use **Save observer → Game compatibility profile → Create starter override** to
create it. The interface reports the exact app-local location; the repository
and diagnostics never record that private absolute path. The starter references
the reviewed profile's exact ID, semantic version, and content hash. Each
mapping array is optional. An included archive slot, stats marker, host slot,
definition operation, or binary-layout ID replaces only the matching reviewed
entry; omitted entries remain inherited.

The generated file uses 64 zeroes as an authoring sentinel in `content_hash`.
On load, the authoritative Rust validator replaces that sentinel in memory with
the SHA-256 hash of canonical profile content. This lets a player make a simple
text edit without running a hash utility while still giving every accepted
revision an exact stored content identity. Reviewed repository profiles may not
use the sentinel: their declared hash must match exactly.

A valid edit activates immediately after the bounded file-watch debounce or a
manual **Reload override**. An invalid edit never replaces the last valid
resolved profile. The Compatibility panel shows the failure reason and keeps
observation available. Removing the local file returns to the reviewed profile.
There is no in-app JSON editor, executable hook, marketplace, or second active
override.

## Interpretation identity and evidence

Raw and interpreted identities are intentionally separate:

```text
raw_payload_hash = SHA-256 of the supported raw stats payload
interpretation_id = SHA-256(raw_payload_hash + parser engine version + resolved profile hash)
```

The same raw save under the same resolved profile is idempotent. The same bytes
under a changed profile create another immutable interpretation. Earlier values,
branch labels, and provenance are not rewritten. **Reinterpret newest save** is
an explicit read-only action and uses the shared critical-task progress system.

Every interpretation records profile ID, version, declared content hash,
resolved hash, exact base hash, source, mapping classification, and parser-engine
version. Reviewed results are `reviewed_mapping`; local-override results are
`player_mapped`. Both may be used immediately in charts and models, but Archive,
receiver evidence, catalogue evidence, and model snapshots retain the badge and
profile identity. A mapping change across a timeline is evidence, not a seamless
continuation to hide.

SQLite owns profile revisions, runtime validation state, immutable observation
interpretations, and the projection outbox. DuckDB retains the same
interpretation/profile identity on observations and catalogue generations.

Reviewed profile revision 1.1.0 adds host-allowlisted market operations for
bounded price, trade, tourism, loan, vehicle-account, and cost fields. These
mappings only expose meanings already owned by the host; they do not add
expressions or make a malformed Markets section fatal to unrelated observation
facts. Earlier reviewed profile documents remain exact inheritance evidence.
A local override based on an older content hash does not silently rebase to the
expanded profile: the player must create or rebase a starter override
deliberately before new market mappings participate.
Upgrading an older database backfills the reviewed legacy provenance and queues
an idempotent warehouse rebuild; it does not change parsed values or branches.

## Authoring workflow

1. Begin with **Create starter override**; do not copy a random profile from an
   unknown game version.
2. Keep the exact `extends` ID, version, and hash. A reviewed-profile update must
   be rebased deliberately.
3. Set narrow target game versions, build IDs, and/or stats format numbers.
4. Declare the smallest replacement needed. Never repeat unrelated base arrays.
   For a mod alias, copy the source-qualified ID and supported-definition hash
   shown by the catalogue and choose `exact` or `track_updates` deliberately.
5. Reload and inspect validation, mapping coverage, and the reviewed/player badge.
6. Reinterpret a representative save and compare its normalised facts with the
   earlier interpretation. Missing facts should remain unavailable, not guessed.
7. For an upstream contribution, add a sanitised minimum fixture and prove the
   reviewed profile preserves all prior normalised fixture output. Never commit
   a save, installed definition, personal path, or game asset.

Use a planning overlay to change capacities, costs, recipes, or construction
assumptions. Use a language pack for interface prose. Use an Analysis Pack for
derived metrics and charts. A compatibility scope only explains source
vocabulary; it never changes installed facts or adds new semantics.

Repository profile changes require schema validation, Rust semantic validation,
stats and definition fixture equivalence, archive and binary boundary tests,
SQLite migration tests, projection tests, localisation checks, and wide/narrow
visual QA. Unverified binary research stays outside first-party profiles.

## Recovery and backups

Compatibility failure cannot block access to earlier SQLite observations or
core dashboards. Warehouse/catalogue refresh failure is analytical lag and
leaves the previous catalogue generation active. Diagnostics records controlled
operation codes and summaries, never profile contents or paths.

Back up the app-local SQLite database, DuckDB warehouse, and the optional local
profile together if reproducible local mappings matter. SQLite remains the
operational authority and DuckDB can be rebuilt through the projection outbox;
neither file belongs on NAS or cloud-synchronised storage. Both remain
unencrypted because this project stores no credentials.
