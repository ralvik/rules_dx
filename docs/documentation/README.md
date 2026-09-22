# Documentation

Accepted v1 direction; no `Supported` claim and no site is published yet.
See [Delivery](delivery.md) for the pipeline and per-track delivery record.

Accepted design: native language tooling extracts API semantics; thin per-language adapters
normalize into one versioned [documentation IR](doc-ir.md); one [site
build](site.md) renders prose plus API reference. SCIP is out of v1.

## Contracts

- [Documentation IR](doc-ir.md): symbol model, validation, fixtures, and drift policy.
- [Site build](site.md): action design, determinism, laziness, and mdBook renderer.
- [Build check serve](build-check-serve.md): `bazel build //docs/...`, lint, and local serve (#620).
- [IR schema and codec](../ir/README.md): `doc_ir.proto` plus `documentation_ir` codec.
- [`dx docs`](../cli/commands/docs.md): build, check, and serve the unified site.

Delivery and adapter runs live in [Delivery](delivery.md).

```sh
bazel build //docs/...
```
