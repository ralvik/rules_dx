# Documentation Site Build

Provisional implementation status: accepted v1 direction; rule labels and evidence requirements freeze under
[O54](../open-decisions.md). mdBook is the decided renderer with no planned
replacement. Cache and hermeticity properties
below are design requirements, not verified claims; verification follows
[Testing](../testing/) before any support statement.

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
- Outputs are deterministic by construction: sorted keys and symbol order,
  workspace-relative paths only, no timestamps, no absolute paths, no host
  environment in outputs, locale-independent ordering, UTF-8.
- Stable symbol IDs ([IR contract](doc-ir.md)) and normalized ordering are necessary but not
  sufficient for reproducibility. Byte equality is required for the same pinned producer and inputs,
  not across serializer or tool upgrades; cache correctness still requires execution evidence.
- Byte-identical rebuild evidence (two builds, diffed) is a required
  milestone gate, not an assumed property.

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

[`dx docs --check`](../cli/commands/docs.md) selects extraction and shared validation but not
rendering; normal build validates and renders. O54 must prove that required link/reference checks
are complete at the pre-render boundary. Neither mode compares against committed IR. Build and check
may write Bazel outputs and cache entries but never write generated IR beside source
files. `--serve` previews the built output locally and is not a build action.

Required fixtures prove a clean build without checked-in IR, cache reuse for unchanged
inputs, appropriate invalidation after source/extractor/configuration changes, and no
source-tree writes. Cache reuse and rebuilds must produce equivalent validated artifacts;
remote-cache claims additionally require the central [testing evidence](../testing/README.md#remote-tests).
Action-graph fixtures must prove check mode selects no rendering action, both modes reject the same
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
