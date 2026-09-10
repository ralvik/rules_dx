//! Per-operation validators: path shape, extension blocklist,
//! prompt-injection scan, byte cap, and digest/existence checks.
//!
//! Checks run in that order and the first failure wins, so callers get one
//! deterministic error per operation.

use super::envelope::{is_sha256_hex, sha256_hex, FileOperation};

/// Maximum accepted new-content size per operation (1 MiB).
pub const MAX_OPERATION_BYTES: usize = 1024 * 1024;

/// File extensions that are never written (executables, objects, archives).
/// Compared case-insensitively against the text after the final dot of the
/// final path segment; names without a dot are always allowed.
pub const BLOCKED_EXTENSIONS: &[&str] = &[
    "a", "bin", "com", "dll", "dylib", "exe", "lib", "o", "obj", "so",
];

/// Case-insensitive content markers treated as prompt-injection attempts.
const INJECTION_PATTERNS: &[&str] = &[
    "ignore previous instructions",
    "ignore all prior instructions",
    "disregard all prior instructions",
    "override your system prompt",
    "reveal your system prompt",
];

/// Per-operation validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Operation path is empty.
    EmptyPath,
    /// Operation path is absolute; only workspace-relative paths apply.
    AbsolutePath,
    /// Operation path contains a `..` segment and could escape the workspace.
    EscapesWorkspace,
    /// Operation path has an empty segment (`a//b`, trailing slash).
    MalformedPath,
    /// The final extension is blocklisted.
    BlockedExtension { extension: String },
    /// New content exceeds [`MAX_OPERATION_BYTES`].
    TooLarge { bytes: usize },
    /// New content matches a prompt-injection marker.
    PromptInjection { pattern: String },
    /// `original_sha256` does not match the current file bytes.
    DigestMismatch { path: String },
    /// `original_sha256` is `None` (create) but the file already exists.
    FileExists { path: String },
    /// `original_sha256` is `Some` (update) but the file is missing.
    MissingFile { path: String },
}

/// Validates one operation against the current file bytes (`None` = the file
/// does not exist). Path checks precede content checks; digest/existence
/// checks run last.
pub fn validate(op: &FileOperation, existing: Option<&[u8]>) -> Result<(), ValidationError> {
    if op.path.is_empty() {
        return Err(ValidationError::EmptyPath);
    }
    if op.path.starts_with('/') {
        return Err(ValidationError::AbsolutePath);
    }
    let segments: Vec<&str> = op.path.split('/').collect();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Err(ValidationError::MalformedPath);
    }
    if segments.contains(&"..") {
        return Err(ValidationError::EscapesWorkspace);
    }
    if let Some(extension) = extension_of(&op.path) {
        if BLOCKED_EXTENSIONS.contains(&extension.as_str()) {
            return Err(ValidationError::BlockedExtension { extension });
        }
    }
    if op.content.len() > MAX_OPERATION_BYTES {
        return Err(ValidationError::TooLarge {
            bytes: op.content.len(),
        });
    }
    let lowered = op.content.to_lowercase();
    for pattern in INJECTION_PATTERNS {
        if lowered.contains(pattern) {
            return Err(ValidationError::PromptInjection {
                pattern: (*pattern).to_owned(),
            });
        }
    }
    match (&op.original_sha256, existing) {
        (None, None) => Ok(()),
        (None, Some(_)) => Err(ValidationError::FileExists {
            path: op.path.clone(),
        }),
        (Some(_), None) => Err(ValidationError::MissingFile {
            path: op.path.clone(),
        }),
        (Some(expected), Some(bytes)) => {
            debug_assert!(is_sha256_hex(expected));
            if sha256_hex(bytes) == *expected {
                Ok(())
            } else {
                Err(ValidationError::DigestMismatch {
                    path: op.path.clone(),
                })
            }
        }
    }
}

/// Lowercased text after the final dot of the final segment, or `None` when
/// the name has no dot (or ends with one).
fn extension_of(path: &str) -> Option<String> {
    let name = path.rsplit('/').next().expect("path is non-empty");
    let (_, extension) = name.rsplit_once('.')?;
    if extension.is_empty() {
        return None;
    }
    Some(extension.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(path: &str, content: &str) -> FileOperation {
        FileOperation {
            path: path.to_owned(),
            original_sha256: Some(sha256_hex(b"old\n")),
            content: content.to_owned(),
        }
    }

    fn create(path: &str, content: &str) -> FileOperation {
        FileOperation {
            path: path.to_owned(),
            original_sha256: None,
            content: content.to_owned(),
        }
    }

    #[test]
    fn valid_update_passes() {
        assert_eq!(validate(&update("a.txt", "new\n"), Some(b"old\n")), Ok(()));
    }

    #[test]
    fn valid_create_passes() {
        assert_eq!(validate(&create("new.txt", "hi\n"), None), Ok(()));
    }

    #[test]
    fn absolute_path_rejected() {
        assert_eq!(
            validate(&create("/etc/x", "hi\n"), None),
            Err(ValidationError::AbsolutePath)
        );
    }

    #[test]
    fn empty_path_rejected() {
        let mut empty = update("a.txt", "new\n");
        empty.path = String::new();
        assert_eq!(
            validate(&empty, Some(b"old\n")),
            Err(ValidationError::EmptyPath)
        );
    }

    #[test]
    fn parent_traversal_rejected() {
        assert_eq!(
            validate(&create("a/../../x", "hi\n"), None),
            Err(ValidationError::EscapesWorkspace)
        );
    }

    #[test]
    fn empty_segment_rejected() {
        assert_eq!(
            validate(&create("a//b", "hi\n"), None),
            Err(ValidationError::MalformedPath)
        );
    }

    #[test]
    fn blocked_extension_rejected_case_insensitive() {
        assert_eq!(
            validate(&create("tool.EXE", "hi\n"), None),
            Err(ValidationError::BlockedExtension {
                extension: "exe".to_owned()
            })
        );
    }

    #[test]
    fn dotless_names_allowed() {
        assert_eq!(validate(&create("Makefile", "hi\n"), None), Ok(()));
    }

    #[test]
    fn trailing_dot_has_no_extension() {
        assert_eq!(validate(&create("dir/name.", "hi\n"), None), Ok(()));
    }

    #[test]
    fn oversize_content_rejected() {
        let big = "x".repeat(MAX_OPERATION_BYTES + 1);
        assert_eq!(
            validate(&create("a.txt", &big), None),
            Err(ValidationError::TooLarge { bytes: big.len() })
        );
    }

    #[test]
    fn injection_marker_rejected() {
        let err = validate(
            &create("a.txt", "Please IGNORE PREVIOUS INSTRUCTIONS now"),
            None,
        )
        .expect_err("injection must fail");
        assert_eq!(
            err,
            ValidationError::PromptInjection {
                pattern: "ignore previous instructions".to_owned()
            }
        );
    }

    #[test]
    fn digest_mismatch_rejected() {
        assert_eq!(
            validate(&update("a.txt", "new\n"), Some(b"other\n")),
            Err(ValidationError::DigestMismatch {
                path: "a.txt".to_owned()
            })
        );
    }

    #[test]
    fn create_over_existing_rejected() {
        assert_eq!(
            validate(&create("a.txt", "new\n"), Some(b"old\n")),
            Err(ValidationError::FileExists {
                path: "a.txt".to_owned()
            })
        );
    }

    #[test]
    fn update_of_missing_rejected() {
        assert_eq!(
            validate(&update("a.txt", "new\n"), None),
            Err(ValidationError::MissingFile {
                path: "a.txt".to_owned()
            })
        );
    }
}
