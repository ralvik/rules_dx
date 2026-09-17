//! Pure consumer-CI check-selection planning (M27 WP1 slices 1-7, WP2 slices 8-9,
//! WP4 slices 10-11, WP6 slice 12).
//!
//! This crate owns the check-selection surface before any reusable
//! workflow, caller template, or reporter lands: the nine accepted CI
//! checks, starter defaults, explicit opt-outs, the explicit
//! platform-selection gate for per-platform checks, revision planning,
//! and scheduling/isolation planning. It plans over
//! injected argument strings only, so selection stays deterministic and
//! unit-testable without GitHub Actions, a Bazel server, or any runner.
//!
//! Frozen check table (`docs/github-ci.md#check-selection`): six checks run
//! once on Linux (`lint`, `typecheck`, `format`, `generate`,
//! `security-audit`, `license-audit`); three run on every
//! consumer-selected platform (`test`, `build`, `coverage`). The starter
//! enables all nine; callers disable individual checks by ID without
//! affecting unrelated checks, and disabled checks are omitted, never
//! reported as passed.
//!
//! Platform identities stay opaque here: when any per-platform check is
//! enabled, the caller must supply an explicit nonempty platform list,
//! and missing/empty selections fail closed. Runner/OS/arch mapping and
//! the supported-identity set arrive in later M27 slices; this crate
//! preserves spellings verbatim and never substitutes an implicit
//! Linux/current-runner/all-platforms default.
//!
//! Scheduling (`docs/github-ci.md#execution`): independent cells run in
//! parallel by default; explicit sequential mode changes overlap only.
//! Failures preserve completed results, fail the overall run, and never
//! cancel independent cells. Starter cells are independent (no
//! prerequisites), so a sibling failure blocks nothing. Every cell owns
//! isolated checkout, report-destination, and Bazel output-base identities;
//! no two cells share an output-base.
//!
//! Out of scope here (M27 qualification): workflow APIs/pins beyond the opaque
//! identities below, event/ref bindings beyond the revision-planning identities
//! below, and any YAML or reporter implementation. Those arrive in later M27
//! slices (workflow/reporter YAML plus GitHub execution stay blocked on
//! qualification). Audit rendering below plans only the
//! accepted presentation/safety boundary; numeric thread limits and frozen
//! runner mappings stay deferred. Untrusted artifact/metadata validation and
//! thread-accounting deltas below plan only the pure binding/accounting rules;
//! transport, API limits, and execution stay deferred. Preset onboarding below
//! plans only the stability discipline and runbook/update shape over opaque
//! labels; the frozen public load-label string, workflow/pin representation,
//! and `dx init` template emission stay deferred (M30).

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

pub mod revision;

pub use revision::{plan_revision, PlannedRevision, Reporting, RevisionError, RevisionRequest};

/// Frozen CI check identifiers, in canonical starter order.
pub const LINT_CHECK: &str = "lint";
/// Frozen CI check identifiers, in canonical starter order.
pub const TYPECHECK_CHECK: &str = "typecheck";
/// Frozen CI check identifiers, in canonical starter order.
pub const FORMAT_CHECK: &str = "format";
/// Frozen CI check identifiers, in canonical starter order.
pub const GENERATE_CHECK: &str = "generate";
/// Frozen CI check identifiers, in canonical starter order.
pub const SECURITY_AUDIT_CHECK: &str = "security-audit";
/// Frozen CI check identifiers, in canonical starter order.
pub const LICENSE_AUDIT_CHECK: &str = "license-audit";
/// Frozen CI check identifiers, in canonical starter order.
pub const TEST_CHECK: &str = "test";
/// Frozen CI check identifiers, in canonical starter order.
pub const BUILD_CHECK: &str = "build";
/// Frozen CI check identifiers, in canonical starter order.
pub const COVERAGE_CHECK: &str = "coverage";

/// Number of accepted CI checks in the starter.
pub const CHECK_COUNT: usize = 9;

/// Where a check executes: once on Linux, or on every selected platform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionScope {
    LinuxOnce,
    PerPlatform,
}

/// One accepted CI check: frozen ID, owning command, and execution scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Check {
    /// Frozen caller-visible identifier.
    pub id: &'static str,
    /// Owning `dx` command invocation (check/consistency mode).
    pub command: &'static str,
    /// Linux-once or every-selected-platform execution.
    pub scope: ExecutionScope,
}

/// Canonical starter order: all nine checks enabled.
pub const ALL_CHECKS: [Check; CHECK_COUNT] = [
    Check {
        id: LINT_CHECK,
        command: "dx lint --check",
        scope: ExecutionScope::LinuxOnce,
    },
    Check {
        id: TYPECHECK_CHECK,
        command: "dx typecheck --check",
        scope: ExecutionScope::LinuxOnce,
    },
    Check {
        id: FORMAT_CHECK,
        command: "dx format --check",
        scope: ExecutionScope::LinuxOnce,
    },
    Check {
        id: GENERATE_CHECK,
        command: "dx generate --check",
        scope: ExecutionScope::LinuxOnce,
    },
    Check {
        id: SECURITY_AUDIT_CHECK,
        command: "dx audit security",
        scope: ExecutionScope::LinuxOnce,
    },
    Check {
        id: LICENSE_AUDIT_CHECK,
        command: "dx audit license",
        scope: ExecutionScope::LinuxOnce,
    },
    Check {
        id: TEST_CHECK,
        command: "dx test",
        scope: ExecutionScope::PerPlatform,
    },
    Check {
        id: BUILD_CHECK,
        command: "dx build",
        scope: ExecutionScope::PerPlatform,
    },
    Check {
        id: COVERAGE_CHECK,
        command: "dx coverage",
        scope: ExecutionScope::PerPlatform,
    },
];

/// Look up one check by frozen ID.
pub fn find_check(id: &str) -> Option<Check> {
    ALL_CHECKS.iter().copied().find(|check| check.id == id)
}

/// Planned CI selection: enabled checks in canonical order plus the
/// verbatim platform list the caller supplied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CiSelection {
    /// Enabled checks in canonical starter order.
    pub enabled: Vec<Check>,
    /// Caller-supplied platform spellings, verbatim. Empty unless a
    /// per-platform check is enabled and platforms were provided.
    pub platforms: Vec<String>,
}

impl CiSelection {
    /// Enabled check IDs in canonical order.
    pub fn enabled_ids(&self) -> Vec<&'static str> {
        self.enabled.iter().map(|check| check.id).collect()
    }

    /// Whether any enabled check runs on every selected platform.
    pub fn requires_platforms(&self) -> bool {
        self.enabled
            .iter()
            .any(|check| check.scope == ExecutionScope::PerPlatform)
    }

    /// CI selection never mutates: checks run in consistency modes and
    /// never authorize automatic fixes, dependency updates, or bot
    /// commits.
    pub fn is_mutating() -> bool {
        false
    }
}

/// Malformed CI selection: unknown opt-outs or missing platform lists
/// fail closed instead of silently narrowing validation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SelectionError {
    /// Caller disabled an ID outside the frozen nine.
    #[error("unknown CI check {value:?}; want one of the nine starter checks")]
    UnknownCheck { value: String },
    /// A per-platform check is enabled but no platforms were supplied.
    #[error("explicit platform selection is required when test, build, or coverage is enabled")]
    MissingPlatforms,
}

/// Plan a CI selection from caller opt-outs and platform spellings.
///
/// `disabled` names checks to omit; every other starter check stays
/// enabled in canonical order. Unknown names fail closed. When any
/// enabled check needs per-platform execution, `platforms` must be
/// nonempty; spellings pass through verbatim for later qualification.
/// Linux-once-only selections accept an empty platform list.
pub fn plan_selection(
    disabled: &[String],
    platforms: &[String],
) -> Result<CiSelection, SelectionError> {
    for name in disabled {
        if find_check(name).is_none() {
            return Err(SelectionError::UnknownCheck {
                value: name.clone(),
            });
        }
    }
    let enabled: Vec<Check> = ALL_CHECKS
        .iter()
        .copied()
        .filter(|check| !disabled.iter().any(|name| name == check.id))
        .collect();
    let needs_platforms = enabled
        .iter()
        .any(|check| check.scope == ExecutionScope::PerPlatform);
    if needs_platforms && platforms.is_empty() {
        return Err(SelectionError::MissingPlatforms);
    }
    Ok(CiSelection {
        enabled,
        platforms: platforms.to_vec(),
    })
}

// ---------------------------------------------------------------------------
// Scheduling and isolation planning (M27 WP1 slice 3).
// ---------------------------------------------------------------------------

/// How independent check cells overlap.
///
/// Parallel is the default; sequential changes overlap only. Cell set,
/// isolation identities, failure preservation, and aggregate semantics are
/// identical in both modes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SchedulingMode {
    /// Independent cells may overlap, subject to runner availability.
    #[default]
    Parallel,
    /// Cells execute without overlap.
    Sequential,
}

/// One executable unit: a check on its execution scope.
///
/// Linux-once checks carry `platform: None`; per-platform checks carry the
/// verbatim caller-supplied platform spelling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionCell {
    /// Frozen check identifier.
    pub check: &'static str,
    /// Verbatim platform spelling, or `None` for Linux-once checks.
    pub platform: Option<String>,
    /// Whether this cell runs once on Linux or per selected platform.
    pub scope: ExecutionScope,
}

/// Isolated execution context identities for one cell.
///
/// Every cell owns distinct checkout, report-destination, and Bazel
/// output-base identities so parallel cells never mutate shared state or
/// serialize on one shared output-base lock. Identities derive
/// deterministically from the cell's check and platform; platform
/// spellings pass through verbatim (runner mapping and filesystem
/// sanitization arrive with the workflow slice).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellIsolation {
    /// Isolated checkout/setup identity.
    pub workdir: String,
    /// Isolated report-destination identity.
    pub report_path: String,
    /// Isolated Bazel output-base identity (never shared between cells).
    pub output_base: String,
}

/// One planned cell with its isolated context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedCell {
    /// The executable unit.
    pub cell: ExecutionCell,
    /// Its isolated execution context.
    pub isolation: CellIsolation,
}

/// Planned schedule: the fixed cell set plus its overlap mode.
///
/// The plan is fixed before execution: a cell failure never removes or
/// cancels independent cells, and completed results are always preserved.
/// Starter cells are independent (no prerequisites), so a failure blocks
/// no sibling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedSchedule {
    /// Every cell to execute, in canonical check/platform order.
    pub cells: Vec<PlannedCell>,
    /// Overlap mode (scheduling only).
    pub mode: SchedulingMode,
}

impl PlannedSchedule {
    /// Starter cells carry no prerequisites: every cell is runnable
    /// regardless of sibling outcomes.
    pub fn is_independent(&self) -> bool {
        true
    }
}

/// Per-cell outcome for aggregate planning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellOutcome {
    Success,
    Failure,
}

/// Overall run outcome: success requires every cell to succeed.
///
/// Failures preserve completed results and fail the run without cancelling
/// independent cells. An empty cell set succeeds vacuously (every selected
/// check — none — completed).
pub fn aggregate_outcome(outcomes: &[CellOutcome]) -> bool {
    outcomes
        .iter()
        .all(|outcome| *outcome == CellOutcome::Success)
}

fn isolation_for(check: &str, platform: Option<&str>) -> CellIsolation {
    let scope = platform.unwrap_or("linux-once");
    CellIsolation {
        workdir: format!("ci-workdir/{check}/{scope}"),
        report_path: format!("ci-reports/{check}/{scope}"),
        output_base: format!("ci-output-base/{check}/{scope}"),
    }
}

/// Plan the schedule for one selection.
///
/// Expands the selection into cells in canonical order (starter check
/// order; per-platform checks fan out in caller platform order) and
/// assigns each cell isolated context identities. Parallel and sequential
/// modes plan the same cells and isolation; only the overlap mode differs.
pub fn plan_schedule(selection: &CiSelection, mode: SchedulingMode) -> PlannedSchedule {
    let mut cells = Vec::new();
    for check in &selection.enabled {
        match check.scope {
            ExecutionScope::LinuxOnce => {
                let isolation = isolation_for(check.id, None);
                cells.push(PlannedCell {
                    cell: ExecutionCell {
                        check: check.id,
                        platform: None,
                        scope: ExecutionScope::LinuxOnce,
                    },
                    isolation,
                });
            }
            ExecutionScope::PerPlatform => {
                for platform in &selection.platforms {
                    let isolation = isolation_for(check.id, Some(platform));
                    cells.push(PlannedCell {
                        cell: ExecutionCell {
                            check: check.id,
                            platform: Some(platform.clone()),
                            scope: ExecutionScope::PerPlatform,
                        },
                        isolation,
                    });
                }
            }
        }
    }
    PlannedSchedule { cells, mode }
}

// ---------------------------------------------------------------------------
// Supersession and queue-revision planning (M27 WP1 slice 4).
// ---------------------------------------------------------------------------

/// Which run context a CI run belongs to.
///
/// Identities are opaque strings; sequencing uses the monotonically
/// increasing `seq` on [`TrackedRun`], never revision comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunScope {
    /// Integration runs for one pull request (draft or ready).
    PullRequest {
        /// Opaque PR identity (number/URL spelling preserved verbatim).
        pr: String,
    },
    /// Default-branch push run.
    DefaultBranch,
    /// Manual dispatch run without PR context.
    Manual {
        /// Opaque dispatch identity.
        id: String,
    },
    /// Merge-queue run bound to one combined queue revision.
    Queue {
        /// Opaque queue identity.
        id: String,
    },
}

/// One tracked run: its scope, the snapshot it validates, and its order.
///
/// `seq` orders runs within a scope (higher supersedes lower); `validated`
/// is the snapshot from [`PlannedRevision`] every selected check uses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackedRun {
    /// Run context.
    pub scope: RunScope,
    /// Snapshot every selected check analyzes.
    pub validated: String,
    /// Monotonic order within the scope (higher is newer).
    pub seq: u64,
}

/// Whether `incoming` supersedes `current` (the old run must cancel).
///
/// Only a newer run for the same PR's integration scope supersedes: same
/// opaque `pr` identity and strictly greater `seq`. This holds in both
/// scheduling modes. A check failure without a new commit never
/// supersedes — independent cells still complete (see [`aggregate_outcome`]).
/// Supersession never reaches across PRs, default-branch runs, unrelated
/// manual runs, or queue runs.
pub fn supersedes(current: &TrackedRun, incoming: &TrackedRun) -> bool {
    match (&current.scope, &incoming.scope) {
        (RunScope::PullRequest { pr: current_pr }, RunScope::PullRequest { pr: incoming_pr }) => {
            current_pr == incoming_pr && incoming.seq > current.seq
        }
        _ => false,
    }
}

/// Whether a finished run may write the live summary/threads.
///
/// Only the current run writes: a late callback from a superseded run
/// (lower `seq`, or a different `validated` snapshot for the same scope)
/// must not overwrite the current summary or modify current threads.
/// Cancelled or unfinished runs never report success.
pub fn may_publish(finished: &TrackedRun, current: &TrackedRun) -> bool {
    finished.scope == current.scope
        && finished.seq == current.seq
        && finished.validated == current.validated
}

/// Whether a stored result satisfies a required revision.
///
/// Results bind to the exact tested snapshot: an earlier merge/queue
/// snapshot never validates a later head/base or queue combination, and a
/// stale queue result never satisfies a newer queue revision. PR
/// supersession never cancels queue runs and queue replacement never
/// cancels PR runs (see [`supersedes`]).
pub fn result_satisfies(stored: &TrackedRun, required_validated: &str) -> bool {
    stored.validated == required_validated
}

// ---------------------------------------------------------------------------
// Reporting and review-thread planning (M27 WP1 slice 5).
// ---------------------------------------------------------------------------

/// One diff-mappable finding with a stable identity.
///
/// `location` is `Some` only for findings with a valid PR-diff location;
/// locationless findings stay in full reports and summary counts and never
/// receive invented review locations or inline annotations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    /// Stable finding identity across reruns (check + rule + path digest).
    pub id: String,
    /// Whether this finding contributes to its check's failure.
    pub contributes_to_failure: bool,
    /// Valid PR-diff location, if mappable.
    pub location: Option<String>,
}

/// One integration-owned review thread.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedThread {
    /// Finding this thread reports.
    pub finding: String,
    /// Whether a human has replied (replies pin resolve-over-delete).
    pub has_human_replies: bool,
}

/// Planned thread updates for one PR assessment.
///
/// Presentation is fixed: review threads for diff-mapped findings plus one
/// integration-owned summary; there is no review-comment opt-out or
/// annotation-only mode, and findings already shown in threads never gain
/// duplicate inline annotations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadPlan {
    /// Threads to keep as-is (finding still present, no repost).
    pub keep: Vec<String>,
    /// Finding IDs to open new threads for (deterministic priority order).
    pub create: Vec<String>,
    /// Bot-only threads without replies whose finding is confirmed gone.
    pub delete: Vec<String>,
    /// Threads with human replies whose finding is confirmed gone.
    pub resolve: Vec<String>,
    /// Diff-mapped findings omitted only by the fixed per-PR limit.
    pub omitted_due_to_limit: Vec<String>,
    /// Findings without a valid diff location (reports/counts only).
    pub unmappable: Vec<String>,
}

/// Plan review-thread updates.
///
/// `findings` are the current complete assessment's findings;
/// `existing` are the integration-owned threads; `confirmed_gone` are
/// finding IDs a later complete assessment of the relevant check and scope
/// confirmed absent (skipped, disabled, cancelled, or incomplete checks,
/// missing reports, and locations moving outside the diff never confirm
/// absence — callers must not list them here). `limit` is the fixed
/// per-PR thread cap (numeric value frozen elsewhere): failure-contributing
/// findings win new threads in deterministic ID order, existing threads are
/// never deleted or resolved to rotate others into view, and full reports
/// retain every finding. Intentional truncation is not a failure.
pub fn plan_threads(
    findings: &[Finding],
    existing: &[OwnedThread],
    confirmed_gone: &[String],
    limit: usize,
) -> ThreadPlan {
    let present: std::collections::BTreeSet<&str> =
        findings.iter().map(|finding| finding.id.as_str()).collect();
    let existing_ids: std::collections::BTreeSet<&str> = existing
        .iter()
        .map(|thread| thread.finding.as_str())
        .collect();
    let mut keep = Vec::new();
    for thread in existing {
        if present.contains(thread.finding.as_str()) {
            keep.push(thread.finding.clone());
        }
    }
    let mut delete = Vec::new();
    let mut resolve = Vec::new();
    for thread in existing {
        if present.contains(thread.finding.as_str()) {
            continue;
        }
        if !confirmed_gone.iter().any(|id| id == &thread.finding) {
            continue;
        }
        if thread.has_human_replies {
            resolve.push(thread.finding.clone());
        } else {
            delete.push(thread.finding.clone());
        }
    }
    keep.sort();
    delete.sort();
    resolve.sort();
    let mut candidates: Vec<&Finding> = findings
        .iter()
        .filter(|finding| finding.location.is_some())
        .filter(|finding| !existing_ids.contains(finding.id.as_str()))
        .collect();
    candidates.sort_by(|a, b| {
        b.contributes_to_failure
            .cmp(&a.contributes_to_failure)
            .then_with(|| a.id.cmp(&b.id))
    });
    let used = keep.len();
    let budget = limit.saturating_sub(used.min(limit));
    let mut create = Vec::new();
    let mut omitted_due_to_limit = Vec::new();
    for finding in candidates {
        if create.len() < budget {
            create.push(finding.id.clone());
        } else {
            omitted_due_to_limit.push(finding.id.clone());
        }
    }
    let mut unmappable: Vec<String> = findings
        .iter()
        .filter(|finding| finding.location.is_none())
        .map(|finding| finding.id.clone())
        .collect();
    unmappable.sort();
    ThreadPlan {
        keep,
        create,
        delete,
        resolve,
        omitted_due_to_limit,
        unmappable,
    }
}

/// Compact per-PR summary (one integration-owned updated comment).
///
/// Carries every selected check's status, truthful finding counts, the
/// analyzed revision/run identity, and completeness — never the exhaustive
/// finding list. Disabled, blocked, skipped, cancelled, or incomplete
/// results are never shown as passed. Retries and concurrent completions
/// must neither duplicate the summary nor let older results replace newer
/// ones (see [`may_publish`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Summary {
    /// Analyzed snapshot identity.
    pub validated: String,
    /// Total findings retained in full reports.
    pub finding_count: usize,
    /// Findings omitted only by the fixed per-PR thread limit.
    pub limit_omitted: usize,
    /// Findings without a valid diff location.
    pub unmappable: usize,
}

/// Build the compact summary counts.
///
/// `full_findings` is every finding in the detailed reports; limit-omitted
/// and unmappable counts come from [`plan_threads`].
pub fn plan_summary(validated: &str, full_findings: usize, plan: &ThreadPlan) -> Summary {
    Summary {
        validated: validated.to_owned(),
        finding_count: full_findings,
        limit_omitted: plan.omitted_due_to_limit.len(),
        unmappable: plan.unmappable.len(),
    }
}

/// Reporting gate: required publication failure fails CI separately.
///
/// Passing analysis with failed required check, review-comment, or summary
/// publication is not success; command outcomes are preserved alongside the
/// reporting failure. Intentionally inapplicable outputs (for example PR
/// comments for queue runs) are not failures — callers pass
/// `reporting_required: false` there.
pub fn reporting_gate(
    analysis_passed: bool,
    reporting_required: bool,
    reporting_succeeded: bool,
) -> bool {
    if reporting_required && !reporting_succeeded {
        return false;
    }
    analysis_passed
}

// ---------------------------------------------------------------------------
// Fork-security and aggregate-gating planning (M27 WP1 slice 6).
// ---------------------------------------------------------------------------

/// Stable aggregate CI check identity for branch protection.
///
/// The identity never changes with check selection or scheduling mode;
/// callers require this one check while individual results stay visible.
pub const AGGREGATE_CHECK: &str = "dx-ci";

/// Approver role for a fork-run approval request.
///
/// GitHub's native `all_external_contributors` policy owns trusted-user,
/// subsequent-commit, and rerun semantics; this crate plans only the
/// approval boundary: outside authors cannot self-approve, and only
/// receiving-repository maintainers/collaborators with the required
/// permission approve gated runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApproverRole {
    Maintainer,
    Collaborator,
    OutsideAuthor,
}

/// Whether an approved fork run executes.
///
/// Approval permits execution but never grants fork code secrets or write
/// credentials; privileged reporting stays separate and never executes
/// fork-controlled code. Artifacts and PR metadata are untrusted inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForkCredentials {
    /// No secrets, no write credentials (the only fork grant).
    ReadOnly,
}

/// Plan fork-run approval.
///
/// Non-external runs need no approval. External-contributor runs require an
/// approver who is a maintainer or collaborator and is not the author;
/// outside-author self-approval is denied. First-time-only approval is not
/// modeled: the native policy (not a custom bot) owns those semantics.
pub fn plan_approval(
    is_external_contributor: bool,
    approver: ApproverRole,
    approver_is_author: bool,
) -> bool {
    if !is_external_contributor {
        return true;
    }
    if approver_is_author {
        return false;
    }
    matches!(
        approver,
        ApproverRole::Maintainer | ApproverRole::Collaborator
    )
}

/// Credentials granted to approved fork execution: always read-only.
pub fn fork_credentials() -> ForkCredentials {
    ForkCredentials::ReadOnly
}

/// Privileged reporting never executes fork-controlled code.
pub fn privileged_reporting_executes_fork_code() -> bool {
    false
}

/// Per-cell state for aggregate gating.
///
/// Only [`AggregateState::Success`] satisfies the gate. Configured no-ops
/// and explicit opt-outs are omitted by selection before aggregation, so
/// they never appear here as passed; missing, blocked, unexpectedly
/// skipped, cancelled, or incomplete selected results never produce
/// success.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AggregateState {
    Success,
    Failure,
    Blocked,
    Skipped,
    Cancelled,
    Incomplete,
    Missing,
}

/// Plan the stable aggregate status.
///
/// Success requires every selected check/platform cell to report success
/// under its command contract and all required reporting to finish
/// successfully (see [`reporting_gate`]). The identity ([`AGGREGATE_CHECK`])
/// is independent of selection and scheduling mode.
pub fn plan_aggregate(
    states: &[AggregateState],
    reporting_required: bool,
    reporting_succeeded: bool,
) -> bool {
    if reporting_required && !reporting_succeeded {
        return false;
    }
    !states.is_empty() && states.iter().all(|state| *state == AggregateState::Success)
}

// ---------------------------------------------------------------------------
// Rerun, scope, retry, and code-scanning planning (M27 WP1 slice 7).
// ---------------------------------------------------------------------------

/// Planned rerun: GitHub's native rerun controls preserve selection,
/// revision identity, and reporting semantics.
///
/// A rerun reuses the original planned revision (same validated snapshot
/// plus head/base identities) and the original reporting mode. Reruns never
/// substitute a different revision, never retry analyzer failures until
/// green, and never accept bot comment commands.
pub fn plan_rerun(original: &PlannedRevision) -> PlannedRevision {
    original.clone()
}

/// Whether a change kind still executes every selected check.
///
/// Documentation-only and mixed changes invoke all selected checks at their
/// normal repository scope: no workflow path filters and no second
/// affected-target calculation. Diff-based review placement never narrows
/// analysis scope.
pub fn scope_runs_all_selected(docs_only: bool) -> bool {
    let _ = docs_only;
    true
}

/// Workflow path filters are never used.
pub fn uses_path_filters() -> bool {
    false
}

/// Planned outcome of bounded transient reporting-transport retries.
///
/// Retries reuse the same identified results and never duplicate comments,
/// change analyzer outcomes, or hide the final reporting failure. Only
/// `transient_failures <= bound` are retried; exhaustion retains the
/// reporting failure for the gate in [`reporting_gate`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReportRetry {
    /// Whether the transport was retried with the same identified results.
    pub retried_same_results: bool,
    /// Whether the final reporting failure is retained (exhaustion).
    pub failure_retained: bool,
    /// Retries never duplicate integration comments.
    pub duplicates_comments: bool,
    /// Retries never change analyzer outcomes.
    pub changes_analysis: bool,
}

/// Plan bounded transient reporting retries.
pub fn plan_report_retry(transient_failures: u32, bound: u32) -> ReportRetry {
    if transient_failures == 0 {
        ReportRetry {
            retried_same_results: false,
            failure_retained: false,
            duplicates_comments: false,
            changes_analysis: false,
        }
    } else if transient_failures <= bound {
        ReportRetry {
            retried_same_results: true,
            failure_retained: false,
            duplicates_comments: false,
            changes_analysis: false,
        }
    } else {
        ReportRetry {
            retried_same_results: true,
            failure_retained: true,
            duplicates_comments: false,
            changes_analysis: false,
        }
    }
}

/// Code Scanning SARIF publication is off in the starter.
pub const CODE_SCANNING_DEFAULT: bool = false;

/// Code-scanning publication planning for an explicit opt-in.
///
/// Default reporting works without Code Scanning permissions or paid
/// security features, and disabled publication is never a reporting
/// failure. After opt-in on an eligible repository, complete scans remain
/// eligible for authoritative upload even when findings fail the command;
/// incomplete scans must not replace the authoritative scan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeScanningPlan {
    /// Publication disabled: not a failure.
    Disabled,
    /// Complete scan may upload authoritatively.
    Upload,
    /// Incomplete scan must not replace the authoritative scan.
    MustNotUpload,
}

/// Plan Code Scanning publication.
pub fn plan_code_scanning(opt_in: bool, complete_scan: bool) -> CodeScanningPlan {
    if !opt_in {
        return CodeScanningPlan::Disabled;
    }
    if complete_scan {
        CodeScanningPlan::Upload
    } else {
        CodeScanningPlan::MustNotUpload
    }
}

/// Coverage aggregation never hides a missing platform or gap.
///
/// Every selected platform must report its coverage cell; combining reports
/// with any platform missing or failing is not success.
pub fn plan_coverage_aggregate(per_platform_ok: &[bool]) -> bool {
    !per_platform_ok.is_empty() && per_platform_ok.iter().all(|ok| *ok)
}

// ---------------------------------------------------------------------------
// Starter triggers, platform qualification mechanics, no-op semantics, and
// latest-target revalidation (M27 WP2 slice 8).
// ---------------------------------------------------------------------------

/// Whether the starter triggers on this revision request.
///
/// PRs (draft or ready), default-branch pushes, manual dispatches, and
/// consumer-enabled merge-queue runs trigger; ordinary non-default-branch
/// pushes alone do not (`docs/testing/github-ci.md#starter-triggers`).
/// Branch names stay opaque: a consumer whose default branch is not
/// `"main"` triggers identically on its own default branch.
pub fn starter_triggers(request: &RevisionRequest<'_>) -> bool {
    match request {
        RevisionRequest::PullRequest { .. }
        | RevisionRequest::PushDefault { .. }
        | RevisionRequest::ManualDispatch { .. }
        | RevisionRequest::MergeQueue { .. } => true,
        RevisionRequest::PushNonDefault { .. } => false,
    }
}

/// Malformed platform selection against an injected supported set.
///
/// The supported identities stay injected here: the frozen runner/platform
/// mapping arrives with workflow qualification (`docs/github-ci.md#qualification`).
/// This plans only the mechanics — explicit nonempty selection, no implicit
/// default or substitution, verbatim spellings — over caller-supplied sets.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PlatformError {
    /// No platforms supplied while a per-platform check is enabled.
    #[error("explicit platform selection is required when test, build, or coverage is enabled")]
    Empty,
    /// A supplied spelling is outside the injected supported set.
    #[error("unsupported platform {value:?}")]
    Unsupported { value: String },
}

/// Validate an explicit platform selection against an injected supported set.
///
/// Fails closed on missing/empty selections and on any unsupported spelling
/// (no skipped validation or platform substitution). Single- and
/// multi-platform selections pass through verbatim in caller order,
/// including selections without Linux. Linux-once quality scope is planned
/// by [`plan_schedule`], not by adding Linux here.
pub fn validate_platforms(
    platforms: &[String],
    supported: &[String],
) -> Result<Vec<String>, PlatformError> {
    if platforms.is_empty() {
        return Err(PlatformError::Empty);
    }
    for platform in platforms {
        if !supported.iter().any(|known| known == platform) {
            return Err(PlatformError::Unsupported {
                value: platform.clone(),
            });
        }
    }
    Ok(platforms.to_vec())
}

/// Whether a missing report event is a collection failure.
///
/// A configured no-op without a report event is expected, not a collection
/// failure; any other missing report is. Failures and counts flow through
/// [`reporting_gate`] and [`plan_summary`] unchanged.
pub fn is_collection_failure(has_report_event: bool, configured_noop: bool) -> bool {
    !has_report_event && !configured_noop
}

/// Whether target-branch advancement requires a fresh validation run.
///
/// The native up-to-date-branch gate (or merge queue validating the current
/// combined revision) owns enforcement; this plans only the identity rule:
/// any base change invalidates the earlier merge snapshot, which stays
/// truthful for its tested combination but never satisfies the new one
/// (see [`result_satisfies`]). Revisions stay opaque strings.
pub fn base_advanced_requires_rerun(old_base: &str, new_base: &str) -> bool {
    old_base != new_base
}

// ---------------------------------------------------------------------------
// Caller-template composition and reviewed pin updates (M27 WP2 slice 9).
// ---------------------------------------------------------------------------

/// Caller-owned inputs: the only surface the consumer-owned caller template
/// configures (`docs/github-ci.md#consumer-setup`, `docs/testing/github-ci.md#consumer-setup`).
///
/// Everything else — per-check execution, output parsing, review threads,
/// and comment management — stays in the maintained versioned reusable
/// workflow and its first-party reporting; callers must not copy that logic.
pub const CALLER_OWNED_INPUTS: [&str; 4] = [
    "disabled_checks",
    "platforms",
    "scheduling_mode",
    "code_scanning_opt_in",
];

/// Workflow-owned responsibilities the caller template must not duplicate.
pub const WORKFLOW_OWNED: [&str; 4] = [
    "execution",
    "parsing",
    "review_threads",
    "comment_management",
];

/// No generated setup command and no new `dx` command back the caller
/// template; do not repurpose `dx setup` or `dx generate`.
pub fn uses_generated_setup_command() -> bool {
    false
}

/// Planned caller configuration: the composed selection plus the overlap
/// mode and Code Scanning opt-in the caller chose.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedCaller {
    /// Enabled checks plus verbatim platform spellings.
    pub selection: CiSelection,
    /// Overlap mode (scheduling only; cells and isolation are fixed).
    pub mode: SchedulingMode,
    /// Explicit Code Scanning opt-in (starter default is off).
    pub code_scanning_opt_in: bool,
}

/// Malformed caller configuration: unknown opt-outs or platform-gate
/// failures fail closed instead of silently narrowing validation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CallerError {
    /// Unknown check ID or missing platform list (see [`SelectionError`]).
    #[error(transparent)]
    Selection(#[from] SelectionError),
    /// Empty or unsupported platform selection (see [`PlatformError`]).
    #[error(transparent)]
    Platform(#[from] PlatformError),
}

/// Plan one caller configuration from its owned inputs.
///
/// Composes [`plan_selection`] with [`validate_platforms`] over the injected
/// supported set: unknown opt-outs fail, per-platform selections must be
/// explicit nonempty supported spellings passed through verbatim, and
/// Linux-once-only selections need no platforms. `supported` is injected —
/// the frozen runner/platform mapping arrives with workflow qualification
/// (`docs/github-ci.md#qualification`). Mode and opt-in pass through
/// unchanged; non-mutating consistency modes are fixed by [`CiSelection`].
pub fn plan_caller(
    disabled: &[String],
    platforms: &[String],
    supported: &[String],
    mode: SchedulingMode,
    code_scanning_opt_in: bool,
) -> Result<PlannedCaller, CallerError> {
    let selection = plan_selection(disabled, platforms).map_err(CallerError::Selection)?;
    if selection.requires_platforms() {
        let validated =
            validate_platforms(&selection.platforms, supported).map_err(CallerError::Platform)?;
        Ok(PlannedCaller {
            selection: CiSelection {
                enabled: selection.enabled,
                platforms: validated,
            },
            mode,
            code_scanning_opt_in,
        })
    } else {
        Ok(PlannedCaller {
            selection,
            mode,
            code_scanning_opt_in,
        })
    }
}

/// Planned pin change: pins stay opaque strings here — workflow/pin
/// representation, compatibility, and release mechanics freeze with workflow
/// qualification, not in this crate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PinUpdate {
    /// Pin unchanged: idempotent, no review needed.
    NoChange,
    /// Explicit reviewed bump from one opaque pin to another.
    ReviewedUpdate {
        /// Previously pinned opaque identity.
        from: String,
        /// Newly pinned opaque identity.
        to: String,
    },
}

/// Malformed pin change: missing pins or silent (unreviewed) upgrades fail
/// closed — updates use reviewed version-pin changes, never silent upgrades.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PinError {
    /// Either pin identity is missing or empty (no implicit latest/default).
    #[error("explicit workflow pin identities are required")]
    MissingPin,
    /// Pin change without review.
    #[error("workflow pin changes require review, not silent upgrades")]
    UnreviewedChange,
}

/// Plan a workflow pin change over opaque pin identities.
///
/// Same pin is [`PinUpdate::NoChange`]; a change requires `reviewed` and
/// yields [`PinUpdate::ReviewedUpdate`], otherwise [`PinError::UnreviewedChange`].
/// Empty identities fail with [`PinError::MissingPin`]: there is no implicit
/// latest or silently active example value.
pub fn plan_pin_update(
    old_pin: &str,
    new_pin: &str,
    reviewed: bool,
) -> Result<PinUpdate, PinError> {
    if old_pin.is_empty() || new_pin.is_empty() {
        return Err(PinError::MissingPin);
    }
    if old_pin == new_pin {
        return Ok(PinUpdate::NoChange);
    }
    if !reviewed {
        return Err(PinError::UnreviewedChange);
    }
    Ok(PinUpdate::ReviewedUpdate {
        from: old_pin.to_owned(),
        to: new_pin.to_owned(),
    })
}

/// Apply a planned pin change to a caller without touching consumer
/// customizations: pin bumps never alter the caller-owned selection, mode,
/// or opt-in.
pub fn apply_pin_update(caller: &PlannedCaller, update: &PinUpdate) -> PlannedCaller {
    let _ = update;
    caller.clone()
}

// ---------------------------------------------------------------------------
// Audit rendering planning (M27 WP4 slice 10).
// ---------------------------------------------------------------------------

/// One audit finding with preserved presentation inputs.
///
/// `severity` and `acceptance` are opaque verbatim strings: this crate
/// preserves them into reporting without interpreting scales or acceptance
/// vocabularies (those freeze with analyzer qualification, not here).
/// `location` is `Some` only for findings with a valid PR-diff location.
/// `contains_restricted_content` marks bodies that must not be published
/// verbatim (credential values, secret values, or privately reported
/// vulnerability material, as classified by the caller-supplied input —
/// this crate claims no generic redaction).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditFinding {
    /// Stable finding identity.
    pub id: String,
    /// Opaque severity spelling, preserved verbatim.
    pub severity: String,
    /// Opaque risk-acceptance spelling, preserved verbatim.
    pub acceptance: String,
    /// Whether this finding contributes to its check's failure.
    pub contributes_to_failure: bool,
    /// Valid PR-diff location, if mappable.
    pub location: Option<String>,
    /// Whether the finding body must not be published verbatim.
    pub contains_restricted_content: bool,
}

/// Where one audit finding is presented.
///
/// Audit uses the same presentation as every other check: diff-mapped
/// findings receive review details; all other findings stay in full reports
/// and summary counts. There is no security-only counts mode and no
/// disclosure toggle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditPlacement {
    /// Diff-mapped finding: review details plus report/count retention.
    ReviewThread,
    /// Finding without a valid diff location: reports and counts only.
    ReportOnly,
}

/// Audit never uses a security-only counts mode.
pub fn audit_uses_counts_only_mode() -> bool {
    false
}

/// Audit has no disclosure toggle.
pub fn audit_has_disclosure_toggle() -> bool {
    false
}

/// Safe rendering never assumes all security findings are private: public
/// repository reporting is not a confidential channel.
pub fn audit_assumes_private() -> bool {
    false
}

/// This crate claims no unqualified generic redaction guarantee.
pub fn audit_claims_generic_redaction() -> bool {
    false
}

/// Plan where one audit finding is presented.
///
/// A valid diff location plans [`AuditPlacement::ReviewThread`]; anything
/// else plans [`AuditPlacement::ReportOnly`] with no invented location.
/// Restricted bodies keep their placement and counts — only the verbatim
/// body is withheld (see [`audit_body_publishable`]) — so completeness and
/// command outcomes are preserved.
pub fn plan_audit_placement(finding: &AuditFinding) -> AuditPlacement {
    if finding.location.is_some() {
        AuditPlacement::ReviewThread
    } else {
        AuditPlacement::ReportOnly
    }
}

/// Whether the finding body may be published verbatim into review details,
/// reports, or summaries.
///
/// Findings flagged with `contains_restricted_content` must not expose
/// credential values, secret values, or privately reported vulnerability
/// material: callers withhold the verbatim body while retaining the finding
/// in counts, completeness, and command outcomes. Unflagged findings are
/// publishable as-is; public reporting is otherwise not confidential.
pub fn audit_body_publishable(finding: &AuditFinding) -> bool {
    !finding.contains_restricted_content
}

// ---------------------------------------------------------------------------
// Untrusted-artifact/metadata validation and thread-accounting deltas
// (M27 WP4 slice 11).
// ---------------------------------------------------------------------------

/// Malformed untrusted artifact: artifacts and PR metadata are untrusted
/// inputs (`docs/github-ci.md#fork-security`) and privileged reporting must
/// validate them before use, never trusting by presence.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ArtifactError {
    /// Artifact snapshot identity missing or empty.
    #[error("artifact snapshot identity is required")]
    MissingArtifact,
    /// Artifact digest identity missing or empty.
    #[error("artifact digest identity is required")]
    MissingDigest,
    /// Artifact snapshot does not bind to the required validated snapshot.
    #[error("artifact does not bind to the required validated snapshot")]
    SnapshotMismatch,
}

/// Validate one untrusted artifact against the required validated snapshot.
///
/// `artifact_validated` is the snapshot the artifact claims; `required` is
/// the [`PlannedRevision::validated`] snapshot every selected check used;
/// `digest` is the opaque artifact identity. Empty identities fail closed and
/// a non-matching snapshot fails with [`ArtifactError::SnapshotMismatch`]:
/// stale or foreign artifacts never satisfy the current run, and presence
/// alone never establishes trust. Digest algorithms and transport stay
/// deferred; this plans only the exact-binding rule.
pub fn validate_artifact_snapshot(
    artifact_validated: &str,
    required: &str,
    digest: &str,
) -> Result<(), ArtifactError> {
    if artifact_validated.is_empty() {
        return Err(ArtifactError::MissingArtifact);
    }
    if digest.is_empty() {
        return Err(ArtifactError::MissingDigest);
    }
    if required.is_empty() || artifact_validated != required {
        return Err(ArtifactError::SnapshotMismatch);
    }
    Ok(())
}

/// Malformed untrusted PR metadata: privileged reporting must validate
/// metadata against the planned revision before creating, updating, or
/// cleaning up review threads or the summary.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum MetadataError {
    /// A metadata identity is missing or empty.
    #[error("PR metadata identities are required")]
    MissingField,
    /// Metadata does not match the planned revision (stale or foreign run).
    #[error("PR metadata does not match the planned revision snapshot")]
    StaleSnapshot,
}

/// Validate untrusted PR metadata against the planned revision.
///
/// `head`/`base`/`validated` are the opaque identities carried by the
/// metadata event; `planned` is the [`PlannedRevision`] the run validated.
/// Empty identities fail with [`MetadataError::MissingField`]; any mismatch
/// with the planned `(validated, head, base)` fails with
/// [`MetadataError::StaleSnapshot`] so stale reporting can neither create nor
/// modify current threads (see [`may_publish`]). Non-PR runs carry no PR
/// metadata: callers must not invent head/base there.
pub fn validate_pr_metadata(
    head: &str,
    base: &str,
    validated: &str,
    planned: &PlannedRevision,
) -> Result<(), MetadataError> {
    if head.is_empty() || base.is_empty() || validated.is_empty() {
        return Err(MetadataError::MissingField);
    }
    let planned_head = planned.head.as_deref().unwrap_or("");
    let planned_base = planned.base.as_deref().unwrap_or("");
    if validated != planned.validated || head != planned_head || base != planned_base {
        return Err(MetadataError::StaleSnapshot);
    }
    Ok(())
}

/// Untrusted inputs never authorize fork-code execution in privileged
/// reporting.
pub fn untrusted_inputs_execute_fork_code() -> bool {
    false
}

/// Untrusted inputs never grant secrets or write credentials to fork code.
pub fn untrusted_inputs_grant_secrets() -> bool {
    false
}

/// Slots occupied against the fixed per-PR review-thread limit.
///
/// Only still-present findings with retained threads ([`ThreadPlan::keep`])
/// occupy slots. Bot-only threads queued for deletion and replied threads
/// queued for resolution are confirmed gone, so they free their slots for new
/// findings; retained resolved discussions do not block new threads. Limit
/// accounting is per PR across checks and platforms with no fresh allowance
/// per job, rerun, retry, or concurrent completion (see [`may_publish`]: only
/// the current run publishes).
pub fn thread_slots_used(plan: &ThreadPlan) -> usize {
    plan.keep.len()
}

/// Retained resolved discussions never count against the new-thread budget.
pub fn resolved_discussions_count_against_limit() -> bool {
    false
}

/// Concurrent runs never receive a fresh per-run thread allowance.
pub fn concurrent_runs_share_limit() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Consumer `.bazelrc` preset onboarding planning (M27 WP6 slice 12).
// ---------------------------------------------------------------------------

/// Setup runbook steps in canonical order (`M27 WP6`): the consumer setup
/// path is the dependency snippet, the generation target, the `.bazelrc`
/// import block, the regen-and-review update loop, and the bot configuration
/// sample. `dx init` template emission stays M30 scope (see
/// [`dx_init_emits_preset_template`]).
pub const PRESET_RUNBOOK_STEPS: [&str; 5] = [
    "dependency_snippet",
    "generation_target",
    "import_block",
    "update_loop",
    "bot_config_sample",
];

/// Malformed preset load label: the public `extra_presets` load label stays
/// opaque here — the frozen public string freezes with workflow qualification
/// (`docs/github-ci.md#qualification`), not in this crate. This plans only the
/// shape: an explicit nonempty label preserved verbatim, never an implicit
/// default or silent substitution.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PresetLabelError {
    /// Label missing or empty (no implicit default).
    #[error("explicit preset load label is required")]
    MissingLabel,
}

/// Validate a consumer preset load label: nonempty opaque spellings pass
/// through verbatim; missing/empty labels fail closed.
pub fn validate_preset_label(label: &str) -> Result<String, PresetLabelError> {
    if label.is_empty() {
        return Err(PresetLabelError::MissingLabel);
    }
    Ok(label.to_owned())
}

/// Stability discipline (`M27 WP6`): preset-affecting changes ship only in
/// minor or major releases, never in a patch. Non-preset-affecting changes
/// carry no such restriction.
pub fn preset_affecting_requires_minor_or_major(preset_affecting: bool) -> bool {
    preset_affecting
}

/// Whether a release level may carry the change: patch releases accept only
/// non-preset-affecting changes.
pub fn preset_change_allowed_in_patch(preset_affecting: bool) -> bool {
    !preset_affecting
}

/// Preset-affecting changes are called out in the release notes; other
/// changes need no such callout.
pub fn preset_change_needs_release_note(preset_affecting: bool) -> bool {
    preset_affecting
}

/// `dx init` does not emit the preset caller template: template emission
/// stays M30 scope.
pub fn dx_init_emits_preset_template() -> bool {
    false
}

/// The preset regen-and-review loop stays manual with review enforcement
/// (O53): no auto-merge, whether bot-owned or human-driven.
pub fn preset_bot_auto_merge_allowed() -> bool {
    false
}

/// Regen without a reviewed flag diff is not an update.
pub fn preset_regen_requires_review() -> bool {
    true
}

/// Incomplete clean-external-consumer preset proof: generation, import, and
/// regen-and-review update must all be exercised through a clean external
/// consumer; repository dogfood alone is insufficient (the caller proves the
/// three legs, this crate only plans the gate).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PresetOnboardingError {
    /// Generation leg missing (no clean-consumer generation proof).
    #[error("clean-consumer preset generation proof is required")]
    MissingGeneration,
    /// Import leg missing (no clean-consumer import proof).
    #[error("clean-consumer preset import proof is required")]
    MissingImport,
    /// Regen leg missing or unreviewed (no reviewed flag-diff update proof).
    #[error("preset regen-and-review update requires a reviewed flag diff")]
    UnreviewedRegen,
}

/// Plan the clean-external-consumer preset proof.
///
/// All three legs must hold: `generation_ok` (the consumer generates from the
/// documented generation target), `import_ok` (the consumer imports the preset
/// fragment through the documented import block), and `regen_reviewed` (a
/// version-bump regen passed with a reviewed flag diff). Preset bumps never
/// alter consumer customizations — callers preserve the caller-owned
/// selection/mode/opt-in exactly as workflow pin bumps do (see
/// [`apply_pin_update`]).
pub fn plan_preset_onboarding(
    generation_ok: bool,
    import_ok: bool,
    regen_reviewed: bool,
) -> Result<(), PresetOnboardingError> {
    if !generation_ok {
        return Err(PresetOnboardingError::MissingGeneration);
    }
    if !import_ok {
        return Err(PresetOnboardingError::MissingImport);
    }
    if !regen_reviewed {
        return Err(PresetOnboardingError::UnreviewedRegen);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().copied().map(str::to_owned).collect()
    }

    #[test]
    fn starter_enables_all_nine_in_canonical_order() {
        let selection = plan_selection(&[], &strings(&["linux_x86_64"])).expect("starter plans");
        assert_eq!(selection.enabled.len(), CHECK_COUNT);
        assert_eq!(
            selection.enabled_ids(),
            vec![
                "lint",
                "typecheck",
                "format",
                "generate",
                "security-audit",
                "license-audit",
                "test",
                "build",
                "coverage",
            ]
        );
    }

    #[test]
    fn linux_once_checks_need_no_platforms() {
        let only_linux: Vec<Check> = ALL_CHECKS
            .iter()
            .copied()
            .filter(|check| check.scope == ExecutionScope::LinuxOnce)
            .collect();
        assert_eq!(only_linux.len(), 6);
        let disabled = strings(&["test", "build", "coverage"]);
        let selection = plan_selection(&disabled, &[]).expect("linux-only plans");
        assert!(!selection.requires_platforms());
        assert_eq!(selection.enabled.len(), 6);
        assert!(selection.platforms.is_empty());
    }

    #[test]
    fn disabling_one_check_omits_only_that_check() {
        let selection =
            plan_selection(&strings(&["lint"]), &strings(&["linux_x86_64"])).expect("plans");
        assert_eq!(selection.enabled.len(), CHECK_COUNT - 1);
        assert!(!selection.enabled_ids().contains(&"lint"));
        assert!(selection.enabled_ids().contains(&"typecheck"));
    }

    #[test]
    fn duplicate_disables_are_idempotent() {
        let once = plan_selection(&strings(&["coverage"]), &strings(&["p1"])).expect("plans");
        let twice =
            plan_selection(&strings(&["coverage", "coverage"]), &strings(&["p1"])).expect("plans");
        assert_eq!(once, twice);
    }

    #[test]
    fn unknown_disable_fails_closed() {
        let error = plan_selection(&strings(&["audit"]), &strings(&["p1"])).expect_err("rejects");
        assert_eq!(
            error,
            SelectionError::UnknownCheck {
                value: "audit".to_owned()
            }
        );
    }

    #[test]
    fn missing_platforms_fail_when_per_platform_enabled() {
        let error = plan_selection(&[], &[]).expect_err("requires platforms");
        assert_eq!(error, SelectionError::MissingPlatforms);
    }

    #[test]
    fn platforms_pass_through_verbatim() {
        let selection =
            plan_selection(&[], &strings(&["linux_x86_64", "macos_arm64"])).expect("plans");
        assert_eq!(
            selection.platforms,
            vec!["linux_x86_64".to_owned(), "macos_arm64".to_owned()]
        );
    }

    #[test]
    fn owning_commands_are_frozen() {
        let by_id = |id: &str| find_check(id).expect("known check").command;
        assert_eq!(by_id("lint"), "dx lint --check");
        assert_eq!(by_id("typecheck"), "dx typecheck --check");
        assert_eq!(by_id("format"), "dx format --check");
        assert_eq!(by_id("generate"), "dx generate --check");
        assert_eq!(by_id("security-audit"), "dx audit security");
        assert_eq!(by_id("license-audit"), "dx audit license");
        assert_eq!(by_id("test"), "dx test");
        assert_eq!(by_id("build"), "dx build");
        assert_eq!(by_id("coverage"), "dx coverage");
        assert_eq!(find_check("audit"), None);
    }

    #[test]
    fn selection_is_non_mutating() {
        assert!(!CiSelection::is_mutating());
    }

    #[test]
    fn parallel_is_the_default_mode() {
        assert_eq!(SchedulingMode::default(), SchedulingMode::Parallel);
    }

    #[test]
    fn starter_expands_to_six_linux_once_plus_per_platform_fanout() {
        let selection =
            plan_selection(&[], &strings(&["linux_x86_64", "macos_arm64"])).expect("plans");
        let schedule = plan_schedule(&selection, SchedulingMode::Parallel);
        // 6 Linux-once + 3 checks x 2 platforms = 12 cells.
        assert_eq!(schedule.cells.len(), 12);
        let linux_once: Vec<_> = schedule
            .cells
            .iter()
            .filter(|cell| cell.cell.scope == ExecutionScope::LinuxOnce)
            .collect();
        assert_eq!(linux_once.len(), 6);
        assert!(linux_once.iter().all(|cell| cell.cell.platform.is_none()));
        let per_platform: Vec<_> = schedule
            .cells
            .iter()
            .filter(|cell| cell.cell.scope == ExecutionScope::PerPlatform)
            .collect();
        assert_eq!(per_platform.len(), 6);
        for cell in &per_platform {
            assert!(cell.cell.platform.is_some());
        }
    }

    #[test]
    fn sequential_mode_plans_identical_cells_and_isolation() {
        let selection = plan_selection(&[], &strings(&["linux_x86_64"])).expect("plans");
        let parallel = plan_schedule(&selection, SchedulingMode::Parallel);
        let sequential = plan_schedule(&selection, SchedulingMode::Sequential);
        assert_eq!(parallel.cells, sequential.cells);
        assert_eq!(parallel.mode, SchedulingMode::Parallel);
        assert_eq!(sequential.mode, SchedulingMode::Sequential);
    }

    #[test]
    fn every_cell_owns_a_distinct_output_base() {
        let selection =
            plan_selection(&[], &strings(&["linux_x86_64", "macos_arm64"])).expect("plans");
        let schedule = plan_schedule(&selection, SchedulingMode::Parallel);
        let mut bases: Vec<_> = schedule
            .cells
            .iter()
            .map(|cell| cell.isolation.output_base.clone())
            .collect();
        bases.sort();
        bases.dedup();
        assert_eq!(bases.len(), schedule.cells.len());
    }

    #[test]
    fn isolation_identities_are_distinct_per_cell() {
        let selection = plan_selection(&[], &strings(&["linux_x86_64"])).expect("plans");
        let schedule = plan_schedule(&selection, SchedulingMode::Parallel);
        let mut workdirs: Vec<_> = schedule
            .cells
            .iter()
            .map(|cell| cell.isolation.workdir.clone())
            .collect();
        workdirs.sort();
        workdirs.dedup();
        assert_eq!(workdirs.len(), schedule.cells.len());
        let mut reports: Vec<_> = schedule
            .cells
            .iter()
            .map(|cell| cell.isolation.report_path.clone())
            .collect();
        reports.sort();
        reports.dedup();
        assert_eq!(reports.len(), schedule.cells.len());
    }

    #[test]
    fn platform_spellings_pass_through_verbatim_into_cells() {
        let selection = plan_selection(&[], &strings(&["Custom-Runner_01"])).expect("plans");
        let schedule = plan_schedule(&selection, SchedulingMode::Parallel);
        let platforms: Vec<_> = schedule
            .cells
            .iter()
            .filter_map(|cell| cell.cell.platform.clone())
            .collect();
        assert!(!platforms.is_empty());
        assert!(platforms.iter().all(|p| p == "Custom-Runner_01"));
    }

    #[test]
    fn starter_cells_are_independent() {
        let selection = plan_selection(&[], &strings(&["linux_x86_64"])).expect("plans");
        let schedule = plan_schedule(&selection, SchedulingMode::Parallel);
        assert!(schedule.is_independent());
    }

    #[test]
    fn any_cell_failure_fails_the_run() {
        use CellOutcome::{Failure, Success};
        assert!(aggregate_outcome(&[Success, Success]));
        assert!(!aggregate_outcome(&[Success, Failure]));
        assert!(!aggregate_outcome(&[Failure]));
        // Completed results are preserved inputs to aggregation: a lone
        // failure still reports the sibling success alongside it.
        let outcomes = [Success, Failure, Success];
        assert!(!aggregate_outcome(&outcomes));
        assert_eq!(outcomes.len(), 3);
    }

    #[test]
    fn empty_schedule_succeeds_vacuously() {
        assert!(aggregate_outcome(&[]));
        let disabled = strings(&[
            "lint",
            "typecheck",
            "format",
            "generate",
            "security-audit",
            "license-audit",
            "test",
            "build",
            "coverage",
        ]);
        let selection = plan_selection(&disabled, &[]).expect("empty plans");
        assert!(selection.enabled.is_empty());
        let schedule = plan_schedule(&selection, SchedulingMode::Sequential);
        assert!(schedule.cells.is_empty());
    }

    fn pr_run(pr: &str, validated: &str, seq: u64) -> TrackedRun {
        TrackedRun {
            scope: RunScope::PullRequest { pr: pr.to_owned() },
            validated: validated.to_owned(),
            seq,
        }
    }

    #[test]
    fn newer_commit_supersedes_older_pr_run_in_either_mode() {
        let old = pr_run("pr-7", "merge-a", 1);
        let new = pr_run("pr-7", "merge-b", 2);
        assert!(supersedes(&old, &new));
        assert!(!supersedes(&new, &old));
        assert!(!supersedes(&old, &old));
    }

    #[test]
    fn supersession_stays_within_one_pr() {
        let old = pr_run("pr-7", "merge-a", 1);
        let other_pr = pr_run("pr-8", "merge-b", 2);
        assert!(!supersedes(&old, &other_pr));
        let default_branch = TrackedRun {
            scope: RunScope::DefaultBranch,
            validated: "landed".to_owned(),
            seq: 2,
        };
        assert!(!supersedes(&old, &default_branch));
        let queue = TrackedRun {
            scope: RunScope::Queue { id: "q".to_owned() },
            validated: "queue-b".to_owned(),
            seq: 2,
        };
        assert!(!supersedes(&old, &queue));
        assert!(!supersedes(&queue, &old));
    }

    #[test]
    fn manual_runs_never_supersede() {
        let first = TrackedRun {
            scope: RunScope::Manual { id: "m".to_owned() },
            validated: "rev-a".to_owned(),
            seq: 1,
        };
        let second = TrackedRun {
            scope: RunScope::Manual { id: "m".to_owned() },
            validated: "rev-b".to_owned(),
            seq: 2,
        };
        assert!(!supersedes(&first, &second));
    }

    #[test]
    fn late_callbacks_must_not_overwrite_current_summary() {
        let current = pr_run("pr-7", "merge-b", 2);
        let superseded = pr_run("pr-7", "merge-a", 1);
        assert!(!may_publish(&superseded, &current));
        assert!(may_publish(&current, &current));
        let same_seq_new_snapshot = pr_run("pr-7", "merge-c", 2);
        assert!(!may_publish(&same_seq_new_snapshot, &current));
    }

    #[test]
    fn stale_queue_results_never_satisfy_newer_revisions() {
        let stored = TrackedRun {
            scope: RunScope::Queue { id: "q".to_owned() },
            validated: "queue-a".to_owned(),
            seq: 1,
        };
        assert!(result_satisfies(&stored, "queue-a"));
        assert!(!result_satisfies(&stored, "queue-b"));
        let stored_pr = pr_run("pr-7", "merge-a", 1);
        assert!(!result_satisfies(&stored_pr, "merge-b"));
    }

    fn finding(id: &str, failure: bool, located: bool) -> Finding {
        Finding {
            id: id.to_owned(),
            contributes_to_failure: failure,
            location: if located {
                Some(format!("file.rs:{id}"))
            } else {
                None
            },
        }
    }

    #[test]
    fn unchanged_findings_keep_threads_without_repost() {
        let findings = vec![finding("f1", true, true)];
        let existing = vec![OwnedThread {
            finding: "f1".to_owned(),
            has_human_replies: false,
        }];
        let plan = plan_threads(&findings, &existing, &[], 10);
        assert_eq!(plan.keep, vec!["f1".to_owned()]);
        assert!(plan.create.is_empty());
    }

    #[test]
    fn unmappable_findings_never_gain_threads() {
        let findings = vec![finding("f1", true, false)];
        let plan = plan_threads(&findings, &[], &[], 10);
        assert!(plan.create.is_empty());
        assert_eq!(plan.unmappable, vec!["f1".to_owned()]);
        let summary = plan_summary("merge-a", 1, &plan);
        assert_eq!(summary.unmappable, 1);
        assert_eq!(summary.finding_count, 1);
    }

    #[test]
    fn failure_contributors_win_limited_threads_deterministically() {
        let findings = vec![
            finding("b-non-failing", false, true),
            finding("a-failing", true, true),
            finding("c-failing", true, true),
        ];
        let plan = plan_threads(&findings, &[], &[], 2);
        assert_eq!(
            plan.create,
            vec!["a-failing".to_owned(), "c-failing".to_owned()]
        );
        assert_eq!(plan.omitted_due_to_limit, vec!["b-non-failing".to_owned()]);
        let rerun = plan_threads(&findings, &[], &[], 2);
        assert_eq!(plan, rerun);
    }

    #[test]
    fn existing_threads_never_rotate_for_new_findings() {
        let existing = vec![OwnedThread {
            finding: "old".to_owned(),
            has_human_replies: false,
        }];
        let findings = vec![finding("old", true, true), finding("new", true, true)];
        let plan = plan_threads(&findings, &existing, &[], 1);
        assert_eq!(plan.keep, vec!["old".to_owned()]);
        assert!(plan.create.is_empty());
        assert_eq!(plan.omitted_due_to_limit, vec!["new".to_owned()]);
    }

    #[test]
    fn confirmed_gone_deletes_bot_only_and_resolves_replied() {
        let existing = vec![
            OwnedThread {
                finding: "gone-quiet".to_owned(),
                has_human_replies: false,
            },
            OwnedThread {
                finding: "gone-discussed".to_owned(),
                has_human_replies: true,
            },
            OwnedThread {
                finding: "unconfirmed".to_owned(),
                has_human_replies: false,
            },
        ];
        let findings = vec![];
        let confirmed = vec!["gone-quiet".to_owned(), "gone-discussed".to_owned()];
        let plan = plan_threads(&findings, &existing, &confirmed, 10);
        assert_eq!(plan.delete, vec!["gone-quiet".to_owned()]);
        assert_eq!(plan.resolve, vec!["gone-discussed".to_owned()]);
        assert!(plan.keep.is_empty());
    }

    #[test]
    fn reporting_failure_fails_ci_despite_passing_analysis() {
        assert!(reporting_gate(true, true, true));
        assert!(!reporting_gate(true, true, false));
        assert!(!reporting_gate(false, true, true));
        // Intentionally inapplicable outputs (queue-run PR comments) are
        // not failures.
        assert!(reporting_gate(true, false, false));
    }

    #[test]
    fn aggregate_identity_is_stable_across_selection_and_mode() {
        assert_eq!(AGGREGATE_CHECK, "dx-ci");
        let full = plan_selection(&[], &["linux_x86_64".to_owned()]).expect("plans");
        let narrow = plan_selection(
            &["coverage".to_owned(), "test".to_owned(), "build".to_owned()],
            &[],
        )
        .expect("plans");
        let parallel = plan_schedule(&full, SchedulingMode::Parallel);
        let sequential = plan_schedule(&narrow, SchedulingMode::Sequential);
        assert_ne!(parallel.cells.len(), sequential.cells.len());
        assert_eq!(AGGREGATE_CHECK, "dx-ci");
    }

    #[test]
    fn outside_authors_cannot_self_approve() {
        assert!(plan_approval(false, ApproverRole::OutsideAuthor, true));
        assert!(!plan_approval(true, ApproverRole::OutsideAuthor, true));
        assert!(!plan_approval(true, ApproverRole::Maintainer, true));
        assert!(plan_approval(true, ApproverRole::Maintainer, false));
        assert!(plan_approval(true, ApproverRole::Collaborator, false));
        assert!(!plan_approval(true, ApproverRole::OutsideAuthor, false));
    }

    #[test]
    fn approved_fork_execution_stays_read_only() {
        assert_eq!(fork_credentials(), ForkCredentials::ReadOnly);
        assert!(!privileged_reporting_executes_fork_code());
    }

    #[test]
    fn aggregate_requires_every_cell_and_reporting() {
        use AggregateState::{Blocked, Cancelled, Failure, Incomplete, Missing, Skipped, Success};
        assert!(plan_aggregate(&[Success, Success], true, true));
        assert!(!plan_aggregate(&[Success, Failure], true, true));
        assert!(!plan_aggregate(&[Success, Blocked], true, true));
        assert!(!plan_aggregate(&[Success, Skipped], true, true));
        assert!(!plan_aggregate(&[Success, Cancelled], true, true));
        assert!(!plan_aggregate(&[Success, Incomplete], true, true));
        assert!(!plan_aggregate(&[Success, Missing], true, true));
        assert!(!plan_aggregate(&[Success, Success], true, false));
        assert!(!plan_aggregate(&[], true, true));
        // Queue-run PR comments are inapplicable, not failures.
        assert!(plan_aggregate(&[Success], false, false));
    }

    #[test]
    fn reruns_preserve_revision_and_reporting_identity() {
        let original = plan_revision(RevisionRequest::PullRequest {
            head: "head-sha",
            base: "base-sha",
            merged: "merge-sha",
            draft: true,
        })
        .expect("plans");
        let rerun = plan_rerun(&original);
        assert_eq!(rerun, original);
        assert_eq!(rerun.validated, "merge-sha");
        assert_eq!(rerun.reporting, Reporting::PrThreadsAndSummary);
    }

    #[test]
    fn docs_only_changes_still_run_everything_without_path_filters() {
        assert!(scope_runs_all_selected(true));
        assert!(scope_runs_all_selected(false));
        assert!(!uses_path_filters());
    }

    #[test]
    fn bounded_retries_reuse_results_and_retain_exhaustion() {
        let none = plan_report_retry(0, 3);
        assert!(!none.retried_same_results);
        assert!(!none.failure_retained);
        let bounded = plan_report_retry(2, 3);
        assert!(bounded.retried_same_results);
        assert!(!bounded.failure_retained);
        assert!(!bounded.duplicates_comments);
        assert!(!bounded.changes_analysis);
        let exhausted = plan_report_retry(4, 3);
        assert!(exhausted.retried_same_results);
        assert!(exhausted.failure_retained);
        assert!(!exhausted.duplicates_comments);
        assert!(!exhausted.changes_analysis);
    }

    #[test]
    fn code_scanning_defaults_off_and_never_fails_when_disabled() {
        assert!(!CODE_SCANNING_DEFAULT);
        assert_eq!(plan_code_scanning(false, true), CodeScanningPlan::Disabled);
        assert_eq!(plan_code_scanning(true, true), CodeScanningPlan::Upload);
        assert_eq!(
            plan_code_scanning(true, false),
            CodeScanningPlan::MustNotUpload
        );
    }

    #[test]
    fn coverage_aggregation_hides_no_platform_gap() {
        assert!(plan_coverage_aggregate(&[true, true]));
        assert!(!plan_coverage_aggregate(&[true, false]));
        assert!(!plan_coverage_aggregate(&[]));
    }

    #[test]
    fn starter_triggers_cover_pr_push_dispatch_queue_but_not_feature_push() {
        assert!(starter_triggers(&RevisionRequest::PullRequest {
            head: "h",
            base: "b",
            merged: "m",
            draft: false,
        }));
        assert!(starter_triggers(&RevisionRequest::PushDefault {
            landed: "l",
            branch: "trunk",
        }));
        assert!(!starter_triggers(&RevisionRequest::PushNonDefault {
            branch: "feature-x",
        }));
        assert!(starter_triggers(&RevisionRequest::ManualDispatch {
            revision: "r",
        }));
        assert!(starter_triggers(&RevisionRequest::MergeQueue {
            combined: "q"
        }));
    }

    #[test]
    fn custom_default_branch_triggers_like_main() {
        // The default branch may carry any name; opaque spellings pass
        // through identically.
        let custom = plan_revision(RevisionRequest::PushDefault {
            landed: "landed-sha",
            branch: "trunk",
        })
        .expect("custom default branch plans");
        assert_eq!(custom.validated, "landed-sha");
        assert!(starter_triggers(&RevisionRequest::PushDefault {
            landed: "l",
            branch: "trunk",
        }));
    }

    #[test]
    fn platform_validation_rejects_empty_and_unsupported_without_substitution() {
        let supported = vec!["linux_x86_64".to_owned(), "macos_arm64".to_owned()];
        assert_eq!(
            validate_platforms(&[], &supported),
            Err(PlatformError::Empty)
        );
        assert_eq!(
            validate_platforms(&["windows_x86_64".to_owned()], &supported),
            Err(PlatformError::Unsupported {
                value: "windows_x86_64".to_owned(),
            })
        );
        // Single- and multi-platform selections pass through verbatim,
        // including one without Linux.
        assert_eq!(
            validate_platforms(&["macos_arm64".to_owned()], &supported),
            Ok(vec!["macos_arm64".to_owned()])
        );
        assert_eq!(
            validate_platforms(
                &["linux_x86_64".to_owned(), "macos_arm64".to_owned()],
                &supported
            ),
            Ok(vec!["linux_x86_64".to_owned(), "macos_arm64".to_owned()])
        );
    }

    #[test]
    fn configured_noop_without_report_is_not_a_collection_failure() {
        assert!(!is_collection_failure(false, true));
        assert!(is_collection_failure(false, false));
        assert!(!is_collection_failure(true, false));
        assert!(!is_collection_failure(true, true));
    }

    #[test]
    fn base_advancement_requires_a_fresh_run() {
        assert!(!base_advanced_requires_rerun("base-a", "base-a"));
        assert!(base_advanced_requires_rerun("base-a", "base-b"));
    }

    #[test]
    fn caller_template_owns_selection_only_not_workflow_logic() {
        assert_eq!(
            CALLER_OWNED_INPUTS,
            [
                "disabled_checks",
                "platforms",
                "scheduling_mode",
                "code_scanning_opt_in"
            ]
        );
        assert_eq!(
            WORKFLOW_OWNED,
            [
                "execution",
                "parsing",
                "review_threads",
                "comment_management"
            ]
        );
        assert!(!uses_generated_setup_command());
    }

    #[test]
    fn starter_caller_composes_selection_mode_and_opt_in() {
        let supported = strings(&["linux_x86_64", "macos_arm64"]);
        let caller = plan_caller(
            &[],
            &strings(&["linux_x86_64", "macos_arm64"]),
            &supported,
            SchedulingMode::Parallel,
            false,
        )
        .expect("starter caller plans");
        assert_eq!(caller.selection.enabled.len(), CHECK_COUNT);
        assert_eq!(
            caller.selection.platforms,
            vec!["linux_x86_64".to_owned(), "macos_arm64".to_owned()]
        );
        assert_eq!(caller.mode, SchedulingMode::Parallel);
        assert!(!caller.code_scanning_opt_in);
    }

    #[test]
    fn caller_opt_out_and_sequential_mode_pass_through() {
        let supported = strings(&["linux_x86_64"]);
        let caller = plan_caller(
            &strings(&["lint"]),
            &strings(&["linux_x86_64"]),
            &supported,
            SchedulingMode::Sequential,
            true,
        )
        .expect("caller plans");
        assert_eq!(caller.selection.enabled.len(), CHECK_COUNT - 1);
        assert!(!caller.selection.enabled_ids().contains(&"lint"));
        assert_eq!(caller.mode, SchedulingMode::Sequential);
        assert!(caller.code_scanning_opt_in);
    }

    #[test]
    fn linux_only_caller_needs_no_supported_set() {
        let caller = plan_caller(
            &strings(&["test", "build", "coverage"]),
            &[],
            &[],
            SchedulingMode::Parallel,
            false,
        )
        .expect("linux-only caller plans");
        assert!(!caller.selection.requires_platforms());
        assert!(caller.selection.platforms.is_empty());
    }

    #[test]
    fn caller_rejects_unknown_checks_and_bad_platforms_closed() {
        let supported = strings(&["linux_x86_64"]);
        assert_eq!(
            plan_caller(
                &strings(&["audit"]),
                &strings(&["linux_x86_64"]),
                &supported,
                SchedulingMode::Parallel,
                false,
            ),
            Err(CallerError::Selection(SelectionError::UnknownCheck {
                value: "audit".to_owned()
            }))
        );
        assert_eq!(
            plan_caller(&[], &[], &supported, SchedulingMode::Parallel, false),
            Err(CallerError::Selection(SelectionError::MissingPlatforms))
        );
        assert_eq!(
            plan_caller(
                &[],
                &strings(&["windows_x86_64"]),
                &supported,
                SchedulingMode::Parallel,
                false,
            ),
            Err(CallerError::Platform(PlatformError::Unsupported {
                value: "windows_x86_64".to_owned()
            }))
        );
    }

    #[test]
    fn pin_changes_require_review_and_explicit_identities() {
        assert_eq!(plan_pin_update("v1", "v1", false), Ok(PinUpdate::NoChange));
        assert_eq!(
            plan_pin_update("v1", "v2", false),
            Err(PinError::UnreviewedChange)
        );
        assert_eq!(
            plan_pin_update("v1", "v2", true),
            Ok(PinUpdate::ReviewedUpdate {
                from: "v1".to_owned(),
                to: "v2".to_owned(),
            })
        );
        assert_eq!(plan_pin_update("", "v2", true), Err(PinError::MissingPin));
        assert_eq!(plan_pin_update("v1", "", true), Err(PinError::MissingPin));
    }

    #[test]
    fn pin_updates_preserve_consumer_customizations() {
        let supported = strings(&["linux_x86_64"]);
        let caller = plan_caller(
            &strings(&["lint"]),
            &strings(&["linux_x86_64"]),
            &supported,
            SchedulingMode::Sequential,
            true,
        )
        .expect("caller plans");
        let update = plan_pin_update("v1", "v2", true).expect("reviewed update plans");
        assert_eq!(apply_pin_update(&caller, &update), caller);
    }

    fn audit_finding(id: &str, located: bool, restricted: bool) -> AuditFinding {
        AuditFinding {
            id: id.to_owned(),
            severity: "high".to_owned(),
            acceptance: "unaccepted".to_owned(),
            contributes_to_failure: true,
            location: if located {
                Some(format!("src/lib.rs:{id}"))
            } else {
                None
            },
            contains_restricted_content: restricted,
        }
    }

    #[test]
    fn audit_diff_mapped_findings_receive_review_details() {
        let finding = audit_finding("audit-1", true, false);
        assert_eq!(plan_audit_placement(&finding), AuditPlacement::ReviewThread);
        assert!(audit_body_publishable(&finding));
    }

    #[test]
    fn audit_locationless_findings_stay_in_reports_and_counts() {
        let finding = audit_finding("audit-2", false, false);
        assert_eq!(plan_audit_placement(&finding), AuditPlacement::ReportOnly);
        assert!(audit_body_publishable(&finding));
    }

    #[test]
    fn audit_preserves_severity_acceptance_and_outcome() {
        let finding = AuditFinding {
            id: "audit-3".to_owned(),
            severity: "critical:custom-scale".to_owned(),
            acceptance: "accepted:risk-42".to_owned(),
            contributes_to_failure: false,
            location: Some("src/lib.rs:audit-3".to_owned()),
            contains_restricted_content: false,
        };
        assert_eq!(plan_audit_placement(&finding), AuditPlacement::ReviewThread);
        assert_eq!(finding.severity, "critical:custom-scale");
        assert_eq!(finding.acceptance, "accepted:risk-42");
        assert!(!finding.contributes_to_failure);
    }

    #[test]
    fn audit_restricted_bodies_withheld_without_hiding_counts() {
        let mapped = audit_finding("audit-secret", true, true);
        assert_eq!(plan_audit_placement(&mapped), AuditPlacement::ReviewThread);
        assert!(!audit_body_publishable(&mapped));
        assert!(mapped.contributes_to_failure);
        let unmapped = audit_finding("audit-secret-offdiff", false, true);
        assert_eq!(plan_audit_placement(&unmapped), AuditPlacement::ReportOnly);
        assert!(!audit_body_publishable(&unmapped));
    }

    #[test]
    fn audit_has_no_counts_only_mode_disclosure_toggle_or_private_guarantee() {
        assert!(!audit_uses_counts_only_mode());
        assert!(!audit_has_disclosure_toggle());
        assert!(!audit_assumes_private());
        assert!(!audit_claims_generic_redaction());
    }

    #[test]
    fn artifacts_bind_exactly_to_the_validated_snapshot() {
        assert_eq!(
            validate_artifact_snapshot("", "merge-a", "digest-1"),
            Err(ArtifactError::MissingArtifact)
        );
        assert_eq!(
            validate_artifact_snapshot("merge-a", "merge-a", ""),
            Err(ArtifactError::MissingDigest)
        );
        assert_eq!(
            validate_artifact_snapshot("merge-a", "merge-b", "digest-1"),
            Err(ArtifactError::SnapshotMismatch)
        );
        // Presence alone never establishes trust: empty required snapshot
        // cannot be satisfied.
        assert_eq!(
            validate_artifact_snapshot("merge-a", "", "digest-1"),
            Err(ArtifactError::SnapshotMismatch)
        );
        assert_eq!(
            validate_artifact_snapshot("merge-a", "merge-a", "digest-1"),
            Ok(())
        );
    }

    #[test]
    fn stale_or_foreign_artifacts_never_satisfy_the_current_run() {
        let planned = plan_revision(RevisionRequest::PullRequest {
            head: "head-sha",
            base: "base-sha",
            merged: "merge-b",
            draft: false,
        })
        .expect("PR plans");
        assert!(validate_artifact_snapshot("merge-a", &planned.validated, "digest-1").is_err());
        assert!(
            validate_artifact_snapshot(&planned.validated, &planned.validated, "digest-1").is_ok()
        );
    }

    #[test]
    fn pr_metadata_must_match_the_planned_revision_exactly() {
        let planned = plan_revision(RevisionRequest::PullRequest {
            head: "head-sha",
            base: "base-sha",
            merged: "merge-sha",
            draft: false,
        })
        .expect("PR plans");
        assert_eq!(
            validate_pr_metadata("head-sha", "base-sha", "merge-sha", &planned),
            Ok(())
        );
        assert_eq!(
            validate_pr_metadata("", "base-sha", "merge-sha", &planned),
            Err(MetadataError::MissingField)
        );
        assert_eq!(
            validate_pr_metadata("head-sha", "base-sha", "merge-stale", &planned),
            Err(MetadataError::StaleSnapshot)
        );
        assert_eq!(
            validate_pr_metadata("head-other", "base-sha", "merge-sha", &planned),
            Err(MetadataError::StaleSnapshot)
        );
        assert_eq!(
            validate_pr_metadata("head-sha", "base-other", "merge-sha", &planned),
            Err(MetadataError::StaleSnapshot)
        );
    }

    #[test]
    fn untrusted_inputs_grant_no_execution_or_secrets() {
        assert!(!untrusted_inputs_execute_fork_code());
        assert!(!untrusted_inputs_grant_secrets());
        assert!(!privileged_reporting_executes_fork_code());
        assert_eq!(fork_credentials(), ForkCredentials::ReadOnly);
    }

    #[test]
    fn cleared_findings_free_thread_slots_for_new_findings() {
        // Limit 1: `old` occupies the only slot, `new` is omitted.
        let first = vec![finding("old", true, true), finding("new", true, true)];
        let existing = vec![OwnedThread {
            finding: "old".to_owned(),
            has_human_replies: false,
        }];
        let full = plan_threads(&first, &existing, &[], 1);
        assert_eq!(full.keep, vec!["old".to_owned()]);
        assert!(full.create.is_empty());
        assert_eq!(thread_slots_used(&full), 1);
        // `old` clears (bot-only, confirmed gone): its slot frees and the
        // next assessment creates `new`.
        let second = vec![finding("new", true, true)];
        let next = plan_threads(&second, &existing, &["old".to_owned()], 1);
        assert_eq!(next.delete, vec!["old".to_owned()]);
        assert_eq!(thread_slots_used(&next), 0);
        let after = plan_threads(
            &second,
            &[OwnedThread {
                finding: "new".to_owned(),
                has_human_replies: false,
            }],
            &["old".to_owned()],
            1,
        );
        assert_eq!(after.keep, vec!["new".to_owned()]);
    }

    #[test]
    fn retained_resolved_discussions_do_not_block_new_threads() {
        assert!(!resolved_discussions_count_against_limit());
        assert!(concurrent_runs_share_limit());
        // Replied thread for a gone finding resolves (discussion retained)
        // and frees its slot: only still-present threads count.
        let existing = vec![OwnedThread {
            finding: "gone-discussed".to_owned(),
            has_human_replies: true,
        }];
        let cleared = plan_threads(&[], &existing, &["gone-discussed".to_owned()], 1);
        assert_eq!(cleared.resolve, vec!["gone-discussed".to_owned()]);
        assert_eq!(thread_slots_used(&cleared), 0);
        let newcomer = vec![finding("fresh", true, true)];
        let next = plan_threads(&newcomer, &[], &["gone-discussed".to_owned()], 1);
        assert_eq!(next.create, vec!["fresh".to_owned()]);
        // Concurrent completions share one budget: no fresh allowance per run.
        let shared = plan_threads(&newcomer, &[], &[], 1);
        assert_eq!(shared.create, vec!["fresh".to_owned()]);
        assert_eq!(shared.create.len(), 1);
    }

    #[test]
    fn preset_runbook_covers_setup_without_dx_init_template() {
        assert_eq!(
            PRESET_RUNBOOK_STEPS,
            [
                "dependency_snippet",
                "generation_target",
                "import_block",
                "update_loop",
                "bot_config_sample"
            ]
        );
        assert!(!dx_init_emits_preset_template());
    }

    #[test]
    fn preset_labels_pass_through_verbatim_without_implicit_default() {
        assert_eq!(
            validate_preset_label("@rules_dx//tools/bazelrc:preset"),
            Ok("@rules_dx//tools/bazelrc:preset".to_owned())
        );
        assert_eq!(
            validate_preset_label(""),
            Err(PresetLabelError::MissingLabel)
        );
    }

    #[test]
    fn preset_affecting_changes_require_minor_or_major_and_notes() {
        assert!(preset_affecting_requires_minor_or_major(true));
        assert!(!preset_affecting_requires_minor_or_major(false));
        assert!(!preset_change_allowed_in_patch(true));
        assert!(preset_change_allowed_in_patch(false));
        assert!(preset_change_needs_release_note(true));
        assert!(!preset_change_needs_release_note(false));
    }

    #[test]
    fn preset_regen_stays_manual_without_auto_merge() {
        assert!(!preset_bot_auto_merge_allowed());
        assert!(preset_regen_requires_review());
    }

    #[test]
    fn clean_consumer_must_prove_generation_import_and_reviewed_regen() {
        assert_eq!(
            plan_preset_onboarding(false, true, true),
            Err(PresetOnboardingError::MissingGeneration)
        );
        assert_eq!(
            plan_preset_onboarding(true, false, true),
            Err(PresetOnboardingError::MissingImport)
        );
        assert_eq!(
            plan_preset_onboarding(true, true, false),
            Err(PresetOnboardingError::UnreviewedRegen)
        );
        assert_eq!(plan_preset_onboarding(true, true, true), Ok(()));
    }
}
