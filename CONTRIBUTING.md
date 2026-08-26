# Contributing

Republic Observatory is at its foundation stage. Contributions that improve
the documented save format, metric definitions, statistical safeguards,
accessibility, or the first vertical slice are welcome.

Before changing code, read [AGENTS.md](AGENTS.md), the
[architecture overview](docs/architecture/overview.md), and the relevant
decision records. Do not attach real saves to an issue. Reduce a reported case
to a sanitised fixture containing only the minimum fields needed to reproduce
it.

## Local checks

```powershell
npm install
npm run format:check
npm run check
npm test
npm run build
```

Pull requests should explain the player question being improved, the provenance
of any new field, the behaviour when that field is unavailable, and the tests
or visual checks performed.

All new interface text must enter the canonical `en-AU` catalogue. Use the
central locale-formatting helpers and explicit typed translation-key mappings;
do not construct keys dynamically. Language-pack contributions should follow
the [localisation guide](docs/localization/README.md), retain Fluent variables,
and avoid protected evidence and safety namespaces. New Analysis Pack prose
should declare its own `default_locale` and is not host UI text.
