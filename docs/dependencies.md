# Dependency decisions

The foundation deliberately reuses WyrmGrid's proven interface stack while
keeping the Observatory independent and small.

| Dependency       | Licence    | Current role                                          | Boundary                                             |
| ---------------- | ---------- | ----------------------------------------------------- | ---------------------------------------------------- |
| Svelte 5         | MIT        | Presentational components and local interaction state | No parsing or analytical business rules              |
| Apache ECharts   | Apache-2.0 | Canvas chart rendering                                | Used only through `ObservatoryChart` and `ChartSpec` |
| `@fluent/bundle` | Apache-2.0 | Message parsing, variables, and catalogue fallback    | Data-only host and community message patterns        |
| Vite             | MIT        | Local development and production webview build        | Build-time only                                      |
| TypeScript       | Apache-2.0 | Interface and contract type checking                  | Build-time only                                      |
| Vitest           | MIT        | Calculation and contract tests                        | Development only                                     |
| Ajv              | MIT        | Draft 2020-12 schema-conformance proofs               | Development only; Rust host remains authoritative    |
| Prettier         | MIT        | Deterministic source formatting                       | Development only                                     |
| Tauri 2          | MIT/Apache | Native desktop/webview boundary                       | Thin bounded commands; no file paths reach Svelte    |
| Dialog plugin    | MIT/Apache | Player-initiated native directory selection           | Directory selection only; no implicit scanning       |
| `zip`            | MIT        | Streaming read of the selected `stats.ini` entry      | Strict archive/entry limits; no extraction           |
| `rusqlite`       | MIT        | App-local observation and settings database           | Bundled unencrypted SQLite; append-only migrations   |
| `sha2`           | MIT/Apache | Statistical-payload content identity                  | Deduplication/provenance, not security attestation   |
| `serde`          | MIT/Apache | Versioned command and storage models                  | Bounded application-owned structures                 |

[OnAir WyrmGrid](https://github.com/phobos-dthorga/onair-wyrmgrid) is a design
and architectural precedent, not a runtime dependency. Both repositories are
MIT-licensed, but shared packages are intentionally deferred until two current
consumers demonstrate genuinely identical semantics.

The small Fluent runtime replaces hand-written interpolation and pluralisation;
it does not permit executable translation modules. The source catalogue and
manifests remain JSON so they can be inspected and validated independently.

The Tauri/Rust/SQLite group is now the authoritative save-observation boundary.
Its purpose is narrow: choose directories with explicit player action, inspect
one bounded archive read-only, normalise supported facts, and persist provenance
locally. Ajv prevents the checked-in Analysis Pack examples and invalid fixtures
from drifting away from Draft 2020-12 during development; it is not a desktop
trust boundary. MapLibre, Three.js, hosted services, executable plugin runtimes,
and data science environments remain outside the dependency set until a
concrete player question requires them.

SQLite encryption, SQLCipher, ORM/database-portability infrastructure, and a
second database engine are intentionally absent. The current database contains
no secrets, so OS file permissions are sufficient and no application key
lifecycle is justified. If future credentials are introduced, they should use
the operating system's credential vault rather than being placed in SQLite.

Before adding a dependency, document:

1. the current player question it answers;
2. why the existing stack or a small local implementation is insufficient;
3. its licence and distribution effect;
4. its replacement boundary and unavailable behaviour; and
5. the tests, security review, or visual QA proportional to its impact.
