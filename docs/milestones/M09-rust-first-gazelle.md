# M09: Rust-First Gazelle

## Outcome

A first-party Rust Gazelle extension generates strict, deterministic initial Rust targets for dogfood.

## Scope

Implement the first concrete Gazelle extension directly for Rust, including source-only/Cargo-aware generation for ordinary
libraries, binaries, and tests, including strict dependency resolution, naming, merge, and stale cleanup.

## Contract References

- [Gazelle decision](../decisions/0015-first-party-gazelle-extensions.md), [naming](../decisions/0004-naming.md), [generation](../generation/), [testing](../testing/), and open decision [O22](../open-decisions.md).

## Deliverables

- Rust-local extension mechanics and private Rust extension entry point.
- Golden, semantic, source-only, Cargo-authority, strict-resolution, and merge fixtures.

## Work Packages

1. Implement Rust-local indexing, exact mapping/ignore, naming, desired/empty-rule, and result primitives.
2. Add conventional Rust crate, module, binary, unit-test, and integration-test ownership.
3. Add Cargo target/dependency authority and deterministic stale cleanup for the initial shapes.

## Milestone-Specific Evidence

- Repository generation is idempotent and creates no integration sidecars or operational activation.
- Fixtures prove target-scoped dependency scope, single ownership, test isolation, and fail-closed ambiguity.

## Out Of Scope

- Shared cross-language abstractions, public `dx generate`, native config binding, advanced Cargo
  targets/build scripts, and other languages.

## Completion Report Additions

- Record generated shapes, recognized syntax/layouts, strict-resolution cases, and blocked Cargo mappings.
