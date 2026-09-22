# Ruby Environment

Ruby contributes provider-derived plans to the private environment collection
described by [Developer Environments](environment.md). Shared identity,
installation, selection, and retention follow
[Managed Environment State](managed-state.md).

## Focused Plan

The `ruby_env_plan` rule in `ruby/env/plan.bzl` plans one `ruby_*` wrapper
target from its `QualitySourcesInfo` direct sources, emitting
`RubyEnvPlanInfo` plus a JSON plan. It never scans the checkout or
reconstructs Bundler metadata. Pinned by `ruby/env/plan_tests.bzl` and
exercised by `ruby/env:hello_lib_plan` over
`//ruby/tests/fixtures/hello:hello_lib`.

## Toolchain And IDE

The plan runs on the default upstream toolchain; per-platform acquisition
stays open under the [native plan](../native-toolchains.md#qualification-questions-and-delivery).
Portable Ruby covers Linux/macOS x86_64/arm64; Windows falls back to
RubyInstaller per ADR 0032 with qualification pending. Dependency bytes
come from the shared Bundler lock; the plan only projects the selected
target closure.

## Evidence

Accepted. Pinned by `bazel run //tools/ci:foundation_maps`. No `Supported`
claim until platform plus consumer plus release evidence passes per the
[support matrix](../product/support-matrix.md).
