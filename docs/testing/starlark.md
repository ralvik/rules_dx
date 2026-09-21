# Starlark Testing

Accepted contract for the project-owned `starlark_test` facade under the
pinned Bazel version, implemented per
[ADR 0009](../decisions/0009-starlark-testing.md). The
[testing strategy](README.md) indexes it alongside the CLI, CI, generation,
environment, tool, and quality matrices.

Public API: `starlark_test` macro, `expect_equal`, `expect_true`,
`expect_false`, `expect_contains`, `expect_match` assertion constructors,
`DxSubjectInfo` plus `DxAspectInfo` plus `DxConfigInfo` providers plus
`dx_aspect_note` aspect, all loadable from `//libs/starlark:defs.bzl`. One
macro call is one addressable Bazel test target with one Bazel result.

## Modes

`starlark_test` takes an explicit `mode` (`load`, `unit`, `analysis`, or
`execution`) and dispatches to the internal per-mode rule implementation:

- `load` asserts values computed while test files load (module top-level
  bindings, public entry calls). Requires non-empty `checks`, rejects
  `subjects`.
- `unit` asserts pure Starlark function results with no I/O. Requires
  non-empty `checks`, rejects `subjects`.
- `analysis` observes subject targets (provider fields, output basenames,
  aspect notes, configuration notes), renders deterministic observations,
  and compares them against `expected_observations`. Requires non-empty
  `subjects`. Every subject carries the `dx_aspect_note` observation
  aspect, which derives `aspect_seen` plus `subject_label` plus
  `has_subject` plus `field_count` plus transitive `deps` notes without
  subject cooperation; observations render sorted `aspect_field` lines.
  Configuration subjects expose `DxConfigInfo` (`config_value` plus
  `select`-resolved `note` plus `platform`-fragment presence plus
  `transition` role, including the transitioned dep value); observations
  render sorted `config_field` lines. The aspect use case lives in
  `../../libs/starlark/tests/fixtures/starlark_futures/aspect_subjects.bzl`
  (leaf plus group with deps, direct plus transitive notes), proven by
  `//libs/starlark/tests:aspect_subject_analysis`. The configuration use
  case lives in
  `../../libs/starlark/tests/fixtures/starlark_futures/config_subjects.bzl`
  (leaf plus group with configurable `select` plus `platform` fragment plus
  outgoing flip transition), proven by
  `//libs/starlark/tests:config_subject_analysis`.
- `execution` greps runfiles fixtures (`file_checks`, AND semantics over
  newline-separated substrings, one result line per substring). Requires
  non-empty `file_checks`, rejects `subjects`.

Any mode accepts `file_checks` additionally. Wrong evidence for the
declared mode (missing required evidence, or `subjects` in a non-analysis
mode) fails analysis with an authoring error: such targets can never pass
silently. Evidence-free tests are rejected the same way.

## Authoring

Test `.bzl` files call `expect_equal(name, actual, expected)`,
`expect_true(name, actual)`, `expect_false(name, actual)`,
`expect_contains(name, haystack, needle)`, or
`expect_match(name, value, want)` while they
load and export a macro that instantiates `starlark_test` with those
records. BUILD files only instantiate the exported macro with a target
name; they never encode assertion data. Values must be JSON-encodable;
records serialize deterministically via `json.encode`.

`expect_true`/`expect_false` assert booleans without pinning full
equality rendering. `expect_contains` is type-aware membership: string
substring, list/tuple element, or dict-key presence. `expect_match`
asserts the stringified rendering contains a substring, so a fingerprint
or rendered list can mention a fragment without pinning full bytes
(where `expect_contains` on a list would demand an exact element).
Absence uses `expect_true` with `not in`. Predicates compute while test
files load; mismatches still report at execution like equality. The
concrete use case lives in
`../../libs/starlark/tests/fixtures/starlark_futures/matchers.bzl`
(greet plus pair-error plus admitted-list plus subject-fields plus
fingerprint), proven by `//libs/starlark/tests:matcher_unit`.

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

Decided under closed #588 plus #790 plus #791 plus #792 plus #793 plus #794 per [ADR 0009](../decisions/0009-starlark-testing.md)
(remaining subjects provisional pending concrete use cases), pinned by fixtures in
`../../libs/starlark/tests/fixtures/starlark_futures/` (`pins.bzl` plus
`starlark_futures.expected` plus `matchers.bzl` plus `aspect_subjects.bzl` plus `toolchain_subjects.bzl` plus `output_group_subjects.bzl` plus `config_subjects.bzl`)
and qualified by
`bazel run //tools/ci:starlark_futures_qualification`. Test framework only;
seed only, no Supported claim. Richer matchers graduated under #790 and
are accepted above; no larger matcher library beyond the five
constructors is committed. Aspect subjects graduated under #791 and are
accepted above; toolchain subjects stay deferred with the platform plus
toolchain mapping plus resolved-report use case pinned under #792 (via
`toolchain_subjects.bzl`, proven by `//libs/starlark/tests:toolchain_unit`);
output-group subjects stay deferred with the group-to-files mapping plus
resolved-report use case pinned under #794 (via
`output_group_subjects.bzl`, proven by
`//libs/starlark/tests:output_group_unit`); configuration subjects graduated
under #793 and are accepted above; action subjects stay deferred.

- Per-check filtering stays wont-fix: target granularity is contract. One
  macro call is one addressable Bazel test target with one Bazel result;
  the generated runner does not interpret `--test_filter` per check, and
  mismatches accumulate within that target. Per-check
  `--test_filter` parsing is rejected; split checks into separate
  `starlark_test` targets for finer filtering, caching, retries, and
  diagnostics.
- Toolchain subjects stay deferred (#792) with the platform plus toolchain
  mapping plus resolved-report use case pinned via `toolchain_subjects.bzl`
  (proven by `//libs/starlark/tests:toolchain_unit`). Toolchain resolution
  needs platform and toolchain context beyond provider-field observation;
  direct toolchain observation stays deferred and consumers expose resolved
  state via `DxSubjectInfo` when observation is needed.
- Configuration subjects graduated under #793 and are accepted above;
  configurable `select` attributes plus `platform`-fragment presence plus
  outgoing `config_flip_transition` are observed as `config_field` lines
  via `DxConfigInfo`, with the leaf plus group use case in
  `config_subjects.bzl` proven by
  `//libs/starlark/tests:config_subject_analysis`.
- Output-group subjects stay deferred (#794) with the group-to-files
  mapping plus resolved-report use case pinned via
  `output_group_subjects.bzl` (proven by
  `//libs/starlark/tests:output_group_unit`). Observation renders
  `DefaultInfo` files only, not `OutputGroupInfo`; wrapper forwarding of
  output groups does not imply observation.
- Action subjects stay deferred (#795), including registered-action, pending a
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
