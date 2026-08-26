# Extension threat model

## Scope

This model covers inert Analysis Packs now and informs future executable Model
Plugins. It does not claim that unimplemented controls already exist.

## Assets to protect

- original save archives and their timestamps;
- local observation history, plans, and annotations;
- filesystem paths and account identity;
- integrity of metrics, provenance, and recommendations;
- availability of save observation and core dashboards;
- application credentials should optional network features ever exist; and
- the player's ability to install and run the Observatory offline.

## Trust boundaries

An extension package is untrusted input. Normalised observations exposed by the
host are a bounded public contract. Host calculation, storage, renderer,
settings, lifecycle, and provenance services remain trusted application code.

First-party placement does not move the boundary. Community and bundled
extensions use the same contract.

## Analysis Pack threats and controls

| Threat                                     | Current schema-proof control                                       |
| ------------------------------------------ | ------------------------------------------------------------------ |
| Script or expression execution             | No expression field; strict unknown-field rejection                |
| HTML or CSS injection                      | No markup field; strings render as text                            |
| Renderer escape through ECharts options    | Templates expose only application-owned chart fields               |
| File, path, URL, SQL, or database access   | No corresponding vocabulary                                        |
| Resource exhaustion                        | Hard limits on metrics, charts, series, and operands               |
| Dependency cycles or hidden forward values | Ordered references; semantic forward-reference and cycle rejection |
| Cross-branch evidence splicing             | Host-controlled alignment within branch, date, and scope           |
| Forged trust or provenance                 | Host assigns provenance from content and source observations       |
| Identity squatting or payload replacement  | Future content identity binds exact ID and version                 |
| Host-language pack rewrites pack claims    | Pack prose stays author-owned and has a declared/defaulted locale  |

Ajv is a development conformance check. The future Rust host must validate the
same structural and semantic rules before installation or evaluation.

## Future Model Plugin threats

Executable programs add arbitrary-code, filesystem, network, persistence,
resource-exhaustion, protocol-confusion, supply-chain, and confused-deputy
risks. Out-of-process execution improves crash containment but is not an
operating-system sandbox.

Before a Model Plugin contract is published, its security review must define:

- signed or content-addressed package identity and immutable version handling;
- bounded message sizes, timeouts, memory and CPU policy, and cancellation;
- deny-by-default capabilities with explicit user approval;
- exact ID, version, content hash, and requested-scope binding;
- language-neutral message framing and compatibility negotiation;
- filesystem and network controls on each supported operating system;
- update, rollback, disable, quarantine, and removal behaviour;
- log redaction and error disclosure limits; and
- tests proving that a crashed, hung, malformed, or malicious plugin cannot
  block observation, storage, core analytics, or another extension.

## Capability model

Capabilities describe narrow host services, not ambient access. A plugin cannot
request “filesystem” or “all data.” Candidate future scopes might name exact
normalised metric families, date-window limits, or a versioned game-definition
model. Raw saves, binaries, SQLite, parser internals, private paths, credentials,
and direct chart rendering remain outside the contract.

Install, permission approval, enable, and start are distinct. Updating content
invalidates the prior content-bound decision until reviewed again.

## External delivery invariant

Extensions are independently obtainable and locally installable. Building one
into the application source tree is not the only supported delivery path. An
optional catalogue may distribute metadata and packages later, but core local
installation cannot depend on catalogue availability or an account.

## Residual risk and unavailable behaviour

Strict declarative schemas reduce attack surface but do not prove that a metric
is meaningful. Misleading formulas, denominator mistakes, and persuasive labels
remain analytical risks. Review, provenance, bounded descriptions, host metric
definitions, and user-visible disable controls address them; they do not remove
the need for judgement.

An invalid, incompatible, or failed extension is unavailable with a reason. It
must not degrade the scanner, parser, storage, Briefing, Broadcast Desk, or other
extensions.
