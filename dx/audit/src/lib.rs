//! Pure `dx audit` request planning (M26 WP1 slice 1).
//!
//! This crate owns the audit command surface before any tool integration
//! lands: family selection, scope defaults, and the non-mutating marker.
//! It plans over injected argument strings only, so selection stays
//! deterministic and unit-testable without a workspace, a Bazel server,
//! or any auditor binary.
//!
//! Frozen command shape (`docs/cli/commands/audit-update-bazel.md`):
//! `dx audit [security|license] [scope ...]`. A bare invocation runs both
//! families (security first); an explicit family runs alone. With no scope
//! the audit selects `//...` independent of the current working directory
//! and never scans the filesystem for sources or lockfiles. Audit is
//! non-mutating: advisory refresh changes analysis inputs, never
//! application manifests, lockfiles, or projections.
//!
//! Out of scope here (O11/O12/O58 qualification): auditor tool wiring,
//! advisory snapshot acquisition, severity/report mappings, risk-exception
//! lifecycle, target-to-dependency-set resolution, and license-policy
//! evaluation. Those arrive in later M26 slices; this crate only records
//! which families run over which scope spellings.

/// Audit family selector. Frozen spellings match the `dx audit` contract
/// so CLI parsing and help text cannot drift from the qualified shape.
pub const SECURITY_FAMILY: &str = "security";
/// Audit family selector. Frozen spellings match the `dx audit` contract
/// so CLI parsing and help text cannot drift from the qualified shape.
pub const LICENSE_FAMILY: &str = "license";

/// Scope selected when the invocation names none: repository-wide audit
/// independent of the current working directory (no filesystem scan).
pub const DEFAULT_SCOPE: &str = "//...";

/// One audit family: secrets plus dependency-vulnerability analysis
/// (`security`), or dependency license-policy analysis (`license`).
/// Tool selection and report mappings stay pending O11/O58.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditFamily {
    Security,
    License,
}

impl AuditFamily {
    /// Canonical CLI spelling of the family.
    pub fn as_str(self) -> &'static str {
        match self {
            AuditFamily::Security => SECURITY_FAMILY,
            AuditFamily::License => LICENSE_FAMILY,
        }
    }

    /// Parse one family selector. Anything else is a scope or an error
    /// for the caller to classify; unknown families never silently
    /// become a both-families run.
    pub fn parse(text: &str) -> Option<AuditFamily> {
        match text {
            SECURITY_FAMILY => Some(AuditFamily::Security),
            LICENSE_FAMILY => Some(AuditFamily::License),
            _ => None,
        }
    }
}

/// Planned audit request: which families run over which scope spellings.
/// Scope resolution (labels, patterns, target-to-dependency-set mapping)
/// is deferred to later slices; the spellings are preserved verbatim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditRequest {
    /// Families to run, in execution order. Bare invocations carry both
    /// (security first); explicit invocations carry exactly one.
    pub families: Vec<AuditFamily>,
    /// Scope arguments after the optional family selector, verbatim.
    /// Empty means [`DEFAULT_SCOPE`].
    pub scopes: Vec<String>,
}

impl AuditRequest {
    /// Effective scopes: explicit spellings, or the `//...` default.
    pub fn effective_scopes(&self) -> Vec<String> {
        if self.scopes.is_empty() {
            vec![DEFAULT_SCOPE.to_owned()]
        } else {
            self.scopes.clone()
        }
    }

    /// Audit never mutates: pinned here so later tool wiring cannot
    /// reclassify the command as mutating by default.
    pub fn is_mutating() -> bool {
        false
    }
}

/// Malformed audit request: the first positional selects a family, so a
/// first positional that is neither family nor scope-shaped fails here
/// instead of silently narrowing the run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditPlanError {
    /// First argument is not a family and not scope-shaped.
    UnknownFamily { value: String },
}

impl std::fmt::Display for AuditPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditPlanError::UnknownFamily { value } => {
                write!(
                    f,
                    "unknown audit family {value:?}; want `{SECURITY_FAMILY}`, `{LICENSE_FAMILY}`, or a scope"
                )
            }
        }
    }
}

impl std::error::Error for AuditPlanError {}

/// Plan an audit request from the arguments after `dx audit`. The first
/// argument selects a family when it spells one; every other argument is
/// a scope. Scope-shaped first arguments (labels, patterns, paths) run
/// both families over those scopes.
pub fn plan_audit(args: &[String]) -> Result<AuditRequest, AuditPlanError> {
    let mut families = vec![AuditFamily::Security, AuditFamily::License];
    let mut scopes: &[String] = args;
    if let Some((first, rest)) = args.split_first() {
        if AuditFamily::parse(first).is_some() {
            families = vec![AuditFamily::parse(first).expect("parsed family")];
            scopes = rest;
        } else if looks_like_scope(first) {
            scopes = args;
        } else {
            return Err(AuditPlanError::UnknownFamily {
                value: first.clone(),
            });
        }
    }
    Ok(AuditRequest {
        families,
        scopes: scopes.to_vec(),
    })
}

/// Scope-shaped arguments select dependency sets without naming a family:
/// Bazel labels, recursive patterns, and relative paths. Anything else in
/// first position is an unknown family, not a scope.
fn looks_like_scope(text: &str) -> bool {
    text.starts_with("//") || text.starts_with('@') || text.starts_with('.') || text.contains('/')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().copied().map(str::to_owned).collect()
    }

    #[test]
    fn bare_audit_runs_both_families_security_first() {
        let request = plan_audit(&[]).expect("bare audit plans");
        assert_eq!(
            request.families,
            vec![AuditFamily::Security, AuditFamily::License]
        );
        assert!(request.scopes.is_empty());
        assert_eq!(request.effective_scopes(), vec!["//...".to_owned()]);
    }

    #[test]
    fn explicit_families_run_alone() {
        let security = plan_audit(&args(&["security"])).expect("security plans");
        assert_eq!(security.families, vec![AuditFamily::Security]);
        assert_eq!(security.effective_scopes(), vec!["//...".to_owned()]);
        let license = plan_audit(&args(&["license"])).expect("license plans");
        assert_eq!(license.families, vec![AuditFamily::License]);
    }

    #[test]
    fn scopes_pass_through_verbatim_with_both_families() {
        let request =
            plan_audit(&args(&["//services/payments/...", "@crates//:lock"])).expect("plans");
        assert_eq!(
            request.families,
            vec![AuditFamily::Security, AuditFamily::License]
        );
        assert_eq!(
            request.effective_scopes(),
            vec![
                "//services/payments/...".to_owned(),
                "@crates//:lock".to_owned()
            ]
        );
    }

    #[test]
    fn family_plus_scopes_narrows_family_keeps_scopes() {
        let request = plan_audit(&args(&["license", "//services/payments/..."])).expect("plans");
        assert_eq!(request.families, vec![AuditFamily::License]);
        assert_eq!(
            request.effective_scopes(),
            vec!["//services/payments/...".to_owned()]
        );
    }

    #[test]
    fn unknown_first_positional_fails_closed() {
        let error = plan_audit(&args(&["licence"])).expect_err("must reject");
        assert_eq!(
            error,
            AuditPlanError::UnknownFamily {
                value: "licence".to_owned()
            }
        );
    }

    #[test]
    fn audit_is_non_mutating() {
        assert!(!AuditRequest::is_mutating());
    }

    #[test]
    fn family_spellings_are_frozen() {
        assert_eq!(AuditFamily::Security.as_str(), "security");
        assert_eq!(AuditFamily::License.as_str(), "license");
        assert_eq!(AuditFamily::parse("security"), Some(AuditFamily::Security));
        assert_eq!(AuditFamily::parse("license"), Some(AuditFamily::License));
        assert_eq!(AuditFamily::parse("both"), None);
    }
}
