# C/C++ Environment

C/C++ contributes provider-derived plans to the private environment
collection described by [Developer Environments](environment.md). Shared
identity, installation, selection, and retention follow
[Managed Environment State](managed-state.md).

## Focused Plan

The `cc_env_plan` rule in `cc/env/plan.bzl` plans one `cc_*` wrapper target
from its `QualitySourcesInfo` direct sources plus the `CcInfo` transitive
header closure and source/test split, emitting `CcEnvPlanInfo` plus
a JSON plan. It never scans the checkout or reconstructs toolchain metadata.
Pinned by `cc/env/plan_tests.bzl` and exercised by `cc/env:hello_lib_plan`,
`cc/env:hello_plan`, and `cc/env:hello_test_plan` over the hello library,
binary, and test.

## Toolchain

The plan runs on the default upstream toolchain; per-platform acquisition
stays open under the [native plan](../native-toolchains.md#qualification-questions-and-delivery).
There is no lockfile to project; hermeticity comes from per-archive
`sha256`/`integrity`.

## Evidence

Accepted. Pinned by `bazel run //tools/ci:foundation_maps`. No `Supported`
claim until platform plus consumer plus release evidence passes per the
[support matrix](../product/support-matrix.md).
