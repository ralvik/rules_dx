# Environments

Developer environment and generated-source projection contracts:

- [Developer Environments](environment.md): PATH bootstrap, language-neutral selection, and managed
  tools.
- [Managed Environment State](managed-state.md): immutable generations, setup selection, reuse,
  concurrency, ownership, and retention.
- [Generated Code](codegen.md): generated-source execution and editor projection.
- [Rust Environment](rust.md): Rust toolchain, source, and rust-analyzer projection.
- [Node Environment](node.md): pnpm importer and managed `node_modules` projection.
- [Python Environment](python-environment.md): later concrete `.venv` projection.
- [Go Environment](go.md): focused `go_env_plan` over wrapper direct sources.
- [Java Environment](java.md): focused `java_env_plan` over wrapper direct sources.
- [Kotlin Environment](kotlin.md): focused `kotlin_env_plan` over wrapper direct sources.
- [Scala Environment](scala.md): focused `scala_env_plan` over wrapper direct sources.
- [C# Environment](csharp.md): focused `csharp_env_plan` over wrapper direct sources.
- [F# Environment](fsharp.md): focused `fsharp_env_plan` over wrapper direct sources.
- [C/C++ Environment](cc.md): focused `cc_env_plan` over wrapper direct sources.
- [Vue Environment](vue.md): focused `vue_env_plan` over `JsInfo` plus direct sources.
- [Svelte Environment](svelte.md): focused `svelte_env_plan` over `JsInfo` plus direct sources.
- [Astro Environment](astro.md): focused `astro_env_plan` over `JsInfo` plus direct sources.
- [MDX Environment](mdx.md): focused `mdx_env_plan` over `JsInfo` plus direct sources.

The delivery order is dogfood-first: PATH/environment bootstrap precedes the Rust language
foundation, and Python follows as a later language projection.

## Language Mapping Qualification

Accepted. Each foundation keeps its provisional upstream; no switch is approved here.

Every implemented foundation (`rust`, `python`, `javascript`, `typescript`, `go`,
`java`, `kotlin`, `scala`, `csharp`, `fsharp`, `cc`) contributes a provider-derived
plan via `<lang>/env/plan.bzl`, pinned by `<lang>/env/plan_tests.bzl` and exercised
by `<lang>/tests/fixtures/hello/`. Rust, Node, and Python projections are further defined in
[Rust Environment](rust.md), [Node Environment](node.md), and
[Python Environment](python-environment.md). Go, Java, Kotlin, Scala, C#, F#,
and C/C++ plans are owned by [Go](go.md), [Java](java.md),
[Kotlin](kotlin.md), [Scala](scala.md), [C#](csharp.md), [F#](fsharp.md), and
[C/C++](cc.md); Vue, Svelte, Astro, and MDX plans are owned by [Vue](vue.md),
[Svelte](svelte.md), [Astro](astro.md), and [MDX](mdx.md). Additional toolchains use the default
upstream toolchain now; per-platform acquisition stays open under the native plan.
Ruby and PowerShell have no environment mapping yet: admitted to v1 by
[ADR 0032](../decisions/0032-ruby-powershell-bandit-swift.md) (superseding the
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) deferral),
with mappings pending in parallel Ruby/PowerShell tracks.
Deferred/excluded env record is decided by [ADR 0032](../decisions/0032-ruby-powershell-bandit-swift.md);
no `ruby/`, `powershell/`, or
`swift/` env plan lands here yet.

Required-core Rust integration is pinned (issue #470): the provider-derived focused-target
`rust_env_plan` in `rust/env/plan.bzl` (`RustEnvPlanInfo` from authoritative
`CrateInfo`/`TestCrateInfo` plus `QualitySourcesInfo`), pinned by
`rust/env/plan_tests.bzl` over the four `rust/env` hello plans exercised by
`rust/tests/fixtures/hello/`.

Required-core exact-target discovery is qualified seed-only under issue #475 per the
[native plan](../native-toolchains.md#qualification-questions-and-delivery)
(`bazel run //tools/ci:exact_target_qualification`; resolver-owned exact labels to
upstream `TARGETS`, hello exact-isolation pair plus `rust/tests/fixtures/discovery/pins.bzl`);
current plans are provider-derived focused-target plans, not exact-target proof beyond that contract.

Admitted additional-foundation env mappings stay owned under issues #476-#484; current plans
are provider-derived focused-target plans on default upstream toolchains, not
per-platform acquisition proof.

Pinned by `bazel run //tools/ci:foundation_maps`.
