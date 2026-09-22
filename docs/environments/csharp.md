# C# Environment

C# contributes provider-derived plans to the private environment collection
described by [Developer Environments](environment.md). Shared identity,
installation, selection, and retention follow
[Managed Environment State](managed-state.md).

## Focused Plan

The `csharp_env_plan` rule in `csharp/env/plan.bzl` plans one `csharp_*`
wrapper target from its `QualitySourcesInfo` direct sources, emitting
`CSharpEnvPlanInfo` plus a JSON plan. It never scans the checkout or
reconstructs Paket metadata. Pinned by `csharp/env/plan_tests.bzl` and
exercised by `csharp/env:hello_lib_plan` over
`//csharp/tests/fixtures/hello:hello_lib`.

## Toolchain

The plan runs on the default upstream toolchain; per-platform acquisition
stays open under the [native plan](../native-toolchains.md#qualification-questions-and-delivery).
Dependency bytes come from the Paket lock; the plan only projects the
selected target closure.

## Evidence

Accepted. Pinned by `bazel run //tools/ci:foundation_maps`. No `Supported`
claim until platform plus consumer plus release evidence passes per the
[support matrix](../product/support-matrix.md).
