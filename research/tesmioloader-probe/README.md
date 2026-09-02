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

The records contain a session contract, date/population snapshots, bounded
anonymous samples of candidate person fields, and a reviewed resource-registry
contract. The resource record is limited to exact tokens, caption IDs and
captions, reviewed kind fields, RUB/USD prices, and buy/sell multipliers.
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
- executable size: `10,308,608` bytes
- Tesmio API: 4

If either executable identity value differs, the companion logs a refusal and
does not install its observation hook. A new build needs a reviewed source
revision; changing the constants is not evidence that the layouts remained
valid.

## Build without installing

Requirements: Windows x64, Visual Studio Build Tools with Desktop development
with C++, and a separate TesmioLoader checkout containing
`src/tesmio_plugin.h`.

```powershell
.\build.ps1 -TesmioLoaderRoot 'C:\path\to\tesmioloader'
```

This creates ignored files under `build/`. It does not copy anything into the
game.

## Deliberate manual experiment

Only after reading the in-app **Legal & notices → Read-only research** screen:

1. Follow TesmioLoader upstream's own installation instructions, but do not
   install or enable its gameplay plugins.
2. Use a dedicated TesmioLoader build folder whose `plugins\` directory contains
   only `build/observatory_probe.dll` and `observatory_probe.ini`.
3. Merge every setting from
   `tesmioloader.observation-only.ini.example` into that folder's
   `tesmioloader.ini`. Preserve upstream `version` and `game_exe` metadata.
4. Verify the folder before every research launch:

   ```powershell
   .\verify-observation-only.ps1 -TesmioBuildRoot 'C:\path\to\tesmioloader\build'
   ```

5. If verification does not pass, do not launch the experiment. Do not bypass
   the upstream or probe executable identity gates.
6. Keep ordinary save backups and load a non-critical test republic.
7. Confirm TesmioLoader's log says the probe is armed. An unsupported build must
   instead say it was refused.
8. Open the Observatory Population workspace. Rust reads only the fixed JSONL
   file, validates it, and shows aggregate person-probe status.
9. To reconcile resources, explicitly enable one assurance mode in the
   Materials Resource Catalogue. Observatory may then retain the validated
   resource registry and its prices in local application data.

Delete the two manually copied plugin files to remove the companion. Anonymous
person samples are never retained by Observatory. Opted-in resource readings
are retained as immutable local snapshots and are labelled **Last verified in
a game session** after restart. They never become historical save facts.

## Bounds

The source clamps settings to 1–32 people, 1–365 days, 1,025–8,192 total
records, at most 512 resource entries, and exactly one fixed output file. The
Rust reader independently caps the file
at 4 MiB, 8,192 lines, and 16 KiB per line, rejects unknown fields, rejects
write/network capability claims, and rejects paths that escape the configured
game directory.
