//! Backend committed-change manifest for `dx update`.
//!
//! See: `docs/cli/commands/audit-update-bazel.md#dx-update`.
//! Owning contract: `docs/cli/output-protocol.md#mutation`.
//!
//! V1 backends refresh standard locks through approved resolver
//! integrations. Each backend declares its committed lockfiles in
//! [`crate::sets::SetId::locks`]; this module validates the backend-owned
//! manifest derived from those declared paths (never a CLI filesystem scan,
//! Git inspection, BUILD parse, or rerun). A validated manifest projects to
//! NDJSON `change`/`mutation` events in the CLI layer (`cli/cli/src/exec/update.rs`)
//! as a minor-1.1 addition; absent or unchanged files project to no events,
//! preserving the v1.0 per-set `notice`/`error` contract. Directory hubs
//! (such as `third_party/dotnet/deps`) are not file changes and never
//! appear here; their owning lock (`paket.lock`) carries the report.

use super::sets::SetId;
use std::collections::BTreeSet;

/// Whether a committed file modifies an existing lock or creates a new one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommittedKind {
    /// Existing lock replaced by exact new bytes.
    Modify,
    /// New lock created with complete content.
    Create,
}

impl CommittedKind {
    /// Stable protocol spelling shared with NDJSON `change.kind`.
    pub fn name(self) -> &'static str {
        match self {
            CommittedKind::Modify => "modify",
            CommittedKind::Create => "create",
        }
    }
}

/// One backend-committed lockfile: intended bytes for NDJSON projection,
/// not whether the CLI has written them (the backend already committed).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedChange {
    /// Normalized workspace-relative lock path from [`SetId::locks`].
    pub path: String,
    /// Modify for a replaced lock, create for a new lock.
    pub kind: CommittedKind,
    /// BLAKE3-256 hex of the pre-commit bytes for `modify`, `None` for `create`.
    pub source_digest: Option<String>,
    /// Pre-commit byte length for `modify`, `0` for `create`.
    pub old_len: u64,
    /// Exact post-commit UTF-8 content for the spanning replacement.
    pub new_content: String,
}

/// Backend-owned manifest for one successful set update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedManifest {
    /// Owning set selector spelling (for example `cargo`).
    pub set: String,
    /// Committed file changes in normalized path-byte order.
    pub changes: Vec<CommittedChange>,
}

/// Manifest validation failure.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ManifestError {
    /// Unknown owning set spelling.
    #[error("unknown set {set:?}: want cargo, go, maven, npm, or nuget")]
    UnknownSet { set: String },
    /// Path is not a normalized workspace-relative lock path.
    #[error("invalid path {path:?}: {reason}")]
    BadPath { path: String, reason: &'static str },
    /// Path is not a declared lock of the owning set.
    #[error("undeclared lock {path:?} for set {set:?}: want one of {locks:?}")]
    UndeclaredLock {
        set: String,
        path: String,
        locks: Vec<String>,
    },
    /// Duplicate committed path.
    #[error("duplicate committed path {path:?}")]
    DuplicatePath { path: String },
    /// Changes are not in normalized path-byte order.
    #[error("unordered manifest: changes must be in normalized path-byte order")]
    Unordered,
    /// Digest spelling or kind pairing is invalid.
    #[error("invalid digest for {path:?}: {reason}")]
    BadDigest { path: String, reason: &'static str },
    /// New content is not valid UTF-8 text (caller passes decoded text;
    /// this guards empty create content only when explicitly empty).
    #[error("invalid content for {path:?}: replacement must be valid UTF-8")]
    BadContent { path: String },
}

fn check_path_shape(path: &str) -> Result<(), &'static str> {
    // Thin wrapper around `dx_path::classify` (sole ladder owner for order).
    // Messages preserve the historical manifest wording; Dot/DotDot share
    // one message. Pinned tests below prove the mapping plus ladder order.
    match dx_path::classify(path) {
        None => Ok(()),
        Some(dx_path::PathProblem::Empty) => Err("path must be non-empty"),
        Some(dx_path::PathProblem::Absolute) => {
            Err("path must be workspace-relative, not absolute")
        }
        Some(dx_path::PathProblem::Backslash) => Err("path must use forward slashes"),
        Some(dx_path::PathProblem::EmptyComponent) => Err("path must have no empty component"),
        Some(dx_path::PathProblem::Dot) | Some(dx_path::PathProblem::DotDot) => {
            Err("path must have no '.' or '..' component")
        }
    }
}

fn is_directory_hub(path: &str) -> bool {
    // Derived output folders are not file changes; their owning lock
    // carries the report. Today only the NuGet `deps` hub qualifies.
    path == "third_party/dotnet/deps"
}

fn check_digest_hex(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// One validated file projection: the exact full-file spanning edit for
/// NDJSON `change` plus its terminal `applied` mutation. The CLI layer
/// (`cli/cli/src/exec/update.rs`) converts this leaf-pure shape into
/// `dx_output` events so `dx_update` keeps no `dx_output` dependency.
/// See: `docs/cli/output-protocol.md#mutation`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedFile {
    /// Normalized workspace-relative lock path.
    pub path: String,
    /// Stable `change.kind` spelling (`modify` or `create`).
    pub kind: CommittedKind,
    /// Pre-commit digest for `modify`, `None` for `create`.
    pub source_digest: Option<String>,
    /// Inclusive start of the single spanning edit (`0` always).
    pub start_byte: u64,
    /// Exclusive end of the spanning edit (`old_len` for `modify`, `0` for `create`).
    pub end_byte: u64,
    /// Exact post-commit text for the spanning replacement.
    pub replacement: String,
}

/// Validates one backend committed-change manifest: owning set is known,
/// every path is a normalized declared file lock of that set, paths are unique
/// and in normalized byte order, `modify` carries a valid digest with its
/// pre-commit length while `create` carries none with zero length, and new
/// content is the exact post-commit text (full-file spanning replacement).
/// Directory hubs (such as `third_party/dotnet/deps`) never validate as
/// file changes; their owning lock (`paket.lock`) carries the report.
pub fn validate(manifest: &CommittedManifest) -> Result<(), ManifestError> {
    let set = SetId::parse(manifest.set.as_str()).ok_or_else(|| ManifestError::UnknownSet {
        set: manifest.set.clone(),
    })?;
    let locks: Vec<String> = set.locks().iter().map(|path| (*path).to_owned()).collect();
    let mut seen = BTreeSet::new();
    let mut previous: Option<&str> = None;
    for change in &manifest.changes {
        check_path_shape(&change.path).map_err(|reason| ManifestError::BadPath {
            path: change.path.clone(),
            reason,
        })?;
        if is_directory_hub(&change.path) {
            return Err(ManifestError::BadPath {
                path: change.path.clone(),
                reason: "directory hubs never appear as file changes",
            });
        }
        if !locks.contains(&change.path) {
            return Err(ManifestError::UndeclaredLock {
                set: manifest.set.clone(),
                path: change.path.clone(),
                locks: locks.clone(),
            });
        }
        if !seen.insert(change.path.clone()) {
            return Err(ManifestError::DuplicatePath {
                path: change.path.clone(),
            });
        }
        if let Some(previous) = previous {
            if previous.as_bytes() >= change.path.as_bytes() {
                return Err(ManifestError::Unordered);
            }
        }
        previous = Some(change.path.as_str());
        match change.kind {
            CommittedKind::Modify => {
                let digest =
                    change
                        .source_digest
                        .as_deref()
                        .ok_or_else(|| ManifestError::BadDigest {
                            path: change.path.clone(),
                            reason: "modify requires a source digest",
                        })?;
                if !check_digest_hex(digest) {
                    return Err(ManifestError::BadDigest {
                        path: change.path.clone(),
                        reason: "want exactly 64 lowercase hex characters",
                    });
                }
            }
            CommittedKind::Create => {
                if change.source_digest.is_some() {
                    return Err(ManifestError::BadDigest {
                        path: change.path.clone(),
                        reason: "create omits its source digest",
                    });
                }
                if change.old_len != 0 {
                    return Err(ManifestError::BadDigest {
                        path: change.path.clone(),
                        reason: "create holds zero pre-commit length",
                    });
                }
            }
        }
        if change.kind == CommittedKind::Create && change.new_content.is_empty() {
            // Empty create content is allowed only when the backend truly
            // committed an empty lock; the CLI still projects it as a
            // `0..0` insertion. No extra gate here beyond UTF-8 (String).
        }
    }
    Ok(())
}

/// Projects one validated manifest to its ordered file records: each
/// `modify` becomes one `0..old_len` spanning replacement with its
/// post-commit text, each `create` becomes one `0..0` insertion.
/// Absent or empty manifests project to no records, preserving the v1.0
/// per-set `notice`/`error` contract. Callers must [`validate`] first;
/// this re-validates and fails closed on any shape violation.
pub fn project(manifest: &CommittedManifest) -> Result<Vec<ProjectedFile>, ManifestError> {
    validate(manifest)?;
    Ok(manifest
        .changes
        .iter()
        .map(|change| match change.kind {
            CommittedKind::Modify => ProjectedFile {
                path: change.path.clone(),
                kind: CommittedKind::Modify,
                source_digest: change.source_digest.clone(),
                start_byte: 0,
                end_byte: change.old_len,
                replacement: change.new_content.clone(),
            },
            CommittedKind::Create => ProjectedFile {
                path: change.path.clone(),
                kind: CommittedKind::Create,
                source_digest: None,
                start_byte: 0,
                end_byte: 0,
                replacement: change.new_content.clone(),
            },
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn modify(path: &str) -> CommittedChange {
        CommittedChange {
            path: path.to_owned(),
            kind: CommittedKind::Modify,
            source_digest: Some(DIGEST.to_owned()),
            old_len: 3,
            new_content: "new\n".to_owned(),
        }
    }

    #[test]
    fn empty_manifest_is_valid_no_changes() {
        // Go no-op success with no file delta projects to no events.
        // See: `docs/cli/output-protocol.md#mutation`.
        let manifest = CommittedManifest {
            set: "go".to_owned(),
            changes: vec![],
        };
        validate(&manifest).expect("empty valid");
    }

    #[test]
    fn cargo_locks_validate_in_order() {
        let manifest = CommittedManifest {
            set: "cargo".to_owned(),
            changes: vec![
                modify("cargo-bazel-lock.json"),
                modify("rust/tests/fixtures/hello/Cargo.lock"),
            ],
        };
        // Normalized byte order: `cargo-...` sorts before `rust/...`.
        validate(&manifest).expect("ordered cargo locks");
        let swapped = CommittedManifest {
            set: "cargo".to_owned(),
            changes: vec![
                modify("rust/tests/fixtures/hello/Cargo.lock"),
                modify("cargo-bazel-lock.json"),
            ],
        };
        assert_eq!(validate(&swapped), Err(ManifestError::Unordered));
    }

    #[test]
    fn npm_lock_validates_and_rejects_undeclared() {
        let manifest = CommittedManifest {
            set: "npm".to_owned(),
            changes: vec![modify("pnpm-lock.yaml")],
        };
        validate(&manifest).expect("npm lock");
        let bad = CommittedManifest {
            set: "npm".to_owned(),
            changes: vec![modify("package.json")],
        };
        assert!(matches!(
            validate(&bad),
            Err(ManifestError::UndeclaredLock { .. })
        ));
    }

    #[test]
    fn create_omits_digest_with_zero_length() {
        // New locks (for example a first-time derived lock) create.
        let manifest = CommittedManifest {
            set: "maven".to_owned(),
            changes: vec![CommittedChange {
                path: "third_party/jvm/maven_install.json".to_owned(),
                kind: CommittedKind::Create,
                source_digest: None,
                old_len: 0,
                new_content: "{}\n".to_owned(),
            }],
        };
        validate(&manifest).expect("create");
        let mut bad = manifest.clone();
        bad.changes[0].source_digest = Some(DIGEST.to_owned());
        assert!(validate(&bad).is_err());
        bad.changes[0].source_digest = None;
        bad.changes[0].old_len = 3;
        assert!(validate(&bad).is_err());
    }

    #[test]
    fn rejects_unknown_set_duplicate_and_bad_shapes() {
        let unknown = CommittedManifest {
            set: "pip".to_owned(),
            changes: vec![],
        };
        assert_eq!(
            validate(&unknown),
            Err(ManifestError::UnknownSet {
                set: "pip".to_owned()
            })
        );
        let duplicate = CommittedManifest {
            set: "npm".to_owned(),
            changes: vec![modify("pnpm-lock.yaml"), modify("pnpm-lock.yaml")],
        };
        assert_eq!(
            validate(&duplicate),
            Err(ManifestError::DuplicatePath {
                path: "pnpm-lock.yaml".to_owned()
            })
        );
        let absolute = CommittedManifest {
            set: "npm".to_owned(),
            changes: vec![CommittedChange {
                path: "/abs".to_owned(),
                ..modify("pnpm-lock.yaml")
            }],
        };
        assert!(matches!(
            validate(&absolute),
            Err(ManifestError::BadPath { .. })
        ));
        let mut bad_digest = CommittedManifest {
            set: "npm".to_owned(),
            changes: vec![modify("pnpm-lock.yaml")],
        };
        bad_digest.changes[0].source_digest = Some("SHORT".to_owned());
        assert!(matches!(
            validate(&bad_digest),
            Err(ManifestError::BadDigest { .. })
        ));
        let mut missing_digest = CommittedManifest {
            set: "npm".to_owned(),
            changes: vec![modify("pnpm-lock.yaml")],
        };
        missing_digest.changes[0].source_digest = None;
        assert!(validate(&missing_digest).is_err());
        // Directory hubs never appear as file changes.
        let dir = CommittedManifest {
            set: "nuget".to_owned(),
            changes: vec![modify("third_party/dotnet/deps")],
        };
        // `deps` is a declared output folder but a directory hub; the
        // manifest fails closed here so no file event is ever forged.
        // Their owning lock (`paket.lock`) carries the report.
        // See: `docs/cli/output-protocol.md#mutation`.
        assert!(matches!(validate(&dir), Err(ManifestError::BadPath { .. })));
        assert!(project(&dir).is_err());
    }

    #[test]
    fn path_messages_are_pinned_to_dx_path_ladder() {
        // Thin wrapper over `dx_path::classify`: exact messages plus ladder
        // order are pinned so drift fails here.
        for (path, reason) in [
            ("", "path must be non-empty"),
            ("/abs", "path must be workspace-relative, not absolute"),
            ("a\\b", "path must use forward slashes"),
            ("a//b", "path must have no empty component"),
            ("a/./b", "path must have no '.' or '..' component"),
            ("a/../b", "path must have no '.' or '..' component"),
        ] {
            assert_eq!(check_path_shape(path), Err(reason), "path: {path:?}");
        }
        assert_eq!(check_path_shape("pnpm-lock.yaml"), Ok(()));
        // Ladder order: absolute beats backslash/dot, dot beats dot-dot
        // (shared message, but the winning rung is order-determined).
        assert_eq!(
            check_path_shape("/a//b"),
            Err("path must be workspace-relative, not absolute")
        );
        assert_eq!(
            check_path_shape("a\\//b"),
            Err("path must use forward slashes")
        );
        assert_eq!(
            check_path_shape("a/./../b"),
            Err("path must have no '.' or '..' component")
        );
    }

    #[test]
    fn projection_spans_full_file_in_path_order() {
        // See: `docs/cli/output-protocol.md#mutation`.
        let manifest = CommittedManifest {
            set: "cargo".to_owned(),
            changes: vec![
                modify("cargo-bazel-lock.json"),
                modify("rust/tests/fixtures/hello/Cargo.lock"),
            ],
        };
        let projected = project(&manifest).expect("project");
        assert_eq!(projected.len(), 2);
        assert_eq!(projected[0].path, "cargo-bazel-lock.json");
        assert_eq!(projected[0].start_byte, 0);
        assert_eq!(projected[0].end_byte, 3);
        assert_eq!(projected[0].replacement, "new\n");
        assert_eq!(projected[0].source_digest.as_deref(), Some(DIGEST));
        assert_eq!(projected[1].path, "rust/tests/fixtures/hello/Cargo.lock");
        // Empty manifests project to no records, preserving v1.0.
        let empty = CommittedManifest {
            set: "go".to_owned(),
            changes: vec![],
        };
        assert!(project(&empty).expect("empty").is_empty());
        // Create projects to a 0..0 insertion.
        let create = CommittedManifest {
            set: "maven".to_owned(),
            changes: vec![CommittedChange {
                path: "third_party/jvm/maven_install.json".to_owned(),
                kind: CommittedKind::Create,
                source_digest: None,
                old_len: 0,
                new_content: "{}\n".to_owned(),
            }],
        };
        let projected = project(&create).expect("create");
        assert_eq!(projected[0].start_byte, 0);
        assert_eq!(projected[0].end_byte, 0);
        assert_eq!(projected[0].replacement, "{}\n");
        assert!(projected[0].source_digest.is_none());
    }
}
