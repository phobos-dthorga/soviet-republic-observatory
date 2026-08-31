# Markets and external economy

Republic Observatory's Markets workspace is a source-backed view of the market
records already present in W&R `stats.ini` payloads. It is not a simulated
exchange, cash-flow statement, or forecast. Rust owns parsing, evidence
validation, persistence, lifecycle decisions, and calculations; Svelte receives
bounded presentation models and never sees SQL, connections, database paths, or
raw archives.

## Evidence boundary

The reviewed W&R 1.1.1.9 compatibility profile maps bounded purchase, sell,
base-price, import, export, tourism, loan, vehicle-account, delivery, labour,
and immigrant-cost fields. The parser retains original resource tokens, source
spellings, source lines, mapping IDs, signed values, modifiers, currency,
channel, scope, profile identity, and record date. Unknown resources remain
source tokens rather than being renamed or discarded.

Structured market blocks have explicit row and resource ceilings. Duplicate
rows, non-finite numbers, malformed endings, and excessive cardinality fail the
market portion closed. A valid receiver or population observation still commits
when Markets is partial or unavailable. Compact token interning prevents each
of thousands of retained records from owning another copy of every resource and
source-field string.

RUB and USD are separate evidence domains. Standard and international trade
channels are separate until source research proves their relationship. Negative
export account values remain signed source evidence and are also identified as
disposal costs; they do not enter positive-export concentration. Resource
quantities are source-native and are never totalled across unlike resources.

City market values are exact selected-save snapshots with neutral numeric
source IDs. W&R does not embed their full history in the observed payload, and
their source windows demonstrably differ from republic history. The application
therefore never reconciles or sums city and republic totals.

## Exact-save indexing

**Index available saves for Markets** considers only files already represented
by an Observatory observation. It matches configured-directory identity,
filename, size, and modification evidence before opening one archive at a time;
the stored raw `stats.ini` hash is the final identity check. An unrecorded file
is never imported by this operation.

The deepest still-available save on each branch is considered first so retained
history prefixes can be content-addressed and reused. Progress reports archives,
records, rows, missing files, changed files, failures, and duplicate variants.
Jobs are durable and idempotent. An interruption can be resumed without
creating a duplicate historical moment, and indexing never moves the selected
branch or analytical head.

An older archive that is missing or changed remains honestly unavailable. A
new interpretation of the same raw save is grouped as a profile variant of the
same observation; earlier interpretations stay immutable.

## Storage and outage behaviour

SQLite owns market coverage, record membership, exact facts, interpretation
variants, indexing jobs, immutable basket/scenario revisions, and selections.
Historical records and rows are content-addressed across retained prefixes and
branch forks. Each committed interpretation queues an idempotent
`market_observation` outbox job.

DuckDB migration 6 stores analytical market projections and aggregates. The
existing write governor limits row volume, permits one writer, records progress,
and uses projection receipts to close the DuckDB-commit/SQLite-acknowledgement
crash gap. Large retained histories are read back from the applied DuckDB
interpretation partition. The interface uses a non-blocking warehouse read; if a
writer is active or the receipt is absent, it immediately retains only the exact
selected-head SQLite ledger and labels historical models as lagging. Rebuilds
clear only derived market partitions and requeue SQLite authority. A warehouse
outage cannot block save recording, Archive, historical selection, or exact
selected-head SQLite ledgers.

## Calculations

- Recorded trade result is `signed export account value − import account
value`, separately for each currency and channel.
- Positive-export HHI uses standard-channel export account values strictly
  above zero. Disposal and zero-value rows remain in the evidence ledger.
- Resource price indices use the first compatible positive base value as 100.
- Named fixed baskets use the Laspeyres form
  `Σ(base quantity × current price) / Σ(base quantity × base price) × 100` and
  report covered versus requested resources.
- Robust price movement is the scaled median absolute deviation of finite
  log-price changes. The interface reports the contributing observation count.
- Terms of trade is `export basket price index / import basket price index ×
100` and is published only when the baskets share a currency and exact base
  record. No implicit alignment is made.
- No source window is annualised or interpolated without a verified period.

Built-in baskets are currency-specific, standard-channel **Observed Imports**
and **Observed Positive Exports**. Player baskets are immutable named revisions
with explicit weights, price side, base record, reason, and currency. Updates
append a revision; selection and rollback change context without rewriting
history.

Break-even scenarios calculate `(domestic unit cost + delivery cost) /
operating efficiency`. Debt-stress scenarios divide selected same-currency,
stressed income components by player-confirmed debt service. Both are visibly
`player_definition` planning calculations. Exchange assumptions are explicit
scenario data; currencies are never merged merely because an assumption field
exists. Currency reserves and liquidity remain unavailable because no supported
cash-balance field has been established.

## Interface and provenance

Every headline and chart has the standard Metric Context help surface: formula,
currency, unit, time basis, exclusions, evidence class, compatibility profile,
source fields, and exact analytical head. Accessible chart ledgers retain the
same values. Original resource identifiers stay visible for mod authors.

The workspace provides deterministic native-review fixtures for ready,
indexing, partial, empty, lagging, and failed states. Browser audits continue to
cover localisation, architecture, accessibility, contrast, geometry, control
sizes, and responsive layouts. The exhaustive native matrix remains reserved
for native-specific failures.

The repeatable Rust suite includes a 25-archive exact-match indexing batch that
proves the selected branch and analytical head do not move. An ignored
reference-machine scale assay publishes 2,805 records, one million trade rows,
and 139 city scopes, then verifies the application read remains bounded to
summary series and selected-head evidence rather than materialising the full
fact table in memory.

## Privacy and backups

Market evidence contains game statistics, not credentials. SQLite and DuckDB
remain app-local, offline, unencrypted, and unsuitable for NAS or
cloud-synchronised placement. Indexing reads configured saves but never copies
archives into the repository or database. Diagnostics and review artifacts omit
personal paths and raw save contents. Back up both databases together while the
application is closed when reproducible historical analytics matter.
