# M22: Additional Native Foundations And Toolchain Parity

## Outcome

Admitted complete native/toolchain language foundations and remaining quality integrations are adapter-tested.

## Scope

Implement the remaining baseline tools whose proven route is a selected toolchain or checksummed
native/self-contained artifact, including C/C++/CUDA, CUE, Go/modules, Jsonnet, Pkl, Proto, QML,
Shell, Terraform, YAML, and other still-open standalone rows. M04-owned Rust, Starlark, TOML, and Vale
adapters receive cross-cohort regression coverage in M24 rather than reimplementation here. Freeze whether Scala/Scalafix uses a native/toolchain
route here or the managed route consumed by M23.

Own the additional native/toolchain foundation cohort under first-release admission; admit/defer/exclude
outcomes live in [ADR 0019](../decisions/0019-first-release-additional-foundations.md).
Go, C/C++, and Scala foundations
are admitted to v1 scope by [ADR 0019](../decisions/0019-first-release-additional-foundations.md). Deliver admitted complete low-cost
upstream foundations after O46 freezes per-language contracts, provider/generation mappings, effort
evidence, and reviewable work packages. Assess Starlark Gazelle reuse here rather than assume it is not applicable.
Swift and SwiftFormat are excluded from v1 and are not in this cohort.

## Contract References

- [Tools](../tools/), [quality](../quality/), [tested platforms](../decisions/0014-tested-platform-release-stack.md), [testing](../testing/), and open decision [O30](../open-decisions.md).
- [V1 admission](../product/scope.md#first-release-admission), [support inventory](../product/support-matrix.md), [generation](../generation/), [environments](../environments/), and [O46](../open-decisions.md).

## Deliverables

- Remaining native/toolchain adapters, acquisition metadata, configs, and generated capability manifests.
- Focused route proofs for Clang, Qt, Buf, and any other provisional toolchain/artifact choice.
- A frozen Scala/Scalafix native/toolchain-versus-managed route selection.
- Upstream-backed foundation wrappers, generation adapters, dependency integration, and target
  environment/IDE plans for each admitted capability; no complete-foundation claim from quality alone.

## Work Packages

1. Freeze each remaining tool's classes, capabilities, native config, route, platforms, and fixtures,
   including Scala/Scalafix route ownership.
2. Review and prove each upstream foundation/toolchain/provider route, including any language
   packaging, source build, or focused patch permitted by the admission policy. Implement admitted
   language capabilities before tools that require their compiler context; prove standalone
   acquisition independently. A foundation deferral does not remove a baseline tool.
3. Implement adapters and rerun foundation, generation, environment, ordering, cache, laziness,
   platform, and external-consumer suites per cohort.

## Milestone-Specific Evidence

- Each matrix row has exact version, acquisition identity, check/fix/format behavior, and claimed-platform evidence.
- Coupled tools follow target-selected semantic identity; standalone tools remain execution-platform lazy.
- Every admitted foundation cell has its own conformance evidence, including test and coverage
  integration where applicable; unresolved upstream or contract gaps block the affected scope.

## Out Of Scope

- Managed Python/Node/JVM/.NET/PowerShell/Ruby cohorts and any Scala route assigned to them, parity
  closure, replacement language stacks, and support promotion.

## Completion Report Additions

- Provide the native/toolchain adapter-test matrix, Scala/Scalafix route decision, version pins,
  per-route effort/review evidence, and performance data. Distinguish recorded additional-foundation
  deferrals and their evidence from required-core blockers and other unresolved required cells;
  account separately for retained quality-tool obligations.
