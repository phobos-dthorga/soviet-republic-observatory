# Republic Observatory Tesmio read-only probe

This directory contains an **optional research companion**, not part of the
Republic Observatory desktop binary. It is designed to compile against a
separately obtained TesmioLoader checkout and run in W&R's process. Nothing in
the normal Observatory installation requires it.

## Licence and ownership

- `observatory_probe.cpp` is GPL-3.0-only. Its SPDX identifier is authoritative;
  the complete [GNU GPL version 3 text](COPYING) is included here and must
  accompany any distributed binary.
- TesmioLoader is separate GPL software owned by its upstream contributors. It
  is not vendored here.
- The main Republic Observatory application remains MIT licensed.
- Workers & Resources: Soviet Republic and its software, formats, names, and
  assets remain the property of their respective owners. This is an independent
  community research project, not an official 3Division or Hooded Horse tool.

See [the repository legal notice](../../docs/legal-and-third-party-notices.md)
for the plain-language boundary. That document is not legal advice.

## What the experiment does

The probe installs one chainable IAT observation hook on the engine's render
call. It calls the original function first, then periodically reads a bounded,
spread sample from the live `Person*` vector. After the resource registry is
stable across consecutive rendered frames, it also emits one bounded registry
record. It writes one fixed JSONL file in TesmioLoader's own build directory:
`republic-observatory-probe.jsonl`.

The records contain a session contract, a bounded collection-stage status,
date/population snapshots, anonymous samples of candidate person fields, a
reviewed resource-registry contract, and an explicitly untrusted facility
comparison record. The stage says whether the probe is waiting for W&R, waiting
for a loaded republic, collecting or publishing a facility comparison, ready,
or unable to roll its report forward.
It never contains an address or a rejected field value. The resource record is
limited to exact tokens, caption IDs and captions, reviewed kind fields,
RUB/USD prices, and buy/sell multipliers.
`vector_index` is ephemeral and must never be treated as a stable citizen ID.

The probe does **not**:

- write a simulation object or serializer buffer;
- open, rename, replace, or write a save;
- write a game definition or asset;
- open SQLite or DuckDB;
- use the network;
- emit names, raw pointers, full objects, or arbitrary paths;
- emit raw resource records, callbacks, assets, or executable configuration;
- establish family links, stable identity, causality, or life histories.

The app's source and configuration audits are regression guardrails, not proof
of an operating-system sandbox. TesmioLoader and its DLLs run in the game
process.

## TesmioLoader itself is not inherently read-only

The standard upstream package is a general modding platform. Its compiled
defaults enable virtual-file redirection, save-manifest reads and writes, and
the loading of every plugin DLL that has not been explicitly disabled. Some
upstream example plugins intentionally change the simulation or write content.

Republic Observatory provides two explicit assurance modes.

**Verified observation-only session** applies only when all of the following
are true:

- the reviewed `observatory_probe.dll` is the only DLL in `plugins\`;
- the exact settings in
  `tesmioloader.observation-only.ini.example` are merged into the active
  `tesmioloader.ini`;
- the upstream executable version gate remains enabled;
- `verify-observation-only.ps1` passes immediately before launch.

The verifier reads the selected installation but does not alter it or the game.
It cannot turn same-process native code into a sandbox.

**Player-managed modded session** permits plugins that the player manages. In
this mode Observatory validates its own probe and telemetry but does not
inspect, enable, disable, or certify other plugins. The complete TesmioLoader
session is not certified as observation-only. Both modes require separate
acknowledgement before Observatory retains a resource reading.

## Exact reviewed build gate

The first experiment is pinned to the executable inspected locally for the
upstream W&R 1.1.1.9 research baseline:

- PE timestamp: `0x6A3EB6AD`
- on-disk executable length: `10,308,608` bytes
- PE `SizeOfImage` reported by TesmioLoader: `11,128,832` bytes (`0xA9D000`)
- Tesmio API: 4

If either runtime executable identity value differs, the companion logs a
refusal and does not install its observation hook. A new build needs a reviewed
source revision; changing the constants is not evidence that the layouts
remained valid.

## Build without installing

Requirements: Windows x64, Visual Studio Build Tools with Desktop development
with C++, and a separate TesmioLoader checkout containing
`src/tesmio_plugin.h`.

```powershell
.\build.ps1 -TesmioLoaderRoot 'C:\path\to\tesmioloader'
```

This creates ignored files under `build/`. It does not copy anything into the
game.

## Guided checked session

Open **Legal & notices → Read-only research → Open research setup**. The guided
assistant performs the formerly manual work in distinct steps:

1. Review and accept the current research notice.
2. Download the exact reviewed source, or choose an existing reviewed folder.
3. Build and verify the Observatory probe.
4. Review the game-folder changes, then prepare the checked session.
5. Review the live-process change, then launch W&R.

Preparation compiles the reviewed loader and launcher locally. It places them,
the Observatory probe, a restrictive configuration, the GPL licence, and a
content manifest under `W&R/tesmioloader/observatory`. The app will not replace
an unmarked folder or another Tesmio installation.

Launch uses only the checked dedicated folder. The version gate, disabled VFS,
disabled save manifest, and sole-plugin rule are verified first. Native code
still runs inside W&R, so launching has a separate confirmation.

TesmioLoader normalises spacing in its configuration and adds its own display
tag on first launch. On Windows, it may also write the same game executable in
extended-path form. Observatory accepts only those known semantic rewrites.
Any new section, plugin, different executable, or changed safety option still
requires repair. The prepared folder remains tied to the exact probe identity
recorded by Observatory, even when the local build files are no longer present.

Probe contract 4 (`observatory_probe` 0.3.0) reads the reviewed calendar from
the executable's in-place game-state object. The earlier 0.2.1 build followed
an unrelated global pointer and could remain at **Waiting for checked report**
even while a republic was running. The corrected build reports its current
waiting stage explicitly.

The checked report is scratch telemetry, not saved analytical history. Version
0.3.0 samples once per seven game days by default. It also checks the reviewed
building collection across multiple frames. Candidate facility rows are usable
only when the matching completion record exists. A changing world or invalid
row rejects the complete candidate set. These rows remain in research storage
and cannot appear in ordinary Environment results without a later reviewed
mapping change. When the configured line
limit is near, it clears only that fixed report, writes a fresh checked session
header, and resumes. The current resource registry is captured again after the
usual consecutive-frame check. This keeps long or accelerated play sessions
bounded without requiring a restart.

The probe writes a session check as soon as it loads. Game-data snapshots begin
only after a republic is loaded and stable rendered frames are available. A
checked launch that closes at the main menu therefore proves that the loader and
probe started, but it does not contain a resource or population snapshot.
Launching W&R later from Steam or its usual shortcut starts an ordinary session;
it does not retroactively attach the checked probe.

The flow does not change the game executable, game assets, Workshop content,
or saves. Normal gameplay can still save if the player chooses to continue
playing and save after launch. Anonymous person samples are never retained.
Opted-in resource readings remain local session snapshots and never become
historical save facts.

## Bounds

The source clamps settings to 1–32 people, 1–365 days, 2,048–40,000 total
records, at most 512 resource entries, 25,000 candidate facilities, 128 facility
reads or one millisecond per rendered frame, and exactly one fixed output file. The
fixed report rolls forward in place before it reaches that bound; it never grows
without limit. A rollover failure stops collection instead of weakening the
limit. The Rust reader independently caps the file at 16 MiB, 40,000 lines, and
16 KiB per line, rejects unknown fields, rejects write/network capability
claims, and rejects paths that escape the configured game directory.
