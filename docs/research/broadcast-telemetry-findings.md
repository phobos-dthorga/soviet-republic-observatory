# Broadcast telemetry research findings

## Purpose

This register records positive and negative Broadcast findings. It prevents an
interesting address, graph, or matching trend from becoming a product claim
before it is independently reproduced. Ordinary Observatory operation remains
save-backed and does not require TesmioLoader or Ghidra.

Last reviewed: **2026-09-02**.

## Findings that may be used now

| Source                               | Finding                                                                                      | Product use                                                                |
| ------------------------------------ | -------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Reviewed `stats.ini` history         | Four aggregate receiver classes: no receiver, radio, television, and computer                | Counts, classified-population shares, changes, and exact-save history      |
| Reviewed `stats.ini` indexed history | Nine citizen-status values for indices 0–8                                                   | Exact status histories and exploratory first-difference comparisons        |
| Active game-definition catalogue     | Nominal worker and professor places for unambiguous radio and television station definitions | Design context only; never actual staffing                                 |
| Markets source resources             | Exact tokens `eletronics` and `ecomponents`                                                  | Related trade and price context with currencies and channels kept separate |
| Game-definition recipes              | Exact production outputs for the two source resources                                        | Related production routes; never observed production or cause of adoption  |

## Negative findings and prohibited inferences

The reviewed save evidence and current probe contract do **not** establish:

- a stable radio or television station identity;
- actual station staffing, operating state, programme allocations, rating, or
  budget;
- potential or current audience;
- a join from receiver ownership to a person, age, education, household, city,
  or other demographic group; or
- a causal path from electronics prices, trade, production, or programme
  settings to receiver uptake or citizen status.

The current Tesmio sampling record contains anonymous citizen status values,
but its vector position is session-local and is not a persistent identity. It
cannot support biographies, demographic receiver ownership, or a cross-save
join. No nearest-date substitution, display-name match, pointer value, filename,
timestamp, or visual correlation may fill these gaps.

## Read-only research track

The next experiments may use the reviewed TesmioLoader headers and Ghidra as
research tools. The application does not install or run a loader. A researcher
must use the observation-only host configuration, the sole-plugin preflight,
backed-up saves, and the separately built bounded probe.

Candidate targets are station identity, station kind, staffing, programme
allocations, potential audience, current audience, rating, and budget. Each
experiment changes one visible game value at a time and records:

1. exact game build and executable identity;
2. the UI value before and after the change;
3. candidate address derivation and expected bytes;
4. bounded probe output before and after save/reload;
5. behaviour with two stations of each kind;
6. restart behaviour and malformed-output handling; and
7. the negative result when the candidate does not track the UI.

## Promotion gate

A candidate remains research-only until all of the following are true:

- its build-specific derivation does not use a hard-coded process pointer;
- controlled one-variable experiments match the game UI;
- the value survives save/reload and application restarts as expected;
- radio, television, and multiple stations remain distinct;
- identity, range, and malformed-output fixtures pass;
- collection does not change game memory, game files, save files, or
  Observatory evidence; and
- the unavailable path remains safe on every unsupported build.

Passing the gate permits a new reviewed compatibility mapping. It does not by
itself justify causal language. Programme-versus-outcome work remains an
association study unless a separate research design supports a stronger claim.
