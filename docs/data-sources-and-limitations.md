# Data sources and limitations

This document records the presently observed evidence boundary. It is not a
promise that every field remains stable across game versions. Parser support
must be fixture-tested and version-aware.

## Save-container observations

Cloud saves observed on 26 August 2026 were ordinary ZIP archives. A save could
be inspected without extraction by opening `stats.ini` directly from the
archive. Large binary entries were also present, including building, worker,
rail, and vehicle payloads.

The proposed observer therefore follows this sequence:

1. notice a candidate ZIP;
2. wait until file size and modification time are stable;
3. validate the archive without modifying it;
4. hash the statistical payload and deduplicate it;
5. parse supported records into application-owned models;
6. determine timeline ancestry or start a separate branch; and
7. commit the observation atomically to local storage.

## `stats.ini` coverage

Observed global historical records contained fields suitable for:

- buy and sell prices in rubles and dollars, plus base prices;
- imports and exports by resource, currency, quantity, and value;
- construction, factory, shop, and vehicle resource use;
- factory, citizen, and demolition waste production;
- vehicle import and export values;
- a `Resources_Produced` collection;
- births, deaths, escapes, and immigration;
- tourism counts, spending, and scores;
- loan balance and interest; and
- several cost scalars.

An observed save contained 1,847 `$STAT_RECORD` blocks covering a long-running
republic. Adjacent record dates were usually five in-game days apart but varied
roughly between four and seven days. Charts must therefore use the actual game
date rather than record position.

The history appeared append-only across successive saves: a later save retained
the earlier record prefix and added new records. Two distinct save archives also
contained byte-identical `stats.ini` payloads, establishing the need for
content-based deduplication.

## Current and city snapshots

`$STAT_CURRENT` and `$STAT_CITY` blocks describe the latest state rather than a
complete embedded history. Their value comes from observing each newly created
save. Repeated observations enable save-sampled national and city trends,
save-to-save comparisons, and intervention studies.

City identifiers are not assumed to be display names. Until names and
coordinates are established by a supported source, the interface must use a
stable neutral label and explain the limitation.

## Game definitions

Installed game definitions can provide resource catalogues and production
recipes independently of a republic save. These are game-definition facts, not
observed republic activity. They can support theoretical material requirements,
limiting-input calculations, and scenario models, but cannot establish that a
specific building operated at that theoretical rate.

Definitions should be imported into versioned application-owned models. No game
assets are copied or redistributed.

## Known limitations

### Production coverage

`Resources_Produced` must not be treated as a complete record of all processed
factory output until coverage is verified resource by resource. A chart may
state “recorded production”; it may not silently relabel this as total domestic
production.

### Material conservation

The first parser does not establish all stock changes, production, loss, or
transfer paths. Therefore:

- sources and uses may be displayed side by side;
- an “accounted-flow” diagram may include an explicit unaccounted remainder;
- the remainder is a measurement residual, not waste or loss; and
- a closed mass balance or actual process yield is prohibited until inventory
  and production coverage are demonstrated.

### Save cadence

Global history is already embedded in a save, so saving more frequently does
not create finer historical records. Frequent observation is still useful for
current and city snapshots, branch detection, and before/after comparisons.

### Binary payloads

Individual factories, routes, vehicles, worker histories, inventories, network
topology, and geographic mapping are later research areas. The presence of a
binary file does not establish a stable or licensed public format.

## Data-quality presentation

Every analytical result carries:

- source kind and source identifier;
- observation or effective game date;
- parser and calculation version;
- coverage status: complete, partial, experimental, or unavailable;
- units, currency, denominator, and time window; and
- material caveats close to the visual.

Missing values remain missing. Zero is used only when the source explicitly
reports zero.
