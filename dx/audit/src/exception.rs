//! Risk-acceptance exception lifecycle (M26 WP1 slice 2).
//!
//! Pure validation over injected exception records and findings, per the
//! audit contract: every exception identifies its advisory and affected
//! dependency, carries a version scope with upstream semantics, states a
//! reason, and expires on a fixed UTC date. Missing, invalid, or expired
//! dates fail validation; an exception with no applicable finding is
//! obsolete (reported for explicit removal, never auto-deleted).
//!
//! Deferred to the resolver-owned slices: version-range evaluation uses
//! upstream ecosystem semantics, not a private solver, so range matching
//! against finding versions is structural here (identity match) and lands
//! with the ecosystem integrations. Accepted findings stay visible;
//! acceptance only excludes them from the failure decision.

/// One risk-acceptance exception: narrow, explained, version-scoped,
/// and expiring. Field shapes mirror the committed policy file so the
/// future TOML loader cannot reinterpret them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskException {
    /// Upstream advisory identity (alias matching is O11 qualification).
    pub advisory: String,
    /// Affected dependency name in its owning set.
    pub package: String,
    /// Owning dependency set (lockfile/workspace scope).
    pub set: String,
    /// Accepted versions or bounded range, upstream version semantics.
    /// Opaque in this slice; evaluated by the ecosystem integration.
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExceptionProblem {
    /// Empty advisory, package, set, versions, or reason.
    MissingField { field: &'static str },
    /// Expiration is not a calendar `YYYY-MM-DD` date.
    InvalidDate { value: String },
    /// Expiration reached as of the injected audit date.
    Expired { expires: String, today: String },
    /// No applicable finding: remove explicitly, never automatically.
    Obsolete { advisory: String, package: String },
}

impl std::fmt::Display for ExceptionProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExceptionProblem::MissingField { field } => {
                write!(f, "risk exception missing {field}")
            }
            ExceptionProblem::InvalidDate { value } => {
                write!(
                    f,
                    "risk exception has invalid expiration {value:?}; want YYYY-MM-DD"
                )
            }
            ExceptionProblem::Expired { expires, today } => {
                write!(
                    f,
                    "risk exception expired {expires} (audit date {today}); renewal needs review"
                )
            }
            ExceptionProblem::Obsolete { advisory, package } => {
                write!(
                    f,
                    "risk exception for {advisory} on {package} matches no finding; remove it explicitly"
                )
            }
        }
    }
}

impl std::error::Error for ExceptionProblem {}

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
    if !is_calendar_date(&exception.expires) {
        return Err(ExceptionProblem::InvalidDate {
            value: exception.expires.clone(),
        });
    }
    if !is_calendar_date(today) {
        return Err(ExceptionProblem::InvalidDate {
            value: today.to_owned(),
        });
    }
    if exception.expires.as_str() <= today {
        return Err(ExceptionProblem::Expired {
            expires: exception.expires.clone(),
            today: today.to_owned(),
        });
    }
    Ok(())
}

/// True when no finding shares the exception's advisory and package
/// identity. Version-range narrowing arrives with the upstream-semantics
/// matcher; until then identity match is the conservative applicability
/// gate (never infers obsolescence from failed analysis).
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

/// Calendar `YYYY-MM-DD` shape with month/day ranges including leap-year
/// February. Lexicographic order matches chronological order, so validated
/// dates compare as strings without clock or timezone inputs.
fn is_calendar_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    for (index, byte) in bytes.iter().enumerate() {
        if index == 4 || index == 7 {
            continue;
        }
        if !byte.is_ascii_digit() {
            return false;
        }
    }
    let number = |from: usize, to: usize| -> u32 { text[from..to].parse().unwrap_or(0) };
    let year = number(0, 4);
    let month = number(5, 7);
    let day = number(8, 10);
    if year == 0 || month == 0 || month > 12 || day == 0 {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if leap {
                29
            } else {
                28
            }
        }
    };
    day <= max_day
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
}
