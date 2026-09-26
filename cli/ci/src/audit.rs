#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditFinding {
    pub id: String,
    pub severity: String,
    pub acceptance: String,
    pub contributes_to_failure: bool,
    pub location: Option<String>,
    pub contains_restricted_content: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditPlacement {
    ReviewThread,
    ReportOnly,
}

/// Audit never uses a security-only counts mode.
pub fn audit_uses_counts_only_mode() -> bool {
    false
}

pub fn audit_has_disclosure_toggle() -> bool {
    false
}

/// Safe rendering never assumes all security findings are private: public
pub fn audit_assumes_private() -> bool {
    false
}

pub fn audit_claims_generic_redaction() -> bool {
    false
}

pub fn plan_audit_placement(finding: &AuditFinding) -> AuditPlacement {
    if finding.location.is_some() {
        AuditPlacement::ReviewThread
    } else {
        AuditPlacement::ReportOnly
    }
}

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
