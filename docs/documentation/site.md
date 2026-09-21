# Documentation Site Build

Implementation status: accepted v1 direction with adapter runs delivered under #779 plus
renderer/site execution delivered seed-only under #780 (successors to closed #581,
live successor to closed #421); site-level byte-identical rebuild proof delivered
seed-only under #781; link/reference completeness delivered seed-only under #782
plus guide-step CI wiring delivered seed-only under #783
plus first-hour timing proof delivered seed-only under #784
plus per-release pin-bump plus drift process delivered seed-only under #785;
no execution gaps remain.
Accepted: the `dx_docs` site-build action planning over the
Bazel-cached extract→aggregate→render graph (no committed IR). Delivered: per-language
extraction runs under #779 (`//docs/adapters:docs_adapters` over pinned inputs) plus fixture-scale
execution in [`docs/site/`](../../docs/site/site.bzl) (`docs_extract` per unit,
`docs_aggregate` with shared validation, `docs_render` with the pinned mdBook
artifact) producing mdBook-compatible prose plus generated API pages plus one search
index, with generated IR in Bazel outputs only. Command dispatch was
removed per [ADR 0020](../decisions/0020-remove-dx-docs-placeholder.md); reintroduction
is open under #786. mdBook is the decided
renderer with no planned replacement. Cache and hermeticity properties
below are delivered seed-only for the fixture-scale site (rebuild proof under #781);
link/reference completeness delivered seed-only under #782; guide-step CI wiring
delivered seed-only under #783; first-hour timing proof delivered seed-only under
#784; per-release pin-bump plus drift process delivered seed-only under #785;
remaining cache-invalidation verification follows
[Testing](../testing/) before any support statement; cache reuse and
invalidation fixtures stay advisory with no owning execution gap.
Fixture-scale site execution plus byte-identical rebuild proof plus link completeness plus guide-step wiring plus first-hour timing plus drift are qualified seed-only; no published site is claimed.

## Action Graph

```text
sources + pinned extractor + pinned toolchain (declared inputs)
        ↓  one DocsExtract action per (language, package) unit
IR shard Protobuf (one generated Bazel output per unit, Bazel-cached)
        ↓  one DocsAggregate action (shards + prose + theme/config, shared validation)
render inputs (Markdown tree + search index records)
        ↓  one DocsRender action (pinned mdBook artifact)
complete static site (single output tree)
```

No custom watcher or refresh engine: Bazel incrementality is the only
rebuild mechanism, per the [automatic-workflow
preference](../product/scope.md#automatic-workflows). A source or dependency-context change
invalidates every shard declaring that input, plus downstream aggregation and render as needed;
unaffected units can reuse cached outputs. Shared headers or imported types can affect several units.

## Cache And Remote Safety

- Extraction actions declare every input: source files, extractor binary,
  language toolchain, configuration, and required dependency context such as generated headers,
  compiler metadata, classpaths, preprocessors, and include closures. No network access inside actions;
  any required upstream data arrives as declared inputs, following the audit
  snapshot precedent.
- Outputs are deterministic seed-only: sorted keys and symbol order,
  workspace-relative paths only, no timestamps, no absolute paths, no host
  environment in outputs, locale-independent ordering, UTF-8. Determinism is delivered
  seed-only via byte-identical rebuild proof under #781 (successor to closed #581).
- Stable symbol IDs ([IR contract](doc-ir.md)) and normalized ordering are necessary but not
  sufficient for reproducibility. Byte equality is required for the same pinned producer and inputs,
  not across serializer or tool upgrades; cache correctness still requires execution evidence.
- Byte-identical rebuild evidence (two builds, diffed) is delivered seed-only under
  #781 via rebuild-proof fixtures plus `docs_pipeline_qualification` live proof, not an assumed
  property.

## Laziness And Scope

Documentation follows normal target scope with no language enable lists.
Units from unused foundations produce no shards unless selected, preserving
strict unused-foundation laziness. Bare scope selects the repository.

## Freshness

IR shards, render inputs, and rendered HTML are ordinary generated Bazel artifacts,
not committed files or source-adjacent snapshots. Aggregation consumes the extraction
outputs for the declared current inputs, whether newly generated or reused from Bazel's
cache. Bazel input tracking determines when rebuilding is needed; a cache miss causes
normal execution, not a freshness failure. No snapshot refresh/apply step or separate
documentation cache is introduced.

The planned [`dx docs --check`](../cli/commands/docs.md) selects extraction and shared validation but not
rendering; normal build validates and renders. Link/reference completeness at the pre-render boundary is delivered seed-only under #782
(successor to closed #581): prose plus generated API pages resolve all internal links/references with no dangling targets;
remote targets are skipped, never fetched; dangling targets fail the aggregate action with no partial outputs.
Neither mode compares against committed IR. Build and check
may write Bazel outputs and cache entries but never write generated IR beside source
files. The planned `--serve` previews the built output locally and is not a build action.

Delivered (seed-only fixture execution under #779 plus #780) fixtures prove a clean build without checked-in IR,
deterministic sorted outputs, and no source-tree writes; the demo chain
(`//docs/site:demo_site`) emits IR shards plus SUMMARY plus API pages plus search records
plus the rendered entry plus the single search index as Bazel-cached outputs.
Delivered (seed-only rebuild proof under #781) fixtures prove identical inputs rebuild to
byte-identical site outputs: two builds are hashed and diffed, outputs stay sorted with
no timestamps and no absolute paths.
Delivered (seed-only link completeness under #782) fixtures prove prose plus generated API pages
resolve all internal links with no dangling targets: the demo prose links `api.md` plus `#getting-started`
plus a skipped remote URL, every shard ID appears as an API heading, and dangling prose, API, anchor, or
unknown targets fail the aggregate action with no partial outputs.
Delivered (seed-only first-hour timing proof under #784) fixtures prove the built
site/docs journey completes in the first hour: `//docs/site:demo_site` cold-server
plus warm-server plus `//docs/...` plus `//docs/site/...` tests, recorded one-shot
2026-09-21 on Linux x86_64 under Bazel 9.2.0 with warm disk cache (see
`tools/ci/tests/fixtures/docs_site/timing.expected`); this is one-shot evidence
per [ADR 0022](../decisions/0022-no-benchmarking.md), not a standing benchmark, with no CI timing budget enforced.
Delivered (seed-only per-release pin-bump plus drift process under #785) fixtures prove
pinned native inputs bump per release with drift testing (contract suite plus golden
fixtures plus determinism evidence, ship only when green plus reviewed): the codec
roundtrip/parity/ordering/compat gates plus adapter version-mismatch plus same-producer
byte-identical proof hold, ordinary API changes require no IR snapshot update with no
committed shards, and users stay on pinned inputs (see
`tools/ci/tests/fixtures/docs_site/drift.expected`).
Delivered (seed-only guide-step wiring under #783) fixtures prove user guide prose stays
executable: the fixture-scale quickstart guide lists the extract, aggregate, and unit-check
steps, every step is executed by `docs_pipeline_qualification` in CI with no unexecuted
steps allowed, and the prose stays in sync with the executable steps file.
Cache reuse and rebuilds must produce equivalent validated artifacts;
remote-cache claims additionally require the central [testing evidence](../testing/README.md#remote-tests).
Cache reuse timing and appropriate invalidation after source/extractor/configuration changes
stay open; site-level byte-identical rebuild evidence is delivered seed-only under #781. Action-graph planning
proves check mode selects no rendering action, both modes reject the same
invalid IR and references, and a full build still exercises renderer failures. This is not a wrapper
around [`mdbook test`](https://rust-lang.github.io/mdBook/cli/test.html), which tests Rust examples.

## Renderer

The public contract is the [IR](doc-ir.md) plus the site layout, URL model,
and search index, not generated Markdown. mdBook is the decided renderer
and prose processor: guides stay
mdBook-compatible Markdown while generated API pages enter through the same
shell with one theme and navigation. There is no planned replacement; the
IR contract would survive one regardless, since no adapter or IR fact
depends on the renderer. The search index is built directly from prose plus
IR; it never parses rendered HTML.

## Related issues

Tracking lives in the [roadmap](../roadmap.md). Site execution delivered under #779 plus #780 plus #781 plus #782 plus #783 plus #784 plus #785; no execution gaps remain. Reintroduction: #786.
