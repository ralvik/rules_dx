//! Audit rendering planning for consumer CI (issue #236, M27 WP4 slice 10).
//!
//! Split from `super` (`lib.rs`): owns [`AuditFinding`],
//! [`AuditPlacement`], [`audit_uses_counts_only_mode`],
//! [`audit_has_disclosure_toggle`], [`audit_assumes_private`],
//! [`audit_claims_generic_redaction`], [`plan_audit_placement`], and
//! [`audit_body_publishable`]. Re-exported through `super` so the public
//! paths stay `dx_ci::{AuditFinding, AuditPlacement, ...}`. Distinct from
//! the selection, revision, scheduling, supersession, reporting,
//! fork/aggregate, rerun, trigger, caller/pin, artifact, metadata,
//! thread-delta, and preset modules.

/// One audit finding with preserved presentation inputs.
///
/// `severity` and `acceptance` are opaque verbatim strings: this crate
/// preserves them into reporting without interpreting scales or acceptance
/// vocabularies (those freeze with analyzer qualification, not here).
/// `location` is `Some` only for findings with a valid PR-diff location.
/// `contains_restricted_content` marks bodies that must not be published
/// verbatim (credential values, secret values, or privately reported
/// vulnerability material, as classified by the caller-supplied input —
/// this crate claims no generic redaction).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditFinding {
    /// Stable finding identity.
    pub id: String,
    /// Opaque severity spelling, preserved verbatim.
    pub severity: String,
    /// Opaque risk-acceptance spelling, preserved verbatim.
    pub acceptance: String,
    /// Whether this finding contributes to its check's failure.
    pub contributes_to_failure: bool,
    /// Valid PR-diff location, if mappable.
    pub location: Option<String>,
    /// Whether the finding body must not be published verbatim.
    pub contains_restricted_content: bool,
}

/// Where one audit finding is presented.
///
/// Audit uses the same presentation as every other check: diff-mapped
/// findings receive review details; all other findings stay in full reports
/// and summary counts. There is no security-only counts mode and no
/// disclosure toggle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditPlacement {
    /// Diff-mapped finding: review details plus report/count retention.
    ReviewThread,
    /// Finding without a valid diff location: reports and counts only.
    ReportOnly,
}

/// Audit never uses a security-only counts mode.
pub fn audit_uses_counts_only_mode() -> bool {
    false
}

/// Audit has no disclosure toggle.
pub fn audit_has_disclosure_toggle() -> bool {
    false
}

/// Safe rendering never assumes all security findings are private: public
/// repository reporting is not a confidential channel.
pub fn audit_assumes_private() -> bool {
    false
}

/// This crate claims no unqualified generic redaction guarantee.
pub fn audit_claims_generic_redaction() -> bool {
    false
}

/// Plan where one audit finding is presented.
///
/// A valid diff location plans [`AuditPlacement::ReviewThread`]; anything
/// else plans [`AuditPlacement::ReportOnly`] with no invented location.
/// Restricted bodies keep their placement and counts — only the verbatim
/// body is withheld (see [`audit_body_publishable`]) — so completeness and
/// command outcomes are preserved.
pub fn plan_audit_placement(finding: &AuditFinding) -> AuditPlacement {
    if finding.location.is_some() {
        AuditPlacement::ReviewThread
    } else {
        AuditPlacement::ReportOnly
    }
}

/// Whether the finding body may be published verbatim into review details,
/// reports, or summaries.
///
/// Findings flagged with `contains_restricted_content` must not expose
/// credential values, secret values, or privately reported vulnerability
/// material: callers withhold the verbatim body while retaining the finding
/// in counts, completeness, and command outcomes. Unflagged findings are
/// publishable as-is; public reporting is otherwise not confidential.
pub fn audit_body_publishable(finding: &AuditFinding) -> bool {
    !finding.contains_restricted_content
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audit_finding(id: &str, located: bool, restricted: bool) -> AuditFinding {
        AuditFinding {
            id: id.to_owned(),
            severity: "high".to_owned(),
            acceptance: "unaccepted".to_owned(),
            contributes_to_failure: true,
            location: if located {
                Some(format!("src/lib.rs:{id}"))
            } else {
                None
            },
            contains_restricted_content: restricted,
        }
    }

    #[test]
    fn audit_diff_mapped_findings_receive_review_details() {
        let finding = audit_finding("audit-1", true, false);
        assert_eq!(plan_audit_placement(&finding), AuditPlacement::ReviewThread);
        assert!(audit_body_publishable(&finding));
    }

    #[test]
    fn audit_locationless_findings_stay_in_reports_and_counts() {
        let finding = audit_finding("audit-2", false, false);
        assert_eq!(plan_audit_placement(&finding), AuditPlacement::ReportOnly);
        assert!(audit_body_publishable(&finding));
    }

    #[test]
    fn audit_preserves_severity_acceptance_and_outcome() {
        let finding = AuditFinding {
            id: "audit-3".to_owned(),
            severity: "critical:custom-scale".to_owned(),
            acceptance: "accepted:risk-42".to_owned(),
            contributes_to_failure: false,
            location: Some("src/lib.rs:audit-3".to_owned()),
            contains_restricted_content: false,
        };
        assert_eq!(plan_audit_placement(&finding), AuditPlacement::ReviewThread);
        assert_eq!(finding.severity, "critical:custom-scale");
        assert_eq!(finding.acceptance, "accepted:risk-42");
        assert!(!finding.contributes_to_failure);
    }

    #[test]
    fn audit_restricted_bodies_withheld_without_hiding_counts() {
        let mapped = audit_finding("audit-secret", true, true);
        assert_eq!(plan_audit_placement(&mapped), AuditPlacement::ReviewThread);
        assert!(!audit_body_publishable(&mapped));
        assert!(mapped.contributes_to_failure);
        let unmapped = audit_finding("audit-secret-offdiff", false, true);
        assert_eq!(plan_audit_placement(&unmapped), AuditPlacement::ReportOnly);
        assert!(!audit_body_publishable(&unmapped));
    }

    #[test]
    fn audit_has_no_counts_only_mode_disclosure_toggle_or_private_guarantee() {
        assert!(!audit_uses_counts_only_mode());
        assert!(!audit_has_disclosure_toggle());
        assert!(!audit_assumes_private());
        assert!(!audit_claims_generic_redaction());
    }
}
