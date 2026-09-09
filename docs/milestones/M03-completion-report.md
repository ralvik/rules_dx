# M03 Completion Report: Quality Source, Policy, Result, Runner, And Evaluator Core

M03 delivers one target-scoped quality core: `QualitySourcesInfo` sources,
typed workspace policy, exact-input capability pipeline actions with
normalized `QualityResult` protobufs in `dx_results`, a deterministic Rust
runner over synthetic adapters, and direct-Bazel per-result threshold
evaluators with synthetic end-to-end evidence. All work runs on the M00
seed host (`sdu-144133.pc.sdu.dk`, Ubuntu 26.04.1, Bazelisk v1.29.0 ->
Bazel 9.2.0).

## Verdicts

- `bazel build //...`: success, 58 actions.
- `bazel test //...`: 27/27 pass (24 prior + `quality_evaluator_test`,
  `quality_evaluator_fmt_test`, `quality_evaluator_clippy_test`).
- `bazel coverage //...` + coverage gate: PASS 2265/2265 executable lines
  (+153 new evaluator lib lines, all covered), exit 0. Starlark
  subjects/tests contribute no `DA` records.
- `//quality/testdata:aspect_presence`: PASSED (default `validate=false`
  pins marker absence: `dx_results` holds only `<name>-<capability>.pb`).
- Direct-Bazel evaluator e2e (recorded invocations, same manifest set the
  `dx` CLI maps its `--fail-on` onto):
  - `validate=true` default `warning`: `fixture_mixed` lint+format
    evaluators fail (exit 1); all clean subjects build with `.validated`
    markers materialized.
  - `fail_on=error` on dirty: both evaluators still fail (lint via
    `lint-b` ERROR, format via replacement-only reason
    `proposes 1 replacement file(s), independently of severity`).
  - `fail_on=info` on all clean subjects: success. Binary matrix on real
    `.pb` files: lint-dirty fails at info/warning/error, format-dirty
    fails at error with zero diagnostics, clean passes at info, malformed
    input fails at error before any comparison.
  - `validate=false` (default) on dirty: success, collection never stops.
  - `--keep_going` over all six subjects: exactly the two
    `fixture_mixed` evaluators fail; all other markers materialize.
  - `aquery`: 4 `DxQuality*` actions (2 analyzers + 2 evaluators); each
    `.pb` appears in exactly one action's `Inputs` (no aggregate);
    analyzer lint block byte-identical across `fail_on=warning/error`
    while the evaluator `ActionKey` differs (threshold-only change
    invalidates evaluators, not analyzers).
  - `cquery --output_groups=dx_results --output=files`: `validate=true`
    yields `.pb` + `.validated` per pipeline; default yields `.pb` only.

## Work Packages

- WP1 (freeze boundaries): exact `QualitySourcesInfo` construction shape,
  47-ID registry, family/aggregate policy providers with fail-fast
  validation, pure applicability formula, frozen `result.proto`, and
  canonical `config` settings (`workspace`, `validate`, `fail_on`).
- WP2a (synthetic adapters/pipeline): `SYNTHETIC_ADAPTERS`
  (`fmt-a`/`lint-a`/`lint-b`), class-to-family map, and pure
  `authorize_classes`/`stage_sources`/`pipeline_stages`/`resolve_pipeline`
  helpers pinned by `:pipeline_unit`.
- WP2b (runner): `//rust/quality_runner` executes one ordered pipeline
  over exact bytes, iterates convergence (max ten rounds,
  repeated-state oscillation), derives full-file edits by final-vs-original
  comparison, normalizes diagnostics, and stores only
  `encode_validated` bytes. Launch/config/protocol failure exits nonzero
  with no output (action failure, never a stored diagnostic).
- WP2c (aspects/fixtures/presence): `lint/format/typecheck/audit_aspect`
  register exact-subset pipeline actions (`DxQuality<Cap>`) from
  `QualitySourcesInfo` only; `quality_source_target` fixtures
  (clean/dirty Python+Rust); `aspect_subject` merges `dx_results`;
  `aspect_presence` pins capability subsets, `no-lint` opt-out, and plain
  non-provider targets producing no action.
- WP3 (evaluators/e2e): `//rust/quality_evaluator` (`Threshold`,
  `parse_threshold`, `evaluate`) plus thin `quality_evaluator` binary;
  aspects register one `DxQualityEval` action per result iff
  `//config:validate`, with `--fail_on` from `//config:fail_on`;
  markers join `dx_results`; docs (`quality-result-protocol.md`, O18)
  record the landing with collectors still pending the `dx` CLI.

## Schema Field Allocation (Frozen, O18)

Normative: `quality/result.proto`. `Severity` (1 INFO, 2 WARNING,
3 ERROR), `Capability` (1 LINT, 2 TYPECHECK, 3 FORMAT, 4 AUDIT),
`Convergence` (1 STABLE, 2 OSCILLATION, 3 ITERATION_LIMIT); zero is
invalid in all three; each reserves 4 to 15. `Diagnostic` 1-8 (severity,
message, tool_id, rule_id, path, start/end_byte, fixable; 9-15
reserved). `FileSnapshot` 1-2 (3-7 reserved). `Edit` 1-3 (4-7 reserved).
`FileEdits` 1-3 (4-7 reserved). `Stage` 1-3 (4-7 reserved).
`QualityResult` 1-12 (schema_major/minor, producer, capability, stages,
completed_rounds, convergence, original/terminal_snapshot,
initial/terminal_diagnostics, replacements; 13-24 reserved).
`SCHEMA_MAJOR=1`, `SCHEMA_MINOR=0`, `MAX_COMPLETED_ROUNDS=10`,
32-byte BLAKE3 digests, no algorithm negotiation.

## Public Load Labels

- `//quality:sources.bzl`: `QualitySourcesInfo`, `check_direct_sources`,
  class IDs, `RUST`.
- `//quality:policy.bzl`: `quality_family`, `workspace_policy`,
  `FamilyPolicyInfo`, `QualityPolicyInfo`; `//quality:fixture_policy`.
- `//quality:applicability.bzl`, `//quality:adapters.bzl`
  (`SYNTHETIC_ADAPTERS`, `SYNTHETIC_CLASS_TO_FAMILY`,
  `adapter_supported_classes`), `//quality:pipeline.bzl`
  (`resolve_pipeline` and helpers).
- `//quality:aspects.bzl`: `lint_aspect`, `format_aspect`,
  `typecheck_aspect`, `audit_aspect`; stable group `dx_results`.
- `//quality:fixtures.bzl`: `quality_source_target`;
  `//quality:aspect_subject.bzl`: `aspect_subject`.
- `//quality:result_proto_rs` (generated `result_proto`);
  `//rust/quality_result` (`validate`, `encode_validated`,
  `decode_validated`, `digest`); `//rust/quality_runner` lib +
  `:quality_evaluator`-shaped `:quality_runner` binary;
  `//rust/quality_evaluator` lib + `:quality_evaluator` binary
  (`--result`, `--fail_on`, `--output`).
- `@rules_dx//config:workspace` (`//dx:config` default, still
  placeholder-must-fail-fast), `@rules_dx//config:validate`,
  `@rules_dx//config:fail_on=info|warning|error` (default `warning`).

## Evaluator Semantics (Frozen For This Core)

Fail iff any holds: (a) an initial or terminal diagnostic severity meets
the threshold (`info <= warning <= error`, no `never`); (b) the result
proposes any replacement, independently of severity; (c) convergence is
not `STABLE` (`OSCILLATION`/`ITERATION_LIMIT` carry no replacements and
fail evaluation). `fixable` never weakens evaluation (no fix is applied,
so every original finding counts, matching CLI check mode). The binary
decodes with `decode_validated` before parsing the threshold, so
malformed results fail at every threshold. Passing writes a deterministic
marker `validated <producer> <capability> fail_on=<t>`; failing exits
nonzero with reasons and writes nothing. One evaluator consumes exactly
one `.pb`; no aggregation exists; analyzer keys never mention policy.

## Unresolved Granularity Measurements (O19)

Confirmed open: the action-granularity/stage-order benchmark corpus,
repetitions, noise threshold, tie-breaks, and batching threshold are
still unapproved. WP2 uses lexical tool-ID order as the stable ruleset
order (flagged for review); permutation-benchmark selection
(correctness/convergence first, then rounds, process starts, wall time)
awaits real multi-tool adapters in M04+.

## Changed Components

- New: `rust/quality_evaluator/` (`Cargo.toml`, `src/lib.rs`,
  `src/main.rs`, `BUILD.bazel` with lib/binary/test/fmt/clippy targets),
  `docs/milestones/M03-completion-report.md`.
- Wired: `quality/aspects.bzl` (`_evaluator`, `_validate`, `_fail_on`
  attrs; `DxQualityEval` action; markers in `dx_results`).
- Registered: `MODULE.bazel` manifest + regenerated `MODULE.bazel.lock`
  and `rust/hello/Cargo.lock` (new `quality_evaluator 0.0.0` entry only;
  anyhow `1.0.104`/blake3 `1.8.7` pins unchanged).
- Covered: `tools/coverage/inventory.txt` (+2 eligible; `main.rs`
  excluded by validated thin-shim markers).
- Documented: `docs/quality/quality-result-protocol.md` (evaluator
  identity, fixture locations), `docs/open-decisions.md` O18 (evaluators
  land; collectors pending `dx` CLI).
- Unchanged contracts: action granularity, configuration composition,
  CLI `--fail-on` mapping, and `validate=false` collection behavior.

## Gaps (Accepted)

- Local Linux only; other ADR 0014 platforms remain gaps per M00.
- No remote cache/execution evidence; collectors/BEP/`dx` CLI own M06.
- Real adapters, curated policy taxonomy, and class-to-family assignment
  stay pending M04+; synthetic `fmt-a`/`lint-a`/`lint-b` prove the
  mechanism only.
