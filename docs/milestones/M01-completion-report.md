# M01 Completion Report: Starlark Testing And Tested-Stack Metadata

M01 delivers the project-owned `starlark_test` facade, six passing
conformance tests plus four manual negative demonstrations, the
`tested_stack` manifest with contract tests, and the evidence-backed
Starlark behavioral fallback. All work runs on the M00 seed host
(`sdu-144133.pc.sdu.dk`, Ubuntu 26.04.1, Bazelisk v1.29.0 -> Bazel 9.2.0).

## Verdicts

- `bazel build //...`: success, 11 actions.
- `bazel test //...`: 8/8 pass (2 Rust/M00, 6 Starlark/M01).
- `bazel coverage //... --combined_report=lcov` + coverage gate: PASS
  1118/1118 executable lines, exit 0. Starlark targets contribute no `DA`
  records (no instrumentation exists); the gate denominator is unchanged.
- Negative demonstrations: 3/3 `FAILED` when named explicitly; wildcard
  expansion finds 0 test targets (manual exclusion holds, `//...` stays
  green). Mode violation fails analysis with the documented authoring
  error.

## Work Packages

- WP1 (phase-correct execution): `arithmetic_load` (load), `arithmetic_unit`
  (unit), `subject_analysis` (analysis over `example_subject` with
  `DxSubjectInfo` + output file), `fixture_execution` (execution over
  runfiles fixtures). Load-phase proof: `expect_equal` values evaluate while
  test `.bzl` files load (`LOADED_SUM` top-level binding).
- WP2 (pure unit tests): `arithmetic_unit`, five assertions over three pure
  functions, no I/O.
- WP3 (coverage route): genuine line instrumentation is infeasible; fallback
  taken with evidence below. `matrix_validation` machine-checks all 10
  `matrix-item:` anchors in `tools/starlark/behavioral_matrix.md` against
  their proof sources.
- WP4/WP5 (failure modes, negatives): `failing_check_demo` (ordered
  FAIL/FAIL/PASS accumulation), `missing_observation_demo` (observation
  diff), `missing_fragment_demo` (absent substring with file path),
  `wrong_phase_demo` (analysis authoring error). Captured outputs follow.
- WP6 (no Bazel-in-Bazel): everything runs in the single Bazel invocation;
  `BAZEL_REAL` is unset and no nested Bazel exists. Execution-phase evidence
  comes from runfiles, not subprocesses.
- WP7 (tested-stack manifest): `//tools/testing:stack` emits `stack.json`;
  `stack_contract` asserts manifest content and cross-checks every version
  against `.bazelversion` and `MODULE.bazel` ground truth.
- WP8/WP9 (protocol + docs): target-level selection, `flaky` passthrough,
  standard logs/XML, `test_suite` grouping (`:all` in both test packages);
  `docs/testing/starlark.md` rewritten as the full contract, ADR 0009
  qualified with M01 findings.

## WP3 Feasibility Evidence (Reproducible)

Probe: a Starlark test rule whose implementation provably executed (subject
output file `sum=42` + passing test) under `bazel coverage`:

- Per-test `coverage.dat`: 0 bytes. Merged `_coverage_report.dat`: 0 `SF`
  records with a scratch-only filter (Rust records correctly absent).
- `bazel help build` exposes no Starlark coverage flags (only the generic
  `--coverage_output_generator`, `--coverage_report_generator`,
  `--coverage_support`).
- Upstream `bazelbuild/bazel#15594` (Starlark coverage recorder) was never
  accepted over maintainability and performance concerns.

Conclusion: reliable line measurement is infeasible on the pinned Bazel
without reimplementing interpretation (rejected per ADR 0009). The
behavioral matrix is used and is never presented as line coverage.

## Environment Findings (Pinned Bazel 9.2.0, Bzlmod, Local Linux)

- Native `sh_*` rules do not exist: `name 'sh_test' is not defined`. The
  framework takes no shell dependency; runners are generated scripts.
- `BUILD_WORKSPACE_DIRECTORY` and `BAZEL_REAL` are unset under `bazel
  test`. Runfiles resolve as `$TEST_SRCDIR/$TEST_WORKSPACE/<short_path>`
  (`TEST_WORKSPACE=_main`), verified against the runfiles tree.
- `str(Label)` renders canonical `@@//...`; the framework normalizes one
  leading `@@` for observations (requalify on pin bumps per O14).
- `json.encode` sorts keys alphabetically (pinned by test); `size` defaults
  must be set explicitly (`small` via the macro) to avoid timeout warnings.
- `attr.label_keyed_string_dict` needs `allow_files = True` for file
  targets; cross-package file labels need `exports_files`.

## Captured Negative Outputs

`failing_check_demo` (exit non-zero, target `FAILED`):

```text
FAIL: deliberately wrong sum
  expected: 3
  actual:   2
FAIL: deliberately wrong product
  expected: 5
  actual:   4
PASS: control that still passes
starlark_test: 1 passed, 2 failed
```

`missing_observation_demo` renders expected vs actual observations;
`missing_fragment_demo` names the file and absent substring;
`wrong_phase_demo` fails analysis with
`starlark_test (load mode): subjects must be empty`.

## Naming And Scope Notes

- Facade named `starlark_test` per ADR 0009 (matches `docs/testing`).
- M01 covers rules/providers/outputs in analysis mode; aspects, toolchains,
  configurations, output groups, and registered actions stay provisional in
  ADR 0009 pending concrete use. Per-function targets, richer matchers, and
  Rust orchestration are labeled future in `docs/testing/starlark.md`.
- `docs/testing/README.md` corrected: feasibility evidence ran in M01, not
  M00 (the M01 spec governs; M00 exited without the probe).

## Gaps (Accepted)

- Local Linux only; other ADR 0014 platforms remain gaps per M00.
- Assertion quality beyond mapping presence is review-based (stated in the
  matrix, not implied).
- No remote cache/execution evidence; no formatter/linter (M04/M05).
