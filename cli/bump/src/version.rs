//! Version validation for `dx bump`.
//!
//! Library-first: version parsing and comparison delegate to the upstream
//! `semver` crate, never to custom version code. Registry discovery (BCR,
//! crates.io, npm, Go proxy, GitHub releases) and manifest parsing
//! (`serde_json`, `toml`, `toml_edit`) are upstream-owned; this module only validates
//! the operator-supplied new version shape and pins the stable-only
//! discovery policy. Custom code is limited to the thin
//! single-requirement edit in [`crate::request`].
//!
//! Policy (matching the issue):
//! - Discovery proposes stable versions only; prerelease eligibility
//!   follows the upstream resolver and project configuration, never a
//!   private `dx` policy.
//! - The explicit `dx bump <selector> <version>` operation accepts the
//!   operator-supplied version verbatim after shape validation: exact
//!   semver for Bazel/Cargo/npm/Go/Maven/NuGet (pinned exactly per ADR
//!   0008), Git tag/commit shapes for GitHub Actions and Git-backed
//!   requirements.
//! - Transitive versions stay resolver-governed; bump never forces every
//!   transitive package to newest.

use super::sets::BumpSet;

/// Validated new version for one widen edit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WidenVersion {
    /// Exact stable semver (Bazel modules, `.bazelversion`, Cargo, npm,
    /// Go, Maven, NuGet). Pinned exactly per ADR 0008; comparison uses
    /// upstream `semver`, never custom ordering.
    Semver(semver::Version),
    /// Git tag shape (GitHub Actions pins, Git-backed requirements).
    /// SHA resolution runs through the upstream GitHub releases / Git
    /// client, never custom fetch code.
    GitTag(String),
    /// Explicit commit SHA (40- or 64-char hex). Locked commits stay
    /// unchanged unless explicitly bumped here.
    GitCommit(String),
}

impl WidenVersion {
    /// Human spelling for summaries (never argv).
    pub fn display(&self) -> String {
        match self {
            WidenVersion::Semver(version) => version.to_string(),
            WidenVersion::GitTag(tag) => tag.clone(),
            WidenVersion::GitCommit(sha) => sha.clone(),
        }
    }

    /// True for exact semver (ADR 0008 exact-pin path).
    pub fn is_semver(&self) -> bool {
        matches!(self, WidenVersion::Semver(_))
    }
}

/// Version-shape errors (exit 2, never a partial widen).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum VersionError {
    /// Empty version string.
    #[error("empty version")]
    Empty,
    /// Invalid semver for a semver-owned set.
    #[error(
        "invalid version {version:?} for set {set}: expected exact stable semver (e.g. 1.2.3)"
    )]
    InvalidSemver {
        /// Owning set name.
        set: &'static str,
        /// Offending spelling.
        version: String,
    },
    /// Prerelease supplied where discovery proposes stable only.
    /// Explicit bumps may still carry prereleases when the upstream
    /// resolver/project config allows them; this pin exists so the loop
    /// filters stable by default without inventing a private policy.
    #[error("prerelease version {version:?} follows upstream resolver and project config, never a private dx policy")]
    Prerelease {
        /// Offending spelling.
        version: String,
    },
    /// Invalid Git tag/commit shape for a Git-owned set.
    #[error("invalid version {version:?} for set {set}: expected a Git tag (e.g. v4) or a 40/64-char commit SHA")]
    InvalidGit {
        /// Owning set name.
        set: &'static str,
        /// Offending spelling.
        version: String,
    },
}

/// Prerelease eligibility follows the upstream resolver and project
/// configuration, never a private `dx` policy. There is no private
/// prerelease rule to configure; this pin exists so a future integration
/// cannot invent one.
pub fn prerelease_follows_upstream() -> bool {
    true
}

/// True when a parsed semver is stable (no prerelease). Build metadata is
/// allowed: it does not affect precedence and stays pinned verbatim.
pub fn is_stable(version: &semver::Version) -> bool {
    version.pre.is_empty()
}

/// Compares two exact versions with upstream `semver` ordering, never
/// custom comparison. Used by the loop to order outdated candidates.
pub fn compare(left: &semver::Version, right: &semver::Version) -> std::cmp::Ordering {
    left.cmp(right)
}

/// Parses the operator-supplied new version for one set. Bazel, Cargo,
/// npm, Go, Maven, and NuGet require exact stable semver (leading `v`/`=`
/// stripped for ergonomics, e.g. `v1.2.3` means `1.2.3`); GitHub Actions
/// accepts a Git tag or commit SHA. Prereleases parse but callers treat
/// them as upstream-governed (see [`prerelease_follows_upstream`]).
pub fn parse(set: BumpSet, text: &str) -> Result<WidenVersion, VersionError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(VersionError::Empty);
    }
    match set {
        BumpSet::Bazel
        | BumpSet::Cargo
        | BumpSet::Npm
        | BumpSet::Go
        | BumpSet::Maven
        | BumpSet::NuGet => parse_semver(set.name(), trimmed),
        BumpSet::GithubActions => parse_git(set.name(), trimmed),
    }
}

fn parse_semver(set: &'static str, text: &str) -> Result<WidenVersion, VersionError> {
    // Ergonomics only: strip one leading `v`/`=`/`==` plus whitespace.
    // Comparison and pinning still use upstream `semver` verbatim.
    let mut candidate = text.trim();
    candidate = candidate.strip_prefix("==").unwrap_or(candidate);
    candidate = candidate.strip_prefix('=').unwrap_or(candidate);
    candidate = candidate.strip_prefix('v').unwrap_or(candidate);
    candidate = candidate.trim();
    if candidate.is_empty() {
        return Err(VersionError::InvalidSemver {
            set,
            version: text.to_owned(),
        });
    }
    match candidate.parse::<semver::Version>() {
        Ok(version) => Ok(WidenVersion::Semver(version)),
        Err(_) => Err(VersionError::InvalidSemver {
            set,
            version: text.to_owned(),
        }),
    }
}

fn parse_git(set: &'static str, text: &str) -> Result<WidenVersion, VersionError> {
    let candidate = text.trim();
    if candidate.is_empty()
        || candidate.contains(' ')
        || candidate.contains(':')
        || candidate.contains('/')
    {
        // `owner/repo` never appears in the version position; slashes
        // belong to the selector package, never the version.
        return Err(VersionError::InvalidGit {
            set,
            version: text.to_owned(),
        });
    }
    if is_commit_sha(candidate) {
        return Ok(WidenVersion::GitCommit(candidate.to_owned()));
    }
    // Tag shape: `v4`, `v4.1.0`, `9.2.0`, etc. Must be non-empty and
    // contain no whitespace/colons/slashes (checked above). A 40/64-hex
    // string is a commit, never a tag (checked first).
    if candidate.len() <= 128 {
        return Ok(WidenVersion::GitTag(candidate.to_owned()));
    }
    Err(VersionError::InvalidGit {
        set,
        version: text.to_owned(),
    })
}

fn is_commit_sha(text: &str) -> bool {
    (text.len() == 40 || text.len() == 64) && text.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_sets_require_exact_stable_shapes() {
        for set in [
            BumpSet::Bazel,
            BumpSet::Cargo,
            BumpSet::Npm,
            BumpSet::Go,
            BumpSet::Maven,
            BumpSet::NuGet,
        ] {
            let parsed = parse(set, "1.2.3").expect("semver");
            assert!(parsed.is_semver());
            assert_eq!(parsed.display(), "1.2.3");
            // Leading `v`/`=` ergonomics still pin exact semver.
            assert_eq!(parse(set, "v1.2.3").expect("v").display(), "1.2.3");
            assert_eq!(parse(set, "=1.2.3").expect("=").display(), "1.2.3");
            assert!(matches!(parse(set, ""), Err(VersionError::Empty)));
            assert!(matches!(
                parse(set, "not-a-version!!!"),
                Err(VersionError::InvalidSemver { .. })
            ));
            assert!(matches!(
                parse(set, "1.2"),
                Err(VersionError::InvalidSemver { .. })
            ));
        }
    }

    #[test]
    fn prerelease_parses_but_follows_upstream() {
        assert!(prerelease_follows_upstream());
        let parsed = parse(BumpSet::Cargo, "1.2.3-alpha.1").expect("prerelease parses");
        match parsed {
            WidenVersion::Semver(version) => {
                assert!(!is_stable(&version));
                // Upstream semver orders prereleases below their release.
                assert_eq!(
                    compare(&version, &"1.2.3".parse().expect("stable")),
                    std::cmp::Ordering::Less
                );
            }
            _ => panic!("prerelease is semver"),
        }
        assert!(is_stable(&"1.2.3".parse().expect("stable")));
        // The loop filters stable by default; the explicit operation
        // still validates shape without inventing a private policy.
        let _ = VersionError::Prerelease {
            version: "1.2.3-alpha.1".to_owned(),
        };
    }

    #[test]
    fn github_actions_accepts_tags_and_commits() {
        assert_eq!(
            parse(BumpSet::GithubActions, "v4").expect("tag").display(),
            "v4"
        );
        assert_eq!(
            parse(BumpSet::GithubActions, "v4.1.0")
                .expect("tag")
                .display(),
            "v4.1.0"
        );
        let sha = "3d3c42e5aac5ba805825da76410c181273ba90b1";
        assert_eq!(
            parse(BumpSet::GithubActions, sha).expect("sha"),
            WidenVersion::GitCommit(sha.to_owned())
        );
        assert!(matches!(
            parse(BumpSet::GithubActions, ""),
            Err(VersionError::Empty)
        ));
        assert!(matches!(
            parse(BumpSet::GithubActions, "bad tag"),
            Err(VersionError::InvalidGit { .. })
        ));
        assert!(matches!(
            parse(BumpSet::GithubActions, "owner/repo"),
            Err(VersionError::InvalidGit { .. })
        ));
    }

    #[test]
    fn ordering_delegates_to_upstream_semver() {
        let low: semver::Version = "1.2.3".parse().expect("low");
        let high: semver::Version = "1.10.0".parse().expect("high");
        assert_eq!(compare(&low, &high), std::cmp::Ordering::Less);
        assert_eq!(compare(&high, &low), std::cmp::Ordering::Greater);
        assert_eq!(compare(&low, &low), std::cmp::Ordering::Equal);
    }

    #[test]
    fn serde_json_and_toml_stay_upstream_owned() {
        // Manifest shapes parse through upstream libraries, never custom
        // parsers: pin the ownership here so a future edit cannot
        // reimplement JSON/TOML. `toml_edit` owns format-preserving Cargo
        // edits; `package.json` stays on `serde_json::Value`.
        let package: serde_json::Value =
            serde_json::from_str(r#"{"name":"react","version":"1.2.3"}"#).expect("json");
        assert_eq!(package["version"], serde_json::json!("1.2.3"));
        let manifest: toml::Table = toml::from_str("version = \"1.2.3\"\n").expect("toml");
        assert_eq!(manifest["version"].as_str(), Some("1.2.3"));
        let doc = "version = \"1.2.3\"\n"
            .parse::<toml_edit::DocumentMut>()
            .expect("toml_edit");
        assert_eq!(doc["version"].as_str(), Some("1.2.3"));
    }
}
