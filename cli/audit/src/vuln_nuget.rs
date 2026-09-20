//! NuGet version comparison (split from `vuln.rs`). No behavior change.

/// Numeric comparison without overflow: stripped (no leading zeros
/// unless `"0"`), longer digit runs are greater, ties break
/// lexicographically.
fn compare_nuget_numeric(left: &str, right: &str) -> std::cmp::Ordering {
    if left.len() != right.len() {
        return left.len().cmp(&right.len());
    }
    left.cmp(right)
}

/// One NuGet prerelease label: numeric labels compare numerically and
/// sort before alphanumeric labels; alphanumeric labels compare
/// case-insensitively.
#[derive(Clone, Debug, Eq, PartialEq)]
enum NugetPrereleaseLabel {
    Numeric(String),
    Alpha(String),
}

/// Parsed NuGet version: four numeric parts (missing trailing parts as
/// `"0"`, leading zeros stripped) plus dot-separated prerelease labels.
/// Build metadata (`+...`) is stripped and never affects ordering.
#[derive(Clone, Debug, Eq, PartialEq)]
struct NugetVersion {
    parts: [String; 4],
    prerelease: Option<Vec<NugetPrereleaseLabel>>,
}

/// Parse one NuGet version per the `NuGetVersion` subset audited here:
/// one to four numeric core parts, optional `-prerelease` with
/// dot-separated labels, optional `+metadata` ignored. Leading zeros
/// strip, missing parts equal zero, prerelease labels compare
/// case-insensitively, and floating `*` never parses (fail closed).
/// Returns `None` for empty, overlong, or malformed inputs.
fn parse_nuget_version(version: &str) -> Option<NugetVersion> {
    let trimmed = version.trim();
    if trimmed.is_empty() || trimmed.len() > 256 {
        return None;
    }
    if trimmed.contains('*') {
        return None;
    }
    // Build metadata never affects ordering; strip at the first `+`.
    let (without_metadata, _) = match trimmed.split_once('+') {
        Some((before, _)) => (before, true),
        None => (trimmed, false),
    };
    let without_metadata = without_metadata.trim();
    if without_metadata.is_empty() {
        return None;
    }
    let (core_str, pre_str) = match without_metadata.split_once('-') {
        Some((core, pre)) => (core.trim(), Some(pre.trim())),
        None => (without_metadata, None),
    };
    if core_str.is_empty() {
        return None;
    }
    // Core must be one to four dot-separated numeric parts; anything
    // else (semver operators, brackets, whitespace) fails closed.
    let raw_parts: Vec<&str> = core_str.split('.').collect();
    if raw_parts.is_empty() || raw_parts.len() > 4 {
        return None;
    }
    let mut normalized: Vec<String> = Vec::new();
    for part in raw_parts {
        let part = part.trim();
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let stripped = part.trim_start_matches('0');
        normalized.push(if stripped.is_empty() {
            "0".to_owned()
        } else {
            stripped.to_owned()
        });
    }
    while normalized.len() < 4 {
        normalized.push("0".to_owned());
    }
    let parts: [String; 4] = [
        normalized[0].clone(),
        normalized[1].clone(),
        normalized[2].clone(),
        normalized[3].clone(),
    ];
    let prerelease = match pre_str {
        None => None,
        Some(pre) => {
            if pre.is_empty() {
                return None;
            }
            // Prerelease remainder may not carry brackets, commas, or
            // whitespace; labels split on `.` only.
            if pre.contains([
                '[', ']', '(', ')', ',', ' ', '\t', '\n', '\r', '+', '>', '<', '=', '^', '~', '!',
                '|', '&', ':', '/',
            ]) {
                return None;
            }
            let mut labels = Vec::new();
            for label in pre.split('.') {
                let label = label.trim();
                if label.is_empty() {
                    return None;
                }
                if !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                {
                    return None;
                }
                if label.bytes().all(|byte| byte.is_ascii_digit()) {
                    let stripped = label.trim_start_matches('0');
                    labels.push(NugetPrereleaseLabel::Numeric(if stripped.is_empty() {
                        "0".to_owned()
                    } else {
                        stripped.to_owned()
                    }));
                } else {
                    labels.push(NugetPrereleaseLabel::Alpha(label.to_ascii_lowercase()));
                }
            }
            if labels.is_empty() {
                return None;
            }
            Some(labels)
        }
    };
    Some(NugetVersion { parts, prerelease })
}

/// NuGet-native version comparison following `NuGetVersion`/
/// `VersionComparer` for the audited subset: four numeric parts
/// numerically, then release greater than any prerelease, then
/// dot-separated prerelease labels (numeric numerically with numeric
/// before alphanumeric, alphanumeric case-insensitively lexically,
/// shorter prefix before longer).
pub fn nuget_compare(left: &str, right: &str) -> std::cmp::Ordering {
    let left_parsed = parse_nuget_version(left);
    let right_parsed = parse_nuget_version(right);
    match (left_parsed, right_parsed) {
        (Some(left_version), Some(right_version)) => {
            for index in 0..4 {
                match compare_nuget_numeric(&left_version.parts[index], &right_version.parts[index])
                {
                    std::cmp::Ordering::Equal => continue,
                    other => return other,
                }
            }
            match (left_version.prerelease, right_version.prerelease) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(left_labels), Some(right_labels)) => {
                    let common = left_labels.len().min(right_labels.len());
                    for index in 0..common {
                        match (&left_labels[index], &right_labels[index]) {
                            (
                                NugetPrereleaseLabel::Numeric(left_num),
                                NugetPrereleaseLabel::Numeric(right_num),
                            ) => match compare_nuget_numeric(left_num, right_num) {
                                std::cmp::Ordering::Equal => continue,
                                other => return other,
                            },
                            (NugetPrereleaseLabel::Numeric(_), NugetPrereleaseLabel::Alpha(_)) => {
                                return std::cmp::Ordering::Less;
                            }
                            (NugetPrereleaseLabel::Alpha(_), NugetPrereleaseLabel::Numeric(_)) => {
                                return std::cmp::Ordering::Greater;
                            }
                            (
                                NugetPrereleaseLabel::Alpha(left_text),
                                NugetPrereleaseLabel::Alpha(right_text),
                            ) => match left_text.cmp(right_text) {
                                std::cmp::Ordering::Equal => continue,
                                other => return other,
                            },
                        }
                    }
                    left_labels.len().cmp(&right_labels.len())
                }
            }
        }
        // Unparseable inputs have no ordering here; equality and scope
        // gates fail closed before reaching comparison.
        _ => std::cmp::Ordering::Equal,
    }
}

/// NuGet equality (normalized comparison): `1.0` equals `1.0.0` equals
/// `1.0.0.0`, leading zeros strip, build metadata ignores, and
/// prerelease compares case-insensitively. Empty or overlong inputs
/// never equal.
pub fn nuget_version_eq(left: &str, right: &str) -> bool {
    let left_trimmed = left.trim();
    let right_trimmed = right.trim();
    if left_trimmed.is_empty() || right_trimmed.is_empty() {
        return false;
    }
    if left_trimmed.len() > 256 || right_trimmed.len() > 256 {
        return false;
    }
    if parse_nuget_version(left_trimmed).is_none() || parse_nuget_version(right_trimmed).is_none() {
        return false;
    }
    nuget_compare(left_trimmed, right_trimmed) == std::cmp::Ordering::Equal
}

/// NuGet-native affected-scope matching (issue #624): bare versions use
/// NuGet equality (so `1.0` matches `1.0.0` but not `1.5.0`; the
/// dependency-requirement `>=` reading of bare versions does not apply
/// to advisory scopes, where `[1.0,)` spells the minimum), bracketed
/// intervals use NuGet ordering with inclusive `[`/`]` versus exclusive
/// `(`/`)` bounds (`[1.0,2.0)`, `(,1.0]`, `[1.5,)`, `[1.0]` exact).
/// Floating `*`, unions, and `(1.0)` single-exclusive stay invalid and
/// fail closed to `false` (never a false positive). Malformed scopes,
/// empty inputs, and overlong inputs fail closed the same way.
pub fn nuget_in_scope(scope: &str, version: &str) -> bool {
    let scope_trimmed = scope.trim();
    let version_trimmed = version.trim();
    if scope_trimmed.is_empty() || version_trimmed.is_empty() {
        return false;
    }
    if scope_trimmed.len() > 4096 || version_trimmed.len() > 256 {
        return false;
    }
    if scope_trimmed.contains('*') {
        return false;
    }
    if parse_nuget_version(version_trimmed).is_none() {
        return false;
    }
    let has_brackets = scope_trimmed.contains('[')
        || scope_trimmed.contains('(')
        || scope_trimmed.contains(']')
        || scope_trimmed.contains(')');
    if !has_brackets {
        return nuget_version_eq(scope_trimmed, version_trimmed);
    }
    // Single bracketed interval covering the whole scope; NuGet has no
    // union syntax, so inner brackets fail closed.
    let chars: Vec<char> = scope_trimmed.chars().collect();
    if chars.len() < 3 {
        return false;
    }
    if (chars[0] != '[' && chars[0] != '(')
        || (chars[chars.len() - 1] != ']' && chars[chars.len() - 1] != ')')
    {
        return false;
    }
    let inner: String = chars[1..chars.len() - 1].iter().collect();
    if inner.contains('[') || inner.contains('(') || inner.contains(']') || inner.contains(')') {
        return false;
    }
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() == 1 {
        let bound = parts[0].trim();
        if bound.is_empty() {
            return false;
        }
        // Single-version intervals are exact only as `[1.0]`; `(1.0)`
        // is invalid per NuGet docs, as are mixed `[1.0)`/`(1.0]`.
        if chars[0] != '[' || chars[chars.len() - 1] != ']' {
            return false;
        }
        if bound.len() > 256 || parse_nuget_version(bound).is_none() {
            return false;
        }
        return nuget_version_eq(bound, version_trimmed);
    }
    if parts.len() != 2 {
        return false;
    }
    let lower = parts[0].trim().to_owned();
    let upper = parts[1].trim().to_owned();
    if lower.is_empty() && upper.is_empty() {
        return false;
    }
    if lower.len() > 256 || upper.len() > 256 {
        return false;
    }
    if !lower.is_empty() && parse_nuget_version(&lower).is_none() {
        return false;
    }
    if !upper.is_empty() && parse_nuget_version(&upper).is_none() {
        return false;
    }
    nuget_interval_matches(
        &lower,
        &upper,
        chars[0] == '[',
        chars[chars.len() - 1] == ']',
        version_trimmed,
    )
}

/// One NuGet interval against a locked version: empty bounds are
/// unbounded, otherwise NuGet ordering with inclusive/exclusive edges.
fn nuget_interval_matches(
    lower: &str,
    upper: &str,
    lower_inclusive: bool,
    upper_inclusive: bool,
    version: &str,
) -> bool {
    if !lower.is_empty() {
        match nuget_compare(version, lower) {
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
        match nuget_compare(version, upper) {
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
