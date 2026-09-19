//! Dependency-vulnerability matching for `dx audit` (issue #18).
//!
//! Pure local matching over injected lockfile packages and OSV-format
//! advisory records, per the audit contract
//! (`docs/cli/commands/audit-update-bazel.md#dx-audit`): download
//! applicable advisory databases and match packages against the
//! identified snapshots within Bazel-owned analysis; never upload
//! lockfiles or send dependency package names and versions through
//! query parameters, request bodies, or auditor telemetry.
//! Package-specific advisory requests that disclose the inventory are
//! never an alternative to local matching, and a query-only upstream
//! service never satisfies this contract.
//!
//! Report known vulnerabilities whether or not a fixed version is
//! available, with the same severity threshold and failure policy in
//! both cases. Lack of a fix never suppresses a finding, downgrades
//! its severity, or exempts it from failure. Upstream remediation
//! information is preserved when available, without treating a
//! dependency-version upgrade as an automatic source fix or mutating
//! dependencies during audit.
//!
//! Known applicable vulnerabilities with no severity rating fail audit
//! by default. The upstream advisory severity is reported as unknown
//! text rather than an invented rating; the normalized diagnostic
//! level stays in the closed `info|warning|error` set owned by the
//! output protocol. A valid explicit risk-acceptance exception may
//! exempt the finding from failure while retaining visibility.
//!
//! If a selected dependency cannot be assessed by the qualified
//! auditor, the audit fails as incomplete and identifies the
//! dependency plus the assessment limitation. Unsupported Git
//! revisions and unidentified private packages are incomplete, never
//! clean. A recognized assessable package with no matching advisories
//! is clean, not a coverage failure. Advisory-specific risk acceptance
//! never waives missing assessment, and an empty findings list alone
//! is never evidence that every selected dependency was assessed.
//!
//! This module matches over injected records only, so severity,
//! fix-preservation, unknown handling, and incomplete mapping stay
//! deterministic and unit-testable without network access or any
//! auditor binary. Version-range narrowing uses upstream semantics
//! through [`crate::exception::version_in_scope`] for semver
//! ecosystems (Cargo/npm/Go); Maven/NuGet scopes stay exact-match in
//! V1 with resolver-owned narrowing deferred, exactly like the
//! exception lifecycle deferral in [`crate::exception`].

use serde::{Deserialize, Serialize};

use crate::exception::{
    check_expiry, version_in_scope, ExceptionProblem, FindingRef, RiskException,
};

/// One locked package extracted from a standard lockfile or equivalent
/// resolved dependency file. Complete-lock coverage audits every entry,
/// including dependencies not used by the particular scoped targets.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LockedPackage {
    /// Package name in its owning set.
    pub name: String,
    /// Locked version string.
    pub version: String,
    /// Owning dependency set (selector spelling, verbatim).
    pub set: String,
    /// True for Git-revision dependencies the V1 matcher cannot assess
    /// (unsupported revisions fail as incomplete, never clean).
    pub is_git: bool,
    /// True for private/unidentified packages with no upstream advisory
    /// identity (unassessed, fail as incomplete).
    pub is_private: bool,
}

/// One OSV-format advisory record from the identified snapshot. Field
/// shapes are the minimal V1 subset: upstream advisory identity,
/// affected package and version scope, severity text, and remediation.
/// Full OSV schema coverage stays open; unknown fields are ignored so
/// snapshot evolution never breaks matching.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Advisory {
    /// Upstream advisory identity (e.g. `GHSA-aaaa-bbbb-cccc`, `RUSTSEC-...`).
    pub id: String,
    /// Affected package name.
    pub package: String,
    /// Affected version scope, upstream version semantics
    /// (`>=1.2.0, <2.0.0` for semver sets; exact version for Maven/NuGet V1).
    pub versions: String,
    /// Upstream severity text (`critical|high|medium|low`), or empty for
    /// unrated advisories (reported as unknown, fail by default).
    pub severity: String,
    /// Fixed versions, when upstream publishes remediation. Empty means
    /// no fix available (still reported, still fails without exception).
    pub fixed: Vec<String>,
    /// Owning dependency set the advisory applies to.
    pub set: String,
}

/// One matched vulnerability finding. Accepted findings stay visible;
/// acceptance only excludes them from the failure decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VulnFinding {
    /// Upstream advisory identity.
    pub advisory: String,
    /// Affected package name.
    pub package: String,
    /// Locked version found vulnerable.
    pub version: String,
    /// Owning dependency set.
    pub set: String,
    /// Upstream severity text, or `unknown` when unrated.
    pub severity: String,
    /// Normalized diagnostic level in the closed `info|warning|error` set.
    pub level: &'static str,
    /// Fixed versions from upstream, possibly empty (no fix still fails).
    pub fixed: Vec<String>,
}

/// Unassessable dependency: the audit fails as incomplete and
/// identifies the dependency plus the limitation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unassessed {
    /// Affected package name.
    pub package: String,
    /// Owning dependency set.
    pub set: String,
    /// Assessment limitation (`unsupported git revision`,
    /// `unidentified private package`).
    pub reason: &'static str,
}

/// Upstream severity text for unrated advisories. Reported verbatim as
/// unknown text; never an invented rating.
pub const UNKNOWN_SEVERITY: &str = "unknown";

/// Stable rule prefix for vulnerability findings in SARIF and events.
pub const VULN_RULE_PREFIX: &str = "vuln";

/// Normalize one upstream severity to the closed diagnostic level:
/// `critical|high` to `error`, `medium|low` to `warning`, and missing
/// or unrecognized text to `error` (unknown fails by default, fail
/// closed). The upstream text itself is preserved in the finding for
/// visibility; only the level normalizes.
pub fn normalize_level(severity: &str) -> &'static str {
    match severity.trim().to_ascii_lowercase().as_str() {
        "critical" | "high" | "error" => "error",
        "medium" | "low" | "moderate" | "warning" => "warning",
        "" | "unknown" => "error",
        _ => "error",
    }
}

/// Canonical upstream severity text: trimmed text, or `unknown` when
/// empty. Never invents a rating for unrated advisories.
pub fn canonical_severity(severity: &str) -> String {
    let trimmed = severity.trim();
    if trimmed.is_empty() {
        UNKNOWN_SEVERITY.to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Whether one locked package version falls in one advisory's affected
/// scope. Cargo/npm/Go use upstream Cargo-flavor semver via
/// [`version_in_scope`]; Maven/NuGet V1 use exact version equality
/// (resolver-owned range narrowing deferred). Unparseable scopes or
/// versions fail closed to `false` for semver sets, and to exact-match
/// only for Maven/NuGet.
pub fn version_affected(set: &str, scope: &str, version: &str) -> bool {
    match set {
        "cargo" | "npm" | "go" => version_in_scope(scope, version),
        _ => scope.trim() == version.trim() && !scope.trim().is_empty(),
    }
}

/// Match locked packages against advisories: every assessable package
/// with an applicable advisory reports a finding (with or without a
/// fix); unassessable packages (Git revisions, private identities)
/// report separately as incomplete, never clean. Matching never
/// filters to a target's resolved closure: callers supply the complete
/// owning-set lock contents.
pub fn match_packages(
    packages: &[LockedPackage],
    advisories: &[Advisory],
) -> (Vec<VulnFinding>, Vec<Unassessed>) {
    let mut findings = Vec::new();
    let mut unassessed = Vec::new();
    for package in packages {
        if package.is_git {
            unassessed.push(Unassessed {
                package: package.name.clone(),
                set: package.set.clone(),
                reason: "unsupported git revision",
            });
            continue;
        }
        if package.is_private {
            unassessed.push(Unassessed {
                package: package.name.clone(),
                set: package.set.clone(),
                reason: "unidentified private package",
            });
            continue;
        }
        for advisory in advisories {
            if advisory.package != package.name || advisory.set != package.set {
                continue;
            }
            if advisory.id.trim().is_empty() {
                continue;
            }
            if !version_affected(&package.set, &advisory.versions, &package.version) {
                continue;
            }
            let severity = canonical_severity(&advisory.severity);
            findings.push(VulnFinding {
                advisory: advisory.id.clone(),
                package: package.name.clone(),
                version: package.version.clone(),
                set: package.set.clone(),
                level: normalize_level(&severity),
                severity,
                fixed: advisory.fixed.clone(),
            });
        }
    }
    findings
        .sort_by(|a, b| (&a.set, &a.package, &a.advisory).cmp(&(&b.set, &b.package, &b.advisory)));
    unassessed.sort_by(|a, b| (&a.set, &a.package).cmp(&(&b.set, &b.package)));
    (findings, unassessed)
}

/// Apply version-scoped, reasoned, expiring risk exceptions to findings:
/// validated exceptions with matching advisory/package/version exclude
/// their finding from failure while retaining visibility. Returns the
/// unexempted findings plus validation/obsolete problems (every problem
/// fails the audit; none auto-repairs).
pub fn apply_exceptions(
    findings: &[VulnFinding],
    exceptions: &[RiskException],
    today: &str,
) -> (Vec<VulnFinding>, Vec<ExceptionProblem>) {
    let mut problems = Vec::new();
    let mut valid: Vec<&RiskException> = Vec::new();
    for exception in exceptions {
        match crate::exception::validate_exception(exception, today) {
            Ok(()) => valid.push(exception),
            Err(problem) => problems.push(problem),
        }
    }
    // Obsolete check runs after validation, against current findings.
    let finding_refs: Vec<FindingRef> = findings
        .iter()
        .map(|finding| FindingRef {
            advisory: finding.advisory.clone(),
            package: finding.package.clone(),
        })
        .collect();
    let mut active: Vec<&RiskException> = Vec::new();
    for exception in valid {
        match crate::exception::check_applies(exception, &finding_refs) {
            Ok(()) => active.push(exception),
            Err(problem) => problems.push(problem),
        }
    }
    let unexempted: Vec<VulnFinding> = findings
        .iter()
        .filter(|finding| {
            !active.iter().any(|exception| {
                exception.advisory == finding.advisory
                    && exception.package == finding.package
                    && exception.set == finding.set
                    && version_in_scope(&exception.versions, &finding.version)
                    && check_expiry(&exception.expires, today).is_ok()
            })
        })
        .cloned()
        .collect();
    (unexempted, problems)
}

/// Parse one OSV-format advisory snapshot document (JSON array of
/// [`Advisory`]) into records. Unknown fields ignore; malformed JSON
/// fails closed with the document error.
pub fn parse_snapshot(text: &str) -> Result<Vec<Advisory>, String> {
    serde_json::from_str(text).map_err(|error| format!("invalid advisory snapshot: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packages() -> Vec<LockedPackage> {
        vec![
            LockedPackage {
                name: "serde".to_owned(),
                version: "1.0.100".to_owned(),
                set: "cargo".to_owned(),
                is_git: false,
                is_private: false,
            },
            LockedPackage {
                name: "react".to_owned(),
                version: "18.2.0".to_owned(),
                set: "npm".to_owned(),
                is_git: false,
                is_private: false,
            },
        ]
    }

    fn advisories() -> Vec<Advisory> {
        vec![
            Advisory {
                id: "GHSA-aaaa-bbbb-cccc".to_owned(),
                package: "serde".to_owned(),
                versions: ">=1.0.0, <1.0.150".to_owned(),
                severity: "high".to_owned(),
                fixed: vec!["1.0.150".to_owned()],
                set: "cargo".to_owned(),
            },
            Advisory {
                id: "GHSA-nofix-0000".to_owned(),
                package: "react".to_owned(),
                versions: ">=18.0.0, <18.3.0".to_owned(),
                severity: String::new(),
                fixed: vec![],
                set: "npm".to_owned(),
            },
        ]
    }

    #[test]
    fn matching_reports_findings_with_and_without_fix() {
        let (findings, unassessed) = match_packages(&packages(), &advisories());
        assert!(unassessed.is_empty());
        assert_eq!(findings.len(), 2);
        let serde_finding = findings
            .iter()
            .find(|finding| finding.package == "serde")
            .expect("serde");
        assert_eq!(serde_finding.advisory, "GHSA-aaaa-bbbb-cccc");
        assert_eq!(serde_finding.level, "error");
        assert_eq!(serde_finding.severity, "high");
        assert_eq!(serde_finding.fixed, vec!["1.0.150".to_owned()]);
        // No fix still reports and still fails (warning/error, not clean).
        let react_finding = findings
            .iter()
            .find(|finding| finding.package == "react")
            .expect("react");
        assert_eq!(react_finding.severity, "unknown");
        assert_eq!(react_finding.level, "error");
        assert!(react_finding.fixed.is_empty());
    }

    #[test]
    fn severity_normalizes_to_closed_levels_with_unknown_failing() {
        assert_eq!(normalize_level("critical"), "error");
        assert_eq!(normalize_level("HIGH"), "error");
        assert_eq!(normalize_level("medium"), "warning");
        assert_eq!(normalize_level("low"), "warning");
        assert_eq!(normalize_level(""), "error");
        assert_eq!(normalize_level("unknown"), "error");
        assert_eq!(normalize_level("bogus"), "error");
        assert_eq!(canonical_severity(""), "unknown");
        assert_eq!(canonical_severity("  "), "unknown");
        assert_eq!(canonical_severity(" high "), "high");
        assert_eq!(UNKNOWN_SEVERITY, "unknown");
        assert_eq!(VULN_RULE_PREFIX, "vuln");
    }

    #[test]
    fn version_matching_uses_semver_for_cargo_npm_go_and_exact_for_maven() {
        assert!(version_affected("cargo", ">=1.0.0, <2.0.0", "1.5.0"));
        assert!(!version_affected("cargo", ">=1.0.0, <2.0.0", "2.0.0"));
        assert!(version_affected("npm", "^18.0.0", "18.2.0"));
        assert!(version_affected("maven", "1.2.0", "1.2.0"));
        assert!(!version_affected("maven", ">=1.0.0", "1.2.0"));
        assert!(!version_affected("nuget", "", "1.0.0"));
    }

    #[test]
    fn git_and_private_packages_are_incomplete_never_clean() {
        let pkgs = vec![
            LockedPackage {
                name: "git-dep".to_owned(),
                version: "abc123".to_owned(),
                set: "cargo".to_owned(),
                is_git: true,
                is_private: false,
            },
            LockedPackage {
                name: "internal".to_owned(),
                version: "0.1.0".to_owned(),
                set: "npm".to_owned(),
                is_git: false,
                is_private: true,
            },
        ];
        let (findings, unassessed) = match_packages(&pkgs, &advisories());
        assert!(findings.is_empty());
        assert_eq!(unassessed.len(), 2);
        assert!(unassessed
            .iter()
            .any(|u| u.reason == "unsupported git revision"));
        assert!(unassessed
            .iter()
            .any(|u| u.reason == "unidentified private package"));
    }

    #[test]
    fn clean_package_with_no_matching_advisory_is_clean() {
        let pkgs = vec![LockedPackage {
            name: "clean".to_owned(),
            version: "9.9.9".to_owned(),
            set: "cargo".to_owned(),
            is_git: false,
            is_private: false,
        }];
        let (findings, unassessed) = match_packages(&pkgs, &advisories());
        assert!(findings.is_empty());
        assert!(unassessed.is_empty());
    }

    #[test]
    fn exceptions_exempt_only_matching_versioned_findings() {
        let (findings, _) = match_packages(&packages(), &advisories());
        let exception = RiskException {
            advisory: "GHSA-aaaa-bbbb-cccc".to_owned(),
            package: "serde".to_owned(),
            set: "cargo".to_owned(),
            versions: ">=1.0.0, <1.0.150".to_owned(),
            reason: "Accepted for this release.".to_owned(),
            expires: "2027-03-01".to_owned(),
        };
        let (unexempted, problems) = apply_exceptions(&findings, &[exception], "2026-09-18");
        assert!(problems.is_empty());
        assert_eq!(unexempted.len(), 1);
        assert_eq!(unexempted[0].package, "react");
    }

    #[test]
    fn exception_problems_fail_and_obsolete_is_reported() {
        let (findings, _) = match_packages(&packages(), &advisories());
        let expired = RiskException {
            advisory: "GHSA-aaaa-bbbb-cccc".to_owned(),
            package: "serde".to_owned(),
            set: "cargo".to_owned(),
            versions: ">=1.0.0, <1.0.150".to_owned(),
            reason: "Old.".to_owned(),
            expires: "2026-01-01".to_owned(),
        };
        let (_, problems) = apply_exceptions(&findings, &[expired], "2026-09-18");
        assert!(!problems.is_empty());
        let obsolete = RiskException {
            advisory: "GHSA-gone".to_owned(),
            package: "gone".to_owned(),
            set: "cargo".to_owned(),
            versions: "*".to_owned(),
            reason: "Stale.".to_owned(),
            expires: "2027-03-01".to_owned(),
        };
        let (_, problems) = apply_exceptions(&findings, &[obsolete], "2026-09-18");
        assert!(problems
            .iter()
            .any(|p| matches!(p, ExceptionProblem::Obsolete { .. })));
    }

    #[test]
    fn snapshot_parses_osv_array_and_rejects_malformed() {
        let text = r#"[{"id":"GHSA-1","package":"serde","versions":">=1.0.0, <2.0.0","severity":"high","fixed":["1.5.0"],"set":"cargo"}]"#;
        let parsed = parse_snapshot(text).expect("parses");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "GHSA-1");
        assert!(parse_snapshot("not json").is_err());
    }
}
