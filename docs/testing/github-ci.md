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
validation. Exercise single- and multi-platform selections, including one without Linux.
Selected test/build/coverage checks must run for every selected platform, while selected
quality checks run once on Linux without silently narrowing declared source/dependency scope.
Verify platform identities remain visible and one failed or missing required platform cell
prevents aggregate success. Coverage aggregation must not hide platform-specific gaps;
qualify any shared coverage/test execution against both commands' result contracts.

## Execution

### Scheduling And Isolation

Verify independent checks are scheduled concurrently by default when runner capacity
permits, and explicit sequential mode executes them without overlap. In both modes,
inject an early check failure and verify independent checks still execute, results are
collected, and the overall run fails. Failed prerequisites block only dependent work.
Exercise isolated setup/checkouts, report paths, and Bazel execution contexts to detect
cross-check mutation, overwritten reports, or shared-output-base serialization. Preserve
each command's own failure semantics rather than injecting a new CLI scheduling policy.

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

Exercise finding sets below, at, and above the qualified fixed per-PR review-thread limit
across multiple checks and platforms. Verify deterministic priority for failure-contributing
findings, no consumer limit setting, and no fresh allowance from retries, reruns, or parallel
job completion. Preserve existing threads and human discussions under the cleanup policy;
do not rotate still-present findings through comments. Verify full reports retain every
finding and the summary distinguishes limit-omitted findings from unmappable locations.
Intentional truncation must not change analysis completeness or CI outcomes, while actual
publication failures still fail reporting. Qualify accounting as findings clear and new
findings appear, including retained resolved discussions and concurrent runs.

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
[GitHub CI contract](../github-ci.md) and [open decisions](../open-decisions.md)
for unresolved qualification work.
