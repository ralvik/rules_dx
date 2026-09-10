# Starlark Testing

Accepted contract for the project-owned `starlark_test` facade under the
pinned Bazel version, implemented in M01 per
[ADR 0009](../decisions/0009-starlark-testing.md). The
[testing strategy](README.md) indexes it alongside the CLI, CI, generation,
environment, tool, and quality matrices.

Public API: `starlark_test` macro, `expect_equal` assertion constructor,
and `DxSubjectInfo` provider, all loadable from
`//libs/starlark:defs.bzl`. One macro call is one addressable Bazel test
target with one Bazel result.

## Modes

`starlark_test` takes an explicit `mode` (`load`, `unit`, `analysis`, or
`execution`) and dispatches to the internal per-mode rule implementation:

- `load` asserts values computed while test files load (module top-level
  bindings, public entry calls). Requires non-empty `checks`, rejects
  `subjects`.
- `unit` asserts pure Starlark function results with no I/O. Requires
  non-empty `checks`, rejects `subjects`.
- `analysis` observes subject targets (provider fields, output basenames),
  renders deterministic observations, and compares them against
  `expected_observations`. Requires non-empty `subjects`.
- `execution` greps runfiles fixtures (`file_checks`, AND semantics over
  newline-separated substrings, one result line per substring). Requires
  non-empty `file_checks`, rejects `subjects`.

Any mode accepts `file_checks` additionally. Wrong evidence for the
declared mode (missing required evidence, or `subjects` in a non-analysis
mode) fails analysis with an authoring error: such targets can never pass
silently. Evidence-free tests are rejected the same way.

## Authoring

Test `.bzl` files call `expect_equal(name, actual, expected)` while they
load and export a macro that instantiates `starlark_test` with those
records. BUILD files only instantiate the exported macro with a target
name; they never encode assertion data. Values must be JSON-encodable;
records serialize deterministically via `json.encode`.

Mismatches accumulate within one target and report together in declaration
order through the standard Bazel test protocol: non-zero exit status and
`test.log` diagnostics. An assertion failure is always execution-phase, so
Bazel reports `FAILED`, never a build breakage. Negative demonstrations
live in `libs/starlark/tests/negative` with `tags = ["manual"]` and are
excluded from wildcard suites; run them explicitly:

```sh
bazel test //libs/starlark/tests/negative:failing_check_demo \
  //libs/starlark/tests/negative:missing_observation_demo \
  //libs/starlark/tests/negative:missing_fragment_demo \
  --nocache_test_results
bazel build //libs/starlark/tests/negative:wrong_phase_demo
```

The last command shows the analysis authoring error for a mode violation.

## Selection, Retries, Logs, Suites

Test selection is by target label; the generated runner does not interpret
`--test_filter` per check. Retries use the standard `flaky` attribute
(passthrough, off by default); caching, timeouts, and sharding follow
ordinary Bazel test semantics. Logs and `test.xml` are Bazel-provided;
analysis-mode runners also print their observations to `test.log`. Tests
may be grouped with ordinary `test_suite` targets
(`//libs/starlark/tests:all`, `//libs/testing:all`).

## Environment

The generated runner is a POSIX shell script and requires the standard
Bazel test environment (`TEST_SRCDIR`, `TEST_WORKSPACE`); runfiles resolve
as `$TEST_SRCDIR/$TEST_WORKSPACE/<short_path>`. `BUILD_WORKSPACE_DIRECTORY`
is unset under `bazel test` and must not be used. Native `sh_*` rules do
not exist under the pinned Bazel with Bzlmod, so the framework depends on
none: executables are generated scripts. No nested Bazel invocation is used
anywhere; everything runs inside the single Bazel command.

## Coverage

Starlark has no executable-line instrumentation under the pinned Bazel, so
there is no Starlark line-coverage percentage. The evidence-backed fallback
is the checked [behavioral matrix](../../libs/starlark/behavioral_matrix.md):
every public entry point and behavior maps to passing tests, the mapping is
machine-checked by `matrix_validation`, and every mapped target carries
evidence by construction. The matrix is repository metadata, not a result
stream, and must never be presented as source-line or branch coverage.

## Future (Not Implemented)

Per-check filtering, richer matchers beyond `expect_equal`, aspect /
toolchain / configuration / output-group / action subjects, per-function
test targets, and Rust orchestration of fixture workspaces with BEP
consumption are explicitly future work. They require concrete use cases and
their own milestones; consult [ADR 0009](../decisions/0009-starlark-testing.md)
and [open decisions](../open-decisions.md) before assuming any of them.
