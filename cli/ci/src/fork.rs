//! Fork-security and aggregate-gating planning for consumer CI (M27 WP1 slice 6).
//!
//! Split from `super` (`lib.rs`): owns [`AGGREGATE_CHECK`] (stable branch
//! protection identity), [`ApproverRole`], [`ForkCredentials`],
//! [`plan_approval`] (outside authors cannot self-approve; only
//! receiving-repository maintainers/collaborators approve gated runs),
//! [`fork_credentials`] (approved fork execution stays read-only),
//! [`privileged_reporting_executes_fork_code`] (never), [`AggregateState`],
//! and [`plan_aggregate`] (success requires every selected cell plus
//! required reporting; see [`super::reporting_gate`]). Re-exported
//! through `super` so the public paths stay
//! `dx_ci::{AGGREGATE_CHECK, ApproverRole, ForkCredentials, plan_approval,
//! fork_credentials, privileged_reporting_executes_fork_code,
//! AggregateState, plan_aggregate}`. Distinct from the revision,
//! selection, scheduling, supersession, reporting, rerun, caller, pin,
//! audit, artifact, metadata, and preset modules.

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
/// successfully (see [`super::reporting_gate`]). The identity ([`AGGREGATE_CHECK`])
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
}
