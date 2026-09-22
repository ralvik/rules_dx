# Svelte Environment

Svelte contributes provider-derived plans to the private environment
collection described by [Developer Environments](environment.md). Shared
identity, installation, selection, and retention follow
[Managed Environment State](managed-state.md). Generation stays owned by the
[framework adapters](../generation/framework-adapters.md).

## Focused Plan

The `svelte_env_plan` rule in `svelte/env/plan.bzl` plans one `svelte_*`
wrapper target from its preserved `JsInfo` transitive sources plus
`QualitySourcesInfo` direct sources, emitting `SvelteEnvPlanInfo` plus a
JSON plan. Binaries and tests share the same closure through
`data`/runfiles. Pinned by `svelte/env/plan_tests.bzl` and exercised by
`svelte/env:hello_lib_plan` over `//svelte/tests/fixtures/hello:hello_lib`.

## Toolchain

The plan runs on the default upstream toolchain; per-platform acquisition
stays open under the [native plan](../native-toolchains.md#qualification-questions-and-delivery).

## Evidence

Accepted. Pinned by `bazel run //tools/ci:foundation_maps`. No `Supported`
claim until platform plus consumer plus release evidence passes per the
[support matrix](../product/support-matrix.md).
