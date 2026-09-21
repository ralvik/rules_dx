# Documentation Build Check Serve

Accepted workflow (#620). Docs only; no `Supported` claim.

Build, check, and serve use existing tools only. A custom docs
linter is rejected; link and reference checks reuse the existing
[repository-owned Markdown checker](../quality/tool-integrations.md)
plus Vale markers. No rendered mdBook site is claimed here; renderer
and site execution are delivered seed-only under #780 (successor to closed #581; see
[Documentation](README.md#contracts)) and execute under `//docs/site`, not here.

## Build

```sh
bazel build //docs/...
```

Bazel owns the build. It assembles the checked-in Markdown corpus
plus the versioned IR schema and codec. It emits no rendered site
and writes no generated files beside sources.

## Check

```sh
bazel run //cli/cli:dx -- lint --check //docs/...
```

The check is non-mutating. Relative link targets, same-file and
resolved-file anchors, heading hierarchy, and fenced code-block
language tags resolve through the existing checker. Remote URLs are
recorded but never fetched. Vale enforces the checked-in marker
style only; prose rules stay out of scope.

## Serve

```sh
python3 -m http.server --directory docs 8000
```

Serve previews the validated Markdown tree locally with the standard
Python library only. It is not a build action and performs no link
validation.

## CI Gate

The `docs-ci` job in `.github/workflows/ci.yml`
self-calls the reusable docs workflow over `//docs/...`. Pull
requests stay check-only; the validated tree publishes only on `main`.
The wiring is pinned by `bazel run //tools/ci:docs_build_qualification`.

## Related issues

Tracking lives in the [roadmap](../roadmap.md). Build workflow: #620. Pipeline gaps: #779 (adapters delivered) plus #780 (site delivered) plus #781 (rebuild delivered) plus #782 (link completeness delivered) plus #784 (timing delivered) plus #785 (drift delivered) plus #783.
