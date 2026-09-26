#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reporting {
    PrThreadsAndSummary,
    NoPrComments,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RevisionRequest<'a> {
    PullRequest {
        head: &'a str,
        base: &'a str,
        merged: &'a str,
        draft: bool,
    },
    PushDefault {
        landed: &'a str,
        branch: &'a str,
    },
    PushNonDefault {
        branch: &'a str,
    },
    ManualDispatch {
        revision: &'a str,
    },
    MergeQueue {
        combined: &'a str,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedRevision {
    pub validated: String,
    pub head: Option<String>,
    pub base: Option<String>,
    pub reporting: Reporting,
}

impl PlannedRevision {
    pub fn snapshot_identity(&self) -> (String, Option<String>, Option<String>) {
        (self.validated.clone(), self.head.clone(), self.base.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RevisionError {
    #[error("PR head revision is required")]
    MissingHead,
    #[error("PR target-branch revision is required")]
    MissingBase,
    #[error("proposed merge unavailable (conflict): validation is blocked, not head-only")]
    BlockedOnConflict,
    #[error("landed revision is required for default-branch pushes")]
    MissingLanded,
    #[error("branch name is required")]
    MissingBranch,
    #[error("non-default-branch pushes alone do not trigger the starter")]
    NotTriggered,
    #[error("dispatch revision is required for manual runs")]
    MissingDispatchRevision,
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
