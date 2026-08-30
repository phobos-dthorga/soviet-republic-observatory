# Dependency decisions

The foundation deliberately reuses WyrmGrid's proven interface stack while
keeping the Observatory independent and small.

| Dependency       | Licence    | Current role                                          | Boundary                                                              |
| ---------------- | ---------- | ----------------------------------------------------- | --------------------------------------------------------------------- |
| Svelte 5         | MIT        | Presentational components and local interaction state | No parsing or analytical business rules                               |
| Apache ECharts   | Apache-2.0 | Canvas chart rendering                                | Used only through `ObservatoryChart` and bounded host chart contracts |
| `@fluent/bundle` | Apache-2.0 | Message parsing, variables, and catalogue fallback    | Data-only host and community message patterns                         |
| `fluent-syntax`  | MIT/Apache | Authoritative Rust Fluent syntax validation           | Parse only; no executable translation modules                         |
| Vite             | MIT        | Local development and production webview build        | Build-time only                                                       |
| TypeScript       | Apache-2.0 | Interface and contract type checking                  | Build-time only                                                       |
| Vitest           | MIT        | Calculation and contract tests                        | Development only                                                      |
| Ajv              | MIT        | Draft 2020-12 schema-conformance proofs               | Development only; Rust host remains authoritative                     |
| Prettier         | MIT        | Deterministic source formatting                       | Development only                                                      |
| Tauri 2          | MIT/Apache | Native desktop/webview boundary                       | Thin bounded commands; no file paths reach Svelte                     |
| Dialog plugin    | MIT/Apache | Player-initiated native directory selection           | Directory selection only                                              |
| `notify`         | CC0-1.0    | Native save-directory event wake-ups                  | Hints only; reconciliation remains authoritative                      |
| `zip`            | MIT        | Streaming read of the selected `stats.ini` entry      | Strict archive/entry limits; no extraction                            |
| `rusqlite`       | MIT        | App-local observation and settings database           | Bundled unencrypted SQLite; append-only migrations                    |
| `duckdb`         | MIT        | Definition catalogue and analytical warehouse         | Pinned bundled engine; no extension autoload/install                  |
| `sha2`           | MIT/Apache | Payload and shared-prefix content identity            | Deduplication/provenance, not security attestation                    |
| `serde`          | MIT/Apache | Versioned command and storage models                  | Bounded application-owned structures                                  |

TesmioLoader is **not** an application dependency. The repository contains one
optional GPL-3.0-only companion source experiment that compiles against a
separately obtained TesmioLoader checkout. No TesmioLoader source, header,
binary, installer, or plugin DLL is bundled in the desktop application. The
main application remains MIT licensed and fully functional without it. See the
[legal and third-party notice](legal-and-third-party-notices.md) and
[ADR-0019](architecture/decisions/0019-optional-read-only-native-research-bridge.md).

[OnAir WyrmGrid](https://github.com/phobos-dthorga/onair-wyrmgrid) is a design
and architectural precedent, not a runtime dependency. Both repositories are
MIT-licensed, but shared packages are intentionally deferred until two current
consumers demonstrate genuinely identical semantics.

The small Fluent runtimes replace hand-written interpolation and pluralisation;
they do not permit executable translation modules. Rust owns desktop manifest
acceptance through `fluent-syntax`; `@fluent/bundle` formats the accepted text
in presentation. The source catalogue and manifests remain JSON so they can be
inspected and validated independently.

The Tauri/Rust/SQLite group is the authoritative save-observation boundary.
Its purpose is narrow: choose directories with explicit player action, inspect
bounded archives read-only, normalise supported facts, compact shared history,
and persist provenance locally. The automatic observer uses `notify` behind a
small native-service boundary. Filesystem events only reduce latency: the
service still reconciles the complete directory periodically, so a dropped or
coalesced platform event cannot become silent data loss. The webview receives
status events but does not drive recorder liveness. Ajv
prevents the checked-in Analysis Pack examples and invalid fixtures from
drifting away from Draft 2020-12 during development. It also proves the public
compatibility-profile contract rejects unknown fields, operations, host slots,
paths, markup, URLs, and excessive mappings; it is not a desktop trust
boundary. MapLibre, Three.js, hosted services, executable plugin runtimes, and
data science environments remain outside the dependency set until a concrete
player question requires them.

SQLite encryption, SQLCipher, and ORM/database-portability infrastructure are
intentionally absent. The SQLite and DuckDB databases contain no secrets, so OS
file permissions are sufficient and no application key
lifecycle is justified. If future credentials are introduced, they should use
the operating system's credential vault rather than being placed in SQLite.

Bundled DuckDB is introduced for the demonstrated catalogue and matrix-query
workload. The crate is pinned to `1.10505.0` and distributed under MIT. Runtime
extension autoloading, automatic installation, and external access are disabled;
the project does not use DuckDB's SQLite extension or download optional
extensions. Data crosses from SQLite only through application-owned Rust models.

Before adding a dependency, document:

1. the current player question it answers;
2. why the existing stack or a small local implementation is insufficient;
3. its licence and distribution effect;
4. its replacement boundary and unavailable behaviour; and
5. the tests, security review, or visual QA proportional to its impact.
