//! Aggregate audit outcome and exit-status selection (WP1 slice 4).
//!
//! Pure dx-level aggregation over injected per-family results, per the
//! accepted audit failure policy
//! (`docs/cli/commands/audit-update-bazel.md#dx-audit`): unexempted
//! findings fail the audit, incomplete assessment fails the audit, and
//! only a fully assessed run with no unexempted findings passes. The
//! aggregate exit code follows the common exit-status contract
//! (`docs/cli/cli-contract.md#exit-status`): clean exits `0`;
//! findings or incomplete assessment exit `1` (the quality-policy
//! failure class). Usage, workspace, scope, and option errors stay
//! exit `2` at the CLI layer before aggregation; signal forwarding is
//! untouched.
//!
//! The findings-versus-operational-error split lives in the report,
//! not the code: both fail the command, and the SARIF/event detail
//! says which. Family-result production runs live (auditor wiring in
//! [`crate::backend`], advisory acquisition in [`crate::advisory`],
//! SARIF triage in [`crate::secrets`], matching in [`crate::vuln`],
//! license evaluation in [`crate::license_expr`], SARIF/SPDX mapping
//! pinned under issue #632); this module aggregates over injected
//! family outcomes only, so the selection stays deterministic and
//! unit-testable without any auditor.

use super::AuditFamily;

/// Terminal status of one audit family in a run, as classified by the
/// future auditor integration. Finding exemption (valid risk
/// acceptance) and assessment completeness are decided before this
/// module: it aggregates verdicts, never evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FamilyStatus {
    /// Assessed with no unexempted findings.
    Clean,
    /// At least one unexempted finding (accepted/suppressed findings
    /// stay visible but do not produce this status).
    Findings,
    /// The family could not assess every selected dependency; an empty
    /// findings list is not evidence of coverage.
    Incomplete,
}

/// One family's aggregated result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyOutcome {
    /// Family that ran.
    pub family: AuditFamily,
    /// Terminal status for that family.
    pub status: FamilyStatus,
}

/// Aggregated audit run: one outcome per family that ran, in canonical
/// family order (security first).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditReport {
    /// Per-family outcomes, canonical order.
    pub outcomes: Vec<FamilyOutcome>,
}

/// Success exit code: every family assessed clean.
pub const EXIT_SUCCESS: i32 = 0;

/// Failure exit code: findings or incomplete assessment.
pub const EXIT_FAILURE: i32 = 1;

impl AuditFamily {
    /// Canonical aggregation order: security runs first.
    fn order(self) -> u8 {
        match self {
            AuditFamily::Security => 0,
            AuditFamily::License => 1,
        }
    }
}

impl AuditReport {
    /// Aggregate one audit run over the families that ran. Outcomes
    /// sort into canonical family order for determinism.
    pub fn aggregate(outcomes: Vec<FamilyOutcome>) -> AuditReport {
        let mut sorted = outcomes;
        sorted.sort_by_key(|outcome| outcome.family.order());
        AuditReport { outcomes: sorted }
    }

    /// True when every family assessed clean: the only passing state.
    pub fn is_clean(&self) -> bool {
        !self.outcomes.is_empty()
            && self
                .outcomes
                .iter()
                .all(|outcome| outcome.status == FamilyStatus::Clean)
    }

    /// Families reporting unexempted findings, in canonical order.
    pub fn with_findings(&self) -> Vec<AuditFamily> {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.status == FamilyStatus::Findings)
            .map(|outcome| outcome.family)
            .collect()
    }

    /// Families that could not complete assessment, in canonical order.
    pub fn incomplete(&self) -> Vec<AuditFamily> {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.status == FamilyStatus::Incomplete)
            .map(|outcome| outcome.family)
            .collect()
    }
}

/// Select the aggregate exit code for one audit run: `0` only for a
/// fully assessed run with no unexempted findings, `1` otherwise. An
/// empty report (no family ran) exits `0`: nothing failed and nothing
/// claims coverage.
pub fn exit_code(report: &AuditReport) -> i32 {
    if report.outcomes.is_empty() || report.is_clean() {
        EXIT_SUCCESS
    } else {
        EXIT_FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(family: AuditFamily, status: FamilyStatus) -> FamilyOutcome {
        FamilyOutcome { family, status }
    }

    #[test]
    fn clean_run_exits_zero() {
        let report = AuditReport::aggregate(vec![
            outcome(AuditFamily::Security, FamilyStatus::Clean),
            outcome(AuditFamily::License, FamilyStatus::Clean),
        ]);
        assert!(report.is_clean());
        assert!(report.with_findings().is_empty());
        assert!(report.incomplete().is_empty());
        assert_eq!(exit_code(&report), EXIT_SUCCESS);
        assert_eq!(exit_code(&report), 0);
    }

    #[test]
    fn findings_exit_one_and_name_the_family() {
        let report = AuditReport::aggregate(vec![
            outcome(AuditFamily::Security, FamilyStatus::Findings),
            outcome(AuditFamily::License, FamilyStatus::Clean),
        ]);
        assert!(!report.is_clean());
        assert_eq!(report.with_findings(), vec![AuditFamily::Security]);
        assert_eq!(exit_code(&report), EXIT_FAILURE);
        assert_eq!(exit_code(&report), 1);
    }

    #[test]
    fn incomplete_assessment_exits_one() {
        let report = AuditReport::aggregate(vec![
            outcome(AuditFamily::Security, FamilyStatus::Clean),
            outcome(AuditFamily::License, FamilyStatus::Incomplete),
        ]);
        assert!(!report.is_clean());
        assert_eq!(report.incomplete(), vec![AuditFamily::License]);
        assert_eq!(exit_code(&report), 1);
    }

    #[test]
    fn aggregation_sorts_canonical_security_first() {
        let report = AuditReport::aggregate(vec![
            outcome(AuditFamily::License, FamilyStatus::Clean),
            outcome(AuditFamily::Security, FamilyStatus::Clean),
        ]);
        let order: Vec<AuditFamily> = report
            .outcomes
            .iter()
            .map(|outcome| outcome.family)
            .collect();
        assert_eq!(order, vec![AuditFamily::Security, AuditFamily::License]);
    }

    #[test]
    fn empty_report_exits_zero_without_claiming_clean() {
        let report = AuditReport::aggregate(vec![]);
        assert!(!report.is_clean());
        assert_eq!(exit_code(&report), 0);
    }
}
