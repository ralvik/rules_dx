# M30a Completion Report: Adoption Docs Release-Blocking (Delivery)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed.

Delivered: the `dx docs` command surface (`build`/`check`/`serve` with
shared extraction/validation planning), the IR version/identity/validation
gates, the drift upgrade gate, the guide/example corpus shape with a first
checked-in consumer example, and O54 frozen mappings. Extractor execution,
per-language adapter runs, renderer/site-build execution, and guide-step
CI wiring remain gaps (none claimed).

Capability transitions: docs surface is Dogfooded (local `dx docs`
build/check/serve validation through `dx_docs` + `dx_adopt::plan_docs`);
no `Supported` claim.

## WP1: Guides And Examples Corpus (delivered shape)

Guide identities frozen (`quickstart`, `tutorial`, `migration`); freshness
rule CI-executed-or-stale enforced by planning gates. `examples/consumer-ci`
lands as the first checked-in executable example (caller template + setup
README with required repository settings). Full guide prose with
CI-executed step verification remains future work beyond this slice.

## WP5: Unified Documentation Pipeline And `dx docs` (delivered surface)

`dx docs [--check] [--serve [--port N]]` validates shared
extraction/validation before any render; `--port` without `--serve` is
rejected; check omits render; serve previews last build outputs locally.
IR v1 (`doc_ir_version` major 1, `language:package:qualified_name` IDs,
additive-only minor, migration on major) with 13-adapter input table and
the nightly-rustdoc exception is frozen under O54. Site build is a
deterministic Bazel-cached extract→aggregate→render graph with no committed
IR; drift ships only on green contract+goldens+determinism plus review.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (727 targets).
- `bazel test //...`: 186/186 pass, including `//dx/docs:dx_docs_test`
  (29 passed) and `//dx/cli:dx_cli_test` docs planning cases, plus
  `rustfmt`/`clippy` gates (warnings as errors).
- `bazel run //dx:generate_check`: clean.

Coverage inventory: no new uncovered executable lines beyond the reconciled
gate. No extractor runs, renderer runs, site artifacts, or guide-step CI
runs; none claimed.

## Changed Components

- `dx/docs/src/lib.rs` (IR/mode/drift gates, 29 tests; O54 frozen).
- `dx/cli/src/adopt.rs` (`dx docs` dispatch via `dx_docs` +
  `dx_adopt::plan_docs`), `src/args.rs` (`Docs` command, `serve`/`port`),
  `src/plan.rs` (`adoption` capability).
- `examples/consumer-ci/` (caller template + setup README).
- This report.

## Open Items

- O54 remaining execution evidence: per-language adapter runs,
  link/reference completeness proofs, renderer/site artifacts, cache and
  determinism measurements, guide-step CI wiring.
- O53/O62 untouched by this report.
- Milestone exclusions respected: no release qualification (M28), no
  publication (M29), no post-release adoption scope beyond the `dx docs`
  surface (M30b owns it).
