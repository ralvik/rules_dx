# Testing Strategy

## Goals

Tests must prove graph ownership, action construction, hermeticity, deterministic
selection, cache invalidation, diagnostics, and CLI transparency. Passing tool
output alone is insufficient.

Focused test matrices define the domain-specific evidence:

- [CLI](cli.md) covers argument handling, output, target resolution, command behavior,
  Bazel forwarding, and end-to-end CLI operation.
- [GitHub CI](github-ci.md) covers consumer setup, check selection, execution, events
  and revisions, reporting, fork security, merge gating, and qualification.
- [Generation](generation.md) covers first-party Gazelle extensions, language wrappers,
  dependency resolution, naming, merge behavior, and generation transport.
- [Environments](environments.md) covers environment, codegen, setup, BEP collection,
  projections, installation, and concurrency.
- [Tools](tools.md) covers acquisition, runtimes, toolchains, platform execution,
  release pins, performance, and unused-foundation laziness.
- [Quality Workflow Testing](../quality/quality-testing.md) covers quality result and
  diagnostic protocols, configured policy, native config binding, source selection,
  evaluators, cache correctness, determinism, apply safety, and tool parity.

Environment-specific contracts are defined in
[Developer Environments](../environments/environment.md) and
[Python Environment](../environments/python-environment.md). Each focused matrix links
to the authoritative product contracts that its tests exercise.

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

**Accepted requirement.** Project-wide first-party implementation requires 100%
coverage of non-ignored executable lines, not a changed-lines-only gate. Additional
implementation languages, such as Go used by first-party Gazelle extensions, also
require Bazel-owned instrumentation and reporting. Starlark follows the investigation
and conditional fallback below. No mandatory branch-coverage percentage applies;
branch coverage may be reported separately for information. Do not round a line-coverage
shortfall up to a pass.

Use each coverage tool's native source-level ignore directives for code that cannot
reasonably be covered. Each ignore requires a nearby explanatory comment, validated
in CI; no separate exception registry or individual approval process is required.
Valid ignores exclude their executable lines from the denominator. Custom Starlark instrumentation uses the same markers.
Ignore syntax is `LCOV_EXCL_LINE` for one line and `LCOV_EXCL_START` / `LCOV_EXCL_STOP` for a range, each with a `reason:` comment on the same or previous line. CI validates the marker and the nearby reason; missing reasons and malformed directives fail the gate.

Non-ignored eligible sources absent from reports or never executed remain in the
denominator and are reported as uncovered. Missing reports and incomplete required
instrumentation fail the gate; they are not source-level ignores. Upstream dependencies and tool-generated boilerplate must
be classified separately from authored first-party implementation. Executed
first-party authored logic remains eligible when emitted through generation;
being generated is not itself an exclusion. Blanket directory exclusions are not
a substitute for reasoned source-level ignores.

For Starlark, first investigate genuine executable-line instrumentation under the
pinned real Bazel, without a second interpreter or emulated Bazel semantics. If
documented evidence demonstrates that reliable line measurement is infeasible,
the custom testing capability may instead use a checked behavioral matrix covering
100% of the inventory of public `.bzl` entry points, pure functions, rules, aspects,
providers, attributes, configurations, actions, and failure paths. Every item
must map to passing tests with meaningful assertions of its behavior, not merely
a loaded file, a test name, or an unverified mapping. Missing inventory items,
mappings, or assertion evidence fail the gate. The matrix is repository metadata
validated by tests, not a custom test-result format.

The fallback is explicitly **not Starlark source-line or branch instrumentation** and
must not be presented as such or combined into a source-coverage percentage.
Loaded-file counts and test counts are not source coverage. Record the investigated
routes, reproducible feasibility evidence, and limitations in the work report before using the
fallback. Mutation tests may supplement, but not replace, the required evidence.

**Resolved measurement mechanics (O47 resolved, standard-practice rules).** Canonical report format is LCOV from `bazel coverage`, merged per required configuration/platform cell. Rust uses the pinned `rules_rust` llvm-cov integration; Starlark uses custom instrumentation emitting LCOV `DA` records with identical line semantics. Executable lines are `DA` records; blank and comment-only lines are not executable; compiler-generated regions are explicitly listed, not silently dropped; a target with no executable lines is listed as no-code, never an implicit pass. Eligible sources are the repo-owned inventory reconciled against Bazel-declared first-party implementation sources plus generated-source provenance, independent of executed tests; test/fixture-only code, schemas, upstream code, and generated boilerplate are classified separately, and authored logic emitted through generation stays eligible. Aggregation deduplicates by authored source and metric identity within each cell, unions hits across that cell's tests, retains zero-hit eligible sources, and requires every required cell to pass at 100% with exact covered/eligible counts and uncovered locations; languages and metrics stay separate, with no cross-platform union, no averaged percentages, and no rounding up. Missing reports, incomplete instrumentation, and absent eligible sources fail the gate. Negative fixtures cover valid ignores and denominator effects, missing reasons, malformed directives, missing reports, and uncovered lines. Empirical Starlark feasibility evidence runs inside M00 against the O14 seed Bazel per the [M00 entry checklist](../milestones/README.md#m00-entry-checklist); the enforced gate passes at M00 exit and is retained through M28.

**Accepted mechanics:**

- Reconcile a repository-owned source inventory with Bazel-declared sources and
  generated-source provenance, independently of executed tests, to detect absent
  reports and unowned implementation. Include authored helpers, scripts, and
  executable templates; explicitly classify test/fixture-only code, schemas,
  upstream code, and generated boilerplate instead of excluding directories.
- Use pinned Bazel coverage integrations and report converters, with line metrics
  for each instrumented implementation language. Define executable
  lines, compiler-generated regions, macro/template attribution,
  and zero-denominator handling explicitly; a missing metric is not an empty one.
- Deduplicate by authored source and metric identity within each required
  configuration/platform cell. Union hits across that cell's tests, retain
  zero-hit eligible sources, and require each cell to pass rather than unioning
  platform results to hide gaps. Keep languages and metrics separate, with exact
  covered/eligible counts and uncovered locations rather than averaged percentages.
- Test valid ignores and their denominator effects, missing reasons, malformed
  directives, missing reports, and uncovered non-ignored lines. Do not assume
  that every upstream instrumenter already supports a suitable comment directive.
- If the Starlark fallback is necessary, version its complete inventory with implementation changes and
  validate each item's assertion evidence through real Bazel tests, including
  negative conformance cases that reject missing or meaningless mappings.

M00 must establish and pass this gate before the first implementation milestone
can exit. M01 supplies the Starlark testing capability and the selected measurement
route. Earlier authored Starlark must satisfy the same instrumentation-first policy
or its evidence-backed fallback; enforcement is not deferred to M01. Subsequent
implementation and M28 release qualification must retain the gate for the full
eligible implementation, including newly introduced languages. Coverage does
not replace the focused behavioral, consumer, or platform suites.

### End-to-End Tests

End-to-end suites use the real Bazel launcher and external consumer fixtures. The
[CLI](cli.md), [Generation](generation.md), [Environments](environments.md),
[Tools](tools.md), and [Quality Workflow](../quality/quality-testing.md) matrices
define their required behavior and evidence.

Tests for a milestone cover only commands implemented by that milestone. The
complete end-to-end matrix is required before API stabilization, not during an
earlier rule prototype or partial CLI milestone.

## GitHub Coverage Reporting

Use Codecov for this project's GitHub coverage reporting under the
[free-infrastructure constraint](#infrastructure-budget). This selects a repository
service, not a required dependency or service for `rules_dx` consumers. Publish reports
from Bazel-owned coverage workflows; Codecov does not replace instrumentation, ignore
validation, or the authoritative [coverage gate](#coverage). A Codecov summary must not
turn missing reports or failing required coverage into success, and any permitted
Starlark behavioral fallback must remain separate from measured line coverage.

Prefer the maintained upstream Codecov GitHub integration. Account/repository activation,
pinned upload tooling, authentication and fork-PR permissions, report paths and identities,
platform/configuration grouping, and upload-failure handling require qualification under
O14. Coverage mappings are resolved in the [coverage gate](#coverage). Verify complete-report publication and failure cases before claiming the
integration works. No workflow or Codecov account configuration exists in this scaffold yet.

## Infrastructure Budget

CI and release infrastructure must use services available free of charge to this
public GitHub repository. Paid runners, caches, remote execution, signing, storage,
and service overages are not approved; any paid exception requires separate approval.
Qualify free-tier eligibility, host availability, quotas, and retention rather than
assuming public-repository status makes every service free. Exhausted quotas or missing
required hosts block affected work; they do not waive platform, coverage, artifact-trust,
or release evidence requirements. Exact service mappings remain technical qualification work.

## Remote Tests

Remote cache tests are required before claiming remote-cache correctness. Remote
execution tests are required before declaring a toolchain remotely executable.
If infrastructure is unavailable, documentation must state that hermeticity is
designed and locally sandbox-tested but remote behavior remains unverified.

## Documentation Checks

Repository formatter, linter, and Markdown link/structure adapters land in
[M04](../milestones/M04-initial-quality-adapters.md), with direct-Bazel dogfood in
[M05](../milestones/M05-direct-bazel-dogfood.md). M00 does not introduce temporary checks.
This design-only checkout has no repository-defined formatter, linter, Bazel module, or test
targets. Manual structure and link checks are the available documentation verification:
check heading hierarchy, relative targets and anchors, code-fence languages, and consistency
between owning contracts and their summaries. Review whitespace in both tracked and untracked
changed files; `git diff --check` alone does not cover untracked files.
Report absent entry points as verification gaps, not passing checks. Add exact commands here
when the repository-owned workflows land.

## Acceptance Evidence

Each implementation milestone reports exact commands, test counts/results,
supported platforms, relevant `aquery` or execution-log evidence, and known gaps.
Warnings are failures. Claims about caching, hermeticity, or remote support include
the evidence that supports them.

Completion reports classify every exercised capability without conflating these
states:

- **Bootstrap-maintained** evidence is currently unused: there
  are no temporary seed tools. If a future bootstrap need arises, its evidence
  would identify the checksummed tool, pin, acquisition identity, command, host
  coverage, and clean-checkout results. It makes no product-adapter or support claim.
- **Dogfooded** evidence identifies the repository corpus and classes exercised,
  exact direct-Bazel or `dx` invocation, exclusions, action counts, no-op and
  narrow/config-change behavior, cache observations, and parity deviations. It
  proves repository use, not the complete adapter or release matrix.
- **Adapter-tested** evidence identifies the adapter and acquisition identity and
  links its external-consumer, exact-input, native-config/no-config, isolation,
  diagnostics/edit, cache, laziness, and applicable platform cells. It does not
  imply supported status.
- **Supported** evidence links every required adapter, external-consumer,
  interoperability, platform, packaging, documentation, and release-acceptance
  cell, plus remote-cache or remote-execution evidence for each corresponding
  public claim. A missing cell remains an explicit gap and blocks that support
  claim.

There are no temporary seed checks and no seed-to-adapter
parity gate. Each product adapter is introduced directly with its own fixtures
over the same repository bytes and corpus; evidence compares selected
files/classes, effective native configuration and exclusions, normalized
diagnostics and proposed edits, exit status, and deterministic reruns. Missing
adapters are accepted gaps until they land, not failures of a temporary check.
