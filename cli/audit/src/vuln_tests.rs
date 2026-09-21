//! Split from `vuln.rs`. No behavior change.
//! Originally the inline `mod tests`.
#![allow(unused_imports)]

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
fn version_matching_uses_semver_for_cargo_npm_go_and_ranges_for_maven_nuget() {
    assert!(version_affected("cargo", ">=1.0.0, <2.0.0", "1.5.0"));
    assert!(!version_affected("cargo", ">=1.0.0, <2.0.0", "2.0.0"));
    assert!(version_affected("npm", "^18.0.0", "18.2.0"));
    assert!(version_affected("go", ">=1.0.0, <2.0.0", "1.5.0"));
    // Go `v` prefixes normalize on both sides (see
    // `go_scopes_normalize_v_prefix` for the full matrix).
    // Maven uses Maven-native intervals; bare versions use Maven
    // equality (so `1.0` matches `1.0.0`).
    assert!(version_affected("maven", "1.2.0", "1.2.0"));
    assert!(version_affected("maven", "1.0", "1.0.0"));
    assert!(version_affected("maven", "[1.0,2.0)", "1.5.0"));
    assert!(!version_affected("maven", "[1.0,2.0)", "2.0.0"));
    assert!(version_affected("maven", "(,1.0]", "1.0.0"));
    assert!(!version_affected("maven", "(,1.0]", "1.0.1"));
    assert!(version_affected("maven", "[1.0]", "1.0.0"));
    assert!(!version_affected("maven", ">=1.0.0", "1.2.0"));
    // NuGet uses NuGet-native intervals; bare versions use NuGet
    // equality (so `1.0` matches `1.0.0` but not `1.5.0`;
    // `[1.0,)` spells the minimum).
    assert!(version_affected("nuget", "1.2.3", "1.2.3"));
    assert!(version_affected("nuget", "1.0", "1.0.0"));
    assert!(!version_affected("nuget", "1.0", "1.5.0"));
    assert!(version_affected("nuget", "[1.0,2.0)", "1.5.0"));
    assert!(!version_affected("nuget", "[1.0,2.0)", "2.0.0"));
    assert!(version_affected("nuget", "(,1.0]", "1.0.0"));
    assert!(!version_affected("nuget", "(,1.0]", "1.0.1"));
    assert!(version_affected("nuget", "[1.0]", "1.0.0"));
    assert!(!version_affected("nuget", ">=1.0.0", "1.2.0"));
    assert!(!version_affected("nuget", "", "1.0.0"));
}

#[test]
fn go_scopes_normalize_v_prefix() {
    // `v` on the locked version, the advisory scope, or both.
    assert!(go_in_scope(">=1.0.0, <2.0.0", "v1.5.0"));
    assert!(go_in_scope(">=v1.0.0, <v2.0.0", "1.5.0"));
    assert!(go_in_scope(">=v1.0.0, <v2.0.0", "v1.5.0"));
    assert!(!go_in_scope(">=v1.0.0, <v2.0.0", "v2.0.0"));
    assert!(version_affected("go", ">=v1.0.0, <v2.0.0", "v1.5.0"));
    assert!(!version_affected("go", ">=v1.0.0, <v2.0.0", "v2.0.0"));
    // Caret and comparator shapes normalize too.
    assert!(go_in_scope("^v1.2.0", "v1.9.0"));
    assert!(!go_in_scope("^v1.2.0", "v2.0.0"));
    assert!(go_in_scope("=v1.2.0", "v1.2.0"));
    assert!(!go_in_scope("=v1.2.0", "v1.2.1"));
    // Pseudo-versions and `+incompatible` suffixes stay assessable:
    // the `v` strips and the remainder parses as semver.
    assert!(go_in_scope(
        ">=v0.0.0-20250930140053-2eb4fccefb52, <v99.0.0",
        "v0.0.0-20250930140053-2eb4fccefb52"
    ));
    assert!(go_in_scope(">=v1.0.0, <v3.0.0", "v2.0.0+incompatible"));
    // Words containing `v` never mangle; garbage fails closed.
    assert_eq!(strip_go_v("very"), "very");
    assert_eq!(strip_go_v(">=v1.0.0, <v2.0.0"), ">=1.0.0, <2.0.0");
    assert!(!go_in_scope("not a range", "v1.2.0"));
    assert!(!go_in_scope(">=v1.0.0", "banana"));
    assert!(!go_in_scope("", "v1.0.0"));
    assert!(!go_in_scope(">=v1.0.0", ""));
}

#[test]
fn npm_ranges_cover_star_or_hyphen_and_prerelease() {
    // Star, `||` unions, hyphen ranges, and prereleases fire through
    // `version_affected` instead of looking clean.
    assert!(version_affected("npm", "*", "18.2.0"));
    assert!(version_affected("npm", "1.2.7 || >=1.2.9 <2.0.0", "1.2.7"));
    assert!(version_affected("npm", "1.2.7 || >=1.2.9 <2.0.0", "1.2.9"));
    assert!(!version_affected("npm", "1.2.7 || >=1.2.9 <2.0.0", "1.2.8"));
    assert!(version_affected("npm", "1.2.3 - 2.3.4", "2.0.0"));
    assert!(!version_affected("npm", "1.2.3 - 2.3.4", "2.3.5"));
    assert!(version_affected("npm", ">=1.2.7 <1.3.0", "1.2.9"));
    assert!(!version_affected("npm", ">=1.2.7 <1.3.0", "1.3.0"));
    assert!(version_affected("npm", "1.2.3", "1.2.3"));
    assert!(!version_affected("npm", "1.2.3", "1.2.4"));
    assert!(!version_affected("npm", ">=1.0.0", "2.0.0-alpha"));
    assert!(version_affected(
        "npm",
        ">=1.0.0-alpha, <2.0.0",
        "1.0.0-alpha"
    ));
    // Cargo keeps Cargo-flavor semantics: bare versions are caret
    // shorthand, `||` and hyphen stay invalid and fail closed.
    assert!(version_affected("cargo", "1.2.0", "1.2.1"));
    assert!(!version_affected("cargo", "1.0.0 || 2.0.0", "1.0.0"));
    assert!(!version_affected("cargo", "1.2.3 - 2.3.4", "1.5.0"));
    // Malformed npm scopes fail closed, never a false positive.
    assert!(!version_affected("npm", "", "1.2.3"));
    assert!(!version_affected("npm", "not a range", "1.2.3"));
    assert!(!version_affected("npm", ">=1.2.7 <1.3.0", "banana"));
}

#[test]
fn npm_range_advisories_report_findings() {
    // Range advisories fire instead of looking clean.
    let packages = vec![LockedPackage {
        name: "react".to_owned(),
        version: "18.2.0".to_owned(),
        set: "npm".to_owned(),
        is_git: false,
        is_private: false,
    }];
    let advisories = vec![
        Advisory {
            id: "GHSA-npm-range".to_owned(),
            package: "react".to_owned(),
            versions: ">=18.0.0 <18.3.0".to_owned(),
            severity: "high".to_owned(),
            fixed: vec!["18.3.0".to_owned()],
            set: "npm".to_owned(),
        },
        Advisory {
            id: "GHSA-npm-miss".to_owned(),
            package: "react".to_owned(),
            versions: ">=18.3.0 <19.0.0".to_owned(),
            severity: "high".to_owned(),
            fixed: vec![],
            set: "npm".to_owned(),
        },
    ];
    let (findings, unassessed) = match_packages(&packages, &advisories);
    assert!(unassessed.is_empty());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].advisory, "GHSA-npm-range");
    assert_eq!(findings[0].version, "18.2.0");
}

#[test]
fn npm_exceptions_narrow_with_npm_semantics() {
    let packages = vec![LockedPackage {
        name: "react".to_owned(),
        version: "18.2.0".to_owned(),
        set: "npm".to_owned(),
        is_git: false,
        is_private: false,
    }];
    let advisories = vec![Advisory {
        id: "GHSA-npm-exception".to_owned(),
        package: "react".to_owned(),
        versions: ">=18.0.0 <18.3.0".to_owned(),
        severity: "medium".to_owned(),
        fixed: vec![],
        set: "npm".to_owned(),
    }];
    let (findings, _) = match_packages(&packages, &advisories);
    assert_eq!(findings.len(), 1);
    let covering = RiskException {
        advisory: "GHSA-npm-exception".to_owned(),
        package: "react".to_owned(),
        set: "npm".to_owned(),
        versions: ">=18.0.0 <18.3.0".to_owned(),
        reason: "Accepted for this release.".to_owned(),
        expires: "2027-03-01".to_owned(),
    };
    let (unexempted, problems) = apply_exceptions(&findings, &[covering], "2026-09-18");
    assert!(problems.is_empty());
    assert!(unexempted.is_empty());
    let missing = RiskException {
        advisory: "GHSA-npm-exception".to_owned(),
        package: "react".to_owned(),
        set: "npm".to_owned(),
        versions: ">=18.3.0 <19.0.0".to_owned(),
        reason: "Wrong range.".to_owned(),
        expires: "2027-03-01".to_owned(),
    };
    let (unexempted, problems) = apply_exceptions(&findings, &[missing], "2026-09-18");
    assert!(problems.is_empty());
    assert_eq!(unexempted.len(), 1);
}

#[test]
fn maven_version_ordering_follows_upstream_subset() {
    // Release equality plus trailing-null trimming.
    assert_eq!(maven_compare("1.0", "1.0.0"), std::cmp::Ordering::Equal);
    assert!(maven_version_eq("1.0", "1.0.0"));
    assert!(maven_version_eq("1.ga", "1"));
    assert!(maven_version_eq("1-final", "1"));
    assert!(maven_version_eq("1.0.0-foo.0.0", "1-foo"));
    assert!(maven_version_eq("1-a1", "1-alpha-1"));
    // Qualifier ladder.
    assert!(maven_compare("1-alpha", "1-beta") == std::cmp::Ordering::Less);
    assert!(maven_compare("1-beta", "1-milestone") == std::cmp::Ordering::Less);
    assert!(maven_compare("1-milestone", "1-rc") == std::cmp::Ordering::Less);
    assert!(maven_compare("1-rc", "1-snapshot") == std::cmp::Ordering::Less);
    assert!(maven_compare("1-snapshot", "1") == std::cmp::Ordering::Less);
    assert!(maven_compare("1", "1-sp") == std::cmp::Ordering::Less);
    // Unknown qualifiers sort after known ones, lexically.
    assert!(maven_compare("1", "1-foo") == std::cmp::Ordering::Less);
    assert!(maven_compare("5.aardvark", "5.zebra") == std::cmp::Ordering::Less);
    // Numbers beat qualifiers; hyphen-numbers sort before dot-numbers.
    assert!(maven_compare("1-K", "1.7") == std::cmp::Ordering::Less);
    assert!(maven_compare("1-foo2", "1-foo10") == std::cmp::Ordering::Less);
    assert_eq!(maven_compare("1.foo", "1-foo"), std::cmp::Ordering::Equal);
    assert!(maven_compare("1-1", "1.1") == std::cmp::Ordering::Less);
    assert!(maven_compare("1", "1.1") == std::cmp::Ordering::Less);
    assert!(maven_compare("2.0-rc1", "2.0") == std::cmp::Ordering::Less);
    assert!(maven_compare("1.10", "1.9") == std::cmp::Ordering::Greater);
}

#[test]
fn maven_ranges_cover_intervals_unions_and_edges() {
    // Bounded intervals, inclusive versus exclusive.
    assert!(maven_in_scope("[1.0,2.0]", "1.0"));
    assert!(maven_in_scope("[1.0,2.0]", "2.0"));
    assert!(!maven_in_scope("(1.0,2.0)", "1.0"));
    assert!(!maven_in_scope("(1.0,2.0)", "2.0"));
    assert!(maven_in_scope("[1.0,2.0)", "1.0"));
    assert!(!maven_in_scope("[1.0,2.0)", "2.0"));
    // Maven includes pre-releases under an exclusive upper bound.
    assert!(maven_in_scope("[1.0,2.0)", "2.0-rc1"));
    // Unbounded sides.
    assert!(maven_in_scope("[1.5,)", "1.5"));
    assert!(maven_in_scope("[1.5,)", "9.9"));
    assert!(!maven_in_scope("[1.5,)", "1.4"));
    assert!(maven_in_scope("(,1.0]", "1.0"));
    assert!(!maven_in_scope("(,1.0]", "1.0.1"));
    assert!(maven_in_scope("(,1.0)", "0.9"));
    assert!(!maven_in_scope("(,1.0)", "1.0"));
    // Single-version intervals and unions.
    assert!(maven_in_scope("[1.0]", "1.0.0"));
    assert!(!maven_in_scope("[1.0]", "1.0.1"));
    assert!(!maven_in_scope("(1.0)", "1.0"));
    assert!(maven_in_scope("(,1.0],[1.2,)", "1.0"));
    assert!(maven_in_scope("(,1.0],[1.2,)", "1.2"));
    assert!(!maven_in_scope("(,1.0],[1.2,)", "1.1"));
    assert!(maven_in_scope("(,1.1),(1.1,)", "1.0"));
    assert!(maven_in_scope("(,1.1),(1.1,)", "1.2"));
    assert!(!maven_in_scope("(,1.1),(1.1,)", "1.1"));
    // Bare versions are Maven equality, not semver ranges.
    assert!(maven_in_scope("1.2.0", "1.2.0"));
    assert!(maven_in_scope("1.0", "1.0.0"));
    assert!(!maven_in_scope("1.2.0", "1.2.1"));
    assert!(!maven_in_scope(">=1.0.0", "1.2.0"));
    // Malformed and empty inputs fail closed, never a false positive.
    assert!(!maven_in_scope("", "1.0"));
    assert!(!maven_in_scope("[1.0,2.0)", ""));
    assert!(!maven_in_scope("[1.0,2.0", "1.5"));
    assert!(!maven_in_scope("[1.0,2.0,3.0]", "1.5"));
    assert!(!maven_in_scope("(,)", "1.0"));
    assert!(!maven_in_scope("[]", "1.0"));
}

#[test]
fn maven_range_advisories_report_findings() {
    // Range advisories fire instead of looking clean.
    let packages = vec![LockedPackage {
        name: "com.google.guava:guava".to_owned(),
        version: "32.0.0".to_owned(),
        set: "maven".to_owned(),
        is_git: false,
        is_private: false,
    }];
    let advisories = vec![
        Advisory {
            id: "GHSA-maven-range".to_owned(),
            package: "com.google.guava:guava".to_owned(),
            versions: "[30.0,33.0)".to_owned(),
            severity: "high".to_owned(),
            fixed: vec!["33.0".to_owned()],
            set: "maven".to_owned(),
        },
        Advisory {
            id: "GHSA-maven-miss".to_owned(),
            package: "com.google.guava:guava".to_owned(),
            versions: "[33.0,34.0)".to_owned(),
            severity: "high".to_owned(),
            fixed: vec![],
            set: "maven".to_owned(),
        },
    ];
    let (findings, unassessed) = match_packages(&packages, &advisories);
    assert!(unassessed.is_empty());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].advisory, "GHSA-maven-range");
    assert_eq!(findings[0].version, "32.0.0");
}

#[test]
fn maven_exceptions_narrow_with_maven_semantics() {
    let packages = vec![LockedPackage {
        name: "junit:junit".to_owned(),
        version: "4.13.2".to_owned(),
        set: "maven".to_owned(),
        is_git: false,
        is_private: false,
    }];
    let advisories = vec![Advisory {
        id: "GHSA-maven-exception".to_owned(),
        package: "junit:junit".to_owned(),
        versions: "[4.0,5.0)".to_owned(),
        severity: "medium".to_owned(),
        fixed: vec![],
        set: "maven".to_owned(),
    }];
    let (findings, _) = match_packages(&packages, &advisories);
    assert_eq!(findings.len(), 1);
    let covering = RiskException {
        advisory: "GHSA-maven-exception".to_owned(),
        package: "junit:junit".to_owned(),
        set: "maven".to_owned(),
        versions: "[4.0,5.0)".to_owned(),
        reason: "Accepted for this release.".to_owned(),
        expires: "2027-03-01".to_owned(),
    };
    let (unexempted, problems) = apply_exceptions(&findings, &[covering], "2026-09-18");
    assert!(problems.is_empty());
    assert!(unexempted.is_empty());
    let missing = RiskException {
        advisory: "GHSA-maven-exception".to_owned(),
        package: "junit:junit".to_owned(),
        set: "maven".to_owned(),
        versions: "[5.0,6.0)".to_owned(),
        reason: "Wrong range.".to_owned(),
        expires: "2027-03-01".to_owned(),
    };
    let (unexempted, problems) = apply_exceptions(&findings, &[missing], "2026-09-18");
    assert!(problems.is_empty());
    assert_eq!(unexempted.len(), 1);
}

#[test]
fn nuget_version_ordering_follows_upstream_subset() {
    // Four-part equality plus missing-part and leading-zero
    // normalization.
    assert_eq!(nuget_compare("1.0", "1.0.0"), std::cmp::Ordering::Equal);
    assert_eq!(nuget_compare("1.0", "1.0.0.0"), std::cmp::Ordering::Equal);
    assert!(nuget_version_eq("1.0", "1.0.0"));
    assert!(nuget_version_eq("1.00", "1.0.0.0"));
    assert!(nuget_version_eq("1.01.1", "1.1.1"));
    assert!(nuget_version_eq("1.0.0.0", "1.0.0"));
    assert!(nuget_version_eq("10.1.201", "10.1.201.0"));
    // Build metadata never affects ordering.
    assert!(nuget_version_eq("1.0.7+r3456", "1.0.7"));
    assert_eq!(
        nuget_compare("1.0.0+build", "1.0.0"),
        std::cmp::Ordering::Equal
    );
    // Numeric ordering without overflow.
    assert!(nuget_compare("1.10", "1.9") == std::cmp::Ordering::Greater);
    assert!(nuget_compare("2.0.0.1", "2.0.0") == std::cmp::Ordering::Greater);
    assert!(nuget_compare("1.0.0", "1.0.0.1") == std::cmp::Ordering::Less);
    // Release beats prerelease on the same core.
    assert!(nuget_compare("1.0.0", "1.0.0-alpha") == std::cmp::Ordering::Greater);
    assert!(nuget_compare("1.0.0-alpha", "1.0.0") == std::cmp::Ordering::Less);
    // Prerelease labels: numeric numerically, numeric before alpha,
    // alpha case-insensitively, shorter prefix first.
    assert!(nuget_compare("1.0.0-alpha.9", "1.0.0-alpha.10") == std::cmp::Ordering::Less);
    assert!(nuget_compare("1.0.0-1", "1.0.0-alpha") == std::cmp::Ordering::Less);
    assert!(nuget_version_eq("1.0.0-alpha", "1.0.0-Alpha"));
    assert!(nuget_version_eq("1.0.0-ALPHA", "1.0.0-alpha"));
    assert!(nuget_compare("1.0.0-alpha", "1.0.0-alpha.1") == std::cmp::Ordering::Less);
    assert!(nuget_compare("1.0.0-beta", "1.0.0-alpha") == std::cmp::Ordering::Greater);
    // Different cores dominate prerelease.
    assert!(nuget_compare("2.0.0-alpha", "1.9.9") == std::cmp::Ordering::Greater);
}

#[test]
fn nuget_ranges_cover_intervals_minimums_and_edges() {
    // Bounded intervals, inclusive versus exclusive.
    assert!(nuget_in_scope("[1.0,2.0]", "1.0"));
    assert!(nuget_in_scope("[1.0,2.0]", "2.0"));
    assert!(!nuget_in_scope("(1.0,2.0)", "1.0"));
    assert!(!nuget_in_scope("(1.0,2.0)", "2.0"));
    assert!(nuget_in_scope("[1.0,2.0)", "1.0"));
    assert!(!nuget_in_scope("[1.0,2.0)", "2.0"));
    assert!(nuget_in_scope("(1.0,2.0]", "2.0"));
    assert!(!nuget_in_scope("(1.0,2.0]", "1.0"));
    // Minimum/maximum spellings: `[1.0,)` is the `>=` minimum.
    assert!(nuget_in_scope("[1.0,)", "1.0"));
    assert!(nuget_in_scope("[1.0,)", "9.9"));
    assert!(!nuget_in_scope("[1.0,)", "0.9"));
    assert!(nuget_in_scope("(1.0,)", "1.0.1"));
    assert!(!nuget_in_scope("(1.0,)", "1.0"));
    assert!(nuget_in_scope("(,1.0]", "1.0"));
    assert!(!nuget_in_scope("(,1.0]", "1.0.1"));
    assert!(nuget_in_scope("(,1.0)", "0.9"));
    assert!(!nuget_in_scope("(,1.0)", "1.0"));
    // Single-version intervals are exact only as `[1.0]`; `(1.0)`
    // stays invalid per NuGet docs.
    assert!(nuget_in_scope("[1.0]", "1.0.0"));
    assert!(!nuget_in_scope("[1.0]", "1.0.1"));
    assert!(!nuget_in_scope("(1.0)", "1.0"));
    assert!(!nuget_in_scope("[1.0)", "1.0"));
    assert!(!nuget_in_scope("(1.0]", "1.0"));
    // Bare versions are NuGet equality, not the dependency `>=`
    // minimum: `[1.0,)` spells the minimum in advisory scopes.
    assert!(nuget_in_scope("1.2.0", "1.2.0"));
    assert!(nuget_in_scope("1.0", "1.0.0"));
    assert!(!nuget_in_scope("1.0", "1.0.1"));
    assert!(!nuget_in_scope("1.0", "0.9"));
    assert!(!nuget_in_scope(">=1.0.0", "1.2.0"));
    // Prereleases order normally: an exclusive upper bound still
    // covers a prerelease below the release.
    assert!(nuget_in_scope("[1.0,2.0)", "2.0.0-beta"));
    assert!(!nuget_in_scope("[1.0,2.0)", "2.0.0"));
    // Malformed, floating, union, and empty inputs fail closed,
    // never a false positive.
    assert!(!nuget_in_scope("", "1.0"));
    assert!(!nuget_in_scope("[1.0,2.0)", ""));
    assert!(!nuget_in_scope("[1.0,2.0", "1.5"));
    assert!(!nuget_in_scope("[1.0,2.0,3.0]", "1.5"));
    assert!(!nuget_in_scope("(,)", "1.0"));
    assert!(!nuget_in_scope("[]", "1.0"));
    assert!(!nuget_in_scope("1.*", "1.5.0"));
    assert!(!nuget_in_scope("[1.*,2.0)", "1.5.0"));
    assert!(!nuget_in_scope("(,1.0],[1.2,)", "1.0"));
    assert!(!nuget_in_scope("[1.0]", "not-a-version"));
    assert!(!nuget_in_scope("not-a-version", "1.0.0"));
}

#[test]
fn nuget_range_advisories_report_findings() {
    // Range advisories fire instead of looking clean (issue #624).
    let packages = vec![LockedPackage {
        name: "Newtonsoft.Json".to_owned(),
        version: "13.0.1".to_owned(),
        set: "nuget".to_owned(),
        is_git: false,
        is_private: false,
    }];
    let advisories = vec![
        Advisory {
            id: "GHSA-nuget-range".to_owned(),
            package: "Newtonsoft.Json".to_owned(),
            versions: "[12.0,13.0.2)".to_owned(),
            severity: "high".to_owned(),
            fixed: vec!["13.0.2".to_owned()],
            set: "nuget".to_owned(),
        },
        Advisory {
            id: "GHSA-nuget-miss".to_owned(),
            package: "Newtonsoft.Json".to_owned(),
            versions: "[13.0.2,14.0)".to_owned(),
            severity: "high".to_owned(),
            fixed: vec![],
            set: "nuget".to_owned(),
        },
    ];
    let (findings, unassessed) = match_packages(&packages, &advisories);
    assert!(unassessed.is_empty());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].advisory, "GHSA-nuget-range");
    assert_eq!(findings[0].version, "13.0.1");
}

#[test]
fn nuget_exceptions_narrow_with_nuget_semantics() {
    let packages = vec![LockedPackage {
        name: "Newtonsoft.Json".to_owned(),
        version: "13.0.1".to_owned(),
        set: "nuget".to_owned(),
        is_git: false,
        is_private: false,
    }];
    let advisories = vec![Advisory {
        id: "GHSA-nuget-exception".to_owned(),
        package: "Newtonsoft.Json".to_owned(),
        versions: "[12.0,13.0.2)".to_owned(),
        severity: "medium".to_owned(),
        fixed: vec![],
        set: "nuget".to_owned(),
    }];
    let (findings, _) = match_packages(&packages, &advisories);
    assert_eq!(findings.len(), 1);
    let covering = RiskException {
        advisory: "GHSA-nuget-exception".to_owned(),
        package: "Newtonsoft.Json".to_owned(),
        set: "nuget".to_owned(),
        versions: "[12.0,13.0.2)".to_owned(),
        reason: "Accepted for this release.".to_owned(),
        expires: "2027-03-01".to_owned(),
    };
    let (unexempted, problems) = apply_exceptions(&findings, &[covering], "2026-09-18");
    assert!(problems.is_empty());
    assert!(unexempted.is_empty());
    let missing = RiskException {
        advisory: "GHSA-nuget-exception".to_owned(),
        package: "Newtonsoft.Json".to_owned(),
        set: "nuget".to_owned(),
        versions: "[13.0.2,14.0)".to_owned(),
        reason: "Wrong range.".to_owned(),
        expires: "2027-03-01".to_owned(),
    };
    let (unexempted, problems) = apply_exceptions(&findings, &[missing], "2026-09-18");
    assert!(problems.is_empty());
    assert_eq!(unexempted.len(), 1);
}

#[test]
fn unassessed_reasons_are_pinned() {
    // Wont-fix pins: reason spellings are contract.
    assert_eq!(REASON_GIT, "unsupported git revision");
    assert_eq!(REASON_PRIVATE, "unidentified private package");
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
        // Incomplete mapping is set-agnostic; Maven/NuGet
        // plus Go unassessed deps fail the same way, never clean.
        LockedPackage {
            name: "git-nuget-dep".to_owned(),
            version: "1.0.0".to_owned(),
            set: "nuget".to_owned(),
            is_git: true,
            is_private: false,
        },
        LockedPackage {
            name: "private-maven".to_owned(),
            version: "2.0.0".to_owned(),
            set: "maven".to_owned(),
            is_git: false,
            is_private: true,
        },
    ];
    let (findings, unassessed) = match_packages(&pkgs, &advisories());
    assert!(findings.is_empty());
    assert_eq!(unassessed.len(), 4);
    assert!(unassessed.iter().any(|u| u.reason == REASON_GIT));
    assert!(unassessed.iter().any(|u| u.reason == REASON_PRIVATE));
    assert!(unassessed.iter().filter(|u| u.reason == REASON_GIT).count() == 2);
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

#[test]
fn osv_typed_cargo_projects_interval_and_matches() {
    // Typed OSV via `osv` crate (issue #676): ranges become semver scopes,
    // fixed preserves, severity from database_specific, no matcher change.
    let text = r#"[{
        "id": "GHSA-cargo-osv-0001",
        "modified": "2026-09-18T00:00:00Z",
        "affected": [{
            "package": {"name": "serde", "ecosystem": "crates.io"},
            "ranges": [{"type": "SEMVER", "events": [{"introduced": "1.0.0"}, {"fixed": "1.5.0"}]}],
            "database_specific": {"severity": "high"}
        }]
    }]"#;
    let parsed = parse_snapshot(text).expect("osv parses");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].id, "GHSA-cargo-osv-0001");
    assert_eq!(parsed[0].package, "serde");
    assert_eq!(parsed[0].set, "cargo");
    assert_eq!(parsed[0].versions, ">=1.0.0, <1.5.0");
    assert_eq!(parsed[0].severity, "high");
    assert_eq!(parsed[0].fixed, vec!["1.5.0".to_owned()]);
    assert!(version_affected("cargo", &parsed[0].versions, "1.2.0"));
    assert!(!version_affected("cargo", &parsed[0].versions, "1.5.0"));
}

#[test]
fn osv_typed_sets_project_to_native_scopes() {
    // npm semver, go v-prefix, maven/nuget intervals (no narrowing change).
    let text = r#"[{
        "id": "GHSA-npm-osv-0001",
        "modified": "2026-09-18T00:00:00Z",
        "affected": [{
            "package": {"name": "react", "ecosystem": "npm"},
            "ranges": [{"type": "SEMVER", "events": [{"introduced": "18.0.0"}, {"fixed": "18.3.0"}]}]
        }]
    },
    {
        "id": "GHSA-go-osv-0001",
        "modified": "2026-09-18T00:00:00Z",
        "affected": [{
            "package": {"name": "github.com/google/go-cmp", "ecosystem": "Go"},
            "ranges": [{"type": "SEMVER", "events": [{"introduced": "v0.5.0"}, {"fixed": "v0.7.0"}]}]
        }]
    },
    {
        "id": "GHSA-maven-osv-0001",
        "modified": "2026-09-18T00:00:00Z",
        "affected": [{
            "package": {"name": "com.google.guava:guava", "ecosystem": "Maven"},
            "ranges": [{"type": "ECOSYSTEM", "events": [{"introduced": "30.0"}, {"fixed": "33.0"}]}]
        }]
    },
    {
        "id": "GHSA-nuget-osv-0001",
        "modified": "2026-09-18T00:00:00Z",
        "affected": [{
            "package": {"name": "Newtonsoft.Json", "ecosystem": "NuGet"},
            "ranges": [{"type": "ECOSYSTEM", "events": [{"introduced": "12.0"}, {"fixed": "13.0.2"}]}]
        }]
    }]"#;
    let parsed = parse_snapshot(text).expect("osv parses");
    assert_eq!(parsed.len(), 4);
    let npm = parsed
        .iter()
        .find(|entry| entry.id == "GHSA-npm-osv-0001")
        .expect("npm");
    assert_eq!(npm.set, "npm");
    assert_eq!(npm.versions, ">=18.0.0, <18.3.0");
    assert!(version_affected("npm", &npm.versions, "18.2.0"));
    assert!(!version_affected("npm", &npm.versions, "18.3.0"));
    let go = parsed
        .iter()
        .find(|entry| entry.id == "GHSA-go-osv-0001")
        .expect("go");
    assert_eq!(go.set, "go");
    assert!(version_affected("go", &go.versions, "v0.6.0"));
    assert!(!version_affected("go", &go.versions, "v0.7.0"));
    let maven = parsed
        .iter()
        .find(|entry| entry.id == "GHSA-maven-osv-0001")
        .expect("maven");
    assert_eq!(maven.set, "maven");
    assert_eq!(maven.versions, "[30.0,33.0)");
    assert!(version_affected("maven", &maven.versions, "32.0.0"));
    assert!(!version_affected("maven", &maven.versions, "33.0"));
    let nuget = parsed
        .iter()
        .find(|entry| entry.id == "GHSA-nuget-osv-0001")
        .expect("nuget");
    assert_eq!(nuget.set, "nuget");
    assert_eq!(nuget.versions, "[12.0,13.0.2)");
    assert!(version_affected("nuget", &nuget.versions, "13.0.1"));
    assert!(!version_affected("nuget", &nuget.versions, "13.0.2"));
}

#[test]
fn osv_typed_withdrawn_unsupported_and_git_skip() {
    // Withdrawn, PyPI ecosystem, and GIT ranges yield zero advisories;
    // explicit versions list yields one scope per version.
    let text = r#"[{
        "id": "GHSA-withdrawn-0001",
        "modified": "2026-09-18T00:00:00Z",
        "withdrawn": "2026-09-19T00:00:00Z",
        "affected": [{
            "package": {"name": "serde", "ecosystem": "crates.io"},
            "ranges": [{"type": "SEMVER", "events": [{"introduced": "1.0.0"}, {"fixed": "2.0.0"}]}]
        }]
    },
    {
        "id": "GHSA-pypi-0001",
        "modified": "2026-09-18T00:00:00Z",
        "affected": [{
            "package": {"name": "django", "ecosystem": "PyPI"},
            "ranges": [{"type": "ECOSYSTEM", "events": [{"introduced": "1.0"}, {"fixed": "2.0"}]}]
        }]
    },
    {
        "id": "GHSA-git-0001",
        "modified": "2026-09-18T00:00:00Z",
        "affected": [{
            "package": {"name": "serde", "ecosystem": "crates.io"},
            "ranges": [{"type": "GIT", "events": [{"introduced": "abc123"}, {"fixed": "def456"}]}]
        }]
    },
    {
        "id": "GHSA-explicit-0001",
        "modified": "2026-09-18T00:00:00Z",
        "affected": [{
            "package": {"name": "serde", "ecosystem": "crates.io"},
            "versions": ["1.2.3", "1.2.4"]
        }]
    }]"#;
    let parsed = parse_snapshot(text).expect("osv parses");
    // Only the explicit-versions entry projects (two scopes).
    assert_eq!(parsed.len(), 2);
    assert!(parsed.iter().all(|entry| entry.id == "GHSA-explicit-0001"));
    assert!(parsed.iter().any(|entry| entry.versions == "1.2.3"));
    assert!(parsed.iter().any(|entry| entry.versions == "1.2.4"));
}

#[test]
fn osv_typed_severity_unknown_fails_closed_and_last_affected_inclusive() {
    // CVSS vectors stay unknown (fail by default); last_affected is
    // inclusive; unbounded lower/upper map to native all-match scopes.
    let text = r#"[{
        "id": "GHSA-cvss-0001",
        "modified": "2026-09-18T00:00:00Z",
        "severity": [{"type": "CVSS_V3", "score": "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"}],
        "affected": [{
            "package": {"name": "serde", "ecosystem": "crates.io"},
            "ranges": [{"type": "SEMVER", "events": [{"introduced": "1.0.0"}, {"last_affected": "1.4.0"}]}]
        }]
    },
    {
        "id": "GHSA-unbounded-0001",
        "modified": "2026-09-18T00:00:00Z",
        "affected": [{
            "package": {"name": "serde", "ecosystem": "crates.io"},
            "ranges": [{"type": "SEMVER", "events": [{"introduced": "0"}, {"fixed": "2.0.0"}]}]
        }]
    }]"#;
    let parsed = parse_snapshot(text).expect("osv parses");
    assert_eq!(parsed.len(), 2);
    let cvss = parsed
        .iter()
        .find(|entry| entry.id == "GHSA-cvss-0001")
        .expect("cvss");
    assert_eq!(cvss.severity, "");
    assert_eq!(cvss.versions, ">=1.0.0, <=1.4.0");
    assert_eq!(canonical_severity(&cvss.severity), "unknown");
    assert_eq!(
        normalize_level(&canonical_severity(&cvss.severity)),
        "error"
    );
    assert!(version_affected("cargo", &cvss.versions, "1.4.0"));
    assert!(!version_affected("cargo", &cvss.versions, "1.4.1"));
    let unbounded = parsed
        .iter()
        .find(|entry| entry.id == "GHSA-unbounded-0001")
        .expect("unbounded");
    assert_eq!(unbounded.versions, "<2.0.0");
    assert!(version_affected("cargo", &unbounded.versions, "1.9.0"));
}

#[test]
fn osv_typed_malformed_fails_closed() {
    // Missing required OSV `modified` fails (fail-closed, never empty clean).
    assert!(parse_snapshot(r#"[{"id":"GHSA-bad","affected":[]}]"#).is_err());
    assert!(parse_snapshot("not json").is_err());
    assert!(parse_snapshot("").is_err());
    // Empty array stays empty (no advisories, clean only with assessment).
    assert_eq!(parse_snapshot("[]").expect("empty"), Vec::new());
}
