# Testing Strategy Details

Test layers, coverage, infrastructure, and evidence contracts.
The [Testing index](README.md) stays short; this document owns the detail record.

## Test Layers

### Starlark Tests

Use the project-owned `starlark_test` facade from the `rules_dx` module under the
pinned Bazel version, with explicit load, unit, and analysis modes over real
Bazel mechanisms. Full mode, assertion, negative-test, filtering, orchestration,
and protocol detail lives in [Starlark Testing](starlark.md).

### Consumer Fixtures

Create focused Bzlmod workspaces under `examples/` or a dedicated test-fixture
location. They should cover direct Python sources, generated dependency context, aliases,
filegroups, namespace packages, multiple ownership, third-party dependencies,
configuration extension, all first-release language families, and competing Ty
granularities.

Examples intended as documentation belong under `examples/`.
Pure test fixtures may live under a test package to avoid presenting unsupported
patterns as user guidance.

Domain matrices define their own fixture requirements. Consumer fixtures verify
those suites against real external Bzlmod workspaces rather than mocked provider
graphs.

### Coverage

Project-wide first-party implementation must pass the single exact gate: the
single exact per-cell gate with zero uncovered non-ignored executable lines in
each required cell's versioned inventory. Run the gate with `dx coverage --min-coverage`; the flag is a
user-facing threshold, informational only for the repo verdict, not a second repo gate. Do not round a shortfall up to a pass.

Use each coverage tool's native source-level ignore directives for code that
cannot reasonably be covered. Each ignore needs a nearby specific short
`reason:` plus `issue:` tracking plus reviewer approval in the owning PR.
Valid ignores leave the denominator. Ignore syntax is `LCOV_EXCL_LINE` for one
line and `LCOV_EXCL_START` / `LCOV_EXCL_STOP` for a range, each with
`reason: <what/why>, issue: <number>, policy: docs/testing/strategy-details.md#coverage`
(at most 120 chars) on the same or previous line (split form allowed).
Bare `policy:` without `reason:` plus `issue:` fails the gate. Markers live in
line comments outside string literals only. Excludes stay budgeted with expiry
review in `tools/coverage/excludes-budget.txt` (60 ranges plus 28 singles,
148 markers total, expires 2027-03-01, tracking issue #1055), enforced by
`bazel run //tools/ci:coverage_excludes_qualification`.

Non-ignored eligible sources absent from reports or never executed stay in the
denominator as uncovered. Missing reports and incomplete instrumentation fail
the gate. Upstream dependencies and tool-generated boilerplate are classified
separately from authored first-party implementation; authored logic emitted
through generation stays eligible. Blanket directory exclusions are not a
substitute for reasoned source-level ignores.

Canonical report format is LCOV from `bazel coverage`, merged per required
configuration/platform cell. Each cell deduplicates by authored source, unions
hits across that cell's tests, retains zero-hit eligible sources, and must pass
on its own; never union cells to hide gaps. Executable lines are `DA` records;
blank and comment-only lines are not executable. A target with no executable
lines is listed as no-code, never an implicit pass.

For Starlark, first investigate genuine executable-line instrumentation under
the pinned real Bazel. If reliable line measurement is infeasible with
documented evidence, use a checked behavioral matrix covering 100% of the
inventory of public `.bzl` entry points, pure functions, rules, aspects,
providers, attributes, configurations, actions, and failure paths, each mapped
to passing tests with meaningful behavior assertions. The fallback is not
source-line instrumentation and must not be presented as a coverage percentage.

Release qualification retains the gate for the full eligible implementation,
including newly introduced languages. Coverage does not replace the focused
behavioral, consumer, or platform suites.

### End-to-End Tests

End-to-end suites use the real Bazel launcher and external consumer fixtures. The
[CLI](cli.md), [GitHub CI](github-ci.md), [Generation](generation.md), [Environments](environments.md),
[Tools](tools.md), and [Quality Workflow](../quality/quality-testing.md) matrices
define their required behavior and evidence. Flakiness plus timeout tuning lives in [GitHub CI](github-ci.md).

Tests cover only implemented commands. Open work is tracked in GitHub issues;
the [support matrix](../product/support-matrix.md) stays the status source.

Non-dogfed paths never run under the standard dogfood gates by design; each has
an explicit execution path: CLI-contract via hermetic pins under `bazel test //...`,
negative fixtures via explicit failure proofs, the no-coverage cohort via
coverage-excluded runs, and shell sources via ownership plus test execution with
no quality class by design. The plan stays green via
`bazel run //tools/ci:non_dogfed_paths` plus
`bazel run //tools/ci:non_dogfed_qualification`.

## GitHub Coverage Reporting

This project uses first-party coverage PR reporting under the
[free-infrastructure constraint](#infrastructure-budget). Every required
cell gates its own combined LCOV through `dx coverage --min-coverage`
against its versioned inventory, with no cross-cell union, no averaged
percentages, and no rounding up. Each cell publishes one marker-owned
updated comment per PR (deduped, never overwriting human comments; fork
pull requests stay step-summary only so fork code never gets write
credentials) plus a step-summary copy, all rendered by the same
`reusable-consumer.yml` coverage job so every cell keeps the same shape.
`tools/coverage/coverage_comment.sh` stays the versioned first-party
renderer (verdict plus Uncovered locations plus LCOV note plus dx rate
line), proven against fixtures by
`bazel run //tools/ci:coverage_report_guards`. A summary comment never turns
missing reports or failing coverage into success, and any Starlark behavioral
fallback stays separate from measured line coverage. Per-cell, Codecov, and
remote evidence is qualified by `bazel run //tools/ci:coverage_qualification`
over the pins fixture `tools/coverage/tests/fixtures/per_cell/pins.bzl`.

Codecov stays at most opt-in and is never required. No Codecov account exists for this repository.

## Infrastructure Budget

CI and release infrastructure must use services available free of charge to this
public GitHub repository: standard GitHub-hosted runners is free, the Actions
cache allowance is 10 GB, artifact storage is 500 MB, and Larger runners are always charged. Paid runners, caches, remote execution, signing, storage, and
service overages are not approved; any paid exception requires
separate approval. Exhausted quotas or missing required hosts block affected
work; they do not waive platform, coverage, artifact-trust, or release evidence
requirements.

GitHub Container Registry image storage and bandwidth are currently free for public repositories, with one month notice before any pricing change (GitHub Packages billing); the private-Packages quotas do not apply to containers.

## Remote Tests

Remote cache tests are required before claiming remote-cache correctness. Remote
execution tests are required before declaring a toolchain remotely executable.
If infrastructure is unavailable, documentation must state that hermeticity is
designed and locally sandbox-tested but remote behavior remains unverified.
That else branch is taken here: hermeticity is designed and locally
sandbox-tested but remote behavior remains unverified, with no remote cache or
executor wired. Dx pipeline plus evaluator actions carry `no-remote-exec`.
Determinism claimed here is local per-cell determinism only, with no
cross-cell union and no remote claim.

Snapshot goldens use schema validation plus byte snapshots with an
UPDATE_EXPECT refresh workflow: run `UPDATE_EXPECT=1 bazel test <target>
--test_env=UPDATE_EXPECT`, review the diff, then commit.

## Documentation Checks

Repository formatter, linter, and Markdown link/structure adapters are
repository-owned, with direct-Bazel dogfood. Run the repository-owned workflows
(`bazel run //cli/cli:dx -- lint`,
`bazel run //cli/cli:dx -- format --check`, plus the corpus dogfood in
[local workflows](../contributing/local-workflows.md#corpus-dogfood)) alongside
manual structure and link checks: check heading hierarchy, relative targets and
anchors, code-fence languages, and consistency between owning contracts and
their summaries. Review whitespace in both tracked and untracked changed files;
`git diff --check` alone does not cover untracked files. Report absent entry
points as verification gaps, not passing checks.

## Acceptance Evidence

Each change reports exact commands, test counts/results, supported platforms,
relevant `aquery` or execution-log evidence, and known gaps. Warnings are
failures. Claims about caching, hermeticity, or remote support include the
evidence that supports them.

Completion reports classify every exercised capability without conflating these
states:

- **Bootstrap-maintained** evidence is currently unused: there are no temporary
  seed tools.
- **Dogfooded** evidence identifies the repository corpus and classes exercised,
  exact invocation, exclusions, action counts, no-op and narrow/config-change
  behavior, cache observations, and parity deviations. It proves repository use,
  not the complete adapter or release matrix.
- **Adapter-tested** evidence identifies the adapter and acquisition identity and
  links its external-consumer, exact-input, native-config/no-config, isolation,
  diagnostics/edit, cache, laziness, and applicable platform cells. It does not
  imply supported status.
- **Supported** evidence links every required adapter, external-consumer,
  interoperability, platform, packaging, documentation, and release-acceptance
  cell, plus remote-cache or remote-execution evidence for each corresponding
  public claim. A missing cell remains an explicit gap and blocks that support
  claim.

There are no temporary seed checks and no seed-to-adapter parity gate. Missing
adapters are accepted gaps until they land, not failures of a temporary check.
