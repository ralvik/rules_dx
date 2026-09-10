# Starlark Behavioral Matrix (M01)

Evidence-backed fallback for Starlark line coverage under the pinned Bazel
(`9.2.0`), per the [coverage policy](../../testing/README.md#coverage) and
[ADR 0009](../../decisions/0009-starlark-testing.md). Genuine executable-line
instrumentation of `.bzl` files is infeasible on this Bazel: a probe rule
whose implementation provably executed (analysis-produced output file plus a
passing test) yielded a 0-byte `coverage.dat` and zero `SF` records, no
Starlark coverage flags exist (`bazel help build` shows only the generic
`--coverage_output_generator`, `--coverage_report_generator`,
`--coverage_support`), and the upstream Starlark-coverage proposal
(`bazelbuild/bazel#15594`) was never accepted. Full probe detail lives in
the [M01 completion report](../../milestones/M01-completion-report.md).

This matrix is repository metadata, not a result stream. Every inventory
item maps to passing tests with meaningful behavior assertions. Presence of
each mapping is machine-checked by `matrix_validation` in
`libs/starlark/tests`; every mapped test target carries evidence by
construction (the framework rejects evidence-free tests at analysis), and CI
runs both the validation and the mapped tests. Assertion quality beyond that
is review-based, which is stated here rather than implied.

Conformance reading guide: `matrix-item: <name>` anchors below are the
machine-checked mapping points. A removed or renamed anchor fails
`matrix_validation`; a mapped test with no assertions cannot exist.

### item: expect_equal

Public assertion constructor in `libs/starlark/defs.bzl`. Converts one
named equality assertion into a deterministic JSON record at loading time.
Proven by `arithmetic_unit` (five behavior assertions over three subject
functions) and by the record-encoding pin in `fixture_execution`.
matrix-item: expect_equal

### item: starlark_test-facade

Public `starlark_test` macro in `libs/starlark/defs.bzl`: one call is one
addressable test target with `size` defaulting to `small`, dispatching to
the internal per-mode rule implementation. Every M01 test target is built
through this facade.
matrix-item: starlark_test-facade

### item: load-mode

Internal load implementation: requires non-empty `checks`, rejects
`subjects`, evaluates records at execution. Proven by `arithmetic_load`,
whose values (`LOADED_SUM` top-level binding, public entry calls) compute
while test files load.
matrix-item: load-mode

### item: unit-mode

Internal unit implementation: requires non-empty `checks`, rejects
`subjects`. Proven by `arithmetic_unit` over pure subject functions with no
I/O.
matrix-item: unit-mode

### item: analysis-mode

Internal analysis implementation: requires non-empty `subjects`, observes
provider fields and output basenames, renders deterministic observations,
compares against `expected_observations` as execution-phase failure (never
an analysis error). Proven by `subject_analysis` over `example_subject`
(`DxSubjectInfo` fields plus output file).
matrix-item: analysis-mode

### item: execution-mode

Internal execution implementation: requires non-empty `file_checks`,
rejects `subjects`, greps runfiles fixtures with AND semantics over
newline-separated substrings. Proven by `fixture_execution` over
`answer_fixture.txt` and `shape_fixture.txt`.
matrix-item: execution-mode

### item: dx-subject-info

`DxSubjectInfo` provider: string-keyed analysis observations from subject
rules. Proven by `subject_analysis`, which asserts all three fields plus
the output basename.
matrix-item: dx-subject-info

### item: failure-rendering

Mismatch diagnostics render `FAIL` with expected and actual values in
declaration order, accumulate across checks, and exit non-zero so Bazel
reports `FAILED`. Proven by the manual negative demonstrations in
`libs/starlark/tests/negative` (captured output in the M01 completion
report) and structurally by every generated runner.
matrix-item: failure-rendering

### item: mode-validation

Wrong evidence for the declared mode (empty required evidence, subjects in
load/unit/execution mode) fails analysis with an authoring error, never a
passing test. Proven by the `wrong_phase_demo` manual target and the
framework's evidence validation.
matrix-item: mode-validation

### item: tested-stack

`tested_stack` rule in `libs/testing/tested_stack.bzl` emitting the pinned
stack manifest. Proven by `stack_contract`, which asserts manifest content
and cross-checks every version against `.bazelversion` and `MODULE.bazel`
ground truth.
matrix-item: tested-stack
