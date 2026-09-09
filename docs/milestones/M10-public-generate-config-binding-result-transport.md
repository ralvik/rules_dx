# M10: Public Generate, Config Binding, And Result Transport

## Outcome

`dx generate` exposes canonical Rust generation with a repository-wide default, explicit scoped
refresh, exact change transport, and typed native-config binding.

## Scope

Add `//dx:generate`, check/default/diff/NDJSON behavior, a versioned Gazelle result manifest, ignored-import
notices, selected-tool native-config target and `aspect_hints` generation using the shared extension,
and the sequential `dx check` / `dx fix` umbrellas over `format`, `lint`, `typecheck`, and `generate`.
Scoped generation and umbrella mechanics remain gated by O48 and O59 respectively.

## Contract References

- [Generation](../generation/), [CLI](../cli/), [quality configuration](../quality/), [testing](../testing/), open decision [O13](../open-decisions.md), and [ADR 0018](../decisions/0018-umbrella-check-fix-cleanup-clean.md).
- [Scoped generate contract](../cli/commands/generate.md#invocation-and-scope) and
  [umbrella contract](../cli/commands/check-fix-clean.md#dx-check-and-dx-fix):
  [O48 and O59](../open-decisions.md) must freeze affected mechanics before implementation.

## Deliverables

- Canonical public generation target and repository-wide/scoped CLI plans.
- Exact private change/outcome transport and typed local config binding for implemented adapters.
- Sequential `check` / `fix` umbrellas under the qualified command contract.

## Work Packages

1. Qualify O48 scoped selection and freshness, then produce versioned exact edit, write-outcome,
   completion, and ignored-import records in one Gazelle run with the qualified scope.
2. Project records through CLI check, default, diff, text, and NDJSON modes.
3. Generate selected native-config targets, closures, visibility, ownership, and direct hints.
4. Qualify O59 output/report composition, then wire the sequential `check` / `fix` umbrellas
   with stop-on-first-failure and phase exit-code preservation.

## Milestone-Specific Evidence

- Direct Gazelle and `dx generate` have identical edits, partial-failure state, diagnostics, and exit status.
- Config add/move/remove, package-boundary, closure, cycle, selection-gating, and narrow-invalidation fixtures pass.
- `check` / `fix` phase order, failure short-circuit, scope mapping, and direct-command parity fixtures pass.
- Scoped selection, freshness boundaries, manifest scope, and Gazelle merge fixtures satisfy the
  qualified generate contract; umbrella output/report evidence satisfies the O59-qualified contract.

## Out Of Scope

- Non-Rust application generation, codegen execution, environment preparation, and Rust BUILD parsing in the CLI.

## Completion Report Additions

- Record manifest schema, canonical target, config kinds/filenames, merge behavior, and partial-write evidence.
- Record O48/O59 qualification and scoped-generation and umbrella-composition evidence.
