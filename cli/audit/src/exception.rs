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
/// wildcards). This is the Cargo matcher; npm scopes evaluate with
/// [`npm_in_scope`], Go scopes with `crate::vuln::go_in_scope`.
/// Unparseable scopes or versions fail closed to `false`: an exception
/// never covers a version the matcher cannot attribute. Pre-releases
/// match only the narrow upstream rule (a requirement with a pre-release
/// on the same version); a bare range never covers a pre-release.
///
/// Go normalization rule (issue #679): `go.mod` versions carry a leading
/// `v` (`v1.2.3`), pseudo-versions (`v0.0.0-20240101-abcdef`,
/// `v1.2.4-0.20240101-abcdef`), and `+incompatible` suffixes
/// (`v2.0.0+incompatible`). `crate::vuln::go_in_scope` strips one leading
/// `v` per version token, then applies Cargo-flavor ordering without the
/// prerelease gate so pseudo-versions match bare ranges they fall inside
/// (`>=v1.0.0, <v2.0.0` covers `v1.2.4-0.20240101-abcdef`); `+incompatible`
/// rides build metadata ignored for precedence. No Go-aware crate:
/// `semver` ordering already matches Go precedence, so preprocessing plus
/// the gate bypass suffices (no new dep per ADR 0008).
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

/// True when a locked npm version falls inside an npm range scope using
/// the audited node-semver subset: `||` unions, hyphen ranges
/// (`1.2.3 - 2.3.4`, partial ends narrow per upstream), comma- or
/// space-separated comparator sets, `v` prefixes, `x`/`X`/`*`
/// wildcards and partials, carets, tildes, and bare versions (full
/// bare versions are exact, partial bares are bounded ranges). Each
/// branch normalizes to Cargo-compatible comparators evaluated with
/// the `semver` crate, so ordering and prerelease gating keep the
/// crate's narrow upstream rule (a prerelease matches only beside a
/// same-tuple prerelease comparator). Unparseable scopes or versions,
/// empty inputs, and overlong inputs fail closed to `false`.
pub fn npm_in_scope(scope: &str, version: &str) -> bool {
    let scope_trimmed = scope.trim();
    let version_trimmed = version.trim();
    if scope_trimmed.is_empty() || version_trimmed.is_empty() {
        return false;
    }
    if scope_trimmed.len() > 4096 || version_trimmed.len() > 256 {
        return false;
    }
    let locked = match parse_npm_locked(version_trimmed) {
        Some(locked) => locked,
        None => return false,
    };
    let branches: Vec<&str> = scope_trimmed.split("||").collect();
    if branches.len() > 64 {
        return false;
    }
    for branch in branches {
        if npm_branch_matches(branch.trim(), &locked) {
            return true;
        }
    }
    false
}

/// One locked npm version: leading `v`/`=` markers stripped, then
/// strict `semver` parsing. Partial locked versions never parse (fail
/// closed); build metadata parses and never affects matching.
fn parse_npm_locked(version: &str) -> Option<semver::Version> {
    let mut text = version.trim();
    loop {
        if let Some(rest) = text.strip_prefix('v').or_else(|| text.strip_prefix('V')) {
            text = rest.trim_start();
            continue;
        }
        if let Some(rest) = text.strip_prefix('=') {
            text = rest.trim_start();
            continue;
        }
        break;
    }
    if text.is_empty() || text.len() > 256 {
        return None;
    }
    semver::Version::parse(text).ok()
}

/// One npm partial version: up to three core parts (numeric, or
/// wildcard as `None`), optional prerelease on full numeric cores
/// only. Build metadata is stripped and ignored.
struct NpmPartial {
    major: String,
    minor: Option<String>,
    patch: Option<String>,
    prerelease: Option<String>,
}

/// Parse one npm partial per the audited subset: optional `v` prefix,
/// one to three dot-separated numeric parts with trailing parts as
/// wildcards (`x`/`X`/`*`) or absent, optional `-prerelease` on full
/// numeric cores, optional `+metadata` ignored. Numeric parts keep
/// strict shape (no leading zeros); a wildcard pins the tail
/// (`1.x.3` stays invalid). Returns `None` for malformed input.
fn parse_npm_partial(text: &str) -> Option<NpmPartial> {
    let text = text.trim();
    let text = text
        .strip_prefix('v')
        .or_else(|| text.strip_prefix('V'))
        .unwrap_or(text);
    let text = text.trim();
    if text.is_empty() || text.len() > 256 {
        return None;
    }
    let core_and_pre = match text.split_once('+') {
        Some((before, _)) => before,
        None => text,
    };
    if core_and_pre.is_empty() {
        return None;
    }
    let (core, prerelease) = match core_and_pre.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (core_and_pre, None),
    };
    let raw: Vec<&str> = core.split('.').collect();
    if raw.is_empty() || raw.len() > 3 {
        return None;
    }
    let mut parts: Vec<Option<String>> = Vec::new();
    for part in raw {
        let part = part.trim();
        if part.is_empty() {
            return None;
        }
        if part == "x" || part == "X" || part == "*" {
            parts.push(None);
        } else if is_npm_numeric(part) {
            parts.push(Some(part.to_owned()));
        } else {
            return None;
        }
    }
    let mut seen_wildcard = false;
    for part in &parts {
        if part.is_none() {
            seen_wildcard = true;
        } else if seen_wildcard {
            return None;
        }
    }
    while parts.len() < 3 {
        parts.push(None);
    }
    let prerelease = match prerelease {
        None => None,
        Some(pre) => {
            if parts.iter().any(Option::is_none) || pre.is_empty() {
                return None;
            }
            for label in pre.split('.') {
                if label.is_empty()
                    || !label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                {
                    return None;
                }
            }
            Some(pre.to_owned())
        }
    };
    Some(NpmPartial {
        major: parts[0].clone()?,
        minor: parts[1].clone(),
        patch: parts[2].clone(),
        prerelease,
    })
}

/// Strict numeric shape (no leading zeros unless `"0"`).
fn is_npm_numeric(part: &str) -> bool {
    if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    part.len() == 1 || !part.starts_with('0')
}

/// Increment one validated numeric part without overflow.
fn increment_npm_numeric(part: &str) -> Option<String> {
    if !is_npm_numeric(part) {
        return None;
    }
    let mut digits: Vec<u8> = part.bytes().map(|byte| byte - b'0').collect();
    let mut carry = true;
    for digit in digits.iter_mut().rev() {
        if carry {
            if *digit == 9 {
                *digit = 0;
            } else {
                *digit += 1;
                carry = false;
            }
        }
    }
    let mut out = String::new();
    if carry {
        out.push('1');
    }
    for digit in digits {
        out.push((digit + b'0') as char);
    }
    Some(out)
}

/// Lower bound for one hyphen/partial start: full versions keep
/// `>=` (with prerelease), partials fill missing parts with zero.
fn npm_partial_lower(partial: &NpmPartial) -> Option<String> {
    let mut bound = format!(
        ">={}.{}.{}",
        partial.major,
        partial.minor.as_deref().unwrap_or("0"),
        partial.patch.as_deref().unwrap_or("0")
    );
    if let Some(pre) = &partial.prerelease {
        bound.push('-');
        bound.push_str(pre);
    }
    Some(bound)
}

/// Exclusive upper bound for one partial end (`1.2` narrows below
/// `1.3.0`, `1` below `2.0.0`); full versions use inclusive `<=`.
/// Returns the bound plus whether it is inclusive.
fn npm_partial_upper(partial: &NpmPartial) -> Option<(String, bool)> {
    if let Some(patch) = &partial.patch {
        let mut bound = format!(
            "{}.{}.{}",
            partial.major,
            partial.minor.as_deref().unwrap_or("0"),
            patch
        );
        if let Some(pre) = &partial.prerelease {
            bound.push('-');
            bound.push_str(pre);
        }
        return Some((bound, true));
    }
    if let Some(minor) = &partial.minor {
        let bumped = increment_npm_numeric(minor)?;
        return Some((format!("<{}.{}.0", partial.major, bumped), false));
    }
    let bumped = increment_npm_numeric(&partial.major)?;
    Some((format!("<{bumped}.0.0"), false))
}

/// Match one `||` branch: an optional hyphen range (`A - B` as three
/// whitespace-separated tokens with a lone `-`), otherwise a
/// comma- and/or space-separated comparator set. A lone operator
/// token (`>=`, `<`, `=`) joins its successor (`>= 1.2.3`).
fn npm_branch_matches(branch: &str, locked: &semver::Version) -> bool {
    if branch.is_empty() || branch.len() > 4096 {
        return false;
    }
    let normalized = branch.replace(',', " ");
    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    if tokens.is_empty() || tokens.len() > 64 {
        return false;
    }
    if tokens.len() == 3 && tokens[1] == "-" {
        return npm_hyphen_matches(tokens[0], tokens[2], locked);
    }
    let mut merged: Vec<String> = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        if (token == "<" || token == ">" || token == "<=" || token == ">=" || token == "=")
            && index + 1 < tokens.len()
        {
            merged.push(format!("{}{}", token, tokens[index + 1]));
            index += 2;
        } else {
            merged.push(token.to_owned());
            index += 1;
        }
    }
    let mut comparators: Vec<String> = Vec::new();
    for token in &merged {
        match normalize_npm_comparator(token) {
            Some(expanded) => comparators.extend(expanded),
            None => return false,
        }
    }
    if comparators.is_empty() {
        return false;
    }
    match semver::VersionReq::parse(&comparators.join(", ")) {
        Ok(requirements) => requirements.matches(locked),
        Err(_) => false,
    }
}

/// Match one hyphen range `A - B`: partial starts fill with zero
/// (`1.2` starts at `1.2.0`), full ends are inclusive, partial ends
/// narrow below the next line (`- 2.3` stops below `2.4.0`). A bare
/// `*` end leaves that side unbounded; both sides unbounded fails
/// closed.
fn npm_hyphen_matches(left: &str, right: &str, locked: &semver::Version) -> bool {
    let left_trimmed = left.trim();
    let right_trimmed = right.trim();
    if left_trimmed.is_empty() || right_trimmed.is_empty() {
        return false;
    }
    let lower = if is_npm_wildcard(left_trimmed) {
        None
    } else {
        match parse_npm_partial(left_trimmed) {
            Some(partial) => npm_partial_lower(&partial),
            None => return false,
        }
    };
    let upper = if is_npm_wildcard(right_trimmed) {
        None
    } else {
        match parse_npm_partial(right_trimmed) {
            Some(partial) => match npm_partial_upper(&partial) {
                Some((bound, true)) => Some(format!("<={bound}")),
                Some((bound, false)) => Some(bound),
                None => return false,
            },
            None => return false,
        }
    };
    match (lower, upper) {
        (None, None) => false,
        (Some(low), None) => match semver::VersionReq::parse(&low) {
            Ok(requirements) => requirements.matches(locked),
            Err(_) => false,
        },
        (None, Some(high)) => match semver::VersionReq::parse(&high) {
            Ok(requirements) => requirements.matches(locked),
            Err(_) => false,
        },
        (Some(low), Some(high)) => match semver::VersionReq::parse(&format!("{low}, {high}")) {
            Ok(requirements) => requirements.matches(locked),
            Err(_) => false,
        },
    }
}

/// Bare `*`/`x` ends (after `v`-strip) leave a hyphen side unbounded.
fn is_npm_wildcard(text: &str) -> bool {
    let text = text.trim();
    let text = text
        .strip_prefix('v')
        .or_else(|| text.strip_prefix('V'))
        .unwrap_or(text);
    text.trim() == "*" || text.trim() == "x" || text.trim() == "X"
}

/// Normalize one whitespace-free npm comparator into Cargo-compatible
/// comparator strings (bare partials expand to a bounded pair).
/// Full bare versions are exact (`1.2.3` means `=1.2.3`); partial
/// bares are bounded ranges (`1.2` means `>=1.2.0, <1.3.0`); operators
/// on partials desugar per upstream (`>1.2` means `>=1.3.0`,
/// `<=1.2` means `<1.3.0`); carets and tildes on partials expand
/// explicitly (`~1` bounds below `2.0.0`, `^0` below `1.0.0`).
/// Returns `None` for malformed comparators (fail closed).
fn normalize_npm_comparator(token: &str) -> Option<Vec<String>> {
    if token.is_empty() || token.len() > 256 {
        return None;
    }
    let (operator, rest) = if let Some(rest) = token.strip_prefix(">=") {
        (">=", rest)
    } else if let Some(rest) = token.strip_prefix("<=") {
        ("<=", rest)
    } else if let Some(rest) = token.strip_prefix('>') {
        (">", rest)
    } else if let Some(rest) = token.strip_prefix('<') {
        ("<", rest)
    } else if let Some(rest) = token.strip_prefix('=') {
        ("=", rest)
    } else if let Some(rest) = token.strip_prefix('^') {
        ("^", rest)
    } else if let Some(rest) = token.strip_prefix('~') {
        ("~", rest)
    } else {
        ("", token)
    };
    if rest.is_empty() {
        return None;
    }
    if rest.starts_with(['=', '>', '<', '^', '~', '!']) {
        return None;
    }
    let rest = rest
        .strip_prefix('v')
        .or_else(|| rest.strip_prefix('V'))
        .unwrap_or(rest);
    if rest.is_empty() {
        return None;
    }
    if rest == "*" || rest == "x" || rest == "X" {
        if operator.is_empty() {
            return Some(vec!["*".to_owned()]);
        }
        return None;
    }
    let partial = parse_npm_partial(rest)?;
    let full = partial.minor.is_some() && partial.patch.is_some();
    match operator {
        "" | "=" => {
            if full {
                let mut exact = format!(
                    "{}.{}.{}",
                    partial.major,
                    partial.minor.as_deref().unwrap_or("0"),
                    partial.patch.as_deref().unwrap_or("0")
                );
                if let Some(pre) = &partial.prerelease {
                    exact.push('-');
                    exact.push_str(pre);
                }
                Some(vec![format!("={exact}")])
            } else {
                npm_partial_range(&partial)
            }
        }
        ">=" => {
            if full {
                Some(vec![format!(">={}", npm_full_text(&partial))])
            } else {
                Some(vec![format!(">={}", npm_fill_zero(&partial))])
            }
        }
        "<=" => {
            if full {
                Some(vec![format!("<={}", npm_full_text(&partial))])
            } else {
                npm_exclusive_upper(&partial).map(|bound| vec![bound])
            }
        }
        ">" => {
            if full {
                Some(vec![format!(">{}", npm_full_text(&partial))])
            } else {
                npm_inclusive_next(&partial).map(|bound| vec![bound])
            }
        }
        "<" => {
            if full {
                Some(vec![format!("<{}", npm_full_text(&partial))])
            } else {
                Some(vec![format!("<{}", npm_fill_zero(&partial))])
            }
        }
        "^" => npm_caret_range(&partial),
        "~" => npm_tilde_range(&partial),
        _ => None,
    }
}

/// Full `major.minor.patch[-prerelease]` text for operator passthrough.
fn npm_full_text(partial: &NpmPartial) -> String {
    let mut out = format!(
        "{}.{}.{}",
        partial.major,
        partial.minor.as_deref().unwrap_or("0"),
        partial.patch.as_deref().unwrap_or("0")
    );
    if let Some(pre) = &partial.prerelease {
        out.push('-');
        out.push_str(pre);
    }
    out
}

/// Partial with missing parts filled by zero (`1.2` reads `1.2.0`).
fn npm_fill_zero(partial: &NpmPartial) -> String {
    format!(
        "{}.{}.{}",
        partial.major,
        partial.minor.as_deref().unwrap_or("0"),
        partial.patch.as_deref().unwrap_or("0")
    )
}

/// Bounded pair for one bare partial (`1.2` reads
/// `>=1.2.0, <1.3.0`; `1` reads `>=1.0.0, <2.0.0`).
fn npm_partial_range(partial: &NpmPartial) -> Option<Vec<String>> {
    Some(vec![
        format!(">={}", npm_fill_zero(partial)),
        npm_exclusive_upper(partial)?,
    ])
}

/// Exclusive upper bound below the next partially-specified line.
fn npm_exclusive_upper(partial: &NpmPartial) -> Option<String> {
    if let Some(minor) = &partial.minor {
        Some(format!(
            "<{}.{}.0",
            partial.major,
            increment_npm_numeric(minor)?
        ))
    } else {
        Some(format!("<{}.0.0", increment_npm_numeric(&partial.major)?))
    }
}

/// Inclusive lower bound at the next partially-specified line
/// (`>1.2` reads `>=1.3.0`; `>1` reads `>=2.0.0`).
fn npm_inclusive_next(partial: &NpmPartial) -> Option<String> {
    if let Some(minor) = &partial.minor {
        Some(format!(
            ">={}.{}.0",
            partial.major,
            increment_npm_numeric(minor)?
        ))
    } else {
        Some(format!(">={}.0.0", increment_npm_numeric(&partial.major)?))
    }
}

/// Caret expansion per upstream: `^1.2.3` bounds below `2.0.0`,
/// `^0.2.3` below `0.3.0`, `^0.0.3` below `0.0.4`; partials pin the
/// missing parts first (`^1.2` reads `^1.2.0`, `^0` below `1.0.0`).
fn npm_caret_range(partial: &NpmPartial) -> Option<Vec<String>> {
    let minor = partial.minor.as_deref().unwrap_or("0");
    let patch = partial.patch.as_deref().unwrap_or("0");
    let mut lower = format!(">={}.{}.{}", partial.major, minor, patch);
    if let Some(pre) = &partial.prerelease {
        lower.push('-');
        lower.push_str(pre);
    }
    let upper = if partial.major != "0" {
        let bumped = increment_npm_numeric(&partial.major)?;
        format!("<{bumped}.0.0")
    } else if partial.minor.as_deref().unwrap_or("0") != "0" {
        let bumped = increment_npm_numeric(minor)?;
        format!("<0.{bumped}.0")
    } else if partial.patch.is_some() {
        let bumped = increment_npm_numeric(patch)?;
        format!("<0.0.{bumped}")
    } else if partial.minor.is_some() {
        let bumped = increment_npm_numeric(minor)?;
        format!("<0.{bumped}.0")
    } else {
        let bumped = increment_npm_numeric(&partial.major)?;
        format!("<{bumped}.0.0")
    };
    Some(vec![lower, upper])
}

/// Tilde expansion per upstream: `~1.2.3` bounds below `1.3.0`,
/// `~1.2` below `1.3.0`, `~1` below `2.0.0`.
fn npm_tilde_range(partial: &NpmPartial) -> Option<Vec<String>> {
    let minor = partial.minor.as_deref().unwrap_or("0");
    let patch = partial.patch.as_deref().unwrap_or("0");
    let mut lower = format!(">={}.{}.{}", partial.major, minor, patch);
    if let Some(pre) = &partial.prerelease {
        lower.push('-');
        lower.push_str(pre);
    }
    let upper = match &partial.minor {
        Some(minor_value) => {
            let bumped = increment_npm_numeric(minor_value)?;
            format!("<{}.{}.0", partial.major, bumped)
        }
        None => {
            let bumped = increment_npm_numeric(&partial.major)?;
            format!("<{bumped}.0.0")
        }
    };
    Some(vec![lower, upper])
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
    fn cargo_edges_pin_star_and_fail_closed_or_hyphen() {
        // Star matches any release; Cargo has no `||` unions or hyphen
        // ranges, so those spellings fail closed instead of looking clean.
        assert!(version_in_scope("*", "1.2.3"));
        assert!(!version_in_scope("1.0.0 || 2.0.0", "1.0.0"));
        assert!(!version_in_scope("1.2.3 - 2.3.4", "1.5.0"));
        assert!(!version_in_scope(">=1.0.0 || <0.5.0", "0.4.0"));
    }

    #[test]
    fn npm_ranges_cover_star_or_hyphen_and_prerelease_edges() {
        // Star and wildcards match any release in range.
        assert!(npm_in_scope("*", "9.9.9"));
        assert!(npm_in_scope("x", "1.2.3"));
        assert!(npm_in_scope("1.2.x", "1.2.9"));
        assert!(!npm_in_scope("1.2.x", "1.3.0"));
        assert!(npm_in_scope("1.2", "1.2.5"));
        assert!(!npm_in_scope("1.2", "1.3.0"));
        // `||` unions fire on any branch.
        assert!(npm_in_scope("1.2.7 || >=1.2.9 <2.0.0", "1.2.7"));
        assert!(npm_in_scope("1.2.7 || >=1.2.9 <2.0.0", "1.2.9"));
        assert!(npm_in_scope("1.2.7 || >=1.2.9 <2.0.0", "1.5.0"));
        assert!(!npm_in_scope("1.2.7 || >=1.2.9 <2.0.0", "1.2.8"));
        assert!(!npm_in_scope("1.2.7 || >=1.2.9 <2.0.0", "2.0.0"));
        // Hyphen ranges are inclusive on full ends, narrowed on partials.
        assert!(npm_in_scope("1.2.3 - 2.3.4", "1.2.3"));
        assert!(npm_in_scope("1.2.3 - 2.3.4", "2.0.0"));
        assert!(npm_in_scope("1.2.3 - 2.3.4", "2.3.4"));
        assert!(!npm_in_scope("1.2.3 - 2.3.4", "1.2.2"));
        assert!(!npm_in_scope("1.2.3 - 2.3.4", "2.3.5"));
        assert!(npm_in_scope("1.2 - 2.3", "2.3.9"));
        assert!(!npm_in_scope("1.2 - 2.3", "2.4.0"));
        // Space- and comma-separated sets, carets, tildes, bare exact.
        assert!(npm_in_scope(">=1.2.7 <1.3.0", "1.2.9"));
        assert!(!npm_in_scope(">=1.2.7 <1.3.0", "1.3.0"));
        assert!(npm_in_scope(">=1.2.7, <1.3.0", "1.2.9"));
        assert!(npm_in_scope("^1.2.3", "1.9.0"));
        assert!(!npm_in_scope("^1.2.3", "2.0.0"));
        assert!(npm_in_scope("~1.2.3", "1.2.9"));
        assert!(!npm_in_scope("~1.2.3", "1.3.0"));
        assert!(npm_in_scope("1.2.3", "1.2.3"));
        assert!(!npm_in_scope("1.2.3", "1.2.4"));
        assert!(npm_in_scope("v1.2.3", "1.2.3"));
        // Prereleases follow the narrow upstream rule: a bare range never
        // covers a prerelease, a same-tuple prerelease comparator does.
        assert!(!npm_in_scope(">=1.0.0", "2.0.0-alpha"));
        assert!(npm_in_scope(">=1.0.0-alpha, <2.0.0", "1.0.0-alpha"));
        // Malformed scopes fail closed, never a false positive.
        assert!(!npm_in_scope("", "1.2.3"));
        assert!(!npm_in_scope("not a range", "1.2.3"));
        assert!(!npm_in_scope("1.x.3", "1.2.3"));
        assert!(!npm_in_scope("01.2.3", "1.2.3"));
        assert!(!npm_in_scope(">=", "1.2.3"));
        assert!(!npm_in_scope("1.2.3", ""));
        assert!(!npm_in_scope("1.2.3", "banana"));
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
