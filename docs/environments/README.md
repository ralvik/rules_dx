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

The delivery order is dogfood-first: PATH/environment bootstrap precedes the Rust language
foundation, and Python follows as a later language projection.

## Language Mapping Qualification

Accepted. Each foundation keeps its provisional upstream; no switch is approved here.

Every implemented foundation (`rust`, `python`, `javascript`, `typescript`, `go`,
`java`, `kotlin`, `scala`, `csharp`, `fsharp`, `cc`) contributes a provider-derived
plan via `<lang>/env/plan.bzl`, pinned by `<lang>/env/plan_tests.bzl` and exercised
by `<lang>/tests/fixtures/hello/`. Rust, Node, and Python projections are further defined in
[Rust Environment](rust.md), [Node Environment](node.md), and
[Python Environment](python-environment.md). Additional toolchains use the default
upstream toolchain now; per-platform acquisition stays open under the native plan.
Ruby and PowerShell have no environment mapping: deferred beyond v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md).
Deferred/excluded env record is decided by [ADR 0019](../decisions/0019-first-release-additional-foundations.md);
no `ruby/`, `powershell/`, or
`swift/` env plan lands here.

Required-core exact-target discovery stays owned under issue #475 per the
[native plan](../native-toolchains.md#qualification-questions-and-delivery);
current plans are provider-derived focused-target plans, not exact-target proof.

Admitted additional-foundation env mappings stay owned under issues #476-#484; current plans
are provider-derived focused-target plans on default upstream toolchains, not
per-platform acquisition proof.

Pinned by `bazel run //tools/ci:foundation_maps`.
