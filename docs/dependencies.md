# Dependency decisions

The foundation deliberately reuses WyrmGrid's proven interface stack while
keeping the Observatory independent and small.

| Dependency     | Licence    | Current role                                          | Boundary                                             |
| -------------- | ---------- | ----------------------------------------------------- | ---------------------------------------------------- |
| Svelte 5       | MIT        | Presentational components and local interaction state | No parsing or analytical business rules              |
| Apache ECharts | Apache-2.0 | Canvas chart rendering                                | Used only through `ObservatoryChart` and `ChartSpec` |
| Vite           | MIT        | Local development and production webview build        | Build-time only                                      |
| TypeScript     | Apache-2.0 | Interface and contract type checking                  | Build-time only                                      |
| Vitest         | MIT        | Calculation and chart-contract tests                  | Development only                                     |
| Prettier       | MIT        | Deterministic source formatting                       | Development only                                     |

[OnAir WyrmGrid](https://github.com/phobos-dthorga/onair-wyrmgrid) is a design
and architectural precedent, not a runtime dependency. Both repositories are
MIT-licensed, but shared packages are intentionally deferred until two current
consumers demonstrate genuinely identical semantics.

Tauri, Rust crates, and SQLite are introduced with the first real save-observer
vertical slice. MapLibre, Three.js, hosted services, plugin runtimes, and data
science environments remain outside the dependency set until a concrete player
question requires them.

Before adding a dependency, document:

1. the current player question it answers;
2. why the existing stack or a small local implementation is insufficient;
3. its licence and distribution effect;
4. its replacement boundary and unavailable behaviour; and
5. the tests, security review, or visual QA proportional to its impact.
