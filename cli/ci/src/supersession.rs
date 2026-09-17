//! Supersession and queue-revision planning for consumer CI (issue #236,
//! M27 WP1 slice 4).
//!
//! Split from `super` (`lib.rs`): owns [`RunScope`], [`TrackedRun`],
//! [`supersedes`] (only a newer run for the same PR integration scope
//! supersedes — same opaque `pr` identity, strictly greater `seq`, in
//! either scheduling mode; failures without a new commit never
//! supersede), [`may_publish`] (only the current run writes the live
//! summary/threads), and [`result_satisfies`] (results bind to the exact
//! tested snapshot). Re-exported through `super` so the public paths
//! stay `dx_ci::{RunScope, TrackedRun, supersedes, may_publish,
//! result_satisfies}`. Distinct from the revision, selection,
//! scheduling, reporting, fork/aggregate, rerun, caller, pin, audit,
//! artifact, metadata, and preset modules.

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
/// is the snapshot from [`super::PlannedRevision`] every selected check uses.
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
/// supersedes — independent cells still complete (see
/// [`super::aggregate_outcome`]).
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
