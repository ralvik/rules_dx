//! Rerun, scope, retry, and code-scanning planning for consumer CI (issue
//! #236, M27 WP1 slice 7).
//!
//! Split from `super` (`lib.rs`): owns [`plan_rerun`] (GitHub native rerun
//! preserves selection, revision identity, and reporting semantics),
//! [`scope_runs_all_selected`], [`uses_path_filters`] (never),
//! [`ReportRetry`], [`plan_report_retry`] (bounded transient retries reuse
//! the same identified results; exhaustion retains the reporting failure
//! for [`super::reporting_gate`]), [`CODE_SCANNING_DEFAULT`],
//! [`CodeScanningPlan`], [`plan_code_scanning`], and
//! [`plan_coverage_aggregate`] (every selected platform must report; no
//! gap hiding). Re-exported through `super` so the public paths stay
//! `dx_ci::{plan_rerun, scope_runs_all_selected, uses_path_filters,
//! ReportRetry, plan_report_retry, CODE_SCANNING_DEFAULT, CodeScanningPlan,
//! plan_code_scanning, plan_coverage_aggregate}`. Distinct from the
//! revision, selection, scheduling, supersession, reporting,
//! fork/aggregate, trigger, platform, caller, pin, audit, artifact,
//! metadata, and preset modules.

use super::PlannedRevision;

/// Planned rerun: GitHub's native rerun controls preserve selection,
/// revision identity, and reporting semantics.
///
/// A rerun reuses the original planned revision (same validated snapshot
/// plus head/base identities) and the original reporting mode. Reruns never
/// substitute a different revision, never retry analyzer failures until
/// green, and never accept bot comment commands.
pub fn plan_rerun(original: &PlannedRevision) -> PlannedRevision {
    original.clone()
}

/// Whether a change kind still executes every selected check.
///
/// Documentation-only and mixed changes invoke all selected checks at their
/// normal repository scope: no workflow path filters and no second
/// affected-target calculation. Diff-based review placement never narrows
/// analysis scope.
pub fn scope_runs_all_selected(docs_only: bool) -> bool {
    let _ = docs_only;
    true
}

/// Workflow path filters are never used.
pub fn uses_path_filters() -> bool {
    false
}

/// Planned outcome of bounded transient reporting-transport retries.
///
/// Retries reuse the same identified results and never duplicate comments,
/// change analyzer outcomes, or hide the final reporting failure. Only
/// `transient_failures <= bound` are retried; exhaustion retains the
/// reporting failure for the gate in [`super::reporting_gate`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReportRetry {
    /// Whether the transport was retried with the same identified results.
    pub retried_same_results: bool,
    /// Whether the final reporting failure is retained (exhaustion).
    pub failure_retained: bool,
    /// Retries never duplicate integration comments.
    pub duplicates_comments: bool,
    /// Retries never change analyzer outcomes.
    pub changes_analysis: bool,
}

/// Plan bounded transient reporting retries.
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

/// Code Scanning SARIF publication is off in the starter.
pub const CODE_SCANNING_DEFAULT: bool = false;

/// Code-scanning publication planning for an explicit opt-in.
///
/// Default reporting works without Code Scanning permissions or paid
/// security features, and disabled publication is never a reporting
/// failure. After opt-in on an eligible repository, complete scans remain
/// eligible for authoritative upload even when findings fail the command;
/// incomplete scans must not replace the authoritative scan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeScanningPlan {
    /// Publication disabled: not a failure.
    Disabled,
    /// Complete scan may upload authoritatively.
    Upload,
    /// Incomplete scan must not replace the authoritative scan.
    MustNotUpload,
}

/// Plan Code Scanning publication.
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
///
/// Every selected platform must report its coverage cell; combining reports
/// with any platform missing or failing is not success.
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
