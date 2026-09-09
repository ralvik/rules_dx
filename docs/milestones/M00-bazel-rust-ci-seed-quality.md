# M00: Bazel, Rust, And CI

## Outcome

A clean checkout has minimal seed Bazel and Rust build/test paths, CI skeleton, and an implementation-coverage gate.

## Scope

Create minimal Bzlmod and Bazelisk configuration, a minimal Rust binary and test, and a minimal repository CI
skeleton for build and test. Seed only what M00 needs; remaining pins, hosts, and quality gates land in
their implementing milestones. There are no temporary seed format/lint/link/structure
checks: quality gates land only when the real Bazel adapters land in M04/M05, and gaps are
accepted until then. Prettier is deferred.
Establish the repository implementation-coverage gate, not a release-only check. Start Linux-first
with local-only execution and record unavailable required hosts as gaps; v1 retains full
[required-host](../decisions/0014-tested-platform-release-stack.md#required-platforms) coverage.
Linux-first is bring-up order, not a scope reduction.

## Contract References

- [Bazel execution](../decisions/0001-bazel-owns-execution.md), [dependency currency](../decisions/0008-dependency-currency.md), [tested platforms](../decisions/0014-tested-platform-release-stack.md), [testing](../testing/), and open decision [O14](../open-decisions.md).
- [Mandatory line-coverage and reasoned-ignore policy](../testing/README.md#coverage);
  mechanics resolved there (O47 resolved); Starlark feasibility evidence runs inside M00,
  and the enforced gate passes at M00 exit.
- [V1 admission](../product/scope.md#first-release-admission) and [O46](../open-decisions.md): the
  reviewed inventory and delivery work packages are prerequisites, not bootstrap implementation scope.

## Deliverables

- Minimal seed pins, Bazel version/configuration, Rust toolchain and targets, and CI skeleton for build and test. Full pins and host coverage freeze in implementing milestones.
- No formatter/linter entry points yet; they land with the real adapters in M04/M05.
- Bazel-owned coverage collection, source-inventory reconciliation, reports, and CI enforcement
  implementing the resolved coverage policy, including native source-level ignores and nearby-reason
  validation, for all eligible M00 implementation.

## Work Packages

1. Pin minimal seed Bazel, Rust, and rulesets.
2. Add minimal Rust build/test and external-dependency fixtures.
3. Add minimal CI skeleton for build and test on the seed host; full host coverage and CI qualification stay in M27; no temporary quality checks.
4. Implement the resolved coverage gate and prove it fails on incomplete evidence, uncovered code,
   and invalid or unreasoned coverage ignores.

## Milestone-Specific Evidence

- Clean-checkout runs reproduce Rust build and test results.
- CI records exact host coverage without claiming remote, quality-adapter, or product support. Missing quality gates are accepted gaps until M04/M05, not failures.
- The central coverage gate passes before this first implementation milestone exits; missing reports,
  incomplete required line instrumentation, and omitted or unexecuted non-ignored eligible sources
  cannot produce a pass. Negative fixtures exercise missing reports, unrecognized ignore directives,
  and ignores without a nearby explanatory reason; valid reasoned ignores affect only their intended lines.
- Any authored M00 Starlark follows the
  [instrumentation-first policy](../testing/README.md#coverage), including Starlark feasibility evidence
  under pinned real Bazel. A behavioral matrix fallback requires documented
  infeasibility and explicit labeling as behavioral evidence, never line coverage; this does not wait
  for M01's public testing API.

## Out Of Scope

- Starlark test APIs, language wrappers, product quality adapters, CLI workflows, and general tool acquisition.

## Completion Report Additions

- Record exact pins, CI hosts, and unavailable required hosts.
- Record the coverage resolution evidence: instrumentation identities, inventory/classification, exact metric counts,
  report artifacts, ignore/reason and missing-report fixtures, Starlark feasibility evidence, and any
  coverage blockers.
