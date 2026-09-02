# TesmioLoader reverse-engineering assessment

## Purpose and provenance

This note assesses whether
[TesmioLoader](https://github.com/MaxLegend/TesmioLoader) can help Republic
Observatory establish meanings for fields that are opaque in saved W&R binary
payloads. It is a research-source assessment, not an endorsement, runtime
dependency, decoded save contract, or claim that an upstream finding has been
independently reproduced.

The source review was pinned to TesmioLoader commit
[`3baa141f9f08921aea9c95f0a400289cabd9960a`](https://github.com/MaxLegend/TesmioLoader/tree/3baa141f9f08921aea9c95f0a400289cabd9960a),
dated 2026-08-11. Upstream addresses and structures target the 64-bit DX11 W&R
1.1.1.9 executable unless the cited file says otherwise. Older narrative notes
may retain 1.1.1.7 addresses to explain how a result was discovered. The
compiled plugin source, its byte guards, and a reproduced local experiment are
stronger evidence than an old address in prose.

TesmioLoader itself is GPL-3.0 and injects native plugins into the game process.
Any distributed companion plugin requires a separate dependency, licensing,
security, and update-compatibility decision. Republic Observatory remains
usable without it.

## Experimental Research Setup assistant

The Population workspace now opens a native setup assistant for the optional
probe. It records explicit acceptance of the current research notice and
validates the two exact header hashes reviewed at commit
`3baa141f9f08921aea9c95f0a400289cabd9960a`. A player may select a local checkout
or explicitly confirm a download of that exact source revision from GitHub.
The managed download keeps only allowlisted build sources, reviewed headers,
the upstream licence, and a provenance record. It rejects redirects, arbitrary
locations, excessive or malformed archives, path traversal, links, duplicate
entries, and mismatched headers. Header identity is calculated after converting
ordinary CRLF checkout line endings to the upstream LF form. This accepts the
same reviewed text from
GitHub and Windows Git without accepting changed source. An interrupted or
rejected download does not replace an existing checkout or built probe.

The assistant checks the local Microsoft C++ toolchain and invokes only the
repository-owned `research/tesmioloader-probe/build.ps1` recipe. Download and
build progress are separate. Build output is bounded, hashed, and the displayed
log redacts both local source roots.

The assistant never downloads a loader binary or elevates. With separate
confirmation, it builds the reviewed host locally and prepares one marked
`W&R/tesmioloader/observatory` folder. Another confirmation is required before
it launches W&R through that folder. Missing prerequisites fail closed and
ordinary save analysis is unaffected.

## What the loader provides

TesmioLoader is a native instrumentation and modification host. Its launcher
starts `SOVIET64.exe` suspended, injects the loader DLL, installs hooks before
normal game code runs, and then resumes the process. It does not modify the
game executable on disk.

Its useful reverse-engineering facilities include:

- import-table hooks for named engine, C runtime, and operating-system calls;
- virtual-table slot replacement for calls made through C++ interfaces;
- checked data-pointer and instruction-operand redirection;
- checked inline hooks and generated spliced code when no safer boundary
  exists;
- readable-memory tests and structured exception filters around uncertain game
  structures;
- bounded guard-page probes that report the instruction reading a watched
  memory page;
- game and plugin logging, file-access tracing, and crash reports containing
  module-relative addresses and registers;
- a virtual filesystem for controlled asset substitution;
- a versioned native plugin host and a small service registry; and
- documented PE, string-cross-reference, Ghidra, shader, BTF, asset, and
  structure-replay tools.

The upstream methodology deliberately prefers PE structure and string
cross-references, then live hooks and probes, and uses Ghidra for control flow,
switches, and structure arithmetic that those approaches cannot settle. Ghidra
is the open-source software reverse-engineering framework created and
maintained by the United States National Security Agency Research Directorate.

These facilities are particularly valuable for binary saves because a probe
can observe both sides of the boundary:

1. hook the function or `fread`/`fwrite` call that handles a save entry;
2. record the bounded file position, byte count, and destination object;
3. observe the game using the resulting in-memory field; and
4. repeat a controlled player action to connect byte change, object field, and
   visible game meaning.

That process can establish semantics. A raw byte comparison alone usually
cannot.

## What the loader does not provide

TesmioLoader is not a general-purpose save decoder and does not presently
publish a Citizen Lives or worker-record API. It does not automatically:

- infer field names or types from arbitrary binary blobs;
- prove that a pointer, vector position, displayed name, or high-cardinality
  tuple is a persistent identity;
- distinguish residence, workplace, school, prison, or a transient visit merely
  because a `Person` points at a building;
- convert a global statistics reason into a per-person event;
- make version-specific addresses safe on an unknown executable;
- protect the game from a faulty plugin; or
- sandbox third-party native code.

Plugins share the game's address space and can crash or corrupt it. Upstream
mitigates this with an executable-version gate, exact expected-byte checks,
bounded reads, pointer validation, and fail-closed hook installation. Those are
necessary safeguards, not a sandbox. Research must use backed-up saves and a
separate test environment.

## Upstream citizen findings

The following are external research candidates found in the reviewed upstream
source. They are not yet Republic Observatory reviewed mappings.

| Candidate                                  | Upstream meaning                                                                                                  | Observatory consequence                                                                                                                                                                        |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Global vector at executable RVA `0x9E75B8` | Live `Person*` entries; `Person` allocation size `0x750`                                                          | Provides a bounded live population to sample, but pointer values and vector positions are session-local identities only.                                                                       |
| `Person + 0x20`                            | Building the person is currently in                                                                               | Useful for controlled movement research; must not be labelled home, work, school, or detention without state evidence.                                                                         |
| `Person + 0xA8`                            | Education level, represented as a 0–3 float                                                                       | Strong candidate for controlled validation against visible citizens and saved records.                                                                                                         |
| `Person + 0xD4`                            | Age in years                                                                                                      | Newer upstream work identifies this through the kindergarten age comparison and average-age calculation. This supersedes two rejected candidates documented by the upstream aging probe.       |
| `Person + 0xD8`                            | Eleven floats: happiness, food, health, `soviet`, alcohol, culture, sport, religion, clothing, electronics, crime | The fourth field may correspond to the player-facing loyalty concept, but Observatory must retain the upstream token `soviet` until UI and save experiments prove the public semantic mapping. |
| `Person + 0x110` and `+0x118`              | Demand count and seven-entry demand array                                                                         | Can reveal current goods/service intentions, not completed visits or durable life events.                                                                                                      |
| `Person + 0x4F0` and `+0x4F8`              | Unsatisfied-demand count and up to ten `{ amount, kind, Resource* }` entries                                      | Supports unmet-needs research with exact resource evidence.                                                                                                                                    |
| `Person + 0x71C`                           | 0 citizen, 1 Soviet tourist, 2 Western tourist                                                                    | Provides a candidate population-class guard.                                                                                                                                                   |
| `Person + 0x734`                           | Money spent in the currency selected by the tourist class                                                         | Candidate transactional measurement, not income or wealth.                                                                                                                                     |
| `Person + 0x2E`                            | Set when a parent of a child under six cannot find kindergarten placement and therefore stays home                | A behavioural flag discovered at a specific call site; it is not itself a child pointer, family identifier, or durable parenthood field.                                                       |

The upstream `needs` plugin also maps a citizen's current demand structures and
can emit bounded live diagnostics. The read-only `aging` probe snapshots whole
`Person` objects across calendar days and diffs typed four-byte slots. Its most
important lesson is negative: values at `+0x70` and `+0x65C` initially looked
age-like but were proved to be a state timer and walk-animation phase. Republic
Observatory should preserve rejected hypotheses as evidence against future
regression.

### Event-ledger leads

TesmioLoader maps the game's global happiness and health reason ledgers. The
happiness ledger contains reason families for prison, orphanage, a child's
death, relocation, expulsion, low government loyalty, and prison/orphanage
upbringing. These are valuable code-navigation anchors, but the ledger values
are aggregate counters and effects. They do not prove which citizen experienced
the event or establish a parent/child relationship.

A defensible next experiment is to locate and hook the writer for one reason at
a time. If the writer receives a `Person*` or related object, the probe can log
a bounded session-local subject plus the before/after state. Only a separately
proved persistent save key could turn that occurrence into a cross-save life
event.

## Binary-blob example: `namepoints.bin`

The upstream cities plugin demonstrates what TesmioLoader can and cannot do
with an opaque save entry. Its research identifies a fixed `0x130`-byte city
record containing position, UTF-16 name, two still-unknown integers, a transient
flag, building count, blob-presence flag, and sixteen zero bytes. The record is
followed by building indices and, conditionally, another `0x80` bytes.

The plugin hooks the game's own city load/save call sites, uses the game's
`FILE*` and imported `fread`/`fwrite`, and appends a separately versioned plugin
block after the records the base game reads. This proves that TesmioLoader can
instrument and safely extend a known serializer. It does **not** assign invented
meanings to the two unknown integers or the optional blob. Unknown bytes remain
unknown until another experiment connects them to behaviour.

That is the correct precedent for `workers.bin`: identify the serializer,
observe the live object and record together, preserve unknown regions, and
publish only independently reproduced fields.

## Implemented bounded research bridge

The current Broadcast-specific evidence register and promotion gate are kept in
[Broadcast telemetry research findings](broadcast-telemetry-findings.md).

The first bridge is now implemented as an optional, separately built
**Observatory Research Probe**. It is not part of the normal desktop application
and is not required to view any save-derived feature. No loader or DLL is
installed automatically.

TesmioLoader itself is a general modding platform, not a read-only dependency.
Its upstream defaults can redirect reads, write save manifests, and load
gameplay-changing plugins. The Observatory experiment therefore also requires
the supplied observation-only host settings, the companion as the sole plugin
DLL, and a passing preflight verifier before launch.

The probe is read-only in its data behaviour and provides no gameplay
modification. It:

- refuse any executable except an exact reviewed build identity;
- pins the reviewed executable's PE timestamp and size and fails closed before
  installing its single chained IAT observation hook;
- observe a bounded number of people and events;
- emits no address, name, pseudonymous subject key, or full object dump;
- emit a versioned, bounded, local JSON Lines research stream;
- includes a strict session contract plus sequence, game date, population count,
  and bounded candidate-field samples;
- emit a bounded readiness stage without exposing addresses or rejected values;
- exclude complete object dumps, raw saves, filesystem paths, and network
  access by default;
- never open Republic Observatory's SQLite or DuckDB databases; and
- never write to a game object, serializer, or save entry.

Republic Observatory derives one fixed telemetry location from the configured
game directory and validates the stream through Rust. It caps the file at 4
MiB, 8,192 lines, and 16 KiB per line; rejects unknown fields, path escapes,
links, inconsistent samples, and any claimed write/network capability; and
returns aggregate status only. The records are **not imported into SQLite or
DuckDB** in this slice. The production save parser remains the authority for
normal observation.

The GPL companion source and build instructions are in
[`research/tesmioloader-probe`](../../research/tesmioloader-probe/README.md).
The app exposes a first-class Legal & notices screen covering ownership,
licensing, same-process risk, the no-sandbox caveat, and evidence limits.

### Calendar-route correction

Probe 0.2.1 treated the global at RVA `0x9941F0` as a pointer to the game-state
object. A checked live session proved that its day and year reads were instead
the two halves of an unrelated pointer, so the bounds check correctly rejected
every frame and no snapshot followed the session record. Static call-site
analysis and a bounded read-only live check identified the reviewed in-place
game-state object at RVA `0x9D4F10`; its day and year remain at offsets `0x590`
and `0x594`. Probe 0.2.2 and later use that route, keep the executable identity
gate, and add allowlisted readiness records so this failure class is visible.

## Priority experiments

1. Locate the `workers.bin` load and save functions and delimit every read/write
   contributing to one `Person` record.
2. Correlate a live `Person*` with its exact saved record without assuming
   vector order.
3. Search for a persistent source key and test it across save/reload, vector
   reordering, birth, death, emigration, and slot reuse.
4. Reproduce age, education, citizen class, and the eleven status floats against
   visible in-game values in controlled saves.
5. Determine what `Person + 0x20` represents in home, work, school, hospital,
   prison, orphanage, transit, and walking states.
6. Follow one event-ledger writer each for prison and orphanage entry, then test
   whether the function exposes subject and related-person pointers.
7. Locate family-assignment and birth functions and test candidate links for
   symmetry, persistence, deletion behaviour, and identifier reuse.
8. Re-run every accepted experiment after a restart and on more than one
   republic before publishing a mapping.

Until experiment 3 succeeds, Republic Observatory must retain its aggregate-only
Population design. TesmioLoader makes trustworthy individual histories more
feasible; it does not by itself make them proven.

## Primary upstream references

- [TesmioLoader architecture](https://github.com/MaxLegend/TesmioLoader/blob/3baa141f9f08921aea9c95f0a400289cabd9960a/docs/01-architecture.md)
- [Reverse-engineering toolkit](https://github.com/MaxLegend/TesmioLoader/blob/3baa141f9f08921aea9c95f0a400289cabd9960a/docs/03-reverse-engineering.md)
- [Plugin host and safety boundary](https://github.com/MaxLegend/TesmioLoader/blob/3baa141f9f08921aea9c95f0a400289cabd9960a/docs/09-plugins.md)
- [Game-internals findings](https://github.com/MaxLegend/TesmioLoader/blob/3baa141f9f08921aea9c95f0a400289cabd9960a/docs/02-findings.md)
- [Read-only citizen aging probe](https://github.com/MaxLegend/TesmioLoader/blob/3baa141f9f08921aea9c95f0a400289cabd9960a/plugins/aging/aging.cpp)
- [Citizen demands and status layout](https://github.com/MaxLegend/TesmioLoader/blob/3baa141f9f08921aea9c95f0a400289cabd9960a/plugins/needs/needs.cpp)
- [Education and age findings](https://github.com/MaxLegend/TesmioLoader/blob/3baa141f9f08921aea9c95f0a400289cabd9960a/docs/16-easystart.md)
- [`namepoints.bin` serializer research](https://github.com/MaxLegend/TesmioLoader/blob/3baa141f9f08921aea9c95f0a400289cabd9960a/docs/14-cities.md)
- [Official Ghidra repository](https://github.com/NationalSecurityAgency/ghidra)
