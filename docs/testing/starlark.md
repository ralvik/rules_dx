# Starlark Testing

Accepted contract for the project-owned `starlark_test` facade under the
pinned Bazel version, implemented per
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
Bazel reports `FAILED`, never a build breakage. Negative proofs live in
`libs/starlark/tests/negative` as green hermetic tests (issue #406):
execution failures as passing `sh_test` goldens, analysis failures via
`failure_test` (`analysistest.expect_failure`); failure lives inside
passing bodies, never as failing targets:

```sh
bazel test //libs/starlark/tests/negative/...
```

CI pins these proofs via `bazel test //...` (no separate prove step):
each red fixture has a passing test asserting the exact user-visible
result, and the fixture stopping to fail fails the test.

## Selection, Retries, Logs, Suites

Test selection is by target label; the generated runner does not interpret
`--test_filter` per check. Retries use the standard `flaky` attribute
(passthrough, off by default); wrappers forward the full kwargs dict to
the private upstream test, so `flaky` lands upstream and never on the
public forwarding wrapper (fixture `//python/tests/fixtures/hello:flaky_passthrough_fixture`).
CI pins this in `bazel run //tools/ci:target_tags`, which also proves
`bazel coverage` skips `no-coverage` tests and the `no-lint` /
`no-typecheck` fixtures stay wired. Caching, timeouts, and sharding follow
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

Decided under issue #588 per [ADR 0009](../decisions/0009-starlark-testing.md)
(provisional pending concrete use cases), pinned by fixtures in
`../../libs/starlark/tests/fixtures/starlark_futures/` (`pins.bzl` plus
`starlark_futures.expected`) and qualified by
`bazel run //tools/ci:starlark_futures_qualification`. Test framework only;
seed only, no Supported claim.

- Per-check filtering stays wont-fix: target granularity is contract. One
  macro call is one addressable Bazel test target with one Bazel result;
  the generated runner does not interpret `--test_filter` per check, and
  mismatches accumulate within that target. Per-check
  `--test_filter` parsing is rejected; split checks into separate
  `starlark_test` targets for finer filtering, caching, retries, and
  diagnostics.
- Richer matchers stay deferred: `expect_equal` only, pending a concrete
  use case plus fixtures plus successor issue. Equality over
  JSON-encodable values plus `file_checks` plus `expected_observations`
  covers current internals; no larger matcher library is committed.
- Aspect subjects stay deferred, pending a concrete use case plus fixtures
  plus successor issue. Analysis observes `DxSubjectInfo` fields plus
  `DefaultInfo` output basenames only; applying aspects to subjects is not
  claimed.
- Toolchain subjects stay deferred, pending a concrete use case plus
  fixtures plus successor issue. Toolchain resolution needs platform and
  toolchain context beyond provider-field observation.
- Configuration subjects stay deferred, including transitions, pending a
  concrete use case plus fixtures plus successor issue. Configurable
  attributes, fragments, and transitions are not observed.
- Output-group subjects stay deferred, pending a concrete use case plus
  fixtures plus successor issue. Observation renders `DefaultInfo` files
  only, not `OutputGroupInfo`; wrapper forwarding of output groups does
  not imply observation.
- Action subjects stay deferred, including registered-action, pending a
  concrete use case plus fixtures plus successor issue. Actions are proven
  via execution-mode `file_checks` or `aquery` evidence, not analysis
  subjects.
- Per-function targets stay wont-fix: explicit macro instantiation is contract.
  BUILD instantiates the exported macro with a target name;
  dynamic function discovery would hide the static `load()` constraint
  with a second Starlark interpreter, which is rejected. One function per
  macro call already gives per-function addressability.
- Rust orchestration of fixture workspaces with BEP consumption stays
  wont-fix: single invocation, no nested Bazel. Everything runs inside the
  single Bazel command with the standard test protocol (`test.log`,
  `test.xml`) suitable for Bazel, BEP, and CI collection; no Rust
  orchestrator consumes BEP. Nested Bazel invocation is rejected.

A second Starlark interpreter is rejected, and the behavioral matrix as
line coverage is rejected: the matrix stays repository metadata, never
source-line or branch coverage.
