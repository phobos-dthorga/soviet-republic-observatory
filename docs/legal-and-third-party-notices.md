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
planning overlays, language packs, themes, analytical databases, and optional
validated resource-registry readings remain local to the player's computer.
The retained resource reading contains only reviewed resource fields and the
prices observed in one captured game session. It is immutable, deduplicated,
and labelled as an earlier session after restart. It never replaces save-backed
Markets history.

Optional live environmental recordings are also app-local. Recording is off by
default and requires a clear player choice. A future checked report may contain
snapshot-local facility indices, neutral positions, building type, production,
pollution, radioactivity, water, and sewage readings. It must not contain raw
memory, personal identities, or arbitrary paths. A focused deletion action
removes these recordings without touching saves, installed content, ordinary
observations, or player carbon-factor revisions.

Carbon factors and their references are supplied by the player. Observatory
does not download or endorse an emissions-factor library. Its estimates are not
official game data and do not claim to measure a complete real-world footprint.

Anonymous person samples remain temporary and are not imported into the
application databases. The current application has no required network service
and keeps no credentials. Its only optional network action is an explicitly
confirmed download of one reviewed TesmioLoader source revision from GitHub.
GitHub receives normal connection details, including the player's IP address.
Its SQLite and DuckDB files are therefore intentionally unencrypted; future
credentials, if any, require an operating-system credential vault and a new
threat model.

## Software licences

The main Republic Observatory application and its original source are licensed
under the repository's MIT License.

The optional source in `research/tesmioloader-probe/observatory_probe.cpp` is
GPL-3.0-only. It is a separately built companion intended to compile against
reviewed TesmioLoader headers. TesmioLoader is separate GPL software owned by
its upstream contributors and governed by its own repository, licence, and
warranty terms. Republic Observatory does not vendor it in the desktop
application or activate it without confirmation.

A distributor of a compiled companion is responsible for satisfying the GPL,
including supplying the corresponding source and complete licence text. The
complete GPL text is included as `research/tesmioloader-probe/COPYING`. The
companion is not included in the Observatory desktop application binary.

After the revised research notice and a separate confirmation, the
in-application Experimental Research Setup assistant may download source for
TesmioLoader commit `3baa141f9f08921aea9c95f0a400289cabd9960a` from GitHub.
It validates the allowlisted build sources and exact header identities. It
retains those files, the upstream licence, and a provenance record. Redirects,
arbitrary URLs, downloaded loader binaries, and installers are not allowed.
Manual local selection remains available offline.

The assistant may compile the separately licensed probe from those reviewed
headers. After another confirmation, it may also build the reviewed loader and
launcher locally and prepare `W&R/tesmioloader/observatory`. That folder
contains only the checked session, licence, and ownership manifest.

Launching W&R is a separate confirmed action. It temporarily runs native code
inside the game process. Preparation never grants launch permission. Neither
action grants permission to edit game assets, Workshop content, or save data.
Observatory's probe requests no save write.

## Read-only research contract

The reviewed probe:

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
the simulation or write content. The complete loader session qualifies for
Observatory's observation-only badge only when the restricted settings are
active, the Observatory companion is the sole plugin DLL, both identity gates
remain enabled, and the supplied check passes. The check does not convert the
process into a sandbox.

The probe can also emit one bounded resource-registry record after the game
registry remains stable across consecutive rendered frames. It contains exact
resource tokens, a bounded caption, reviewed type fields, RUB and USD prices,
and validated buy and sell multipliers. It emits no raw records, pointers,
assets, paths, callbacks, or executable configuration. Observatory derives the
displayed buy and sell quotes and may retain the validated record only after
the player explicitly enables ingestion.

The checked report may also contain one of a small set of readiness labels.
These labels explain whether collection is waiting for W&R, a loaded republic,
or unable to roll its bounded scratch report forward. The report replaces only
its own temporary samples when nearing its line limit, then continues with a
fresh checked header. It does not grow without limit. These labels contain no
memory address, rejected live value, game path, person identity, or save content.

Two assurance modes are available:

- **Verified observation-only session** requires the documented restricted
  loader settings, the Observatory companion as the sole plugin DLL, both
  executable identity gates, and a fresh successful verification before each
  reading.
- **Player-managed modded session** validates Observatory's own probe and
  telemetry contract but does not install, enable, disable, or certify other
  plugins. The whole TesmioLoader session is not described as observation-only.

Both modes are optional. An unavailable, invalid, incompatible, disabled, or
absent probe cannot block save observation, Archive, installed-resource
browsing, catalogue refresh, Markets history, or aggregate Population
analytics.

## Evidence boundary

Probe records are reverse-engineering evidence. A vector position is not a
stable citizen identity. Sampled person fields do not prove family
relationships, causal effects, or a continuous life history. They are not
retained by Observatory.

A retained resource-registry reading describes only the captured session. It
does not prove that the same resources or prices remain active after restart,
and it is never attached to an earlier save as historical evidence. Exact
source tokens establish resource identity; captions, live indices, and similar
names do not.

## Warranty

The project and optional companion are provided under their respective licence
terms without warranty. Stop using a research build if the game or loader
behaves unexpectedly.
