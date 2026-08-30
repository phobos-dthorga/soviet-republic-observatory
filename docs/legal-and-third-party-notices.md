# Legal and third-party notices

This document records Republic Observatory's intended ownership, licence,
privacy, and optional native-research boundaries in plain language. It is a
project notice, **not legal advice**, and does not replace review by qualified
counsel.

## Independent community project

Republic Observatory is an independent community project. It is not affiliated
with, endorsed by, sponsored by, or supported by 3Division or Hooded Horse.
Workers & Resources: Soviet Republic, its name, software, formats, and assets
belong to their respective owners. The repository must not redistribute game
assets, save archives, installed definitions, or proprietary binaries.

## Local application data

Observed-save metadata, normalized facts, settings, catalogue generations,
planning overlays, language packs, themes, and analytical databases remain
local to the player's computer. The current application has no required
network service and keeps no credentials. Its SQLite and DuckDB files are
therefore intentionally unencrypted; future credentials, if any, require an
operating-system credential vault and a new threat model.

## Software licences

The main Republic Observatory application and its original source are licensed
under the repository's MIT License.

The optional source in `research/tesmioloader-probe/observatory_probe.cpp` is
GPL-3.0-only. It is a separately built companion intended to compile against a
separately obtained TesmioLoader checkout. TesmioLoader is separate GPL software
owned by its upstream contributors and governed by its own repository, licence,
installation instructions, and warranty terms. Republic Observatory does not
vendor, bundle, silently install, or activate TesmioLoader.

A distributor of a compiled companion is responsible for satisfying the GPL,
including supplying the corresponding source and complete licence text. The
complete GPL text is included as `research/tesmioloader-probe/COPYING`. The
companion is not included in the Observatory desktop application binary.

The in-application Experimental Research Setup assistant may compile the
separately licensed probe from local reviewed sources. It does not obtain or
install TesmioLoader, inject into the game, launch W&R, or run the probe. A
successful build records only the bounded artifact identity and must not be
interpreted as installation, activation, compatibility, or a sandbox claim.

## Read-only research contract

The reviewed experiment:

- reads bounded fields from objects already loaded by the game;
- installs one chainable observation hook and calls the original function;
- writes only a fixed, bounded telemetry file of its own;
- does not write simulation objects, serializer buffers, saves, game files,
  Observatory databases, or network resources;
- emits no citizen names, raw addresses, full object dumps, or arbitrary paths;
- refuses an executable identity that has not been reviewed.

“Read-only” describes the probe's intended data behaviour. TesmioLoader and a
plugin DLL still run inside the game process, and installing a hook changes that
process's execution environment. This is not operating-system sandboxing and is
not risk-free. Players should keep ordinary save backups and use a non-critical
test republic for initial experiments.

TesmioLoader is a general modding platform and is not inherently read-only. Its
compiled defaults enable virtual-file redirection, save-manifest handling, and
the loading of plugin DLLs; upstream gameplay plugins may intentionally alter
the simulation or write content. The Observatory experiment is within this
contract only when its documented observation-only settings are active, the
Observatory companion is the sole plugin DLL, both executable identity gates
remain enabled, and the supplied configuration verifier passes before launch.
The verifier does not convert the process into a sandbox.

The experiment is optional. An unavailable, invalid, incompatible, or absent
probe cannot block save observation, Archive, catalogue refresh, or aggregate
Population analytics.

## Evidence boundary

Probe records are reverse-engineering evidence. A vector position is not a
stable citizen identity. Sampled fields do not prove family relationships,
causal effects, or a continuous life history. Republic Observatory must keep
those features unavailable until independent save/runtime observations and
fixtures establish a safe contract.

## Warranty

The project and optional companion are provided under their respective licence
terms without warranty. Stop using a research build if the game or loader
behaves unexpectedly.
