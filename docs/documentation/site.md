# Documentation Site Build

Implementation status: accepted v1 direction with provisional inputs;
renderer/site execution delivered seed-only under #780 (successor to closed #581,
live successor to closed #421); site-level byte-identical rebuild proof delivered
seed-only under #781; remaining execution open (#779, #782-#785).
Accepted: the `dx_docs` site-build action planning over the
Bazel-cached extract→aggregate→render graph (no committed IR). Delivered: fixture-scale
execution in [`docs/site/`](../../docs/site/site.bzl) (`docs_extract` per unit,
`docs_aggregate` with shared validation, `docs_render` with the pinned mdBook
artifact) producing mdBook-compatible prose plus generated API pages plus one search
index, with generated IR in Bazel outputs only. Command dispatch was
removed per [ADR 0020](../decisions/0020-remove-dx-docs-placeholder.md); reintroduction
is open under #786. mdBook is the decided
renderer with no planned replacement. Cache and hermeticity properties
below are delivered seed-only for the fixture-scale site (rebuild proof under #781);
remaining timing and invalidation verification follows
[Testing](../testing/) before any support statement. Open under #779, #782-#785
(successors to closed #581; see [Documentation](README.md#contracts) for the full list):
adapter runs, link/reference completeness,
cache reuse and invalidation fixtures, guide-step CI wiring, and first-hour timing proof.
Fixture-scale site execution plus byte-identical rebuild proof are qualified seed-only; no published site is claimed.

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
rendering; normal build validates and renders. Completeness of required link/reference checks
at the pre-render boundary remains a gap (#782, successor to closed #581). Neither mode compares against committed IR. Build and check
may write Bazel outputs and cache entries but never write generated IR beside source
files. The planned `--serve` previews the built output locally and is not a build action.

Delivered (seed-only fixture execution under #780) fixtures prove a clean build without checked-in IR,
deterministic sorted outputs, and no source-tree writes; the demo chain
(`//docs/site:demo_site`) emits IR shards plus SUMMARY plus API pages plus search records
plus the rendered entry plus the single search index as Bazel-cached outputs.
Delivered (seed-only rebuild proof under #781) fixtures prove identical inputs rebuild to
byte-identical site outputs: two builds are hashed and diffed, outputs stay sorted with
no timestamps and no absolute paths.
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

Tracking lives in the [roadmap](../roadmap.md). Site execution delivered under #780; rebuild proof delivered under #781; remaining: #779, #782-#785. Reintroduction: #786.
