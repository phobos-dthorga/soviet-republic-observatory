# Analysis Pack v1 authoring guide

## Current status

Analysis Pack v1 is a live local extension contract. The Rust desktop host is
the authoritative structural and semantic validator. Ajv remains a separate
development conformance check for the published schemas and examples.

Start from
[`receiver-adoption-laboratory.roanalysis.json`](../../examples/analysis-packs/receiver-adoption-laboratory.roanalysis.json)
and validate against
[`analysis-pack-v1.schema.json`](../../schemas/analysis-pack-v1.schema.json).

## Identity

Every pack declares:

- `schema_version: 1`;
- a lower-case reverse-domain `id`;
- a semantic `version`;
- `host_api_version: 1`;
- a `default_locale` identifying the language of author-owned prose for every
  newly authored pack;
- bounded `name`, `author`, and `description`; and
- arrays of derived metrics and chart templates.

Identity is security-sensitive. A future permission grant will bind ID,
version, content hash, and requested scope, not just the display name.

## Local lifecycle

In the desktop application's **Extensions** workspace:

1. choose **Inspect local pack**, or inspect the included example;
2. review its identity, content hash, inputs, metrics, charts, and validation
   result;
3. choose **Import inspected pack**; import stores an immutable SQLite revision
   and leaves it disabled;
4. enable the desired revision explicitly; and
5. disable, roll back, export, or remove it independently.

Importing changed content under the same pack ID creates a new revision. It does
not silently replace the enabled revision. Enabled packs are evaluated only
against the currently selected timeline branch and geographic scope. The JSON
file is read as content: its source path is not part of the public host API.

The host localises controls surrounding the pack but does not translate or
silently rewrite the pack's name, description, metric labels, or chart prose.
Those strings are displayed as author-owned content tagged with
`default_locale`. The field was added compatibly after schema v1 was published:
an older v1 file without it remains valid and is treated as `en-AU`. A future
multilingual package requires an explicit versioned resource contract; do not
encode translations into IDs or publish several conflicting files under the
same ID and version.

## Published core metrics

The schema accepts syntactically valid core metric references; the host API
decides which are actually published. Host API 1 currently proves these four:

- `core.citizens.electronics.none`
- `core.citizens.electronics.radio`
- `core.citizens.electronics.television`
- `core.citizens.electronics.computer`

Referencing an unregistered metric is a semantic validation error.

## Derived metrics

A pack may declare up to 64 metrics in order. A metric can reference a published
core metric or a **previously declared** local derived metric. Forward references
and cycles are invalid.

Version 1 has five operations:

| Operation    | Contract                                                        |
| ------------ | --------------------------------------------------------------- |
| `sum`        | Two to 16 operands; unavailable if an operand is unavailable    |
| `difference` | One minuend minus one subtrahend                                |
| `product`    | Two to 16 operands; unavailable if an operand is unavailable    |
| `safe_ratio` | Numerator divided by denominator, with optional numeric `scale` |
| `scale`      | One operand multiplied by a numeric `factor`                    |

`safe_ratio` returns unavailable when its denominator is missing, non-finite,
or zero. Other operations also reject missing or non-finite inputs. The host
does not silently substitute zero.

Inputs are aligned only within the same timeline branch, observation date, and
geographic scope. A pack cannot select another branch, splice histories, or
request interpolation. Observation gaps remain gaps.

## Chart templates

A pack may declare up to 16 chart templates, with up to 12 series each. Schema
version 1 supports `line`, `area`, and `bar`. A template selects metric IDs and
may declare labels, orientation, line style, stack identity, unit, and a fixed
value domain.

The template contains no observations or provenance. The host resolves aligned
metric results into its concrete `ChartSpec`, assigns evidence, supplies textual
summaries, and renders through the application-owned adapter.

Authors cannot provide renderer options, tooltip callbacks, formatters, colour
tokens, HTML, CSS, or ECharts configuration. Promote a useful chart into primary
navigation by changing host placement—not its data access.

## Limits and validation

Schema validation rejects unknown properties, unsafe IDs, malformed semantic
versions, unsupported operations, excessive arrays, and renderer or executable
payload fields. Semantic validation rejects:

- duplicate derived metric, chart, or within-chart series IDs;
- forward or unknown derived-metric references;
- unavailable core metrics; and
- value domains where `min >= max`.

The development suite also tests script, HTML, and ECharts injection attempts.
These strings are never evaluated; unknown payload fields fail validation.

Run the development conformance suite with:

```powershell
npm test
```

## Provenance

An Analysis Pack does not declare itself trusted, accurate, complete, or causal.
For every concrete result, the host records:

- pack ID and semantic version;
- content hash;
- calculation rule and host API version;
- source observation identities, branch, date, and geographic scope;
- coverage and unavailable reasons; and
- concrete chart schema version where applicable.

The public evidence kind is `extension_calculation`, distinct from a built-in
calculation.

## When the vocabulary is insufficient

Do not smuggle an expression into a label or encode a model in renderer
configuration. Document the desired model, required inputs, assumptions,
outputs, failure behaviour, and why the five operations cannot represent it.
That demonstrated case will inform the later Model Plugin security and protocol
review.
