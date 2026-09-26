use super::RevisionRequest;

pub fn starter_triggers(request: &RevisionRequest<'_>) -> bool {
    match request {
        RevisionRequest::PullRequest { .. }
        | RevisionRequest::PushDefault { .. }
        | RevisionRequest::ManualDispatch { .. }
        | RevisionRequest::MergeQueue { .. } => true,
        RevisionRequest::PushNonDefault { .. } => false,
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PlatformError {
    #[error("explicit platform selection is required when any check is enabled")]
    Empty,
    #[error("unsupported platform {value:?}")]
    Unsupported { value: String },
}

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

pub fn is_collection_failure(has_report_event: bool, configured_noop: bool) -> bool {
    !has_report_event && !configured_noop
}

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
