# Dynamic resource catalogue and live reconciliation

Republic Observatory builds one resource catalogue from evidence that is
already available on the player's computer. Installed definitions, recorded
saves, optional running-game readings, and player planning notes remain
separate origins. An exact source token is the identity. Similar captions,
indices, or spellings never merge resources.

## Normal offline discovery

Offline discovery is the default and remains fully functional without
TesmioLoader. Rust combines supported resource references from the base game,
DLC, Workshop items, local work-in-progress definitions, recorded Markets
fields, and player overlays. The interface receives only bounded catalogue
entries and source summaries. It never receives complete game files or private
paths.

Installed `soviet*.btf` language files are read on demand through a bounded
big-endian decoder. Only requested caption numbers and their resolved labels
are kept. A malformed file is ignored without hiding the exact source token.
Labels improve presentation but never become database identities.

The display-name order is:

1. Matching installed game language.
2. Installed English fallback.
3. Caption from a checked running-game reading.
4. A reviewed Observatory alias.
5. A readable form of the exact token.

## Optional running-game reconciliation

The separate GPL research probe can emit one resource registry after it remains
stable across consecutive rendered frames. The record is limited to 512
resources and reviewed fields. It contains exact tokens, captions, type fields,
RUB and USD finished and base prices, and buy and sell multipliers. It contains
no pointers, raw objects, paths, assets, callbacks, or executable settings.

The player must accept the current research notice and explicitly enable one
mode:

- **Verified observation-only session** checks the restricted loader settings,
  sole-probe rule, and executable identity before every reading.
- **Player-managed modded session** checks only Observatory's own probe and
  report. Other plugins remain under the player's control, so the complete
  loader session is not certified as observation-only.

The host validates the whole record before storing anything. It derives buy and
sell quotes from the captured finished price and multiplier, then stores one
immutable, content-addressed local snapshot. Repeated reports deduplicate.
Changing assurance mode creates a separate receipt and cannot relabel an older
reading.

### Reviewed build-specific layout

The initial registry contract is deliberately limited to the inspected W&R
1.1.1.9 executable with PE timestamp `0x6A3EB6AD` and size `10,308,608` bytes.
For that build, the probe checks the registry vector at RVA `0x9E11C0`, an
832-byte record stride, exact token storage, caption and type fields, and the
reviewed RUB/USD price and multiplier offsets. It refuses every other
executable identity. It also rejects the complete snapshot if the vector,
stride, count, tokens, indices, or numeric bounds do not agree with that
contract.

These offsets are compatibility evidence for one build, not a promise about
future game versions. A later executable needs a fresh derivation and a
controlled comparison with the game's visible resource and price information.
Changing an address or weakening a bound is not enough to promote a new build.

## History boundary

Live prices describe one captured session. They appear beside a matching exact
resource and currency in Markets, but never replace save-backed price history.
After restart, the interface says **Last verified in a game session**. A live
resource is never inserted into an older save, used to fill a historical gap,
or treated as proof that the current game still has it loaded.

The normalized analysis-database copy is disposable. The local SQLite snapshot
remains available when background analysis is unavailable and is projected
again through the normal recovery queue.

## Contributor rules

- Add no hard-coded resource inventory to interface code.
- Keep reviewed relationship mappings, fixtures, and compatibility evidence
  narrow and clearly labelled.
- Use exact tokens for joins and related-data navigation.
- Keep RUB and USD separate.
- Do not infer installed, historical, or active state from a caption or index.
- A new probe field needs a build-specific derivation, bounds, malformed-input
  tests, controlled game comparison, and an updated legal notice.

Run `npm run audit:resources` and `npm run audit:tesmio-probe` after changing
this boundary.
