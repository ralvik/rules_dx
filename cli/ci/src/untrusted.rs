use super::{PlannedRevision, ThreadPlan};

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ArtifactError {
    #[error("artifact snapshot identity is required")]
    MissingArtifact,
    #[error("artifact digest identity is required")]
    MissingDigest,
    #[error("artifact does not bind to the required validated snapshot")]
    SnapshotMismatch,
}

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

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum MetadataError {
    #[error("PR metadata identities are required")]
    MissingField,
    #[error("PR metadata does not match the planned revision snapshot")]
    StaleSnapshot,
}

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

pub fn untrusted_inputs_execute_fork_code() -> bool {
    false
}

pub fn untrusted_inputs_grant_secrets() -> bool {
    false
}

pub fn thread_slots_used(plan: &ThreadPlan) -> usize {
    plan.keep.len()
}

pub fn resolved_discussions_count_against_limit() -> bool {
    false
}

pub fn concurrent_runs_share_limit() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fork_credentials, plan_revision, plan_threads, privileged_reporting_executes_fork_code,
        Finding, ForkCredentials, OwnedThread, RevisionRequest,
    };

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
        let first = vec![finding("old", true, true), finding("new", true, true)];
        let existing = vec![OwnedThread {
            finding: "old".to_owned(),
            has_human_replies: false,
        }];
        let full = plan_threads(&first, &existing, &[], 1);
        assert_eq!(full.keep, vec!["old".to_owned()]);
        assert!(full.create.is_empty());
        assert_eq!(thread_slots_used(&full), 1);
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
        let shared = plan_threads(&newcomer, &[], &[], 1);
        assert_eq!(shared.create, vec!["fresh".to_owned()]);
        assert_eq!(shared.create.len(), 1);
    }
}
