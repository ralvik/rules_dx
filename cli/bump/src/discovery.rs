//! Outdated discovery planning for the `dx bump` loop.
//!
//! Contract: `docs/cli/commands/audit-update-bazel.md#dx-bump`.
//!
//! Library-first (ADR 0008): registry enumeration uses upstream registry
//! clients (BCR / crates.io / npm registry / Go proxy / Maven Central /
//! NuGet / GitHub releases), never custom HTTP. Version parsing and
//! comparison delegate to upstream `semver`, never custom version code.
//! This module plans over injected version snapshots only, so discovery
//! stays deterministic and unit-testable without a workspace, a Bazel
//! server, registries, or any upstream updater. Fetching stays in the
//! upstream clients plus the scheduled `bump.yml` runner; custom code here
//! is limited to stable-only filtering, semver ordering, and the next-candidate
//! selection for the one-dep-per-PR loop.
//!
//! Policy (issue #639):
//! - Discovery proposes stable versions only; prerelease eligibility
//!   follows the upstream resolver and project configuration, never a
//!   private `dx` policy (`version::prerelease_follows_upstream`).
//! - Candidates order via upstream `semver` comparison
//!   (`version::compare`), never custom ordering; the loop takes the
//!   first candidate in selector order (never batch).
//! - Transitive versions stay resolver-governed; discovery never forces
//!   every transitive to newest.
//! - Manual selector only is rejected: scheduled discovery enumerates
//!   outdated via the upstream clients, never requires an operator
//!   `selector`/`version` pair to make progress.
//! - GitHub Actions tags need SHA resolution through the upstream GitHub
//!   releases client before the file edit (issue #640 owns the auto
//!   resolution); discovery lists the tag candidate but never invents a SHA.

use super::sets::BumpSet;
use super::version::{compare, is_stable};

/// Declared requirement plus its registry snapshot: the pinned current
/// version and the available versions reported by the upstream registry
/// client for this selector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    /// Full widen selector (`set:package`, e.g. `cargo:anyhow`).
    pub selector: String,
    /// Owning set (selects the upstream registry client).
    pub set: BumpSet,
    /// Pinned current version (semver sets only; GHA resolves via SHA).
    pub current: semver::Version,
    /// Available versions reported by the upstream client (injected).
    pub available: Vec<semver::Version>,
}

/// One outdated candidate: the declared selector is behind its latest
/// stable available version.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutdatedCandidate {
    /// Full widen selector (`set:package`).
    pub selector: String,
    /// Pinned current version.
    pub current: semver::Version,
    /// Latest stable available version above current.
    pub latest: semver::Version,
}

/// Discovery planning errors (never a partial enumeration).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DiscoveryError {
    /// Empty selector or version text.
    #[error("empty discovery entry; expected `set:package` plus semver")]
    Empty,
    /// Unknown set or malformed `set:package` shape.
    #[error("unknown discovery selector {selector:?}; expected bazel|cargo|go|maven|npm|nuget as `set:package`")]
    UnknownSelector {
        /// Offending spelling.
        selector: String,
    },
    /// Invalid semver for a semver-owned set.
    #[error("invalid version {version:?} for selector {selector:?}: expected exact semver")]
    InvalidVersion {
        /// Offending selector.
        selector: String,
        /// Offending spelling.
        version: String,
    },
}

/// Upstream registry client owning enumeration for one set (never custom
/// HTTP). GitHub Actions enumerates tags via the upstream GitHub releases
/// client; SHA resolution stays owned under issue #640.
pub fn registry_client(set: BumpSet) -> &'static str {
    match set {
        BumpSet::Bazel => "BCR",
        BumpSet::Cargo => "crates.io",
        BumpSet::Npm => "npm registry",
        BumpSet::Go => "Go proxy",
        BumpSet::Maven => "Maven Central",
        BumpSet::NuGet => "NuGet",
        BumpSet::GithubActions => "GitHub releases",
    }
}

/// Parses one semver discovery entry (`selector` plus pinned `current`
/// text). GitHub Actions has no semver current (SHA-plus-tag pins), so it
/// fails closed here; its tag enumeration stays owned under issue #640.
pub fn parse_declared(selector: &str, current: &str) -> Result<Snapshot, DiscoveryError> {
    if selector.is_empty() || current.is_empty() {
        return Err(DiscoveryError::Empty);
    }
    let (head, tail) = match selector.split_once(':') {
        Some((head, tail)) => (head, tail),
        None => {
            return Err(DiscoveryError::UnknownSelector {
                selector: selector.to_owned(),
            });
        }
    };
    let set = match BumpSet::parse(head) {
        Some(set) => set,
        None => {
            return Err(DiscoveryError::UnknownSelector {
                selector: selector.to_owned(),
            });
        }
    };
    if tail.is_empty() {
        return Err(DiscoveryError::UnknownSelector {
            selector: selector.to_owned(),
        });
    }
    if set == BumpSet::GithubActions {
        return Err(DiscoveryError::UnknownSelector {
            selector: selector.to_owned(),
        });
    }
    // Maven carries `group:artifact` after the set prefix; every other
    // semver set rejects a second colon (mirrors `request.rs`).
    if set != BumpSet::Maven && tail.contains(':') {
        return Err(DiscoveryError::UnknownSelector {
            selector: selector.to_owned(),
        });
    }
    let parsed: semver::Version = current
        .trim()
        .strip_prefix('v')
        .unwrap_or(current.trim())
        .parse()
        .map_err(|_| DiscoveryError::InvalidVersion {
            selector: selector.to_owned(),
            version: current.to_owned(),
        })?;
    Ok(Snapshot {
        selector: selector.to_owned(),
        set,
        current: parsed,
        available: Vec::new(),
    })
}

/// Latest stable available version above `current`, ordered via upstream
/// `semver` comparison. Prereleases in `available` never win: discovery
/// proposes stable only while prerelease eligibility follows the upstream
/// resolver and project configuration. Returns `None` when current is
/// already latest (nothing outdated).
pub fn latest_stable(
    current: &semver::Version,
    available: &[semver::Version],
) -> Option<semver::Version> {
    let mut best: Option<&semver::Version> = None;
    for candidate in available {
        if !is_stable(candidate) {
            continue;
        }
        if compare(current, candidate) != std::cmp::Ordering::Less {
            continue;
        }
        match best {
            None => best = Some(candidate),
            Some(winner) => {
                if compare(winner, candidate) == std::cmp::Ordering::Less {
                    best = Some(candidate);
                }
            }
        }
    }
    best.cloned()
}

/// Plans the outdated candidate for one snapshot, if any. `None` means
/// up-to-date (no widen proposed for this selector).
pub fn outdated_from_snapshot(snapshot: &Snapshot) -> Option<OutdatedCandidate> {
    latest_stable(&snapshot.current, &snapshot.available).map(|latest| OutdatedCandidate {
        selector: snapshot.selector.clone(),
        current: snapshot.current.clone(),
        latest,
    })
}

/// Collects outdated candidates across snapshots, ordered deterministically
/// by selector (loop takes the first; never batch). Ordering within a
/// selector uses upstream `semver` comparison via [`latest_stable`]; the
/// cross-selector order is plain selector text so the weekly run proposes
/// the same next dep on a fixed snapshot.
pub fn collect_outdated(snapshots: &[Snapshot]) -> Vec<OutdatedCandidate> {
    let mut out: Vec<OutdatedCandidate> = snapshots
        .iter()
        .filter_map(outdated_from_snapshot)
        .collect();
    out.sort_by(|left, right| {
        left.selector
            .cmp(&right.selector)
            .then_with(|| compare(&left.latest, &right.latest))
    });
    out
}

/// Next single widen for the loop (first in selector order), if any. One
/// dep per run, never batch; `None` means nothing outdated.
pub fn next_outdated(candidates: &[OutdatedCandidate]) -> Option<&OutdatedCandidate> {
    candidates.first()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(selector: &str, current: &str, available: &[&str]) -> Snapshot {
        let mut entry = parse_declared(selector, current).expect("declared");
        entry.available = available
            .iter()
            .map(|text| text.parse::<semver::Version>().expect("available"))
            .collect();
        entry
    }

    #[test]
    fn semver_sets_parse_declared_with_client_mapping() {
        // Issue #639: discovery covers the six semver sets; each maps to
        // its upstream registry client, never custom HTTP.
        for (selector, client) in [
            ("bazel:rules_rust", "BCR"),
            ("cargo:anyhow", "crates.io"),
            ("npm:jest", "npm registry"),
            ("go:example.com/mod", "Go proxy"),
            ("maven:junit:junit", "Maven Central"),
            ("nuget:FSharp.Core", "NuGet"),
        ] {
            let entry = parse_declared(selector, "1.2.3").expect("declared");
            assert_eq!(registry_client(entry.set), client, "{selector}");
        }
        assert_eq!(registry_client(BumpSet::GithubActions), "GitHub releases");
    }

    #[test]
    fn github_actions_has_no_semver_current() {
        // GHA pins are tag/SHA-shaped: discovery lists tags via GitHub
        // releases but never invents a SHA here (issue #640 owns auto).
        assert!(matches!(
            parse_declared("github-actions:actions/checkout", "v4"),
            Err(DiscoveryError::UnknownSelector { .. })
        ));
    }

    #[test]
    fn malformed_entries_fail_closed() {
        assert!(matches!(
            parse_declared("", "1.2.3"),
            Err(DiscoveryError::Empty)
        ));
        assert!(matches!(
            parse_declared("cargo:anyhow", ""),
            Err(DiscoveryError::Empty)
        ));
        assert!(matches!(
            parse_declared("cargo", "1.2.3"),
            Err(DiscoveryError::UnknownSelector { .. })
        ));
        assert!(matches!(
            parse_declared("bogus:anyhow", "1.2.3"),
            Err(DiscoveryError::UnknownSelector { .. })
        ));
        assert!(matches!(
            parse_declared("cargo:anyhow", "not-a-version"),
            Err(DiscoveryError::InvalidVersion { .. })
        ));
    }

    #[test]
    fn latest_stable_proposes_max_above_current() {
        // Upstream semver orders candidates; the max stable above current
        // wins via `compare`, never custom ordering.
        let current: semver::Version = "1.2.3".parse().expect("current");
        let available: Vec<semver::Version> = ["1.2.4", "1.10.0", "1.2.3"]
            .iter()
            .map(|text| text.parse().expect("available"))
            .collect();
        assert_eq!(
            latest_stable(&current, &available)
                .expect("latest")
                .to_string(),
            "1.10.0"
        );
    }

    #[test]
    fn stable_only_prerelease_never_wins() {
        // Discovery proposes stable only; prereleases stay upstream-governed
        // and never become the loop candidate.
        let current: semver::Version = "1.2.3".parse().expect("current");
        let available: Vec<semver::Version> = ["1.2.4-alpha.1", "2.0.0-beta.1"]
            .iter()
            .map(|text| text.parse().expect("available"))
            .collect();
        assert_eq!(latest_stable(&current, &available), None);
        // A stable above current still wins when prereleases sort higher
        // in mixed snapshots.
        let mixed: Vec<semver::Version> = ["1.2.4-alpha.1", "1.2.4"]
            .iter()
            .map(|text| text.parse().expect("available"))
            .collect();
        assert_eq!(
            latest_stable(&current, &mixed).expect("stable").to_string(),
            "1.2.4"
        );
    }

    #[test]
    fn up_to_date_has_no_candidate() {
        let entry = snapshot("cargo:anyhow", "1.10.0", &["1.2.3", "1.10.0"]);
        assert_eq!(outdated_from_snapshot(&entry), None);
        let entry = snapshot("npm:jest", "30.3.0", &["30.2.0", "30.3.0"]);
        assert_eq!(outdated_from_snapshot(&entry), None);
    }

    #[test]
    fn collect_orders_by_selector_never_batch() {
        // Two outdated snapshots order by selector text so the weekly run
        // proposes the same next dep; the loop still takes one (never batch).
        let entries = vec![
            snapshot("npm:jest", "30.2.0", &["30.3.0"]),
            snapshot("cargo:anyhow", "1.0.0", &["1.2.3"]),
        ];
        let candidates = collect_outdated(&entries);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].selector, "cargo:anyhow");
        assert_eq!(candidates[1].selector, "npm:jest");
        let next = next_outdated(&candidates).expect("next");
        assert_eq!(next.selector, "cargo:anyhow");
        assert_eq!(next.latest.to_string(), "1.2.3");
    }

    #[test]
    fn maven_group_artifact_orders_with_semver() {
        let entry = snapshot("maven:junit:junit", "4.13.2", &["4.13.3", "5.0.0"]);
        let candidate = outdated_from_snapshot(&entry).expect("outdated");
        assert_eq!(candidate.latest.to_string(), "5.0.0");
    }

    #[test]
    fn serde_json_and_semver_stay_upstream_owned() {
        // Registry payloads parse through upstream libraries, never custom
        // parsers: pin the ownership here so a future edit cannot
        // reimplement JSON or version ordering.
        let payload: serde_json::Value =
            serde_json::from_str(r#"{"name":"jest","version":"30.3.0"}"#).expect("json");
        assert_eq!(payload["version"], serde_json::json!("30.3.0"));
        let low: semver::Version = "1.2.3".parse().expect("low");
        let high: semver::Version = "1.10.0".parse().expect("high");
        assert_eq!(compare(&low, &high), std::cmp::Ordering::Less);
        assert!(is_stable(&high));
    }
}
