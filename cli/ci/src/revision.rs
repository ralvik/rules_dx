//! Event and revision planning for consumer CI (WP1 slice 2).
//!
//! Split from `super` (`lib.rs`): owns [`Reporting`] (where PR-linked
//! reporting may write), [`RevisionRequest`] (opaque caller-supplied
//! revision identities), [`PlannedRevision`] (the single snapshot every
//! selected check validates, with [`PlannedRevision::snapshot_identity`]),
//! [`RevisionError`], and [`plan_revision`] (PRs validate the proposed
//! merge, never the contributor branch alone; draft PRs plan identically
//! to ready PRs). Re-exported through `super` so the public paths stay
//! `dx_ci::{Reporting, RevisionRequest, PlannedRevision, RevisionError,
//! plan_revision}`. Distinct from the selection, scheduling, run,
//! thread, approval, aggregate, coverage, platform, caller, pin, audit,
//! artifact, metadata, and preset modules.

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
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RevisionError {
    /// PR head revision missing or empty.
    #[error("PR head revision is required")]
    MissingHead,
    /// PR target-branch revision missing or empty.
    #[error("PR target-branch revision is required")]
    MissingBase,
    /// Test-merge snapshot unavailable (merge conflict): validation is
    /// blocked. GitHub may suppress the PR workflow for conflicting PRs;
    /// the blocked-status path must not execute fork code with reporting
    /// privileges.
    #[error("proposed merge unavailable (conflict): validation is blocked, not head-only")]
    BlockedOnConflict,
    /// Default-branch landed revision missing or empty.
    #[error("landed revision is required for default-branch pushes")]
    MissingLanded,
    /// Default-branch name missing or empty.
    #[error("branch name is required")]
    MissingBranch,
    /// Ordinary non-default-branch pushes alone do not trigger the starter.
    #[error("non-default-branch pushes alone do not trigger the starter")]
    NotTriggered,
    /// Manual-dispatch revision missing or empty.
    #[error("dispatch revision is required for manual runs")]
    MissingDispatchRevision,
    /// Merge-queue combined revision missing or empty.
    #[error("combined queue revision is required for merge-queue runs")]
    MissingQueueRevision,
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
