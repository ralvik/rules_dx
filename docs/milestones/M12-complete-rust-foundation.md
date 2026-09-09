# M12: Complete Rust Foundation

## Outcome

Rust has a complete adapter-tested v1 application, generation, test, coverage, compiler-diagnostic,
IDE, and provider-derived environment-plan foundation for language targets.

## Scope

Complete pinned `rules_rs`/patched `rules_rust` integration, Cargo targets, proc macros, crate types,
examples, benchmarks, build scripts, version selection, coverage, compiler diagnostics/typecheck,
rust-analyzer behavior, and Rust-native environment plan/projection contributions for focused targets.
Public repository/root/exact-target env planning, collection, and orchestration remain in M25.

## Contract References

- [Rust foundation decision](../decisions/0013-rust-javascript-typescript-foundations.md), [toolchain versions](../decisions/0012-language-toolchain-versions.md), [generation](../generation/), [environments](../environments/), [quality](../quality/), [testing](../testing/), and open decision [O24](../open-decisions.md).

## Deliverables

- Complete Rust wrappers, providers, Gazelle semantics, tests/coverage, compiler-diagnostic/typecheck
  ownership, IDE integration, and language-target environment plan adapter.
- Public-upstream conformance gates for every accepted Cargo target and build-script shape.

## Work Packages

1. Complete wrapper/provider and default/per-target version behavior.
2. Add all v1 Cargo target kinds, entries, examples, benchmarks, and fail-closed unsupported cases.
3. Add build-script policy, compiler diagnostics/typecheck, IDE discovery/flycheck, coverage, and
   provider-derived target environment projection.
4. Run full Rust platform, consumer, laziness, generation, and quality interaction suites.

## Milestone-Specific Evidence

- Cargo-lock-only and source-only fixtures preserve native crate ownership and authoritative metadata.
- Build scripts prove the [generation defaults and kept opt-out](../generation/rust.md#build-scripts),
  declared inputs, warning behavior, and remote execution. Close the source-derived opt-out and
  native-integration gaps in the [qualification plan](../native-toolchains.md).
- Rust compiler-diagnostic fixtures prove target ownership, normalized typecheck findings, and failure status.
- Rust IDE and focused environment fixtures use authoritative providers without repository/root/exact-target
  CLI orchestration or mutation of environment/codegen selection.

## Out Of Scope

- Python, JavaScript/TypeScript, frameworks, and promotion from adapter-tested to supported.

## Completion Report Additions

- Record the complete Cargo matrix, version behavior, build-script mappings, compiler-diagnostic
  coverage, IDE shape, focused environment-plan contents, and unsupported semantics.
