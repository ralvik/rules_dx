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
//! prefixes, pseudo-versions, and `+incompatible` suffixes with
//! Cargo-flavor ordering but no prerelease gate, npm-native ranges
//! through [`crate::exception::npm_in_scope`] for npm, Maven-native
//! ordering plus interval matching for Maven, and NuGet-native ordering
//! plus interval matching for NuGet, exactly like the exception lifecycle
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

/// One OSV-format advisory record from the identified snapshot. This is
/// the matching shape projected from typed OSV via the upstream `osv`
/// crate (`schema` feature only, offline; issue #676): upstream advisory
/// identity, affected package and version scope, severity text, and
/// remediation. Legacy V1 minimal snapshots still parse; unknown fields
/// ignore so snapshot evolution never breaks matching.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Advisory {
    /// Upstream advisory identity (e.g. `GHSA-aaaa-bbbb-cccc`, `RUSTSEC-...`).
    pub id: String,
    /// Affected package name.
    pub package: String,
    /// Affected version scope, upstream version semantics
    /// (`>=1.2.0, <2.0.0` for Cargo semver sets, Go same plus `v`-prefix
    /// normalization with pseudo-versions matching bare ranges and
    /// `+incompatible` as build metadata; npm ranges such as `>=1.2.7 <1.3.0`,
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
/// [`version_in_scope`]; Go uses Cargo-flavor ordering via [`go_in_scope`]
/// (`v`-prefix normalization plus pseudo-versions matching bare ranges,
/// `+incompatible` as build metadata); npm uses npm-native ranges via
/// [`npm_in_scope`]; Maven uses Maven-native ordering plus interval
/// matching via [`maven_in_scope`]; NuGet uses NuGet-native ordering plus
/// interval matching via [`nuget_in_scope`]. Unparseable scopes or
/// versions fail closed to `false` for semver, npm, Maven, and NuGet
/// sets, and to exact-match only for other sets.
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
/// pseudo-versions, `+incompatible` suffixes) normalize to bare semver
/// for the Go matcher below.
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
/// advisory scope and the locked version, then Cargo-flavor ordering
/// without the Cargo prerelease gate (issue #679).
///
/// Spike result: `v`-strip preprocessing suffices, no Go-aware crate.
/// `semver` ordering already matches Go precedence (pseudo-versions sort
/// as prereleases below their release, above the prior tag; `+incompatible`
/// rides build metadata ignored for precedence), so only the match gate
/// differs. Cargo's `VersionReq::matches` excludes prereleases from bare
/// ranges (a same-tuple prerelease comparator is required), which would
/// hide every pseudo-version behind a false negative. Go evaluates each
/// comparator by ordering alone ([`go_matches_impl`]), so
/// `>=v1.0.0, <v2.0.0` covers `v1.2.4-0.20240101120000-abcdef123456`
/// while `>=v1.2.4` still excludes it and `=v1.2.4` stays exact.
/// Unparseable inputs fail closed to `false`, never a false positive.
pub fn go_in_scope(scope: &str, version: &str) -> bool {
    let scope_norm = strip_go_v(scope);
    let version_norm = strip_go_v(version);
    let requirements = match semver::VersionReq::parse(&scope_norm) {
        Ok(requirements) => requirements,
        Err(_) => return false,
    };
    let version = match semver::Version::parse(&version_norm) {
        Ok(version) => version,
        Err(_) => return false,
    };
    // `*` parses to zero comparators (`VersionReq::STAR`): `all` on empty
    // is true, so star covers pseudo-versions (unlike Cargo's gate).
    requirements
        .comparators
        .iter()
        .all(|comparator| go_matches_impl(comparator, &version))
}

/// One Go comparator by ordering alone, mirroring upstream `semver`
/// `matches_impl` without the `pre_is_compatible` gate. Exact and
/// wildcard keep prerelease equality (so `=1.2.3` never covers
/// `1.2.3-0.20240101-abcdef`); ranges, carets, and tildes compare by
/// precedence, so pseudo-versions match bare ranges they fall inside.
/// Future unknown operators fail closed.
fn go_matches_impl(comparator: &semver::Comparator, version: &semver::Version) -> bool {
    match comparator.op {
        semver::Op::Exact | semver::Op::Wildcard => go_matches_exact(comparator, version),
        semver::Op::Greater => go_matches_greater(comparator, version),
        semver::Op::GreaterEq => {
            go_matches_exact(comparator, version) || go_matches_greater(comparator, version)
        }
        semver::Op::Less => go_matches_less(comparator, version),
        semver::Op::LessEq => {
            go_matches_exact(comparator, version) || go_matches_less(comparator, version)
        }
        semver::Op::Tilde => go_matches_tilde(comparator, version),
        semver::Op::Caret => go_matches_caret(comparator, version),
        _ => false,
    }
}

/// Exact core match with prerelease equality (mirrors upstream).
fn go_matches_exact(comparator: &semver::Comparator, version: &semver::Version) -> bool {
    if version.major != comparator.major {
        return false;
    }
    if let Some(minor) = comparator.minor {
        if version.minor != minor {
            return false;
        }
    }
    if let Some(patch) = comparator.patch {
        if version.patch != patch {
            return false;
        }
    }
    version.pre == comparator.pre
}

/// Greater ordering (mirrors upstream).
fn go_matches_greater(comparator: &semver::Comparator, version: &semver::Version) -> bool {
    if version.major != comparator.major {
        return version.major > comparator.major;
    }
    let Some(minor) = comparator.minor else {
        return false;
    };
    if version.minor != minor {
        return version.minor > minor;
    }
    let Some(patch) = comparator.patch else {
        return false;
    };
    if version.patch != patch {
        return version.patch > patch;
    }
    version.pre > comparator.pre
}

/// Less ordering (mirrors upstream).
fn go_matches_less(comparator: &semver::Comparator, version: &semver::Version) -> bool {
    if version.major != comparator.major {
        return version.major < comparator.major;
    }
    let Some(minor) = comparator.minor else {
        return false;
    };
    if version.minor != minor {
        return version.minor < minor;
    }
    let Some(patch) = comparator.patch else {
        return false;
    };
    if version.patch != patch {
        return version.patch < patch;
    }
    version.pre < comparator.pre
}

/// Tilde ordering (mirrors upstream).
fn go_matches_tilde(comparator: &semver::Comparator, version: &semver::Version) -> bool {
    if version.major != comparator.major {
        return false;
    }
    if let Some(minor) = comparator.minor {
        if version.minor != minor {
            return false;
        }
    }
    if let Some(patch) = comparator.patch {
        if version.patch != patch {
            return version.patch > patch;
        }
    }
    version.pre >= comparator.pre
}

/// Caret ordering (mirrors upstream).
fn go_matches_caret(comparator: &semver::Comparator, version: &semver::Version) -> bool {
    if version.major != comparator.major {
        return false;
    }
    let Some(minor) = comparator.minor else {
        return true;
    };
    let Some(patch) = comparator.patch else {
        if comparator.major > 0 {
            return version.minor >= minor;
        }
        return version.minor == minor;
    };
    if comparator.major > 0 {
        if version.minor != minor {
            return version.minor > minor;
        }
        if version.patch != patch {
            return version.patch > patch;
        }
    } else if minor > 0 {
        if version.minor != minor {
            return false;
        }
        if version.patch != patch {
            return version.patch > patch;
        }
    } else if version.minor != minor || version.patch != patch {
        return false;
    }
    version.pre >= comparator.pre
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
/// normalization, pseudo-versions match bare ranges, `+incompatible` as
/// build metadata), npm uses [`npm_in_scope`], Maven uses
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

/// Parse one OSV-format advisory snapshot document (JSON array) into
/// records. Typed OSV parsing via the upstream `osv` crate (`schema`
/// feature only, offline local matching, no inventory upload; issue
/// #676) projects `osv::schema::Vulnerability` onto [`Advisory`] with no
/// matching-semantics change: withdrawn entries skip, unsupported
/// ecosystems skip, `affected[].ranges[].events`
/// (`introduced`/`fixed`/`last_affected`/`limit`) become per-interval
/// `versions` scopes in the set's upstream syntax so [`version_affected`]
/// applies unchanged, explicit `versions` lists become one scope per
/// version, `fixed` events preserve, severity keeps known
/// `critical|high|medium|low` words else empty (unknown, fails by
/// default). Unknown fields ignore. Legacy V1 minimal `Vec<Advisory>`
/// snapshots still parse (no format break); malformed JSON fails closed
/// with the document error.
pub fn parse_snapshot(text: &str) -> Result<Vec<Advisory>, String> {
    if let Ok(vulns) = serde_json::from_str::<Vec<osv::schema::Vulnerability>>(text) {
        return Ok(project_osv_snapshot(&vulns));
    }
    serde_json::from_str(text).map_err(|error| format!("invalid advisory snapshot: {error}"))
}

/// Map one OSV ecosystem to the owning V1 dependency set. Only the five
/// audited sets project; other ecosystems skip (never match, never clean
/// by themselves).
fn ecosystem_to_set(ecosystem: &osv::schema::Ecosystem) -> Option<&'static str> {
    match ecosystem {
        osv::schema::Ecosystem::CratesIO => Some("cargo"),
        osv::schema::Ecosystem::Npm => Some("npm"),
        osv::schema::Ecosystem::Go => Some("go"),
        osv::schema::Ecosystem::Maven(_) => Some("maven"),
        osv::schema::Ecosystem::NuGet => Some("nuget"),
        _ => None,
    }
}

/// One known severity word, else `None` (caller maps to empty/unknown).
/// Only `critical|high|medium|low|moderate` project; CVSS vectors and
/// other texts stay unknown and fail by default via [`normalize_level`].
fn known_severity_word(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "critical" | "high" | "medium" | "low" | "moderate" => Some(trimmed.to_owned()),
        _ => None,
    }
}

/// Severity text for one OSV `affected` entry: first known word in
/// affected `severity`, then top-level `severity`, then `severity` string
/// fields in affected/top `database_specific`/`ecosystem_specific`
/// objects (e.g. GHSA `{"severity":"high"}`); else empty (unknown).
fn osv_severity_text(
    vuln: &osv::schema::Vulnerability,
    affected: &osv::schema::Affected,
) -> String {
    if let Some(list) = affected.severity.as_ref() {
        for entry in list {
            if let Some(word) = known_severity_word(&entry.score) {
                return word;
            }
        }
    }
    if let Some(list) = vuln.severity.as_ref() {
        for entry in list {
            if let Some(word) = known_severity_word(&entry.score) {
                return word;
            }
        }
    }
    for value in [
        affected.database_specific.as_ref(),
        affected.ecosystem_specific.as_ref(),
        vuln.database_specific.as_ref(),
    ] {
        if let Some(serde_json::Value::Object(map)) = value {
            if let Some(serde_json::Value::String(score)) = map.get("severity") {
                if let Some(word) = known_severity_word(score) {
                    return word;
                }
            }
        }
    }
    String::new()
}

/// One OSV range timeline to affected intervals: `(lower, upper,
/// upper_inclusive)` where `None` is unbounded. `Introduced("0")` means
/// unbounded lower. A `fixed`/`last_affected`/`limit` without a preceding
/// `introduced` means unbounded lower. A trailing open `introduced`
/// means unbounded upper. Malformed timelines emit the conservative
/// intervals (extra findings, never missed vulns).
fn range_events_to_intervals(
    events: &[osv::schema::Event],
) -> Vec<(Option<String>, Option<String>, bool)> {
    let mut out: Vec<(Option<String>, Option<String>, bool)> = Vec::new();
    let mut open: Option<Option<String>> = None;
    for event in events {
        match event {
            osv::schema::Event::Introduced(version) => {
                if let Some(prev) = open.take() {
                    out.push((prev, None, false));
                }
                let trimmed = version.trim();
                if trimmed == "0" || trimmed.is_empty() {
                    open = Some(None);
                } else {
                    open = Some(Some(trimmed.to_owned()));
                }
            }
            osv::schema::Event::Fixed(version) => {
                let upper = version.trim().to_owned();
                if let Some(lower) = open.take() {
                    out.push((lower, Some(upper), false));
                } else {
                    out.push((None, Some(upper), false));
                }
            }
            osv::schema::Event::LastAffected(version) => {
                let upper = version.trim().to_owned();
                if let Some(lower) = open.take() {
                    out.push((lower, Some(upper), true));
                } else {
                    out.push((None, Some(upper), true));
                }
            }
            osv::schema::Event::Limit(version) => {
                let upper = version.trim().to_owned();
                if let Some(lower) = open.take() {
                    out.push((lower, Some(upper), false));
                } else {
                    out.push((None, Some(upper), false));
                }
            }
            _ => {}
        }
    }
    if let Some(lower) = open.take() {
        out.push((lower, None, false));
    }
    out
}

/// One affected interval to the set's upstream `versions` scope syntax so
/// [`version_affected`] applies unchanged: semver comparators for
/// cargo/npm/go (`*`, `<x`, `<=x`, `>=x`, `>=a, <b`, `>=a, <=b`), bracketed
/// intervals for maven/nuget (`[0,)`, `(,x)`, `(,x]`, `[x,)`,
/// `[a,b)`, `[a,b]`). Empty bounds are unbounded. Returns `None` for
/// unknown sets or empty scopes (caller skips).
fn interval_to_scope(
    set: &str,
    lower: &Option<String>,
    upper: &Option<String>,
    upper_inclusive: bool,
) -> Option<String> {
    let lower = lower
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let upper = upper
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    match set {
        "cargo" | "npm" | "go" => match (lower, upper) {
            (None, None) => Some("*".to_owned()),
            (None, Some(upper)) => {
                if upper_inclusive {
                    Some(format!("<={upper}"))
                } else {
                    Some(format!("<{upper}"))
                }
            }
            (Some(lower), None) => Some(format!(">={lower}")),
            (Some(lower), Some(upper)) => {
                if upper_inclusive {
                    Some(format!(">={lower}, <={upper}"))
                } else {
                    Some(format!(">={lower}, <{upper}"))
                }
            }
        },
        "maven" | "nuget" => match (lower, upper) {
            (None, None) => Some("[0,)".to_owned()),
            (None, Some(upper)) => {
                if upper_inclusive {
                    Some(format!("(,{upper}]"))
                } else {
                    Some(format!("(,{upper})"))
                }
            }
            (Some(lower), None) => Some(format!("[{lower},)")),
            (Some(lower), Some(upper)) => {
                if upper_inclusive {
                    Some(format!("[{lower},{upper}]"))
                } else {
                    Some(format!("[{lower},{upper})"))
                }
            }
        },
        _ => None,
    }
}

/// Project one OSV vulnerability's affected entries onto [`Advisory`]:
/// one advisory per explicit version plus one per range interval, with
/// shared id/package/set/severity/fixed. Withdrawn, empty ids, missing
/// packages, empty names, unsupported ecosystems, and empty scopes skip
/// (never match, never fail the snapshot).
fn project_osv_affected(
    vuln: &osv::schema::Vulnerability,
    affected: &osv::schema::Affected,
) -> Vec<Advisory> {
    let id = vuln.id.trim();
    if id.is_empty() {
        return Vec::new();
    }
    let package = match affected.package.as_ref() {
        Some(package) => package,
        None => return Vec::new(),
    };
    let name = package.name.trim();
    if name.is_empty() {
        return Vec::new();
    }
    let set = match ecosystem_to_set(&package.ecosystem) {
        Some(set) => set,
        None => return Vec::new(),
    };
    let severity = osv_severity_text(vuln, affected);
    let mut fixed: Vec<String> = Vec::new();
    if let Some(ranges) = affected.ranges.as_ref() {
        for range in ranges {
            if matches!(&range.range_type, osv::schema::RangeType::Git) {
                continue;
            }
            for event in &range.events {
                if let osv::schema::Event::Fixed(version) = event {
                    let trimmed = version.trim();
                    if !trimmed.is_empty() && !fixed.iter().any(|seen| seen == trimmed) {
                        fixed.push(trimmed.to_owned());
                    }
                }
            }
        }
    }
    let mut scopes: Vec<String> = Vec::new();
    if let Some(versions) = affected.versions.as_ref() {
        for version in versions {
            let trimmed = version.trim();
            if !trimmed.is_empty() && !scopes.iter().any(|seen| seen == trimmed) {
                scopes.push(trimmed.to_owned());
            }
        }
    }
    if let Some(ranges) = affected.ranges.as_ref() {
        for range in ranges {
            if matches!(&range.range_type, osv::schema::RangeType::Git) {
                continue;
            }
            for (lower, upper, inclusive) in range_events_to_intervals(&range.events) {
                if let Some(scope) = interval_to_scope(set, &lower, &upper, inclusive) {
                    if !scope.trim().is_empty() && !scopes.iter().any(|seen| seen == &scope) {
                        scopes.push(scope);
                    }
                }
            }
        }
    }
    scopes
        .into_iter()
        .map(|versions| Advisory {
            id: id.to_owned(),
            package: name.to_owned(),
            versions,
            severity: severity.clone(),
            fixed: fixed.clone(),
            set: set.to_owned(),
        })
        .collect()
}

/// Project a typed OSV snapshot onto [`Advisory`]. Withdrawn
/// vulnerabilities skip entirely; vulnerabilities without affected
/// entries yield zero advisories (never an error).
fn project_osv_snapshot(vulns: &[osv::schema::Vulnerability]) -> Vec<Advisory> {
    let mut out = Vec::new();
    for vuln in vulns {
        if vuln.withdrawn.is_some() {
            continue;
        }
        if let Some(entries) = vuln.affected.as_ref() {
            for affected in entries {
                out.extend(project_osv_affected(vuln, affected));
            }
        }
    }
    out
}

#[path = "vuln_nuget.rs"]
mod vuln_nuget;

pub use vuln_nuget::*;

#[cfg(test)]
#[path = "vuln_tests.rs"]
mod vuln_tests;
