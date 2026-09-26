#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunScope {
    PullRequest {
        pr: String,
    },
    DefaultBranch,
    Manual {
        id: String,
    },
    Queue {
        id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackedRun {
    pub scope: RunScope,
    pub validated: String,
    pub seq: u64,
}

/// Whether `incoming` supersedes `current` (the old run must cancel).
pub fn supersedes(current: &TrackedRun, incoming: &TrackedRun) -> bool {
    match (&current.scope, &incoming.scope) {
        (RunScope::PullRequest { pr: current_pr }, RunScope::PullRequest { pr: incoming_pr }) => {
            current_pr == incoming_pr && incoming.seq > current.seq
        }
        _ => false,
    }
}

pub fn may_publish(finished: &TrackedRun, current: &TrackedRun) -> bool {
    finished.scope == current.scope
        && finished.seq == current.seq
        && finished.validated == current.validated
}

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
