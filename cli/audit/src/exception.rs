//! Risk-acceptance exception lifecycle (WP1 slice 2).
//!
//! Pure validation over injected exception records and findings, per the
//! audit contract: every exception identifies its advisory and affected
//! dependency, carries a version scope with upstream semantics, states a
//! reason, and expires on a fixed UTC date. Missing, invalid, or expired
//! dates fail validation; an exception with no applicable finding is
//! obsolete (reported for explicit removal, never auto-deleted).
//!
//! Deferred to the resolver-owned slices: version-range narrowing
//! against finding versions uses upstream ecosystem semantics through
//! [`version_in_scope`], not a private solver, so the check here stays
//! the structural identity match and narrowing lands with the ecosystem
//! integrations. Accepted findings stay visible; acceptance only
//! excludes them from the failure decision.

use chrono::{Datelike, NaiveDate};

/// Versioned risk-exception schema.
///
/// Exceptions are data validated via `validate_exception` / `check_expiry` /
/// `check_applies` plus `version_in_scope`, never a hardcoded allowlist:
/// adding an advisory, package, or version scope edits the policy file data
/// only. This version marks the validated struct shape; bumps are explicit,
/// never silent drift.
pub const EXCEPTION_SCHEMA_VERSION: u32 = 1;

/// One risk-acceptance exception: narrow, explained, version-scoped,
/// and expiring. Field shapes mirror the committed policy file so the
/// future TOML loader cannot reinterpret them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskException {
    /// Upstream advisory identity (alias matching is qualification).
    pub advisory: String,
    /// Affected dependency name in its owning set.
    pub package: String,
    /// Owning dependency set (lockfile/workspace scope).
    pub set: String,
    /// Accepted versions or bounded range, upstream version semantics.
    /// Cargo-flavor scopes evaluate with [`version_in_scope`];
    /// non-semver ecosystem scopes stay opaque for the resolver-owned
    /// ecosystem integration.
    pub versions: String,
    /// Explanatory reason. Empty reasons fail validation.
    pub reason: String,
    /// Expiration date, ISO-8601 UTC `YYYY-MM-DD`, evaluated at audit time.
    pub expires: String,
}

/// One assessed finding an exception may apply to. Visibility is never
/// waived: an accepted finding is reported as accepted with its reason.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindingRef {
    /// Upstream advisory identity.
    pub advisory: String,
    /// Affected dependency name.
    pub package: String,
}

/// Validation failures. Every variant fails the audit; none auto-repair.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ExceptionProblem {
    /// Empty advisory, package, set, versions, or reason.
    #[error("risk exception missing {field}")]
    MissingField { field: &'static str },
    /// Expiration is not a calendar `YYYY-MM-DD` date.
    #[error("risk exception has invalid expiration {value:?}; want YYYY-MM-DD")]
    InvalidDate { value: String },
    /// Expiration reached as of the injected audit date.
    #[error("risk exception expired {expires} (audit date {today}); renewal needs review")]
    Expired { expires: String, today: String },
    /// No applicable finding: remove explicitly, never automatically.
    #[error("risk exception for {advisory} on {package} matches no finding; remove it explicitly")]
    Obsolete { advisory: String, package: String },
}

/// Validate one exception against the injected audit date (`YYYY-MM-DD`
/// UTC). Checks field presence, calendar-date shape, and expiry. An
/// earlier cached acceptance never passes a later audit after expiry
/// because the date is always an explicit input, never ambient clock
/// state.
pub fn validate_exception(exception: &RiskException, today: &str) -> Result<(), ExceptionProblem> {
    for (field, value) in [
        ("advisory", exception.advisory.as_str()),
        ("package", exception.package.as_str()),
        ("set", exception.set.as_str()),
        ("versions", exception.versions.as_str()),
        ("reason", exception.reason.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ExceptionProblem::MissingField { field });
        }
    }
    check_expiry(&exception.expires, today)
}

/// Check one expiration date against the injected audit date
/// (`YYYY-MM-DD` UTC). Shared by the vulnerability risk-acceptance
/// lifecycle above and the license-family exceptions: an earlier cached
/// acceptance never passes a later audit after expiry because the date
/// is always an explicit input, never ambient clock state. The boundary
/// is inclusive: an exception expiring today is expired.
pub fn check_expiry(expires: &str, today: &str) -> Result<(), ExceptionProblem> {
    let expires_date = parse_audit_date(expires)?;
    let today_date = parse_audit_date(today)?;
    if expires_date <= today_date {
        return Err(ExceptionProblem::Expired {
            expires: expires.to_owned(),
            today: today.to_owned(),
        });
    }
    Ok(())
}

/// True when a finding version falls inside an exception's accepted
/// version scope, using upstream Cargo-flavor semver semantics via the
/// `semver` crate (ranges like `>=1.2.0, <2.0.0`, carets, tildes,
/// wildcards). Unparseable scopes or versions fail closed to `false`:
/// an exception never covers a version the matcher cannot attribute,
/// and non-semver ecosystem scopes stay opaque for the resolver-owned
/// integrations. Pre-releases match only the narrow upstream rule (a
/// requirement with a pre-release on the same version); a bare range
/// never covers a pre-release.
///
/// Shared by the vulnerability risk-acceptance lifecycle above and the
/// license-family exceptions. The identity gates ([`is_obsolete`]) stay
/// the conservative applicability check here; resolver-owned ecosystem
/// integrations call this to narrow coverage by version.
pub fn version_in_scope(scope: &str, version: &str) -> bool {
    let requirements = match semver::VersionReq::parse(scope) {
        Ok(requirements) => requirements,
        Err(_) => return false,
    };
    let version = match semver::Version::parse(version) {
        Ok(version) => version,
        Err(_) => return false,
    };
    requirements.matches(&version)
}

/// True when no finding shares the exception's advisory and package
/// identity. Version-range narrowing calls [`version_in_scope`] in the
/// resolver-owned ecosystem integrations; until then identity match is
/// the conservative applicability gate (never infers obsolescence from
/// failed analysis).
pub fn is_obsolete(exception: &RiskException, findings: &[FindingRef]) -> bool {
    !findings.iter().any(|finding| {
        finding.advisory == exception.advisory && finding.package == exception.package
    })
}

/// Check an exception for obsolescence after validation: returns the
/// [`ExceptionProblem::Obsolete`] failure when no finding applies.
pub fn check_applies(
    exception: &RiskException,
    findings: &[FindingRef],
) -> Result<(), ExceptionProblem> {
    if is_obsolete(exception, findings) {
        return Err(ExceptionProblem::Obsolete {
            advisory: exception.advisory.clone(),
            package: exception.package.clone(),
        });
    }
    Ok(())
}

/// Parse one audit date (`YYYY-MM-DD` UTC) into a calendar date. The
/// fixed-width shape gate runs first so only zero-padded text reaches
/// the parser: non-padded spellings (`2027-3-1`) fail here even where
/// the parser would accept them, and lexicographic order keeps matching
/// chronological order for validated dates. The calendar itself —
/// month lengths, leap-year February — is the upstream `chrono`
/// parser's, not a private table. Year zero is rejected to preserve the
/// previous validation (it would otherwise parse and always compare as
/// expired, changing the failure variant).
///
/// Dependency evaluation (adopted): calendar validation uses the
/// upstream `chrono` crate (`NaiveDate::parse_from_str`, `Datelike`); the
/// fixed-width `YYYY-MM-DD` shape gate stays hand-rolled so only zero-padded
/// text reaches the parser and no `time`-family second date engine is added.
///
/// Dependency evaluation (keep): `jiff 0.2` spike rejected —
/// trivial `NaiveDate` parse plus `Datelike::year` needs no `tzdb`/civil-time
/// arithmetic, `jiff` default pulls the `tzdb` bundle plus `portable-atomic`
/// tree (~23 locks vs `chrono alloc-only` 4) for mechanical churn
/// (`Date::strptime` arg-order swap, `year()` `i16`, year-zero parses where
/// this gate maps it to `InvalidDate`), pre-`1.0` single-owner churn; MSRV
/// fits on both sides (`chrono 1.62`, `jiff 1.70` vs pinned `1.98`),
/// `chrono 0.4.45` still releasing with no `unmaintained` banner, the
/// `chronotope` wind-down stays an open discretionary proposal and
/// `arrow-rs` is explicitly not urgent (wait for `jiff 1.0`, slipped
/// with a 1-year grace); re-evaluate on `jiff 1.0`.
fn parse_audit_date(value: &str) -> Result<NaiveDate, ExceptionProblem> {
    if !is_date_shape(value) {
        return Err(ExceptionProblem::InvalidDate {
            value: value.to_owned(),
        });
    }
    match NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        Ok(date) if date.year() >= 1 => Ok(date),
        _ => Err(ExceptionProblem::InvalidDate {
            value: value.to_owned(),
        }),
    }
}

/// Fixed `YYYY-MM-DD` shape gate: length, dash positions, and digits.
/// See [`parse_audit_date`] for why the shape stays strict while the
/// calendar comes from upstream.
fn is_date_shape(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> RiskException {
        RiskException {
            advisory: "GHSA-aaaa-bbbb-cccc".to_owned(),
            package: "some-copyleft-lib".to_owned(),
            set: "cargo-lock".to_owned(),
            versions: ">=1.2.0, <2.0.0".to_owned(),
            reason: "Legal approved for internal fork.".to_owned(),
            expires: "2027-03-01".to_owned(),
        }
    }

    fn finding() -> FindingRef {
        FindingRef {
            advisory: "GHSA-aaaa-bbbb-cccc".to_owned(),
            package: "some-copyleft-lib".to_owned(),
        }
    }

    #[test]
    fn valid_exception_passes_and_applies() {
        let exception = sample();
        validate_exception(&exception, "2026-09-14").expect("valid");
        check_applies(&exception, &[finding()]).expect("applies");
    }

    #[test]
    fn empty_reason_fails_validation() {
        let mut exception = sample();
        exception.reason = "  ".to_owned();
        assert_eq!(
            validate_exception(&exception, "2026-09-14"),
            Err(ExceptionProblem::MissingField { field: "reason" })
        );
    }

    #[test]
    fn malformed_dates_fail_validation() {
        let mut exception = sample();
        for bad in [
            "2027-3-1",
            "2027/03/01",
            "2027-13-01",
            "2027-02-30",
            "2027-04-31",
            "2027-00-10",
            "2027-01-00",
            "0000-01-01",
            "not-a-date",
        ] {
            exception.expires = bad.to_owned();
            assert!(
                matches!(
                    validate_exception(&exception, "2026-09-14"),
                    Err(ExceptionProblem::InvalidDate { .. })
                ),
                "{bad} must fail"
            );
        }
        assert!(validate_exception(&sample(), "today").is_err());
    }

    #[test]
    fn expiry_boundary_fails_on_the_date_itself() {
        let exception = sample();
        assert_eq!(
            validate_exception(&exception, "2027-03-01"),
            Err(ExceptionProblem::Expired {
                expires: "2027-03-01".to_owned(),
                today: "2027-03-01".to_owned(),
            })
        );
        assert!(validate_exception(&exception, "2027-03-02").is_err());
        validate_exception(&exception, "2027-02-28").expect("day before passes");
    }

    #[test]
    fn leap_year_february_validates() {
        let mut exception = sample();
        exception.expires = "2028-02-29".to_owned();
        validate_exception(&exception, "2026-09-14").expect("leap day valid");
        exception.expires = "2027-02-29".to_owned();
        assert!(validate_exception(&exception, "2026-09-14").is_err());
    }

    #[test]
    fn exception_without_finding_is_obsolete() {
        let exception = sample();
        assert!(is_obsolete(&exception, &[]));
        assert_eq!(
            check_applies(&exception, &[]),
            Err(ExceptionProblem::Obsolete {
                advisory: "GHSA-aaaa-bbbb-cccc".to_owned(),
                package: "some-copyleft-lib".to_owned(),
            })
        );
        let other = FindingRef {
            advisory: "GHSA-xxxx-yyyy-zzzz".to_owned(),
            package: "some-copyleft-lib".to_owned(),
        };
        assert!(is_obsolete(&exception, &[other]));
        assert!(!is_obsolete(&exception, &[finding()]));
    }

    #[test]
    fn version_scopes_match_cargo_flavor_ranges() {
        let scope = ">=1.2.0, <2.0.0";
        assert!(version_in_scope(scope, "1.2.0"));
        assert!(version_in_scope(scope, "1.9.0"));
        assert!(!version_in_scope(scope, "1.1.9"));
        assert!(!version_in_scope(scope, "2.0.0"));
        assert!(version_in_scope("^1.2.0", "1.9.0"));
        assert!(!version_in_scope("^1.2.0", "2.0.0"));
        assert!(version_in_scope("~1.2.0", "1.2.9"));
        assert!(!version_in_scope("~1.2.0", "1.3.0"));
        assert!(version_in_scope("1.2.0", "1.2.0"));
        // Bare versions are caret shorthand upstream: "1.2.0" means
        // ^1.2.0, so exact pins spell "=1.2.0".
        assert!(version_in_scope("1.2.0", "1.2.1"));
        assert!(version_in_scope("=1.2.0", "1.2.0"));
        assert!(!version_in_scope("=1.2.0", "1.2.1"));
        assert!(version_in_scope("*", "9.9.9"));
    }

    #[test]
    fn version_scopes_fail_closed_on_unparseable_input() {
        assert!(!version_in_scope("not a range", "1.2.0"));
        assert!(!version_in_scope(">=1.2.0, <2.0.0", "1.2"));
        assert!(!version_in_scope(">=1.2.0, <2.0.0", "banana"));
        assert!(!version_in_scope("", "1.2.0"));
    }

    #[test]
    fn version_scopes_exclude_prereleases_from_bare_ranges() {
        assert!(!version_in_scope(">=1.0.0", "2.0.0-alpha"));
        assert!(version_in_scope(">=1.0.0-alpha, <2.0.0", "1.0.0-alpha"));
    }

    #[test]
    fn exception_schema_stays_versioned_without_allowlist() {
        assert_eq!(EXCEPTION_SCHEMA_VERSION, 1);
        // New advisories/packages/scopes are data validated via the shared
        // lifecycle, never struct edits.
        let mut novel = sample();
        novel.advisory = "GHSA-novel-0000-0001".to_owned();
        novel.package = "brand-new-dep".to_owned();
        novel.versions = ">=9.0.0, <10.0.0".to_owned();
        validate_exception(&novel, "2026-09-14").expect("novel data validates");
    }
}
