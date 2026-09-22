# First-Hour Timing

One-shot evidence, not a standing benchmark, per [ADR 0022](../decisions/0022-no-benchmarking.md).
No CI timing budget is enforced and no baseline or comparison machinery exists.

## Clone-To-Green Path

Fresh clone to green follows [Bootstrap](local-workflows.md#bootstrap):
pinned Bazelisk install, `bazel build //...`, `bazel test //...`,
then `bazel run //cli/cli:dx -- update --check`. The editor plus
direnv plus hooks one-shot runs after the green build.

## Measured Record

The timed representative journey is the built docs/site flow,
recorded one-shot 2026-09-21 on Linux x86_64 under Bazel 9.2.0 with
warm disk cache (cold-server means after `bazel shutdown` with disk
cache warm, never `bazel clean`):

- `//docs/site:demo_site` cold-server 1955 ms wall, warm-server 161 ms wall.
- Docs corpus cold-server 4688 ms wall, warm-server 3250 ms wall.
- Site tests warm-server 264 ms wall.

Source: `tools/ci/tests/fixtures/docs_site/timing.expected`; see the
[site contract](../documentation/site.md) for qualification scope.
Seed-only; no Supported claim.
