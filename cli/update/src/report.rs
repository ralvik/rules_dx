//! Aggregate exit-status selection for `dx update` (M26 WP2 slice 4).
//!
//! Pure mapping from the aggregated per-set [`UpdateReport`] in
//! [`outcome`] to the process exit code, per the accepted update
//! contract (`docs/cli/commands/audit-update-bazel.md#dx-update`) and
//! the common exit-status contract (`docs/cli/cli-contract.md#exit-status`):
//! any failed selected set fails the invocation overall with exit `1`
//! (the CLI-originated operational-failure class); a run with no
//! failure exits `0` with successful changes preserved.
//!
//! Exactly one aggregate code exists: per-set success/failure/blocked
//! detail rides the per-set report (which lands with the resolver
//! backends), never a per-set exit code. Usage, workspace, scope, and
//! option errors stay exit `2` at the CLI layer before aggregation;
//! signal forwarding is untouched. There is no Bazel-subprocess code
//! to preserve here: the update aggregate owns the invocation status
//! under the narrow continuation exception, not the first-failure
//! rule. Backend operation boundaries and per-set reporting stay O12
//! qualification; this module maps over injected reports only, so the
//! selection stays deterministic and unit-testable without any updater.

use super::outcome::UpdateReport;

/// Success exit code: no selected set failed.
pub const EXIT_SUCCESS: i32 = 0;

/// Overall-failure exit code: at least one selected set failed.
pub const EXIT_FAILURE: i32 = 1;

/// Select the aggregate exit code for one update run.
///
/// The verdict derives from the report's `overall_failure` flag, the
/// single source of truth computed by [`outcome::aggregate`] (blocked
/// alone never sets it; any failure implies it). A hand-built report
/// with blocked entries but no failure therefore exits `0`: blocked
/// without a failed dependency is not a failure, and only an actual
/// failure fails the run.
pub fn exit_code(report: &UpdateReport) -> i32 {
    if report.overall_failure {
        EXIT_FAILURE
    } else {
        EXIT_SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::super::outcome::{ReportedOutcome, ReportedStatus};
    use super::*;

    fn report(outcomes: &[(&str, ReportedStatus)], overall_failure: bool) -> UpdateReport {
        UpdateReport {
            outcomes: outcomes
                .iter()
                .map(|(set, status)| ReportedOutcome {
                    set: (*set).to_owned(),
                    status: status.clone(),
                })
                .collect(),
            overall_failure,
        }
    }

    #[test]
    fn clean_run_exits_zero() {
        let clean = report(&[("cargo-lock", ReportedStatus::Success)], false);
        assert_eq!(exit_code(&clean), EXIT_SUCCESS);
        assert_eq!(exit_code(&clean), 0);
    }

    #[test]
    fn empty_selection_exits_zero() {
        assert_eq!(exit_code(&report(&[], false)), 0);
    }

    #[test]
    fn any_failure_exits_one() {
        let failed = report(
            &[
                ("cargo-lock", ReportedStatus::Success),
                ("npm-root", ReportedStatus::Failed),
            ],
            true,
        );
        assert_eq!(exit_code(&failed), EXIT_FAILURE);
        assert_eq!(exit_code(&failed), 1);
    }

    #[test]
    fn failure_with_blocked_dependents_exits_one() {
        let failed = report(
            &[
                ("base-set", ReportedStatus::Failed),
                ("app-set", ReportedStatus::Blocked),
            ],
            true,
        );
        assert_eq!(exit_code(&failed), 1);
    }

    #[test]
    fn verdict_follows_overall_flag_not_blocked_presence() {
        // `aggregate` never produces blocked without a failure, but the
        // mapping must honor the flag alone: blocked without failure is
        // not a failure.
        let blocked_only = report(&[("app-set", ReportedStatus::Blocked)], false);
        assert_eq!(exit_code(&blocked_only), 0);
    }
}
