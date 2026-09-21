//! GitHub Actions tag-to-SHA auto resolution planning for `dx bump`.
//!
//! Contract: `docs/cli/commands/audit-update-bazel.md#dx-bump`.
//!
//! Library-first (ADR 0008): tag enumeration plus SHA resolution use the
//! upstream GitHub releases client, never custom HTTP. Version shapes
//! validate through `version::parse`, never custom version code. This module
//! plans over injected tag snapshots only, so resolution stays deterministic
//! and unit-testable without a workspace, a Bazel server, registries, or any
//! upstream updater. Fetching stays in the upstream client plus the scheduled
//! `bump.yml` runner; custom code here is limited to snapshot matching plus
//! shape validation for the one-tag-per-run loop.
//!
//! Policy (See: `docs/cli/commands/audit-update-bazel.md#dx-bump`, issue #640):
//! - Tags auto-resolve to SHA via upstream snapshots; manual SHA only is
//!   rejected as the sole route for the automatic goal.
//! - Discovery lists the tag candidate but never invents a SHA
//!   (`discovery::parse_declared` stays semver-only); this module owns the
//!   auto resolution before the file edit.
//! - Unknown tags fail closed with no invented SHA (nothing widened).
//! - SHA shapes stay 40/64-char hex via `version::parse`, never custom fetch.

use super::sets::BumpSet;
use super::version::{self, WidenVersion};

/// Upstream registry client owning GHA tag enumeration plus SHA resolution
/// (never custom HTTP). Mirrors `discovery::registry_client` for the
/// GitHub Actions set.
pub fn upstream_client() -> &'static str {
    "GitHub releases"
}

/// Injected tag snapshot from the upstream GitHub releases client: one
/// `owner/repo` tag plus its resolved commit SHA. The scheduled `bump.yml`
/// runner fetches these via `gh api`; this module only matches them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TagSnapshot {
    /// Action identity (`owner/repo`).
    pub package: String,
    /// Tag spelling (e.g. `v5`).
    pub tag: String,
    /// Resolved 40/64-char commit SHA.
    pub sha: String,
}

/// Tag-to-SHA resolution errors (never a partial widen).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum GhaError {
    /// Empty package, tag, or SHA text.
    #[error("empty GHA tag entry; expected `owner/repo` plus tag plus SHA")]
    Empty,
    /// Invalid action identity for the GitHub Actions set.
    #[error("invalid package {package:?} for github-actions: expected owner/repo")]
    InvalidPackage {
        /// Offending spelling.
        package: String,
    },
    /// Invalid tag shape for the GitHub Actions set.
    #[error("invalid tag {tag:?} for {package:?}: expected a Git tag (e.g. v4)")]
    InvalidTag {
        /// Owning action.
        package: String,
        /// Offending spelling.
        tag: String,
    },
    /// Invalid SHA shape in the upstream snapshot.
    #[error("invalid SHA {sha:?} for {package:?} tag {tag:?}: expected a 40/64-char commit SHA")]
    InvalidSha {
        /// Owning action.
        package: String,
        /// Tag sought.
        tag: String,
        /// Offending spelling.
        sha: String,
    },
    /// No upstream snapshot for this tag (nothing widened, never invent).
    #[error("unknown tag {tag:?} for {package:?}: no upstream GitHub releases snapshot (nothing widened; never invent a SHA)")]
    UnknownTag {
        /// Action sought (`owner/repo`).
        package: String,
        /// Tag sought.
        tag: String,
    },
}

/// Parses one injected upstream snapshot (`owner/repo` plus tag plus SHA).
/// Shapes validate through `version::parse` for the GitHub Actions set;
/// package identity mirrors `request` owner/repo rules.
pub fn parse_snapshot(package: &str, tag: &str, sha: &str) -> Result<TagSnapshot, GhaError> {
    if package.is_empty() || tag.is_empty() || sha.is_empty() {
        return Err(GhaError::Empty);
    }
    if !is_owner_repo(package) {
        return Err(GhaError::InvalidPackage {
            package: package.to_owned(),
        });
    }
    match version::parse(BumpSet::GithubActions, tag) {
        Ok(WidenVersion::GitTag(_)) => {}
        _ => {
            return Err(GhaError::InvalidTag {
                package: package.to_owned(),
                tag: tag.to_owned(),
            });
        }
    }
    match version::parse(BumpSet::GithubActions, sha) {
        Ok(WidenVersion::GitCommit(_)) => {}
        _ => {
            return Err(GhaError::InvalidSha {
                package: package.to_owned(),
                tag: tag.to_owned(),
                sha: sha.to_owned(),
            });
        }
    }
    Ok(TagSnapshot {
        package: package.to_owned(),
        tag: tag.to_owned(),
        sha: sha.to_owned(),
    })
}

/// Auto-resolves one tag to its SHA from injected upstream snapshots.
/// Returns the resolved SHA; fails closed with `UnknownTag` when no snapshot
/// matches (nothing widened, never invents a SHA). Shapes validate before
/// matching so malformed inputs never scan snapshots.
pub fn resolve_tag(
    package: &str,
    tag: &str,
    snapshots: &[TagSnapshot],
) -> Result<String, GhaError> {
    if package.is_empty() || tag.is_empty() {
        return Err(GhaError::Empty);
    }
    if !is_owner_repo(package) {
        return Err(GhaError::InvalidPackage {
            package: package.to_owned(),
        });
    }
    match version::parse(BumpSet::GithubActions, tag) {
        Ok(WidenVersion::GitTag(_)) => {}
        _ => {
            return Err(GhaError::InvalidTag {
                package: package.to_owned(),
                tag: tag.to_owned(),
            });
        }
    }
    for snapshot in snapshots {
        if snapshot.package == package && snapshot.tag == tag {
            // Snapshot SHAs stay validated: a hand-built snapshot carrying a
            // non-SHA fails closed instead of widening a corrupt pin.
            match version::parse(BumpSet::GithubActions, &snapshot.sha) {
                Ok(WidenVersion::GitCommit(_)) => return Ok(snapshot.sha.clone()),
                _ => {
                    return Err(GhaError::InvalidSha {
                        package: package.to_owned(),
                        tag: tag.to_owned(),
                        sha: snapshot.sha.clone(),
                    });
                }
            }
        }
    }
    Err(GhaError::UnknownTag {
        package: package.to_owned(),
        tag: tag.to_owned(),
    })
}

/// True for `owner/repo` identities (mirrors the `request` GHA rule: single
/// slash, no spaces, dotted `owner`/`repo`).
fn is_owner_repo(package: &str) -> bool {
    let (owner, repo) = match package.split_once('/') {
        Some((owner, repo)) => (owner, repo),
        None => return false,
    };
    if owner.is_empty()
        || repo.is_empty()
        || repo.contains('/')
        || repo.contains(' ')
        || owner.contains(' ')
    {
        return false;
    }
    is_dotted(owner) && is_dotted(repo)
}

/// True for `[A-Za-z0-9_.-]` names (mirrors the `request` dotted rule).
fn is_dotted(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Vec<TagSnapshot> {
        vec![
            parse_snapshot(
                "actions/checkout",
                "v5",
                "3d3c42e5aac5ba805825da76410c181273ba90b1",
            )
            .expect("snapshot"),
            parse_snapshot(
                "actions/cache",
                "v4",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .expect("snapshot"),
        ]
    }

    #[test]
    fn upstream_client_is_github_releases() {
        // Issue #640 (See: `docs/cli/commands/audit-update-bazel.md#dx-bump`): GHA tags enumerate plus auto-resolve through the
        // upstream GitHub releases client, never custom HTTP.
        assert_eq!(upstream_client(), "GitHub releases");
        assert_eq!(
            super::super::discovery::registry_client(BumpSet::GithubActions),
            "GitHub releases"
        );
    }

    #[test]
    fn tag_auto_resolves_to_sha_from_upstream_snapshot() {
        // Issue #640 (See: `docs/cli/commands/audit-update-bazel.md#dx-bump`): `v5` auto-resolves to its upstream SHA before the file
        // edit; manual SHA only stays rejected as the sole route.
        let snapshots = snapshot();
        assert_eq!(
            resolve_tag("actions/checkout", "v5", &snapshots).expect("resolve"),
            "3d3c42e5aac5ba805825da76410c181273ba90b1"
        );
        assert_eq!(
            resolve_tag("actions/cache", "v4", &snapshots).expect("resolve"),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
    }

    #[test]
    fn unknown_tag_fails_closed_without_inventing_sha() {
        // No snapshot means nothing widened; the resolver never invents a
        // SHA and manual SHA only stays rejected for the automatic goal.
        let snapshots = snapshot();
        assert!(matches!(
            resolve_tag("actions/checkout", "v9", &snapshots),
            Err(GhaError::UnknownTag { .. })
        ));
        assert!(matches!(
            resolve_tag("actions/unknown", "v5", &snapshots),
            Err(GhaError::UnknownTag { .. })
        ));
        let empty: Vec<TagSnapshot> = Vec::new();
        assert!(matches!(
            resolve_tag("actions/checkout", "v5", &empty),
            Err(GhaError::UnknownTag { .. })
        ));
    }

    #[test]
    fn malformed_entries_fail_closed() {
        let snapshots = snapshot();
        assert!(matches!(
            resolve_tag("", "v5", &snapshots),
            Err(GhaError::Empty)
        ));
        assert!(matches!(
            resolve_tag("actions/checkout", "", &snapshots),
            Err(GhaError::Empty)
        ));
        assert!(matches!(
            parse_snapshot("", "v5", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            Err(GhaError::Empty)
        ));
        assert!(matches!(
            resolve_tag("not-an-action", "v5", &snapshots),
            Err(GhaError::InvalidPackage { .. })
        ));
        assert!(matches!(
            parse_snapshot(
                "not-an-action",
                "v5",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            ),
            Err(GhaError::InvalidPackage { .. })
        ));
        // SHA-shaped text is not a tag; tag position rejects it as InvalidTag
        // so a commit never re-resolves through the tag path.
        assert!(matches!(
            resolve_tag(
                "actions/checkout",
                "3d3c42e5aac5ba805825da76410c181273ba90b1",
                &snapshots
            ),
            Err(GhaError::InvalidTag { .. })
        ));
        assert!(matches!(
            resolve_tag("actions/checkout", "bad tag", &snapshots),
            Err(GhaError::InvalidTag { .. })
        ));
    }

    #[test]
    fn invalid_snapshot_sha_fails_closed() {
        assert!(matches!(
            parse_snapshot("actions/checkout", "v5", "not-a-sha!!!"),
            Err(GhaError::InvalidSha { .. })
        ));
        // Hand-built snapshots bypassing `parse_snapshot` still fail closed
        // on resolve instead of widening a corrupt pin.
        let corrupt = vec![TagSnapshot {
            package: "actions/checkout".to_owned(),
            tag: "v5".to_owned(),
            sha: "not-a-sha".to_owned(),
        }];
        assert!(matches!(
            resolve_tag("actions/checkout", "v5", &corrupt),
            Err(GhaError::InvalidSha { .. })
        ));
    }

    #[test]
    fn snapshot_shapes_validate_through_upstream_parse() {
        // Shapes delegate to upstream `version::parse`, never custom version
        // code: tags parse as GitTag, SHAs as GitCommit.
        assert!(matches!(
            version::parse(BumpSet::GithubActions, "v5"),
            Ok(WidenVersion::GitTag(_))
        ));
        assert!(matches!(
            version::parse(
                BumpSet::GithubActions,
                "3d3c42e5aac5ba805825da76410c181273ba90b1"
            ),
            Ok(WidenVersion::GitCommit(_))
        ));
    }
}
