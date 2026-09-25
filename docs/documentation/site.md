# Documentation Site Build

`dx docs` builds the static documentation site from versioned documentation IR
through Bazel-cached extraction, aggregation, and rendering.
Fixture-scale execution is delivered seed-only; there is no committed IR and
no published site. Open work is tracked in GitHub issues.

Run the fixture-scale site with `bazel build //docs/site:demo_site`.
Validate without rendering with `dx docs --check`; normal build validates
then renders. Preview locally with `dx docs --serve`.

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
rebuild mechanism. A source or dependency-context change invalidates every
shard declaring that input, plus downstream aggregation and render as needed.

## Cache And Remote Safety

- Extraction actions declare every input: source files, extractor binary,
  language toolchain, configuration, and required dependency context. No
  network access inside actions; required upstream data arrives as declared
  inputs.
- Outputs are deterministic: sorted keys and symbol order,
  workspace-relative paths only, no timestamps, no absolute paths, no host
  environment in outputs, locale-independent ordering, UTF-8.
- Stable symbol IDs and normalized ordering are necessary but not sufficient
  for reproducibility. Byte equality is required for the same pinned producer
  and inputs, not across serializer or tool upgrades.

## Laziness And Scope

Documentation follows normal target scope with no language enable lists.
Units from unused foundations produce no shards unless selected, preserving
strict unused-foundation laziness. Bare scope selects the repository.

## Freshness

IR shards, render inputs, and rendered HTML are ordinary generated Bazel artifacts,
not committed files or source-adjacent snapshots. Aggregation
consumes the extraction outputs for the declared current inputs, whether
newly generated or reused from Bazel's cache. Bazel input tracking determines
when rebuilding is needed. No snapshot refresh/apply step or separate
documentation cache is introduced.

`dx docs --check` selects extraction and shared validation but not rendering.
Prose plus generated API pages must resolve all internal links and references
with no dangling targets; remote targets are skipped, never fetched;
dangling targets fail the aggregate action with no partial outputs. Neither
mode compares against committed IR. Build and check may write Bazel outputs and
cache entries but never write generated IR beside source files.

Guide prose stays executable: the quickstart guide lists the extract,
aggregate, and unit-check steps, every step is executed in CI with no
unexecuted steps allowed, and the prose stays in sync with the executable
steps file. Cache reuse and rebuilds must produce equivalent validated
artifacts; remote-cache claims additionally require the central testing
evidence. This is not a wrapper around `mdbook test`.

## Renderer

The public contract is the [IR](doc-ir.md) plus the site layout, URL model,
and search index, not generated Markdown. mdBook is the decided renderer and
prose processor: guides stay mdBook-compatible Markdown while generated API
pages enter through the same shell with one theme and navigation.
There is no planned replacement. The search index is built directly from
prose plus IR; it never parses rendered HTML.

## Related issues

Tracking lives in GitHub issues.
