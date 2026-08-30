# Republic Observatory

**A local-first planning and statistical observatory for _Workers & Resources:
Soviet Republic_ saves.**

Republic Observatory watches newly created save archives, preserves their
branch-aware history, and turns the republic's statistics into explanations,
plans, experiments, and forecasts. It is intended to answer four recurring
questions:

1. What changed?
2. Is the change ordinary variation or a meaningful signal?
3. What probably contributed to it?
4. What happens if the player intervenes?

> **Project status:** native near-live observation and branch-aware archive. The
> Tauri desktop program reacts to new save-file events, reconciles the folder as
> a fallback, waits for newly written saves to stabilise, parses supported
> history plus current/city snapshots from `stats.ini`, compact shared history
> prefixes in app-local SQLite, records a crash-recoverable candidate ledger,
> projects idempotently into a bundled DuckDB analytical warehouse, catalogues
> local base/DLC/Workshop/WIP definitions, and supports inert planning overlays.
> It resolves branches, compares states on one branch, and presents Observer
> Health, Republic Pulse, and the Industrial Catalogue. Unrelated dashboard
> values remain visibly synthetic.

![Republic Observatory interface foundation](assets/screenshots/interface-foundation.png)

## Proposed experience

- **Republic Briefing** — plan attainment, external dependency, demographic
  resilience, guardrails, and a concise save-to-save dispatch.
- **Republic Monitor** — native recorder health, candidate lifecycle, recording
  cadence, branch warnings, and latest same-branch receiver movement.
- **Five-Year Plan** — targets, actual-versus-plan progress, variance bridges,
  confidence ranges, milestones, and scenario testing.
- **Material Periodic Table** — every resource presented as a compact cell with
  trade, price, use, risk, and provenance context.
- **Industrial Laboratory** — production-chain diagrams, limiting-reagent
  analysis, theoretical yield, sensitivity, and optimisation.
- **Broadcast Desk** — receiver adoption, audience research, programme
  formulation, influence assays, intervention notes, and a deterministic
  Evening Bulletin.
- **Community Extensions** — inert Analysis Packs over normalised metrics first,
  with isolated executable models deferred until a demonstrated need and
  security review.
- **Population and Welfare** — demographic decomposition, statistically useful
  control charts, and city comparison without hiding behind national averages.
- **Trade and Markets** — price indices, concentration, currency exposure,
  break-even analysis, market response, debt, and tourism yield.

The complete proposal is in the [project brief](docs/project-brief.md),
[analytical catalogue](docs/analytical-catalogue.md), and
[interface specification](docs/dashboard-and-interface.md).

## Interface foundation

The interface follows the methodology established by
[OnAir WyrmGrid](https://github.com/phobos-dthorga/onair-wyrmgrid):

- Svelte 5 and TypeScript;
- semantic theme tokens and a dense operational workspace;
- explicit live/historical and source states;
- provenance attached to every chart;
- Apache ECharts behind one declarative application-owned adapter;
- canonical `en-AU`, Fluent formatting, and strict inert community language
  packs with complete current-interface coverage;
- a local Analysis Pack library with authoritative Rust validation, immutable
  SQLite revisions, explicit enablement, host evaluation, and host-rendered
  charts;
- a local-first, summary-to-diagnosis information hierarchy; and
- live phase/counter reporting for long catalogue work and a bounded,
  user-readable local diagnostic log derived from WyrmGrid's operational model.

It is deliberately not a WyrmGrid reskin. The Observatory has its own visual
identity, resource vocabulary, navigation, statistical contracts, and game-save
architecture. See [ADR-0002](docs/architecture/decisions/0002-wyrmgrid-interface-methodology.md).

## Run the program

Requires Node.js 22.12 or newer, npm 10 or newer, Rust 1.88 or newer, and the
normal [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for
Windows.

```powershell
npm install
npm run desktop
```

Open **Save observer** in the upper-right corner, choose the folder containing
the game's save ZIP files, and either select **Observe newest save** or enable
**Observe new stable saves automatically**. Automatic observation runs while
the desktop program is open, responds to native file events, reconciles every 15
seconds as a fallback, waits for unchanged file metadata before reading, and
retries temporary incomplete archives. The source archive remains untouched.
Use **Monitor** to inspect recorder health and Republic Pulse, and **Archive** to
inspect ancestry, select a timeline branch, and
compare two distinct states on that branch. Configure the game installation
folder to build the Industrial Catalogue from local base, DLC, subscribed
Workshop, and WIP definitions. A separate Workshop-folder selector is available
when Steam discovery cannot resolve external Workshop content. The header shows
active critical work; **Materials** provides the staged Catalogue ledger with
bounded per-file detail, and **Diagnostics** exposes the local operational
record when a run needs investigation. Startup work is recovered from a durable
native snapshot, so mounting the interface late does not hide an active task.

For interface work that does not need native folder selection or save parsing,
`npm run dev` opens the synthetic browser preview.

Validate the foundation with:

```powershell
npm run format:check
npm run check
npm test
npm run build
npm run rust:check
npm run rust:test
```

## Proposed technical foundation

| Area                          | Direction                                                    |
| ----------------------------- | ------------------------------------------------------------ |
| Desktop shell                 | Tauri 2                                                      |
| Save observation and services | Rust, native folder events, and bounded read-only ZIP access |
| Interface                     | Svelte 5 and TypeScript                                      |
| Charts                        | Apache ECharts behind `ObservatoryChart`                     |
| Operational storage           | App-local unencrypted SQLite with append-only migrations     |
| Catalogue and analytics       | App-local bundled DuckDB with independent migrations         |
| Input                         | ZIP saves read non-destructively; `stats.ini` first          |
| Statistical models            | Application-owned, versioned and provenance-labelled         |

The browser preview and desktop program share one presentation layer. Native
folder access, archive parsing, provenance, and persistence stay behind the
small Tauri command boundary and are unavailable to ordinary browser code.

## Documentation

- [Documentation index](docs/README.md)
- [Data sources and limitations](docs/data-sources-and-limitations.md)
- [Dependency decisions](docs/dependencies.md)
- [Metric definitions](docs/metric-definitions.md)
- [Material Periodic Table and Industrial Laboratory](docs/material-periodic-table.md)
- [Broadcast Desk](docs/broadcast-desk.md)
- [Community Extensions](docs/extensions/overview.md)
- [Localisation and language-pack authoring](docs/localization/README.md)
- [Architecture](docs/architecture/overview.md)
- [Roadmap](docs/roadmap.md)
- [Contributing](CONTRIBUTING.md)

## Independence and trademarks

Republic Observatory is an independent community project. It is not affiliated
with, endorsed by, or sponsored by 3Division or Hooded Horse. _Workers &
Resources: Soviet Republic_ and related names may be trademarks of their
respective owners. No game assets or save data are distributed by this project.

Source code and documentation are available under the [MIT License](LICENSE).
