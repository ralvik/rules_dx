//! Untrusted-artifact/metadata validation and thread-accounting deltas for
//! consumer CI (WP4 slice 11).
//!
//! Split from `super` (`lib.rs`): owns [`ArtifactError`],
//! [`validate_artifact_snapshot`], [`MetadataError`],
//! [`validate_pr_metadata`], [`untrusted_inputs_execute_fork_code`],
//! [`untrusted_inputs_grant_secrets`], [`thread_slots_used`],
//! [`resolved_discussions_count_against_limit`], and
//! [`concurrent_runs_share_limit`]. Re-exported through `super` so the
//! public paths stay `dx_ci::{ArtifactError, validate_artifact_snapshot,
//! ...}`. Distinct from the selection, revision, scheduling, supersession,
//! reporting, fork/aggregate, rerun, trigger, caller/pin, audit, and preset
//! modules.

use super::{PlannedRevision, ThreadPlan};

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
/// modify current threads (see [`super::may_publish`]). Non-PR runs carry no
/// PR metadata: callers must not invent head/base there.
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
/// per job, rerun, retry, or concurrent completion (see
/// [`super::may_publish`]: only the current run publishes).
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
}
