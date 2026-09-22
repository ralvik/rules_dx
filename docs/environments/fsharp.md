# F# Environment

F# contributes provider-derived plans to the private environment collection
described by [Developer Environments](environment.md). Shared identity,
installation, selection, and retention follow
[Managed Environment State](managed-state.md).

## Focused Plan

The `fsharp_env_plan` rule in `fsharp/env/plan.bzl` plans one `fsharp_*`
wrapper target from its `QualitySourcesInfo` direct sources plus the
`DotnetAssembly*` transitive closure and source/test split, emitting
`FSharpEnvPlanInfo` plus a JSON plan. It never scans the checkout or
reconstructs Paket metadata. Pinned by `fsharp/env/plan_tests.bzl` and
exercised by `fsharp/env:hello_lib_plan`, `fsharp/env:hello_plan`, and
`fsharp/env:hello_test_plan` over the hello library, binary, and test.

## Toolchain

The plan runs on the default upstream toolchain; per-platform acquisition
stays open under the [native plan](../native-toolchains.md#qualification-questions-and-delivery).
Dependency bytes come from the Paket lock; the plan only projects the
selected target closure.

## Evidence

Accepted. Pinned by `bazel run //tools/ci:foundation_maps`. No `Supported`
claim until platform plus consumer plus release evidence passes per the
[support matrix](../product/support-matrix.md).
