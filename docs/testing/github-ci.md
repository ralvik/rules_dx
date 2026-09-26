# GitHub CI Test Matrix

These tests define required evidence, not executed tests or verified support.

## Scope And Status

This matrix exercises the consumer [GitHub CI contract](../github-ci.md).

## Consumer Setup

Exercise onboarding through the consumer-owned caller template and versioned reusable
workflow without copied execution/reporting logic, a generated setup command, or a custom
bot service. Verify reporting uses the first-party implementation without reviewdog or
another reporting framework. Verify check selection through caller configuration, reviewed pin updates,
preservation of consumer customizations, and no silent workflow upgrades. Document required
repository settings separately from the caller file.

## Check Selection

### Selected Checks And Outcomes

Exercise all nine supported CI checks through a clean external consumer following the
documented combined setup path, both individually selected and together. Verify canonical
configuration and module-matched CLI bootstrap, non-mutating consistency modes, and one
summary containing every selected check. Unselected checks must not execute. Report failed,
blocked, skipped, and configured no-op outcomes truthfully; a configured no-op without a
report event must not be confused with failed report collection. Check modes must leave
tracked sources, manifests, locks, and BUILD files unchanged. Preserve command failures
through reporting, and do not invent standard reports for commands that have none.

### Defaults And Opt-Outs

Verify the unmodified starter selects all nine checks. Disable each check individually
and verify only that check is omitted, unrelated checks remain selected, and disabled
checks are not reported as passed. Defaults must retain configured no-op semantics and
unused-foundation laziness rather than forcing inactive analyzers or languages to run.

### Platform Selection

Verify onboarding requires explicit supported platform selection for enabled tests, build,
or coverage, without an implicit Linux/current-runner/all-platforms default. Missing, empty,
or unsupported selections must fail clearly rather than produce skipped or substituted
validation. Exercise single- and multi-platform selections, including one without Linux,
one with `linux_arm64` (which must queue an `ubuntu-24.04-arm` runner, never
the macOS runner), one with `macos_arm64` (which must queue a
`macos-14` runner, never a Linux runner; issue #412), one with
`windows_x86_64` (macOS x86_64 Not planned per #976 with no runner),
(which must queue a `windows-latest` runner with shell `bash`, never a
Linux runner; issue #414). The host matrix across the five qualified
platforms is pinned by `bazel run //tools/ci:ci_matrix_qualification`
(issue #415). Selected test/build/coverage checks must run for every
selected platform, while selected
quality checks run once on Linux without silently narrowing declared source/dependency scope.
Verify platform identities remain visible and one failed or missing required platform cell
prevents aggregate success. Coverage aggregation must not hide platform-specific gaps;
qualify any shared coverage/test execution against both commands' result contracts.

## Execution

### Scheduling And Isolation

Verify independent checks are scheduled concurrently by default when runner capacity
permits. Sequential mode is qualification-open: the reusable workflow declares
the `scheduling_mode` input but `platforms-gate` fails closed on
`sequential` (see the [contract](../github-ci.md#execution)), so matrix
cases prove parallel scheduling plus the fail-closed sequential rejection.
In parallel mode, inject an early check failure and verify independent checks still execute, results are
collected, and the overall run fails. Failed prerequisites block only dependent work.
Exercise isolated setup/checkouts, report paths, and Bazel execution contexts to detect
cross-check mutation, overwritten reports, or shared-output-base serialization. Preserve
each command's own failure semantics rather than injecting a new CLI scheduling policy.

### Workflow Hygiene

Verify Bazelisk installation comes from the pinned
`bazel-contrib/setup-bazel` action (commit SHA plus trailing tag comment)
rather than copied shell blocks, with the Bazelisk version single-sourced
per workflow through `BAZELISK_VERSION` (canonical
`.devcontainer/Dockerfile.prebuilt`), and
every third-party action reference is pinned to a commit SHA (tag in a trailing comment).
Verify the `platforms-gate` job rejects missing, empty, or unsupported platform selections
before any per-platform job queues a runner, and the aggregate still fails when the gate
does. Verify every Bazel job passes the shared BuildBuddy remote cache
config (`--config=ci` from the workspace `.bazelrc` plus the API-key
header through `$BB_ARGS`, with both vars empty when the
`BUILDBUDDY_API_KEY` secret is absent so no remote-cache flag is set at
all, pull requests upload nothing via `--config=ci-pr` and
`--noremote_upload_local_results`, issue #1059
policy preserved under the reversal of closed #618) and pulls the
Bazelisk download cache through `bazelisk-cache: true` with `cache-save`
off for pull requests (free-tier eligible per the
infrastructure budget; host matrix pinned by
`bazel run //tools/ci:ci_matrix_qualification`, issue #415), with no
`actions/cache` usage anywhere under `.github/workflows/` (the legacy
`restore-bazel-cache` disk cache is deleted). Dx pipeline plus evaluator actions carry `no-remote-exec`
(local-only execution until remote is qualified; `--remote_executor` plus
`--bes_backend` stay absent). Qualified seed-only via
`bazel run //tools/ci:action_execution_cache_qualification` (aquery
`ExecutionInfo` plus `ActionKey` shape plus local execution-log hit/miss;
remote cache read/write stays unverified by that harness). Pass `--noshow_progress` to Bazel invocations, run the corpus
ownership audit through `bazel run //tools/ci:corpus_audit`, and never rely on a local
`user.bazelrc` override. Verify docs publishing stages its artifact outside the checkout
and leaves the tree clean.

Verify CI flakiness plus timeout tuning (issue #619, qualified seed-only via
`bazel run //tools/ci:flakiness_qualification`; CI only, no Supported claim):
bounded retries plus caps live in `.bazelrc` (`test --flaky_test_attempts=3`
plus `test --test_timeout=300` plus `test --local_test_jobs=4`) so every
entrypoint inherits them, with per-invocation `--flaky_test_attempts=3` plus
`--test_timeout=300` where a direct `bazel test` runs, for bounded
transient-flake retries with a per-test 300s
cap; GitHub `timeout-minutes` stay tuned (build/coverage 60, freshness
30, no blanket 90); reusable-consumer timeouts
stay pinned (gate 5, Linux-once 30, per-platform 60, aggregate 10, no 90)
with every `sh_test` carrying explicit per-target `size` plus `timeout`
(issue #932); sharding stays per-host/per-stage
jobs with Bazel intra-job test sharding under ordinary semantics (no
`strategy.matrix`, per the issue #415 policy). Long timeouts only is rejected:
timeouts without bounded retries stay a failure mode, and retry-until-green
analyzer behavior stays rejected — reruns use GitHub's native rerun controls
with unchanged selection plus revision identity. See the
[testing strategy](strategy-details.md#infrastructure-budget) for the free-tier budget
that keeps this local-only.

### Repository Scope And Reruns

Verify documentation-only and mixed changes still invoke all selected checks at their
normal repository scopes without workflow path filters or a second affected-target graph.
Use GitHub's native rerun controls and verify unchanged selection, revision identity, and
idempotent reporting without bot comment commands or retry-until-green analyzer behavior.
Exercise bounded transient reporting retries and exhaustion without duplicating comments,
changing analyzer outcomes, or hiding the final reporting failure.

## Events And Revisions

### Starter Triggers

Verify starter triggers for PRs, default-branch pushes, and manual dispatch, including
a consumer whose default branch is not `main`. Default-branch checks must analyze the
landed revision. Runs without PR context must retain checks and workflow summaries without
writing PR comments. Ordinary non-default-branch pushes alone must not trigger the starter;
PR supersession must not cancel default-branch or unrelated manual runs.

### Draft Pull Requests

Open and update draft PRs and verify the same selected checks, review reporting, and
aggregate-status behavior as ready-for-review PRs, without waiting for a ready transition.
Exercise draft/ready transitions without suppressing validation or duplicating reporting;
fork approval and execution-permission boundaries must remain unchanged in either state.

### Test-Merge Revisions

Use a fixture where the PR branch passes alone but its proposed merge fails, and verify
PR CI detects the integration failure. Verify all selected checks use the same test-merge
snapshot with recorded head/base identities and review locations mapped to the PR diff.
Unmappable findings must remain in reports and counts without invented review locations.
Exercise merge conflicts, including GitHub suppressing the PR workflow: validation must
be reported as blocked without branch-only fallback, aggregate success, or privileged
fork-code execution. Qualify base advancement and revision races so an earlier merge
snapshot is never presented as validation of a later head/base combination.

### Supersession And Cancellation

In both scheduling modes, trigger a new commit while an older run for the same PR is
queued or running and verify superseded runs are cancelled without deleting their history.
Unrelated PRs and workflows must remain unaffected. Race old-run completion and reporting
against cancellation and verify old results cannot overwrite the current summary; cancelled
or unfinished checks must not be shown as passed. A check failure without a new commit
must still allow independent checks to complete.

### Merge-Queue Revisions

Enable a consumer merge queue and verify selected checks analyze its combined revision,
including a case where PRs pass separately but fail together. The same aggregate identity
must satisfy the qualified queue required-check mapping only for the tested queue revision.
Retain reports and workflow summaries without creating, updating, or cleaning up PR comments
or review threads. Exercise queue replacement/removal and concurrent PR runs; stale queue
results must not satisfy newer revisions, and PR supersession must not cancel queue runs.
Queue execution must not grant fork code secrets or write credentials. Setup must not
enable or reconfigure the consumer's merge queue automatically.

## Reporting

### Checks, Summary Identity, And Completeness

Verify the consumer [GitHub reporting integration](../github-ci.md)
with checks, valid-diff-location review threads, and one updated summary comment per PR.
Cover initial creation, later runs, retries, concurrent completions, and an older revision
finishing after a newer one. Preserve human comments and avoid duplicate integration comments.
Check that revision/run identity, workflow failure, and incomplete results remain visible;
locationless findings must not receive invented review locations. Complete failing SARIF
scans remain eligible for authoritative publication, while incomplete scans do not replace
the authoritative scan. Exact permissions and publication mappings require qualification.

### Review Locations And Deduplication

Verify replyable review threads are the required presentation for diff-mapped findings,
with no review-comment disable switch or alternative annotation-only mode. Findings outside
the diff remain in detailed reports and truthful summary counts without changing command
failure. Test exact revision/diff mapping, rerun deduplication, preservation of human replies,
and no duplicate inline annotation for a finding already presented as a review thread.
Race stale reporting against newer revisions and verify it cannot create or modify current
threads. Qualify thread matching, deletion/resolution APIs, outdated-location handling, and
API limits before implementation.

### Security-Audit Findings

Verify security-audit findings follow the same review/report workflow without a security-only
counts mode: diff-mapped findings receive review details, and other findings remain in reports
and summary counts. Preserve severity, acceptance status, completeness, and command outcomes.
Qualify safe rendering and sensitive-content handling with synthetic fixtures, and document
that public-repository reporting is not a private vulnerability-reporting channel.

### Thread Lifecycle

Verify unchanged findings retain their threads without reposting. After a complete relevant
assessment confirms a finding is gone, verify bot-only threads without replies are deleted
and threads with human replies are resolved with the discussion preserved. Preserve unrelated
threads and human comments, including replies racing cleanup. Exercise a complete check
that clears one finding but fails on another. Skipped, disabled, cancelled, or incomplete
checks, missing reports, and findings moving outside the diff must not trigger cleanup.
Prior workflow results must retain their normal retention behavior.

### Compact Summaries

Exercise a large finding set and verify the compact summary retains check statuses,
truthful finding counts, completeness, and working links to detailed results without
expanding into a full finding list or hiding failures.

### Review-Thread Limit

Exercise finding sets below, at, and above the frozen per-PR review-thread limit of 50
open integration-owned threads
(`tools/ci/tests/fixtures/review_threads/pins.bzl` plus `review_threads.expected`
via `bazel run //tools/ci:review_threads_qualification`, issue #592)
across multiple checks and platforms. Verify deterministic priority for failure-contributing
findings, no consumer limit setting, and no fresh allowance from retries, reruns, or parallel
job completion. Preserve existing threads and human discussions under the cleanup policy;
do not rotate still-present findings through comments. Verify full reports retain every
finding and the summary distinguishes limit-omitted findings from unmappable locations.
Intentional truncation must not change analysis completeness or CI outcomes, while actual
publication failures still fail reporting. Qualify accounting as findings clear and new
findings appear, including retained resolved discussions outside the open count, bot-only
deletion freeing a slot, skipped/disabled/cancelled/incomplete/missing/outside-diff never
proving gone, and concurrent runs with late callbacks never overwriting current threads
or summaries.

### Optional Code Scanning

Verify Code Scanning publication is off in the starter and default reporting works without
its permissions or paid security features. Exercise explicit opt-in on an eligible
repository, preserving SARIF completeness and publication-failure behavior. Disabled
publication must not fail CI; unsupported eligibility or missing permissions after opt-in
must be reported rather than silently treated as successful publication.

### Publication Failures

Inject required check, review-comment, and summary-comment publication failures and verify
the workflow fails while retaining individual command outcomes. Passing commands must
not hide failed reporting, and reporting errors must not be mislabeled as analyzer findings.
Intentionally inapplicable outputs, including non-PR comments, must not cause failure.

## Fork Security

Verify GitHub's `all_external_contributors` policy gates outside-contributor fork runs
on receiving-repository approval and does not permit outside-author self-approval. After approval,
verify execution still lacks secrets and write credentials. Test later commits, reruns,
and previously contributing fork authors against GitHub's native approval mapping, including
trusted-collaborator behavior; do not assume approval is required for every fork commit. Verify
privileged reporting never executes PR code and validates untrusted artifacts and metadata.
The setup guide must identify required repository settings rather than implying the
workflow template enables approval controls by itself.

## Merge Gating

### Stable Aggregate Check

Verify a single stable aggregate check can be required in consumer branch protection while
individual results remain visible. Vary check selection and scheduling mode without changing
its identity or requiring branch-protection edits. Verify aggregate success waits for every
selected check and required reporting, preserving configured no-op semantics and opt-outs.
Inject command and reporting failures, missing results, blocked work, unexpected skips,
cancellation, and incomplete collection; none may produce aggregate success. Qualify revision
binding and required-check behavior, including stale runs. The setup guide must explain the
manual branch-protection step, and the integration must not change those settings itself.

### Latest-Target Validation

After a successful PR run, advance the target branch and verify the documented native
up-to-date-branch gate prevents merging on obsolete validation until the updated combination
passes. Exercise the merge-queue alternative, including target advancement during validation,
and verify GitHub requires valid current queue results. Preserve historical results without
presenting them as evidence for an untested combination. Verify setup documents both native
routes and does not install an all-open-PR rerun bot or modify repository settings.

## Qualification

Qualification requirements are retained with the cases above. See the
[GitHub CI contract](../github-ci.md) and open decisions
for unresolved qualification work. Consumer CI qualification plus automation policy
qualified seed-only under issue #509 with fixture evidence
(`tools/ci/tests/fixtures/consumer_ci/pins.bzl` plus `platforms.expected` plus `revisions.expected` plus `reporting.expected`, qualified by `bazel run //tools/ci:consumer_ci_qualification`; platform, runner, isolation, cache, ordering evidence; merge, diff, queue, cancellation, aggregate binding; thread identity, ordering, limits; fork, untrusted, sensitive, retries, Code-Scanning qualification; sequential mode fail-closed pending qualification (fail-closed as-built, ordering unqualified); tag hygiene plus release-input gaps; native-bot follow-ups (native-only updater, issue #461)). Reusable-workflow contract plus caller
plus gate/aggregate plus per-gap decisions fixture evidence qualified seed-only under #509
(`bazel run //tools/ci:consumer_ci_qualification`; nine checks, explicit
platforms, fail-closed sequential, stable aggregate, hygiene, concurrency,
permissions, per-cell coverage with fork-safe comments, self-call dx test plus dx coverage
(issue #408 plus Phase 1 #607 coverage superset, verbatim `//...`), native bump loop (sole updater, issue #461), migrate syntax plus
manifest selection (delivered CLI with fail-closed execution, issue #462) plus run multirun
(issue #463 delivered), tag hygiene as-built, with the per-gap decisions above pinned in
`tools/ci/tests/fixtures/consumer_ci/pins.bzl`; build-only self-call forever
rejected per #408; platform plus release evidence stays owned gap; no Supported claim).
Review-thread limit plus accounting frozen at 50 open threads seed-only under issue #592
(`tools/ci/tests/fixtures/review_threads/pins.bzl` plus `review_threads.expected` via
`bazel run //tools/ci:review_threads_qualification`; no consumer setting, no fresh allowance,
failure-first check/file/line/rule ordering, no rotation, bot-only frees with resolved history
outside the open count, full reports with summary-distinguished truncation, no overwrite;
CI-only, no Supported claim).
Runner Plus SDK Rotation qualified seed-only under issue #642
(`tools/ci/tests/fixtures/runner_rotation/pins.bzl` plus `runner_rotation.expected` via
`bazel run //tools/ci:runner_rotation_qualification`; quarterly plus on retirement notice
plus on hermetic-llvm release, sole-maintainer owner, retirement handling in one reviewed PR,
customer-flows-only build/test/coverage with no new CI job, permanent rotation job rejected;
infra only, no Supported claim).
GHCR Rebuild Plus Signing Rotation qualified seed-only under issue #647
(`tools/ci/tests/fixtures/ghcr_rebuild_rotation/pins.bzl` plus
`ghcr_rebuild_rotation.expected` via
`bazel run //tools/ci:ghcr_rebuild_rotation_qualification`; manual on-demand
rebuild plus rotation with base plus Bazelisk plus Cosign plus TUF pins,
sole-maintainer owner, one reviewed PR, customer-flows-only harness plus
on-demand local docker build with no push/schedule trigger and no extra CI
job, scheduled CI rebuild rejected; infra only, no Supported claim).
Devcontainer boot manual plus wont-fix qualified seed-only under issue #648
(`tools/ci/tests/fixtures/devcontainer_boot/pins.bzl` plus
`devcontainer_boot.expected` via
`bazel run //tools/ci:devcontainer_boot_qualification`; seed-host
linux/amd64 manual boot verify on every Dockerfile or definition change
plus before any gated GHCR push with evidence in the same reviewed PR,
sole-maintainer owner, customer-flows-only harness plus on-demand local
docker build plus devcontainer up with no boot job in CI and no extra CI
job, non-Linux plus linux/arm64 boot plus arm64 prebuilt variant
wont-fix; infra only, no Supported claim).
Consumer CI, devcontainer, and perf honesty
is delivered for the self-call path with owned gaps elsewhere: self-call dx test plus dx coverage
per #408 plus Phase 1 #607 coverage superset (verbatim `//...`, qualified seed-only under #509 via
`bazel run //tools/ci:consumer_ci_qualification`; build-only self-call forever
rejected); devcontainer parity plus definition shape delivered with
seed-host boot manually verified plus non-Linux/arm64 wont-fix under #648
(no boot job in CI); perf report-not-gate rejected per ADR 0022
(no standing benchmarking). Closed #325 carries no open scope.
