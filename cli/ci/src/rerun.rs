use super::PlannedRevision;

pub fn plan_rerun(original: &PlannedRevision) -> PlannedRevision {
    original.clone()
}

pub fn scope_runs_all_selected(docs_only: bool) -> bool {
    let _ = docs_only;
    true
}

/// Workflow path filters are never used.
pub fn uses_path_filters() -> bool {
    false
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReportRetry {
    pub retried_same_results: bool,
    pub failure_retained: bool,
    pub duplicates_comments: bool,
    pub changes_analysis: bool,
}

pub fn plan_report_retry(transient_failures: u32, bound: u32) -> ReportRetry {
    if transient_failures == 0 {
        ReportRetry {
            retried_same_results: false,
            failure_retained: false,
            duplicates_comments: false,
            changes_analysis: false,
        }
    } else if transient_failures <= bound {
        ReportRetry {
            retried_same_results: true,
            failure_retained: false,
            duplicates_comments: false,
            changes_analysis: false,
        }
    } else {
        ReportRetry {
            retried_same_results: true,
            failure_retained: true,
            duplicates_comments: false,
            changes_analysis: false,
        }
    }
}

pub const CODE_SCANNING_DEFAULT: bool = false;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeScanningPlan {
    Disabled,
    Upload,
    MustNotUpload,
}

pub fn plan_code_scanning(opt_in: bool, complete_scan: bool) -> CodeScanningPlan {
    if !opt_in {
        return CodeScanningPlan::Disabled;
    }
    if complete_scan {
        CodeScanningPlan::Upload
    } else {
        CodeScanningPlan::MustNotUpload
    }
}

/// Coverage aggregation never hides a missing platform or gap.
pub fn plan_coverage_aggregate(per_platform_ok: &[bool]) -> bool {
    !per_platform_ok.is_empty() && per_platform_ok.iter().all(|ok| *ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plan_revision, Reporting, RevisionRequest};

    #[test]
    fn reruns_preserve_revision_and_reporting_identity() {
        let original = plan_revision(RevisionRequest::PullRequest {
            head: "head-sha",
            base: "base-sha",
            merged: "merge-sha",
            draft: true,
        })
        .expect("plans");
        let rerun = plan_rerun(&original);
        assert_eq!(rerun, original);
        assert_eq!(rerun.validated, "merge-sha");
        assert_eq!(rerun.reporting, Reporting::PrThreadsAndSummary);
    }

    #[test]
    fn docs_only_changes_still_run_everything_without_path_filters() {
        assert!(scope_runs_all_selected(true));
        assert!(scope_runs_all_selected(false));
        assert!(!uses_path_filters());
    }

    #[test]
    fn bounded_retries_reuse_results_and_retain_exhaustion() {
        let none = plan_report_retry(0, 3);
        assert!(!none.retried_same_results);
        assert!(!none.failure_retained);
        let bounded = plan_report_retry(2, 3);
        assert!(bounded.retried_same_results);
        assert!(!bounded.failure_retained);
        assert!(!bounded.duplicates_comments);
        assert!(!bounded.changes_analysis);
        let exhausted = plan_report_retry(4, 3);
        assert!(exhausted.retried_same_results);
        assert!(exhausted.failure_retained);
        assert!(!exhausted.duplicates_comments);
        assert!(!exhausted.changes_analysis);
    }

    #[test]
    fn code_scanning_defaults_off_and_never_fails_when_disabled() {
        assert!(!CODE_SCANNING_DEFAULT);
        assert_eq!(plan_code_scanning(false, true), CodeScanningPlan::Disabled);
        assert_eq!(plan_code_scanning(true, true), CodeScanningPlan::Upload);
        assert_eq!(
            plan_code_scanning(true, false),
            CodeScanningPlan::MustNotUpload
        );
    }

    #[test]
    fn coverage_aggregation_hides_no_platform_gap() {
        assert!(plan_coverage_aggregate(&[true, true]));
        assert!(!plan_coverage_aggregate(&[true, false]));
        assert!(!plan_coverage_aggregate(&[]));
    }
}
