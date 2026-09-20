//! Dependency-vulnerability matching for `dx audit`.
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
//! clean (both wont-fix, auditor-owned by this module
//! plus [`crate::locks`]; Git SHAs carry no OSV version identity and
//! SHA-to-version mapping needs a network resolver forbidden by the
//! offline contract, while private packages have no upstream identity
//! by definition). A recognized assessable package with no matching
//! advisories is clean, not a coverage failure. Advisory-specific
//! risk acceptance never waives missing assessment, and an empty
//! findings list alone is never evidence that every selected
//! dependency was assessed.
//!
//! This module matches over injected records only, so severity,
//! fix-preservation, unknown handling, and incomplete mapping stay
//! deterministic and unit-testable without network access or any
//! auditor binary. Version-range narrowing uses upstream semantics:
//! Cargo-flavor semver through [`crate::exception::version_in_scope`]
//! for Cargo, Go via [`go_in_scope`] which normalizes `go.mod` `v`
//! prefixes first, npm-native ranges through
//! [`crate::exception::npm_in_scope`] for npm, Maven-native ordering
//! plus interval matching for Maven, and NuGet-native ordering plus
//! interval matching for NuGet, exactly like the exception lifecycle
//! deferral in [`crate::exception`].

use serde::{Deserialize, Serialize};

use crate::exception::{
    check_expiry, npm_in_scope, version_in_scope, ExceptionProblem, FindingRef, RiskException,
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
    /// (`>=1.2.0, <2.0.0` for Cargo semver sets, Go same plus `v`-prefix
    /// normalization; npm ranges such as `>=1.2.7 <1.3.0`,
    /// `1.2.7 || >=1.2.9 <2.0.0`, or `1.2.3 - 2.3.4`; Maven intervals
    /// such as `[1.0,2.0)` or exact versions; NuGet intervals such as
    /// `[1.0,2.0)` or exact versions).
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

/// Assessment limitation for Git-revision dependencies
/// wont-fix, auditor-owned): the SHA carries no OSV version identity.
pub const REASON_GIT: &str = "unsupported git revision";
/// Assessment limitation for private packages with no upstream advisory
/// identity (wont-fix, auditor-owned): callers mark via
/// [`LockedPackage::is_private`]; lock readers never infer it.
pub const REASON_PRIVATE: &str = "unidentified private package";

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
/// scope. Cargo uses upstream Cargo-flavor semver via
/// [`version_in_scope`]; Go uses the same semantics via [`go_in_scope`]
/// (which normalizes `go.mod` `v` prefixes first); npm uses npm-native
/// ranges via [`npm_in_scope`]; Maven uses Maven-native ordering plus
/// interval matching via [`maven_in_scope`]; NuGet uses NuGet-native
/// ordering plus interval matching via [`nuget_in_scope`]. Unparseable
/// scopes or versions fail closed to `false` for semver, npm, Maven,
/// and NuGet sets, and to exact-match only for other sets.
pub fn version_affected(set: &str, scope: &str, version: &str) -> bool {
    match set {
        "cargo" => version_in_scope(scope, version),
        "npm" => npm_in_scope(scope, version),
        "go" => go_in_scope(scope, version),
        "maven" => maven_in_scope(scope, version),
        "nuget" => nuget_in_scope(scope, version),
        _ => scope.trim() == version.trim() && !scope.trim().is_empty(),
    }
}

/// Strip one Go `v` prefix where a version token starts: at the text
/// start or after a comparator, separator, or opening boundary, and only
/// before a digit, so words containing `v` never mangle. Both advisory
/// scopes (`>=v1.0.0, <v2.0.0`) and locked versions (`v0.6.0`,
/// pseudo-versions, `+incompatible` suffixes) normalize to the bare
/// Cargo-flavor semver the shared matcher owns.
fn strip_go_v(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (index, current) in chars.iter().enumerate() {
        if *current == 'v' || *current == 'V' {
            let prev_ok = index == 0
                || matches!(
                    chars[index - 1],
                    ' ' | '\t' | ',' | '<' | '>' | '=' | '~' | '^' | '!' | '('
                );
            let next_ok = chars
                .get(index + 1)
                .is_some_and(|next| next.is_ascii_digit());
            if prev_ok && next_ok {
                continue;
            }
        }
        out.push(*current);
    }
    out
}

/// Go affected-scope matching: [`strip_go_v`] normalization on both the
/// advisory scope and the locked version, then upstream Cargo-flavor
/// semver via [`version_in_scope`]. Unparseable inputs fail closed to
/// `false`, never a false positive.
pub fn go_in_scope(scope: &str, version: &str) -> bool {
    version_in_scope(&strip_go_v(scope), &strip_go_v(version))
}

/// Maven version token after normalization: numeric tokens compare
/// numerically (length then lexicographic, no overflow), qualifier
/// tokens compare via Maven ordering (case-insensitive, `ga`/`final`/
/// `release` as release, `cr` as `rc`, single-letter `a`/`b`/`m`
/// followed by a digit as `alpha`/`beta`/`milestone`).
#[derive(Clone, Debug, Eq, PartialEq)]
enum MavenToken {
    Numeric(String),
    Qualifier(String),
}

/// True for trailing-null tokens trimmed per hyphen segment: numeric
/// zero plus release qualifiers (`""`, already covering `ga`/`final`/
/// `release` via aliasing).
fn is_maven_null_token(token: &MavenToken) -> bool {
    match token {
        MavenToken::Numeric(value) => value == "0",
        MavenToken::Qualifier(value) => value.is_empty(),
    }
}

/// Numeric comparison without overflow: stripped (no leading zeros
/// unless `"0"`), longer digit runs are greater, ties break
/// lexicographically.
fn compare_maven_numeric(left: &str, right: &str) -> std::cmp::Ordering {
    if left.len() != right.len() {
        return left.len().cmp(&right.len());
    }
    left.cmp(right)
}

/// Maven qualifier ordering: `alpha < beta < milestone < rc < snapshot
/// < "" < sp`, with unknown qualifiers after all known ones in lexical
/// order (case-insensitive, inputs already lowercased). This matches
/// `ComparableVersion` (`unknown after known`, `ga`/`final`/`release`
/// as release).
fn compare_maven_qualifier(left: &str, right: &str) -> std::cmp::Ordering {
    const KNOWN: [&str; 7] = ["alpha", "beta", "milestone", "rc", "snapshot", "", "sp"];
    let mut left_index: Option<usize> = None;
    let mut right_index: Option<usize> = None;
    for (index, known) in KNOWN.iter().enumerate() {
        if *known == left {
            left_index = Some(index);
        }
        if *known == right {
            right_index = Some(index);
        }
    }
    match (left_index, right_index) {
        (Some(left_pos), Some(right_pos)) => left_pos.cmp(&right_pos),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.cmp(right),
    }
}

/// One remaining token against null (exhausted peer): numeric zero and
/// release qualifiers equal null, smaller qualifiers (e.g. `snapshot`)
/// are less, larger ones (`sp`, unknowns) and non-zero numbers are
/// greater.
fn maven_token_vs_null(token: &MavenToken) -> std::cmp::Ordering {
    match token {
        MavenToken::Numeric(value) => {
            if value == "0" {
                std::cmp::Ordering::Equal
            } else {
                std::cmp::Ordering::Greater
            }
        }
        MavenToken::Qualifier(value) => compare_maven_qualifier(value, ""),
    }
}

/// Raw Maven tokenization: split between `.`/`-`/`_` plus digit to
/// non-digit transitions (transitions and `_` normalize to `-`).
/// Empty tokens become numeric `"0"`. No aliasing here; that lands in
/// normalization with next-token lookahead.
fn tokenize_maven_raw(version: &str) -> Vec<(char, String, bool)> {
    let mut out: Vec<(char, String, bool)> = Vec::new();
    let mut current = String::new();
    let mut current_is_digit: Option<bool> = None;
    let mut next_sep: char = ' ';
    for c in version.chars() {
        if c == '.' || c == '-' || c == '_' {
            match current_is_digit {
                None => {
                    out.push((next_sep, "0".to_owned(), true));
                }
                Some(is_digit) => {
                    out.push((next_sep, std::mem::take(&mut current), is_digit));
                }
            }
            current_is_digit = None;
            next_sep = if c == '.' { '.' } else { '-' };
        } else if c.is_ascii_digit() {
            match current_is_digit {
                Some(false) => {
                    out.push((next_sep, std::mem::take(&mut current), false));
                    next_sep = '-';
                    current.push(c);
                    current_is_digit = Some(true);
                }
                _ => {
                    current.push(c);
                    current_is_digit = Some(true);
                }
            }
        } else if current_is_digit == Some(true) {
            out.push((next_sep, std::mem::take(&mut current), true));
            next_sep = '-';
            current.push(c);
            current_is_digit = Some(false);
        } else {
            current.push(c);
            current_is_digit = Some(false);
        }
    }
    match current_is_digit {
        None => {
            if !out.is_empty() {
                out.push((next_sep, "0".to_owned(), true));
            }
        }
        Some(is_digit) => {
            out.push((next_sep, current, is_digit));
        }
    }
    out
}

/// Normalized Maven token list: lowercased qualifiers with aliases,
/// numerics stripped, trailing nulls trimmed per hyphen segment with
/// trailing empty segments dropped. Flat list keeps `-` for segment
/// starts and `.` within segments (`_` already normalized).
fn parse_maven_version(version: &str) -> Vec<(char, MavenToken)> {
    let trimmed = version.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let raw = tokenize_maven_raw(trimmed);
    if raw.is_empty() {
        return Vec::new();
    }
    let mut mapped: Vec<(char, MavenToken)> = Vec::new();
    for (index, (sep, text, is_digit)) in raw.iter().enumerate() {
        if *is_digit {
            let stripped = text.trim_start_matches('0');
            let normalized = if stripped.is_empty() {
                "0".to_owned()
            } else {
                stripped.to_owned()
            };
            mapped.push((*sep, MavenToken::Numeric(normalized)));
        } else {
            let lower = text.to_ascii_lowercase();
            let aliased = if lower == "ga" || lower == "final" || lower == "release" {
                String::new()
            } else if lower == "cr" {
                "rc".to_owned()
            } else if (lower == "a" || lower == "b" || lower == "m")
                && index + 1 < raw.len()
                && raw[index + 1].2
            {
                if lower == "a" {
                    "alpha".to_owned()
                } else if lower == "b" {
                    "beta".to_owned()
                } else {
                    "milestone".to_owned()
                }
            } else {
                lower
            };
            mapped.push((*sep, MavenToken::Qualifier(aliased)));
        }
    }
    let mut segments: Vec<Vec<(char, MavenToken)>> = Vec::new();
    let mut current_seg: Vec<(char, MavenToken)> = Vec::new();
    for (sep, token) in mapped {
        if sep == '-' && !current_seg.is_empty() {
            segments.push(std::mem::take(&mut current_seg));
            current_seg.push(('-', token));
        } else {
            current_seg.push((sep, token));
        }
    }
    if !current_seg.is_empty() {
        segments.push(current_seg);
    }
    for segment in segments.iter_mut() {
        while let Some((_, token)) = segment.last() {
            if is_maven_null_token(token) {
                segment.pop();
            } else {
                break;
            }
        }
    }
    segments.retain(|segment| !segment.is_empty());
    let mut out: Vec<(char, MavenToken)> = Vec::new();
    for (seg_index, segment) in segments.into_iter().enumerate() {
        for (tok_index, (_, token)) in segment.into_iter().enumerate() {
            if seg_index == 0 && tok_index == 0 {
                out.push((' ', token));
            } else if tok_index == 0 {
                out.push(('-', token));
            } else {
                out.push(('.', token));
            }
        }
    }
    out
}

/// Maven-native version comparison following `ComparableVersion` for
/// the audited subset: numeric numerically, qualifiers by Maven order
/// (unknown after known, lexical), qualifier before numeric, and
/// hyphen-number before dot-number regardless of value. Trailing
/// nulls already trimmed, so exhaustion compares remainders against
/// null.
pub fn maven_compare(left: &str, right: &str) -> std::cmp::Ordering {
    let left_tokens = parse_maven_version(left);
    let right_tokens = parse_maven_version(right);
    let common = left_tokens.len().min(right_tokens.len());
    for index in 0..common {
        let (sep_left, token_left) = &left_tokens[index];
        let (sep_right, token_right) = &right_tokens[index];
        match (token_left, token_right) {
            (MavenToken::Numeric(left_num), MavenToken::Numeric(right_num)) => {
                if sep_left == sep_right {
                    match compare_maven_numeric(left_num, right_num) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    // Hyphen-number sorts before dot-number even when
                    // values differ (`1-2 < 1.1` per List vs Int).
                    let left_rank = if *sep_left == '-' { 0 } else { 1 };
                    let right_rank = if *sep_right == '-' { 0 } else { 1 };
                    if left_rank != right_rank {
                        if left_rank < right_rank {
                            return std::cmp::Ordering::Less;
                        }
                        return std::cmp::Ordering::Greater;
                    }
                    match compare_maven_numeric(left_num, right_num) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    }
                }
            }
            (MavenToken::Qualifier(left_q), MavenToken::Qualifier(right_q)) => {
                match compare_maven_qualifier(left_q, right_q) {
                    std::cmp::Ordering::Equal => continue,
                    other => return other,
                }
            }
            (MavenToken::Qualifier(_), MavenToken::Numeric(_)) => {
                return std::cmp::Ordering::Less;
            }
            (MavenToken::Numeric(_), MavenToken::Qualifier(_)) => {
                return std::cmp::Ordering::Greater;
            }
        }
    }
    if left_tokens.len() == right_tokens.len() {
        return std::cmp::Ordering::Equal;
    }
    if left_tokens.len() > right_tokens.len() {
        for (_, token) in left_tokens.iter().skip(common) {
            match maven_token_vs_null(token) {
                std::cmp::Ordering::Equal => continue,
                other => return other,
            }
        }
        return std::cmp::Ordering::Equal;
    }
    for (_, token) in right_tokens.iter().skip(common) {
        match maven_token_vs_null(token) {
            std::cmp::Ordering::Equal => continue,
            std::cmp::Ordering::Less => return std::cmp::Ordering::Greater,
            std::cmp::Ordering::Greater => return std::cmp::Ordering::Less,
            // Equal handled above; no other variants exist, but keep
            // exhaustive for clarity.
        }
    }
    std::cmp::Ordering::Equal
}

/// Maven equality (normalized comparison): `1.0` equals `1.0.0`,
/// `1.ga` equals `1`, `1-a1` equals `1-alpha-1`. Empty or overlong
/// inputs never equal.
pub fn maven_version_eq(left: &str, right: &str) -> bool {
    let left_trimmed = left.trim();
    let right_trimmed = right.trim();
    if left_trimmed.is_empty() || right_trimmed.is_empty() {
        return false;
    }
    if left_trimmed.len() > 256 || right_trimmed.len() > 256 {
        return false;
    }
    maven_compare(left_trimmed, right_trimmed) == std::cmp::Ordering::Equal
}

/// Maven-native affected-scope matching: bare versions use
/// Maven equality, bracketed intervals use Maven ordering with
/// inclusive `[`/`]` versus exclusive `(`/`)` bounds, unions via
/// comma-separated intervals such as `(,1.0],[1.2,)`, and empty bounds
/// as unbounded. Malformed scopes, empty inputs, and overlong inputs
/// fail closed to `false` (never a false positive). Note Maven
/// includes pre-releases under exclusive upper bounds (e.g.
/// `[1.0,2.0)` matches `2.0-rc1`), matching upstream ordering.
pub fn maven_in_scope(scope: &str, version: &str) -> bool {
    let scope_trimmed = scope.trim();
    let version_trimmed = version.trim();
    if scope_trimmed.is_empty() || version_trimmed.is_empty() {
        return false;
    }
    if scope_trimmed.len() > 4096 || version_trimmed.len() > 256 {
        return false;
    }
    let has_brackets = scope_trimmed.contains('[')
        || scope_trimmed.contains('(')
        || scope_trimmed.contains(']')
        || scope_trimmed.contains(')');
    if !has_brackets {
        return maven_version_eq(scope_trimmed, version_trimmed);
    }
    let chars: Vec<char> = scope_trimmed.chars().collect();
    let mut index: usize = 0;
    let mut matched = false;
    let mut found_interval = false;
    while index < chars.len() {
        let current = chars[index];
        if current == '[' || current == '(' {
            let start = current;
            let mut close_index = index + 1;
            while close_index < chars.len()
                && chars[close_index] != ']'
                && chars[close_index] != ')'
            {
                close_index += 1;
            }
            if close_index >= chars.len() {
                return false;
            }
            let end = chars[close_index];
            let content: String = chars[index + 1..close_index].iter().collect();
            found_interval = true;
            let parts: Vec<&str> = content.split(',').collect();
            let (lower, upper, lower_inclusive, upper_inclusive, valid) = if parts.len() == 1 {
                let bound = parts[0].trim();
                if bound.is_empty() {
                    (String::new(), String::new(), false, false, false)
                } else {
                    (
                        bound.to_owned(),
                        bound.to_owned(),
                        start == '[',
                        end == ']',
                        true,
                    )
                }
            } else if parts.len() == 2 {
                let low = parts[0].trim().to_owned();
                let high = parts[1].trim().to_owned();
                if low.is_empty() && high.is_empty() {
                    (String::new(), String::new(), false, false, false)
                } else {
                    (low, high, start == '[', end == ']', true)
                }
            } else {
                (String::new(), String::new(), false, false, false)
            };
            if valid
                && lower.len() <= 256
                && upper.len() <= 256
                && interval_matches(
                    &lower,
                    &upper,
                    lower_inclusive,
                    upper_inclusive,
                    version_trimmed,
                )
            {
                matched = true;
            }
            index = close_index + 1;
        } else if current == ',' || current.is_whitespace() {
            index += 1;
        } else {
            return false;
        }
    }
    if !found_interval {
        return false;
    }
    matched
}

/// One Maven interval against a locked version: empty bounds are
/// unbounded, otherwise Maven ordering with inclusive/exclusive edges.
fn interval_matches(
    lower: &str,
    upper: &str,
    lower_inclusive: bool,
    upper_inclusive: bool,
    version: &str,
) -> bool {
    if !lower.is_empty() {
        match maven_compare(version, lower) {
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => {
                if !lower_inclusive {
                    return false;
                }
            }
            std::cmp::Ordering::Greater => {}
        }
    }
    if !upper.is_empty() {
        match maven_compare(version, upper) {
            std::cmp::Ordering::Greater => return false,
            std::cmp::Ordering::Equal => {
                if !upper_inclusive {
                    return false;
                }
            }
            std::cmp::Ordering::Less => {}
        }
    }
    true
}

/// Set-aware exception version narrowing: Cargo uses
/// [`version_in_scope`], Go uses [`go_in_scope`] (`v`-prefix
/// normalization, same semver), npm uses [`npm_in_scope`], Maven uses
/// [`maven_in_scope`], NuGet uses [`nuget_in_scope`], and remaining
/// sets stay exact-match.
fn exception_version_in_scope(set: &str, scope: &str, version: &str) -> bool {
    match set {
        "cargo" => version_in_scope(scope, version),
        "npm" => npm_in_scope(scope, version),
        "go" => go_in_scope(scope, version),
        "maven" => maven_in_scope(scope, version),
        "nuget" => nuget_in_scope(scope, version),
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
                reason: REASON_GIT,
            });
            continue;
        }
        if package.is_private {
            unassessed.push(Unassessed {
                package: package.name.clone(),
                set: package.set.clone(),
                reason: REASON_PRIVATE,
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
                    && exception_version_in_scope(
                        &finding.set,
                        &exception.versions,
                        &finding.version,
                    )
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

#[path = "vuln_nuget.rs"]
mod vuln_nuget;

pub use vuln_nuget::*;

#[cfg(test)]
#[path = "vuln_tests.rs"]
mod vuln_tests;
