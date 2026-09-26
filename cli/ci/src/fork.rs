pub const AGGREGATE_CHECK: &str = "dx-ci";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApproverRole {
    Maintainer,
    Collaborator,
    OutsideAuthor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForkCredentials {
    ReadOnly,
}

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

pub fn fork_credentials() -> ForkCredentials {
    ForkCredentials::ReadOnly
}

pub fn privileged_reporting_executes_fork_code() -> bool {
    false
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plan_schedule, plan_selection, SchedulingMode};

    #[test]
    fn aggregate_identity_is_stable_across_selection_and_mode() {
        assert_eq!(AGGREGATE_CHECK, "dx-ci");
        let full = plan_selection(&[], &["linux_x86_64".to_owned()]).expect("plans");
        let narrow = plan_selection(
            &["coverage".to_owned(), "test".to_owned(), "build".to_owned()],
            &["linux_x86_64".to_owned()],
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
        assert!(plan_aggregate(&[Success], false, false));
    }
}
