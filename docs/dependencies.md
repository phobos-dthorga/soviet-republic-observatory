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

[OnAir WyrmGrid](https://github.com/phobos-dthorga/onair-wyrmgrid) is a design
and architectural precedent, not a runtime dependency. Both repositories are
MIT-licensed, but shared packages are intentionally deferred until two current
consumers demonstrate genuinely identical semantics.

The small Fluent runtime replaces hand-written interpolation and pluralisation;
it does not permit executable translation modules. The source catalogue and
manifests remain JSON so they can be inspected and validated independently.

Tauri, Rust crates, and SQLite are introduced with the first real save-observer
vertical slice. Ajv prevents the checked-in Analysis Pack examples and invalid
fixtures from drifting away from Draft 2020-12 during development; it is not a
desktop trust boundary. MapLibre, Three.js, hosted services, executable plugin
runtimes, and data science environments remain outside the dependency set until
a concrete player question requires them.

Before adding a dependency, document:

1. the current player question it answers;
2. why the existing stack or a small local implementation is insufficient;
3. its licence and distribution effect;
4. its replacement boundary and unavailable behaviour; and
5. the tests, security review, or visual QA proportional to its impact.
