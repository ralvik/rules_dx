# MDX Environment

MDX contributes provider-derived plans to the private environment collection
described by [Developer Environments](environment.md). Shared identity,
installation, selection, and retention follow
[Managed Environment State](managed-state.md). Generation stays owned by the
[framework adapters](../generation/framework-adapters.md).

## Focused Plan

The `mdx_env_plan` rule in `mdx/env/plan.bzl` plans one `mdx_*` wrapper
target from its preserved `JsInfo` transitive sources plus
`QualitySourcesInfo` direct sources, emitting `MdxEnvPlanInfo` plus a JSON
plan. Binaries and tests share the same closure through `data`/runfiles.
Pinned by `mdx/env/plan_tests.bzl` and exercised by
`mdx/env:hello_lib_plan` over `//mdx/tests/fixtures/hello:hello_lib`.

## Toolchain

The plan runs on the default upstream toolchain; per-platform acquisition
stays open under the [native plan](../native-toolchains.md#qualification-questions-and-delivery).

## Evidence

Accepted. Pinned by `bazel run //tools/ci:foundation_maps`. No `Supported`
claim until platform plus consumer plus release evidence passes per the
[support matrix](../product/support-matrix.md).
