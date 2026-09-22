# Environment Foundation Qualification

Provider-derived env-plan mapping and per-foundation evidence.
The [Environments index](README.md) stays short; this document owns the qualification record.

## Language Mapping Qualification

Accepted. Each foundation keeps its provisional upstream; no switch is approved here.

Every implemented foundation (`rust`, `python`, `javascript`, `typescript`, `go`, `java`, `kotlin`, `scala`, `csharp`, `fsharp`, `cc`, `ruby`, `powershell`) contributes a provider-derived plan via `<lang>/env/plan.bzl`, pinned by `<lang>/env/plan_tests.bzl` and exercised by `<lang>/tests/fixtures/hello/`. Rust, Node, and Python projections are further defined in [Rust Environment](rust.md), [Node Environment](node.md), and [Python Environment](python-environment.md). Go, Java, Kotlin, Scala, C#, F#, C/C++, Ruby, and PowerShell plans are owned by [Go](go.md), [Java](java.md), [Kotlin](kotlin.md), [Scala](scala.md), [C#](csharp.md), [F#](fsharp.md), [C/C++](cc.md), [Ruby](ruby.md), and [PowerShell](powershell.md); Vue, Svelte, Astro, and MDX plans are owned by [Vue](vue.md), [Svelte](svelte.md), [Astro](astro.md), and [MDX](mdx.md). Additional toolchains use the default upstream now; per-platform acquisition stays open under the native plan. Ruby mapping is delivered in the Ruby track and PowerShell mapping is delivered provisionally under issue #972. Deferred/excluded env record is decided by [ADR 0032](../decisions/0032-ruby-powershell-bandit-swift.md); no `swift/` env plan lands here yet.

Required-core Rust integration is pinned (issue #470): provider-derived focused-target `rust_env_plan` in `rust/env/plan.bzl` (`RustEnvPlanInfo` from `CrateInfo`/`TestCrateInfo` plus `QualitySourcesInfo`), pinned by `rust/env/plan_tests.bzl` over the four `rust/env` hello plans in `rust/tests/fixtures/hello/`.

Required-core exact-target discovery is qualified seed-only under issue #475 per the [native plan](../native-toolchains.md#qualification-questions-and-delivery) (`bazel run //tools/ci:exact_target_qualification`; resolver-owned exact labels to upstream `TARGETS`, hello exact-isolation pair plus `rust/tests/fixtures/discovery/pins.bzl`); current plans are provider-derived focused-target plans, not exact-target proof beyond that contract.

Admitted additional-foundation env mappings stay owned under issues #476-#484; current plans are provider-derived focused-target plans on default upstream toolchains, not per-platform acquisition proof.

Pinned by `bazel run //tools/ci:foundation_maps`.
