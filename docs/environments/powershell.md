# PowerShell Environment

PowerShell contributes provider-derived plans to the private environment collection
described by [Developer Environments](environment.md). Shared identity,
installation, selection, and retention follow
[Managed Environment State](managed-state.md).

## Focused Plan

The `powershell_env_plan` rule in `powershell/env/plan.bzl` plans one
`pwsh_*` wrapper target from its `QualitySourcesInfo` direct sources,
emitting `PowershellEnvPlanInfo` plus a JSON plan. It never scans the
checkout or reconstructs Gallery metadata. Pinned by
`powershell/env/plan_tests.bzl` and exercised by
`powershell/env:hello_lib_plan` over
`//powershell/tests/fixtures/hello:hello_lib`.

## Toolchain And IDE

The plan runs over the portable `pwsh` 7.5.4 toolchain (per-platform
archives via the upstream `powershell.toolchain` extension, lazy by
execution platform); per-platform acquisition stays open under the
[native plan](../native-toolchains.md#qualification-questions-and-delivery).
Dependency bytes come from the Gallery lock; the plan only projects the
selected target closure. No editor driver is proven: no PowerShell language
server, debugger, or qualified test-runner IDE mapping ships with the
ruleset or here. IDE stays an explicitly stated gap, not an assumed
integration.

## Evidence

Provisional. Pinned by `bazel run //tools/ci:foundation_maps`. No `Supported`
claim until platform plus consumer plus release evidence passes per the
[support matrix](../product/support-matrix.md).
