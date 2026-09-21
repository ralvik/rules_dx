# ADR 0027: Depcheck Checker Rust Delivery

## Status

Accepted.

## Context

[ADR 0026](./0026-rust-product-code.md) scopes the Rust product-code migration
with a phase index: archiver/hasher delivered Rust, plus provisional preset,
depcheck, SBOM/BCR, deploy launchers, and the deferred CI-driver stance.
Each phase lands only with an accepted successor plus its guard update.

`tools/depcheck/` ran the offline lockfile-consistency plus usage checker as
Python (`depcheck.py` plus split modules) under 24 Linux-only `sh_test`
wrappers. Interpreter startup plus bash on the hot path contributes nothing
to platform evidence. The checker CLI (args, exit codes, offline contract,
category/exception/obsolete semantics) plus the `testdata/**` goldens are the
stable contract owned by `docs/quality/quality-testing.md`.

## Decision

Depcheck checker is Rust: `tools/depcheck` owns `dx_depcheck` (`src/lib.rs`
plus thin `src/main.rs`), `rust_binary :depcheck`, portable
`rust_test :depcheck_test` over the existing `testdata/**` goldens, plus
`rustfmt`/`clippy` gates. The Python sources plus `sh_test` harness are
removed. CLI args, exit codes (0 ok, 1 findings, 2 actionable errors),
offline/no-network behavior, and category/exception/obsolete diagnostics stay
stable for CI callers. Testdata fixtures stay (exempt testing).

## Consequences

- `//tools/ci:product_runtime_guards` pins the Rust delivery (no `.py`,
  `rust_binary` present, no `sh_test`).
- `//tools/ci:depcheck_contract` pins the Rust checker plus portable
  `rust_test` (no Linux-only pins).
- Qualification callers (`cc_hermetic`, `godeps`, `paket`, layer-2, audit,
  evidence gate) invoke `bazel run //tools/depcheck:depcheck` and grep the
  Rust sources, never `depcheck.py`.

## Rejected Alternatives

- Keep `py_binary` plus `sh_test`: preserves hot-path interpreter cost and
  Linux-only pins where Rust is cross-platform.
- Per-ecosystem split crates: allowed within this phase but unnecessary; a
  single core with per-ecosystem parsers keeps the CLI stable.
