# Consumer GitHub CI

## Scope And Status

This is the accepted execution and reporting contract for projects using `rules_dx` on
GitHub Actions. GitHub Actions is the only supported CI platform for v1; GitLab,
Buildkite, Jenkins, and other platforms are explicitly unsupported. CLI output
contracts (NDJSON, SARIF, JUnit, LCOV) stay platform-agnostic so a later port
does not require workflow redesign. It is not limited to this repository's dogfood CI. The policies below are
decided; exact public interfaces and implementation evidence remain under
[Qualification](#qualification). No working integration or implementation approval is claimed.

Bazel and the existing [CLI commands](cli/commands/README.md) own analysis, execution,
configuration, scope, and failure policy. The integration consumes their normalized output
and [standard reports](cli/standard-reports.md); it does not introduce another analyzer,
dependency graph, report format, or CLI scheduler. CI never authorizes automatic fixes,
dependency updates, or bot commits.

## Consumer Setup

Provide a small consumer-owned caller workflow template that delegates execution and
reporting to a maintained, versioned reusable GitHub Actions workflow. Consumers must not
copy per-check execution, parsing, review, or comment-management logic. Updates use reviewed
version-pin changes, not silent upgrades, and preserve consumer customizations. Keep
substantive examples in `examples/` once executable APIs are available.

Reporting is first-party `rules_dx` code within the Actions integration, not reviewdog or
another reporting framework. Standard GitHub API clients may be reused. This ownership
includes API compatibility, untrusted-artifact handling, deduplication, and lifecycle tests;
it does not replace upstream analyzers or GitHub's scheduler and approval controls.
Consumers must not deploy a separate bot service or GitHub App. No generated setup command
or new `dx` command is selected; do not repurpose `dx setup` or `dx generate`.

Preserve canonical workspace configuration and module-matched CLI bootstrap. Document
prerequisite consumer Bazel setup, permissions, and required repository settings separately
from the caller file: a template cannot configure those settings by itself. The integration
does not manage consumer repository governance or prescribe CODEOWNERS/reviewer policies.
This repository's [Codecov selection](testing/README.md#github-coverage-reporting) does not
require consumers to use Codecov.

## Check Selection

All nine checks are enabled in the starter. Consumers may explicitly disable individual
checks without disabling unrelated checks or presenting disabled checks as passed.

| CI check | Existing command | Execution platforms |
| --- | --- | --- |
| Lint | `dx lint --check` | Linux once |
| Typecheck | `dx typecheck --check` | Linux once |
| Formatting consistency | `dx format --check` | Linux once |
| Generated BUILD consistency | `dx generate --check` | Linux once |
| Security audit | `dx audit security` | Linux once |
| License audit | `dx audit license` | Linux once |
| Tests | `dx test` | Every consumer-selected platform |
| Build | `dx build` | Every consumer-selected platform |
| Coverage | `dx coverage` | Every consumer-selected platform |

When tests, build, or coverage are selected, require an explicit nonempty supported platform
selection. There is no implicit Linux, current-runner, or all-platforms default, including
a silently active example value in the template. Missing, empty, or unsupported selections
produce actionable configuration failures, not skipped validation or platform substitution.
Linux execution for shared quality checks does not add Linux to the validation matrix.

Selection does not change language activation, analyzer applicability, configured no-op
behavior, or dormant-foundation laziness. Run selected checks at their normal repository
scope without workflow path filters or a second affected-target calculation. Documentation-only
changes do not skip CI; diff-based review placement does not narrow analysis scope. Linux
quality execution must retain declared source, configuration, and dependency scope, including
other-platform inputs, rather than silently omit them. Unsupported upstream routes block
qualification.

Coverage evidence must remain complete for every required selected platform/configuration;
combining reports must not hide a missing platform or coverage gap. Exact measurement and
aggregation mechanics remain qualification work, not a new reporting-layer coverage engine.
Consumer platform selection does not reduce this project's own
[coverage and release evidence obligations](testing/README.md#coverage).

## Execution

Independent selected checks run in parallel by default, subject to runner availability.
An explicit sequential mode changes scheduling only. In both modes, failures preserve
completed results, fail the overall run, and do not cancel independent checks. Failed
prerequisites block only actual dependents. Individual `dx` and Bazel fail-fast semantics
remain unchanged.

Use GitHub Actions scheduling. Isolate mutable checkouts/setup state, report destinations,
and Bazel execution contexts; do not advertise parallelism while jobs serialize on one
shared Bazel output-base lock. Reuse coverage-enabled test execution for test results only
where both existing command/report contracts demonstrably permit it.

Use GitHub's native rerun controls, not bot comment commands. Reruns preserve selection,
revision identity, and reporting semantics without substituting a different revision or
retrying analyzer failures until green. Bounded transient reporting-transport retries may
reuse the same identified results; exhausted retries retain reporting failure.

## Events And Revisions

| Run context | Validated revision | PR reporting |
| --- | --- | --- |
| Pull request, draft or ready | Identified proposed merge with its target branch | Review threads and updated summary |
| Push to default branch | Landed revision | No PR comment/thread changes |
| Manual dispatch without PR context | Identified dispatch revision | No PR comment/thread changes |
| Consumer-enabled merge queue | Identified proposed combined queue revision | No PR comment/thread changes |

The starter supports these triggers; ordinary non-default-branch pushes alone do not trigger
it. Do not assume the default branch is named `main`. Draft PRs get the same checks,
reporting, aggregate semantics, and fork approval as ready-for-review PRs. Non-PR runs retain
individual checks, detailed reports, and workflow summaries.

### PR Validation

Validate the proposed merge, not the contributor branch alone. All selected checks in a
run use the same test-merge snapshot, with its PR-head and target-branch revisions recorded.
Map findings back to valid PR-diff source locations; unmappable findings stay in reports and
summary counts, never on invented lines. A tested snapshot is not evidence that a later
head/base combination was validated.

If conflicts prevent test-merge creation, report validation as blocked without head-only
fallback or aggregate success. GitHub can suppress `pull_request` workflows for conflicting
PRs; the blocked-status path must be qualified without executing fork code with reporting
privileges.

### Supersession And Queues

New PR commits automatically cancel superseded integration runs for that PR in either
scheduling mode. This differs from an ordinary check failure, which does not cancel
independent work. Retain prior run history under normal retention; unfinished or cancelled
checks are not passed. Late callbacks must not overwrite current summaries or modify current
threads, even when cancellation races completion.

Scope PR supersession to that PR's integration runs, not other PRs, unrelated workflows,
default-branch runs, unrelated manual runs, or merge-queue runs. Queue checks use the same
selected checks and stable aggregate identity, bound to the queue revision rather than an
individual PR head. Queue runs never create, update, or clean up PR comments/threads, and
obsolete queue results cannot satisfy a newer queue revision. Queue membership does not
grant fork code secrets or write credentials. Consumers enable and configure their queue.

## Reporting

### Presentation

Use one supported presentation: individual GitHub checks, replyable/resolvable PR review
threads for diff-mapped findings, and one integration-owned updated summary comment per PR.
There is no review-comment opt-out or annotation-only mode. Per-check opt-outs select
execution, not presentation. Avoid duplicate inline check annotations for findings already
shown in review threads. Findings outside the diff or without valid locations stay in full
reports and summary counts without changing command outcomes.

Keep the summary compact: every selected check's status, finding counts, and links to
detailed results, not an exhaustive finding list. Identify the analyzed revision/run and
platforms, failures, stale results, and completeness. Disabled, blocked, skipped, cancelled,
or incomplete results are not passed. Preserve configured no-op semantics; an expected no-op
without a report event is not a collection failure. Retries and concurrent completions must
neither duplicate the summary nor let older results replace newer ones.

### Thread Lifecycle And Limits

Preserve human comments, replies, and unrelated threads. Deduplicate integration-owned
findings across reruns and bind new comments to the analyzed revision. Keep a thread while
its finding remains, rather than deleting and reposting it. When a later complete assessment
of the relevant check and owning scope confirms a finding is gone, delete bot-only threads
without replies; resolve threads with human replies instead, preserving the discussion.

Skipped, disabled, cancelled, or incomplete checks, missing reports, and locations moving
outside the diff do not prove a finding is gone. A complete check may clear one finding
while failing on another. Prior workflow results retain normal retention, not a separate
lint-history store.

Apply one fixed, documented per-PR review-thread limit across checks and platforms, without
a consumer setting or fresh allowance per job/rerun. Prioritize failure-contributing findings
for new threads using deterministic ordering, not job completion order. Preserve existing
threads under the lifecycle policy; do not delete or resolve still-present findings to rotate
others into view. Full reports retain every finding. The summary distinguishes findings
omitted due to the limit from those without a valid diff location. Intentional truncation
is not incomplete analysis or publication failure and never changes CI outcomes. The numeric
limit and thread-accounting mechanics are not yet frozen.

### Audit And Code Scanning

Audit uses the same presentation, including diff-mapped review details, not a security-only
counts mode or disclosure toggle. Preserve severity, risk-acceptance status, completeness,
and failure policy. Repository visibility determines the audience; public CI reporting is
not confidential. It does not authorize exposing credentials, secret values, or privately
reported vulnerability material. Safe rendering must not assume all security findings are
private or claim an unqualified generic redaction guarantee.

SARIF publication to GitHub's Security > Code Scanning tab is optional and off by default.
Consumers opt in on eligible repositories with documented permissions. Default reporting
works without Code Scanning or paid GitHub security features. Preserve the
[SARIF completeness contract](cli/standard-reports.md#sarif-210): complete scans remain
eligible for authoritative upload even when findings fail the command; incomplete scans
must not replace authoritative scans. Disabled publication is not a reporting failure.

### Aggregate Status

Expose one stable aggregate CI check for consumers to require in branch protection, retaining
individual results. Its identity does not change with check selection or scheduling mode.
Success requires every selected check/platform cell to complete successfully under its
command contract and all required reporting to finish successfully. Preserve explicit
opt-outs and configured no-ops; missing, blocked, unexpectedly skipped, cancelled, or
incomplete selected results cannot produce success.

Required check, review-comment, or summary publication failure fails CI separately from
analyzer findings, preserving command outcomes. Passing analysis with failed reporting is
not full success. Intentionally inapplicable outputs, such as PR comments for queue runs,
are not failures; actual publication errors cannot be disguised as intentional truncation.

## Fork Security

Use GitHub's native `all_external_contributors` approval policy, not first-time-only
approval or a custom approval bot. Receiving-repository maintainers/collaborators with
GitHub's required permission approve gated runs; outside authors cannot self-approve.
Native trusted-user, subsequent-commit, and rerun semantics apply, not a promise of fresh
approval for every fork commit or trusted collaborator.

Approval permits execution but does not give fork code secrets or write credentials.
Privileged reporting stays separate and never executes fork-controlled code. Treat artifacts
and PR metadata as untrusted. Document and qualify the receiving-repository settings,
role mappings, event paths, and trusted reporting handoff.

## Merge Gating

Document how consumers require the single aggregate check with latest-target validation,
using GitHub's native up-to-date-branch requirement or a merge queue validating the current
combined revision. The integration does not modify branch protection, enable queues, or
add a bot that reruns all open PRs on target-branch pushes. Without a queue, contributors
may need to update branches and wait for CI again. Historical success remains truthful
for its tested snapshot but cannot satisfy the configured gate for an untested combination.

## Qualification

These are unresolved engineering and public-interface qualifications, not invitations to
reopen the accepted policies above. Resolve them before affected implementation:

- Freeze workflow inputs, caller/pin representation, compatibility, and release/update mechanics.
- Map supported platform identifiers, runner/OS/architecture identities, shared Linux quality
  scope, scheduling isolation, cache/resource use, sequential ordering, and coverage/test reuse.
- Qualify event/ref delivery, merge snapshots and diff mapping, conflicting-PR blocked reporting,
  queue lifecycle, base advancement, cancellation races, and aggregate required-check bindings.
- Freeze finding/thread identity, deterministic ordering, numeric limit and accounting, safe
  deletion/resolution with concurrent human replies, outdated locations, and GitHub API limits.
- Qualify fork roles/settings, untrusted artifact and metadata validation, privileged reporting,
  sensitive-content handling, bounded transport retries, and opt-in Code Scanning publication.

[M27](milestones/M27-consumer-ci.md) owns qualification and implementation delivery;
[M28](milestones/M28-stabilization-release-qualification.md) owns release qualification and
[M29](milestones/M29-release-publication.md) owns publication of qualified identities.
This assignment does not approve any milestone. Track unresolved work in the
[open-decision register](open-decisions.md#consumer-ci-qualification) and prove the contract
through the [consumer CI test matrix](testing/github-ci.md).
