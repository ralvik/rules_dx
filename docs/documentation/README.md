# Documentation

Accepted v1 direction; adapter runs plus renderer/site execution plus rebuild
proof plus link/reference completeness plus guide-step wiring plus first-hour timing
plus per-release pin-bump plus drift process delivered seed-only (see Contracts). No `Supported`
claim; fixture-scale adapter plus site execution plus rebuild proof plus link completeness plus guide-step wiring plus timing plus drift are qualified
and no site is published yet.

Accepted design: native language tooling extracts API semantics; thin per-language adapters
normalize into one versioned [documentation IR](doc-ir.md); one [site
build](site.md) renders prose plus API reference with a single theme,
navigation, URL model, and search. SCIP code intelligence is out of v1; it is not needed for docs.

## Pipeline

```text
native machine output (rustdoc JSON, TypeDoc JSON, Griffe, Doclet, ...)
        ↓
thin language adapter (one package per language, pinned inputs)
        ↓
versioned documentation IR (generated Bazel outputs, cached by Bazel)
        ↓
site build (Bazel-cached actions: extract → aggregate → render)
        ↓
mdBook-compatible prose + generated API pages + one search index
```

Generated IR stays in Bazel outputs, not beside source files or in Git. The `.proto`
schema and `documentation_ir` codec tests are checked in; per-language adapter runs
with golden fixtures delivered under #779 (successor to closed #581). Ordinary
API changes require no IR snapshot update.

## Contracts

- [Documentation IR](doc-ir.md): common symbol model, language extensions,
  validation, fixtures, and drift policy with the per-release pin-bump plus
  drift process. Authoritative for IR facts.
- [Site build](site.md): Bazel cache-friendly action design, determinism
  rules plus delivered byte-identical rebuild proof, laziness, generated-artifact lifecycle,
  and the decided mdBook renderer.
  Authoritative for build facts.
- [Build check serve](build-check-serve.md): accepted `bazel build //docs/...`,
  `dx lint --check //docs/...`, and local serve with existing tools only
  (#620). Authoritative for build/check/serve facts.
- [IR schema and codec](../ir/README.md): checked-in `doc_ir.proto` plus
  `documentation_ir` codec; design facts stay in the contracts above.
- Command surface delivered under #786 (successor to closed #581,
  live successor to closed #421; see [`dx docs`](../cli/commands/docs.md)).
  [ADR 0006](../decisions/0006-cli-command-surface.md) records build versus
  validation-only check; exact mappings are implemented per that split.
  Adapter runs with pins and mappings delivered under #779 (successor
  to closed #581); renderer and site execution delivered seed-only under #780;
  site-level byte-identical rebuild proof delivered seed-only under #781;
  link and reference completeness delivered seed-only under #782;
  guide prose with guide-step CI wiring delivered seed-only under #783;
  first-hour timing proof delivered seed-only under #784 as one-shot evidence
  per [ADR 0022](../decisions/0022-no-benchmarking.md), not a standing benchmark;
  per-release pin-bump plus drift process delivered seed-only under #785.
  Docs pipeline gaps stay open under no open issue (successors
  to closed #581; #779 plus #780 plus #781 plus #782 plus #783 plus #784 plus #785
  delivered seed-only;
  fixture-scale execution qualified and no site is published yet).

Rust uses pinned nightly `rustdoc --output-format json`; Scala TASTy spike
delivered; Astro/MDX prose-only confirmed.
Accepted scope covers thirteen adapter scopes. Per-language input pins,
mappings, and adapter runs delivered under #779 with golden fixtures;
renderer/site-build execution delivered seed-only under #780; rebuild proof
delivered seed-only under #781; link and reference completeness delivered
seed-only under #782; guide prose with guide-step CI wiring delivered
seed-only under #783; first-hour timing proof delivered seed-only under #784;
per-release pin-bump plus drift process delivered seed-only under #785;
no items above stay open;
no published site exists today.

## Related issues

Tracking lives in GitHub issues. Docs pipeline: #779 (adapters delivered) plus #780 (site delivered) plus #781 (rebuild delivered) plus #782 (link completeness delivered) plus #783 (guide-step wiring delivered) plus #784 (timing delivered) plus #785 (drift delivered). Reintroduction: #786 (delivered). Build workflow: #620.
