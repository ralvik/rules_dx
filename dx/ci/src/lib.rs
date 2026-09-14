//! Pure consumer-CI check-selection planning (M27 WP1 slices 1-2).
//!
//! This crate owns the check-selection surface before any reusable
//! workflow, caller template, or reporter lands: the nine accepted CI
//! checks, starter defaults, explicit opt-outs, and the explicit
//! platform-selection gate for per-platform checks. It plans over
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
//! Out of scope here (M27 qualification): workflow APIs/pins, event/ref
//! bindings beyond the revision-planning identities below, scheduling,
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
}
