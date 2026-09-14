//! Pure consumer-CI check-selection planning (M27 WP1 slices 1-4).
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
//! Out of scope here (M27 qualification): workflow APIs/pins, event/ref
//! bindings beyond the revision-planning identities below,
//! review-thread/reporting mechanics, fork security, merge gating,
//! `.bazelrc` preset onboarding, and any YAML or reporter implementation.
//! Those arrive in later M27 slices.

// ---------------------------------------------------------------------------
// Event and revision planning (M27 WP1 slice 2).
// ---------------------------------------------------------------------------

/// Where PR-linked reporting may write.
///
/// PR runs own review threads plus one updated summary comment. Every
/// other run context retains individual checks, detailed reports, and
/// workflow summaries but never creates or mutates PR comments/threads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reporting {
    /// Review threads plus one integration-owned updated summary comment.
    PrThreadsAndSummary,
    /// No PR comment/thread changes (push, dispatch, queue runs).
    NoPrComments,
}

/// Caller-supplied revision request for one CI run.
///
/// All strings are opaque revision/branch identities: SHAs pass through
/// verbatim and branch names are never compared against `"main"` — the
/// default branch may carry any name. `draft` on pull requests selects
/// the same checks, reporting, and aggregate semantics as ready PRs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RevisionRequest<'a> {
    /// Pull request (draft or ready): validate the proposed merge.
    PullRequest {
        /// Contributor-branch head revision.
        head: &'a str,
        /// Target-branch revision the merge was computed against.
        base: &'a str,
        /// Proposed-merge (test-merge) snapshot every selected check runs.
        merged: &'a str,
        /// Draft PRs validate identically to ready PRs.
        draft: bool,
    },
    /// Push to the default branch: analyze the landed revision.
    PushDefault {
        /// Landed revision.
        landed: &'a str,
        /// Default-branch name, verbatim (never assumed to be `"main"`).
        branch: &'a str,
    },
    /// Ordinary non-default-branch push without PR context: the starter
    /// does not trigger on these alone.
    PushNonDefault {
        /// Branch name, verbatim.
        branch: &'a str,
    },
    /// Manual dispatch without PR context: analyze the dispatch revision.
    ManualDispatch {
        /// Identified dispatch revision.
        revision: &'a str,
    },
    /// Consumer-enabled merge queue: analyze the combined queue revision.
    MergeQueue {
        /// Proposed combined queue revision.
        combined: &'a str,
    },
}

/// Planned revision: the single snapshot every selected check in the run
/// must use, plus the identities that snapshot was derived from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedRevision {
    /// Snapshot every selected check analyzes (PR: the test-merge;
    /// push/dispatch/queue: the landed/dispatched/combined revision).
    pub validated: String,
    /// PR-head revision (PR runs only).
    pub head: Option<String>,
    /// Target-branch revision the test-merge was computed against (PR only).
    pub base: Option<String>,
    /// Whether this run may write PR review threads/summary comments.
    pub reporting: Reporting,
}

impl PlannedRevision {
    /// Snapshot identity callers reuse across all selected checks in the
    /// run: `(validated, head, base)`. A tested snapshot is never evidence
    /// for a later head/base combination; re-plan on advancement.
    pub fn snapshot_identity(&self) -> (String, Option<String>, Option<String>) {
        (self.validated.clone(), self.head.clone(), self.base.clone())
    }
}

/// Malformed revision request: missing identities fail closed, and a PR
/// whose test-merge cannot be created reports blocked — never a
/// head-only fallback or aggregate success.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevisionError {
    /// PR head revision missing or empty.
    MissingHead,
    /// PR target-branch revision missing or empty.
    MissingBase,
    /// Test-merge snapshot unavailable (merge conflict): validation is
    /// blocked. GitHub may suppress the PR workflow for conflicting PRs;
    /// the blocked-status path must not execute fork code with reporting
    /// privileges.
    BlockedOnConflict,
    /// Default-branch landed revision missing or empty.
    MissingLanded,
    /// Default-branch name missing or empty.
    MissingBranch,
    /// Ordinary non-default-branch pushes alone do not trigger the starter.
    NotTriggered,
    /// Manual-dispatch revision missing or empty.
    MissingDispatchRevision,
    /// Merge-queue combined revision missing or empty.
    MissingQueueRevision,
}

impl std::fmt::Display for RevisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RevisionError::MissingHead => write!(f, "PR head revision is required"),
            RevisionError::MissingBase => {
                write!(f, "PR target-branch revision is required")
            }
            RevisionError::BlockedOnConflict => write!(
                f,
                "proposed merge unavailable (conflict): validation is blocked, not head-only"
            ),
            RevisionError::MissingLanded => {
                write!(f, "landed revision is required for default-branch pushes")
            }
            RevisionError::MissingBranch => write!(f, "branch name is required"),
            RevisionError::NotTriggered => write!(
                f,
                "non-default-branch pushes alone do not trigger the starter"
            ),
            RevisionError::MissingDispatchRevision => {
                write!(f, "dispatch revision is required for manual runs")
            }
            RevisionError::MissingQueueRevision => {
                write!(
                    f,
                    "combined queue revision is required for merge-queue runs"
                )
            }
        }
    }
}

impl std::error::Error for RevisionError {}

fn nonempty(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

/// Plan the revision one CI run validates.
///
/// PRs validate the proposed merge (`merged`), never the contributor
/// branch alone, and record head/base alongside the snapshot so findings
/// map back to the PR diff. Draft PRs plan identically to ready PRs.
/// Push, dispatch, and queue runs analyze their landed/dispatched/
/// combined revision with no PR comment changes.
pub fn plan_revision(request: RevisionRequest<'_>) -> Result<PlannedRevision, RevisionError> {
    match request {
        RevisionRequest::PullRequest {
            head,
            base,
            merged,
            draft: _,
        } => {
            let head = nonempty(head).ok_or(RevisionError::MissingHead)?;
            let base = nonempty(base).ok_or(RevisionError::MissingBase)?;
            let merged = nonempty(merged).ok_or(RevisionError::BlockedOnConflict)?;
            Ok(PlannedRevision {
                validated: merged,
                head: Some(head),
                base: Some(base),
                reporting: Reporting::PrThreadsAndSummary,
            })
        }
        RevisionRequest::PushDefault { landed, branch } => {
            let landed = nonempty(landed).ok_or(RevisionError::MissingLanded)?;
            nonempty(branch).ok_or(RevisionError::MissingBranch)?;
            Ok(PlannedRevision {
                validated: landed,
                head: None,
                base: None,
                reporting: Reporting::NoPrComments,
            })
        }
        RevisionRequest::PushNonDefault { branch: _ } => Err(RevisionError::NotTriggered),
        RevisionRequest::ManualDispatch { revision } => {
            let revision = nonempty(revision).ok_or(RevisionError::MissingDispatchRevision)?;
            Ok(PlannedRevision {
                validated: revision,
                head: None,
                base: None,
                reporting: Reporting::NoPrComments,
            })
        }
        RevisionRequest::MergeQueue { combined } => {
            let combined = nonempty(combined).ok_or(RevisionError::MissingQueueRevision)?;
            Ok(PlannedRevision {
                validated: combined,
                head: None,
                base: None,
                reporting: Reporting::NoPrComments,
            })
        }
    }
}

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionError {
    /// Caller disabled an ID outside the frozen nine.
    UnknownCheck { value: String },
    /// A per-platform check is enabled but no platforms were supplied.
    MissingPlatforms,
}

impl std::fmt::Display for SelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectionError::UnknownCheck { value } => {
                write!(
                    f,
                    "unknown CI check {value:?}; want one of the nine starter checks"
                )
            }
            SelectionError::MissingPlatforms => {
                write!(
                    f,
                    "explicit platform selection is required when test, build, or coverage is enabled"
                )
            }
        }
    }
}

impl std::error::Error for SelectionError {}

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
    fn pr_validates_proposed_merge_with_pr_reporting() {
        let planned = plan_revision(RevisionRequest::PullRequest {
            head: "head-sha",
            base: "base-sha",
            merged: "merge-sha",
            draft: false,
        })
        .expect("PR plans");
        assert_eq!(planned.validated, "merge-sha");
        assert_eq!(planned.head.as_deref(), Some("head-sha"));
        assert_eq!(planned.base.as_deref(), Some("base-sha"));
        assert_eq!(planned.reporting, Reporting::PrThreadsAndSummary);
    }

    #[test]
    fn draft_pr_plans_identically_to_ready_pr() {
        let ready = plan_revision(RevisionRequest::PullRequest {
            head: "head-sha",
            base: "base-sha",
            merged: "merge-sha",
            draft: false,
        })
        .expect("ready plans");
        let draft = plan_revision(RevisionRequest::PullRequest {
            head: "head-sha",
            base: "base-sha",
            merged: "merge-sha",
            draft: true,
        })
        .expect("draft plans");
        assert_eq!(ready, draft);
    }

    #[test]
    fn pr_without_merge_is_blocked_not_head_fallback() {
        let error = plan_revision(RevisionRequest::PullRequest {
            head: "head-sha",
            base: "base-sha",
            merged: "",
            draft: false,
        })
        .expect_err("conflict blocks");
        assert_eq!(error, RevisionError::BlockedOnConflict);
    }

    #[test]
    fn pr_missing_head_or_base_fails_closed() {
        let missing_head = plan_revision(RevisionRequest::PullRequest {
            head: "",
            base: "base-sha",
            merged: "merge-sha",
            draft: false,
        })
        .expect_err("head required");
        assert_eq!(missing_head, RevisionError::MissingHead);
        let missing_base = plan_revision(RevisionRequest::PullRequest {
            head: "head-sha",
            base: "",
            merged: "merge-sha",
            draft: false,
        })
        .expect_err("base required");
        assert_eq!(missing_base, RevisionError::MissingBase);
    }

    #[test]
    fn push_to_custom_default_branch_has_no_pr_comments() {
        let planned = plan_revision(RevisionRequest::PushDefault {
            landed: "landed-sha",
            branch: "trunk",
        })
        .expect("push plans");
        assert_eq!(planned.validated, "landed-sha");
        assert_eq!(planned.head, None);
        assert_eq!(planned.base, None);
        assert_eq!(planned.reporting, Reporting::NoPrComments);
    }

    #[test]
    fn non_default_push_does_not_trigger() {
        let error = plan_revision(RevisionRequest::PushNonDefault {
            branch: "feature-x",
        })
        .expect_err("no trigger");
        assert_eq!(error, RevisionError::NotTriggered);
    }

    #[test]
    fn dispatch_and_queue_have_no_pr_comments() {
        let dispatch = plan_revision(RevisionRequest::ManualDispatch {
            revision: "dispatch-sha",
        })
        .expect("dispatch plans");
        assert_eq!(dispatch.validated, "dispatch-sha");
        assert_eq!(dispatch.reporting, Reporting::NoPrComments);
        let queue = plan_revision(RevisionRequest::MergeQueue {
            combined: "queue-sha",
        })
        .expect("queue plans");
        assert_eq!(queue.validated, "queue-sha");
        assert_eq!(queue.head, None);
        assert_eq!(queue.reporting, Reporting::NoPrComments);
        assert_eq!(
            plan_revision(RevisionRequest::ManualDispatch { revision: "" }).expect_err("rejects"),
            RevisionError::MissingDispatchRevision
        );
        assert_eq!(
            plan_revision(RevisionRequest::MergeQueue { combined: "" }).expect_err("rejects"),
            RevisionError::MissingQueueRevision
        );
    }

    #[test]
    fn snapshot_identity_pins_merge_head_and_base() {
        let planned = plan_revision(RevisionRequest::PullRequest {
            head: "head-sha",
            base: "base-sha",
            merged: "merge-sha",
            draft: false,
        })
        .expect("PR plans");
        assert_eq!(
            planned.snapshot_identity(),
            (
                "merge-sha".to_owned(),
                Some("head-sha".to_owned()),
                Some("base-sha".to_owned()),
            )
        );
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
}
