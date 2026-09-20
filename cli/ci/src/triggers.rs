//! Starter triggers, platform qualification mechanics, no-op semantics,
//! and latest-target revalidation for consumer CI (M27 WP2 slice 8).
//!
//! Split from `super` (`lib.rs`): owns [`starter_triggers`] (PRs, default
//! pushes, manual dispatches, and merge-queue runs trigger; ordinary
//! non-default pushes do not), [`PlatformError`], [`validate_platforms`]
//! (explicit nonempty supported spellings pass through verbatim; no
//! implicit default or substitution), [`is_collection_failure`], and
//! [`base_advanced_requires_rerun`]. Re-exported through `super` so the
//! public paths stay `dx_ci::{starter_triggers, PlatformError,
//! validate_platforms, is_collection_failure,
//! base_advanced_requires_rerun}`. Distinct from the revision, selection,
//! scheduling, supersession, reporting, fork/aggregate, rerun, caller,
//! pin, audit, artifact, metadata, and preset modules.

use super::RevisionRequest;

/// Whether the starter triggers on this revision request.
///
/// PRs (draft or ready), default-branch pushes, manual dispatches, and
/// consumer-enabled merge-queue runs trigger; ordinary non-default-branch
/// pushes alone do not (`docs/testing/github-ci.md#starter-triggers`).
/// Branch names stay opaque: a consumer whose default branch is not
/// `"main"` triggers identically on its own default branch.
pub fn starter_triggers(request: &RevisionRequest<'_>) -> bool {
    match request {
        RevisionRequest::PullRequest { .. }
        | RevisionRequest::PushDefault { .. }
        | RevisionRequest::ManualDispatch { .. }
        | RevisionRequest::MergeQueue { .. } => true,
        RevisionRequest::PushNonDefault { .. } => false,
    }
}

/// Malformed platform selection against an injected supported set.
///
/// The supported identities stay injected here: the frozen runner/platform
/// mapping arrives with workflow qualification (`docs/github-ci.md#qualification`).
/// This plans only the mechanics — explicit nonempty selection, no implicit
/// default or substitution, verbatim spellings — over caller-supplied sets.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PlatformError {
    /// No platforms supplied while a per-platform check is enabled.
    #[error("explicit platform selection is required when test, build, or coverage is enabled")]
    Empty,
    /// A supplied spelling is outside the injected supported set.
    #[error("unsupported platform {value:?}")]
    Unsupported { value: String },
}

/// Validate an explicit platform selection against an injected supported set.
///
/// Fails closed on missing/empty selections and on any unsupported spelling
/// (no skipped validation or platform substitution). Single- and
/// multi-platform selections pass through verbatim in caller order,
/// including selections without Linux. Linux-once quality scope is planned
/// by [`super::plan_schedule`], not by adding Linux here.
pub fn validate_platforms(
    platforms: &[String],
    supported: &[String],
) -> Result<Vec<String>, PlatformError> {
    if platforms.is_empty() {
        return Err(PlatformError::Empty);
    }
    for platform in platforms {
        if !supported.iter().any(|known| known == platform) {
            return Err(PlatformError::Unsupported {
                value: platform.clone(),
            });
        }
    }
    Ok(platforms.to_vec())
}

/// Whether a missing report event is a collection failure.
///
/// A configured no-op without a report event is expected, not a collection
/// failure; any other missing report is. Failures and counts flow through
/// [`super::reporting_gate`] and [`super::plan_summary`] unchanged.
pub fn is_collection_failure(has_report_event: bool, configured_noop: bool) -> bool {
    !has_report_event && !configured_noop
}

/// Whether target-branch advancement requires a fresh validation run.
///
/// The native up-to-date-branch gate (or merge queue validating the current
/// combined revision) owns enforcement; this plans only the identity rule:
/// any base change invalidates the earlier merge snapshot, which stays
/// truthful for its tested combination but never satisfies the new one
/// (see [`super::result_satisfies`]). Revisions stay opaque strings.
pub fn base_advanced_requires_rerun(old_base: &str, new_base: &str) -> bool {
    old_base != new_base
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plan_revision, RevisionRequest};

    #[test]
    fn starter_triggers_cover_pr_push_dispatch_queue_but_not_feature_push() {
        assert!(starter_triggers(&RevisionRequest::PullRequest {
            head: "h",
            base: "b",
            merged: "m",
            draft: false,
        }));
        assert!(starter_triggers(&RevisionRequest::PushDefault {
            landed: "l",
            branch: "trunk",
        }));
        assert!(!starter_triggers(&RevisionRequest::PushNonDefault {
            branch: "feature-x",
        }));
        assert!(starter_triggers(&RevisionRequest::ManualDispatch {
            revision: "r",
        }));
        assert!(starter_triggers(&RevisionRequest::MergeQueue {
            combined: "q"
        }));
    }

    #[test]
    fn custom_default_branch_triggers_like_main() {
        // The default branch may carry any name; opaque spellings pass
        // through identically.
        let custom = plan_revision(RevisionRequest::PushDefault {
            landed: "landed-sha",
            branch: "trunk",
        })
        .expect("custom default branch plans");
        assert_eq!(custom.validated, "landed-sha");
        assert!(starter_triggers(&RevisionRequest::PushDefault {
            landed: "l",
            branch: "trunk",
        }));
    }

    #[test]
    fn platform_validation_rejects_empty_and_unsupported_without_substitution() {
        let supported = vec!["linux_x86_64".to_owned(), "macos_arm64".to_owned()];
        assert_eq!(
            validate_platforms(&[], &supported),
            Err(PlatformError::Empty)
        );
        assert_eq!(
            validate_platforms(&["windows_x86_64".to_owned()], &supported),
            Err(PlatformError::Unsupported {
                value: "windows_x86_64".to_owned(),
            })
        );
        // Single- and multi-platform selections pass through verbatim,
        // including one without Linux.
        assert_eq!(
            validate_platforms(&["macos_arm64".to_owned()], &supported),
            Ok(vec!["macos_arm64".to_owned()])
        );
        assert_eq!(
            validate_platforms(
                &["linux_x86_64".to_owned(), "macos_arm64".to_owned()],
                &supported
            ),
            Ok(vec!["linux_x86_64".to_owned(), "macos_arm64".to_owned()])
        );
    }

    #[test]
    fn configured_noop_without_report_is_not_a_collection_failure() {
        assert!(!is_collection_failure(false, true));
        assert!(is_collection_failure(false, false));
        assert!(!is_collection_failure(true, false));
        assert!(!is_collection_failure(true, true));
    }

    #[test]
    fn base_advancement_requires_a_fresh_run() {
        assert!(!base_advanced_requires_rerun("base-a", "base-a"));
        assert!(base_advanced_requires_rerun("base-a", "base-b"));
    }
}
