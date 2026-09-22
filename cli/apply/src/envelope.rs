//! Typed JSON envelope for file-mutating operations.
//!
//! Agents propose mutations as an [`Envelope`]: a versioned list of
//! [`FileOperation`] values carrying full new file contents plus the SHA-256
//! the agent saw when it read the file (`None` means "create; the file must
//! not exist"). Parsing rejects unknown keys, wrong versions, empty
//! operation lists, and malformed digests so malformed agent output fails
//! closed before anything touches the filesystem.

use serde::{Deserialize, Serialize};

/// Envelope schema version. Parsers accept exactly this version.
pub const ENVELOPE_VERSION: u32 = 1;

/// A single file mutation: full new content plus the digest the proposer
/// based its edit on (`None` = create).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileOperation {
    /// Workspace-relative path (validated by [`crate::validators`]).
    pub path: String,
    /// Expected SHA-256 (lowercase hex) of the current file bytes.
    pub original_sha256: Option<String>,
    /// Full new file content.
    pub content: String,
}

/// Versioned list of file mutations proposed as one unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    /// Must equal [`ENVELOPE_VERSION`].
    pub version: u32,
    /// At least one operation; order is application order.
    pub operations: Vec<FileOperation>,
}

/// Envelope parse/structural failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EnvelopeError {
    /// Not valid JSON, or a required key is missing, or an unknown key or
    /// mistyped value is present.
    #[error("invalid envelope JSON: {0}")]
    InvalidJson(String),
    /// `version` is not [`ENVELOPE_VERSION`].
    #[error("unsupported envelope version: {got}")]
    UnsupportedVersion { got: u32 },
    /// `operations` is empty.
    #[error("envelope has no operations")]
    EmptyOperations,
    /// An operation path is empty.
    #[error("envelope operation has empty path")]
    EmptyPath,
    /// The envelope value cannot be represented as JSON. Unreachable for
    /// well-typed values (serialization only fails on maps with
    /// non-string keys, which this schema has none of); kept as a
    /// `Result` so library callers never panic on serialization.
    #[error("envelope is not serializable: {0}")]
    Unserializable(String),
    /// `original_sha256` is present but not 64 lowercase hex digits.
    #[error("invalid digest: {path}")]
    InvalidDigest { path: String },
}

/// Parses and structurally validates an envelope document.
pub fn parse_envelope(json: &str) -> Result<Envelope, EnvelopeError> {
    let envelope: Envelope =
        serde_json::from_str(json).map_err(|err| EnvelopeError::InvalidJson(err.to_string()))?;
    if envelope.version != ENVELOPE_VERSION {
        return Err(EnvelopeError::UnsupportedVersion {
            got: envelope.version,
        });
    }
    if envelope.operations.is_empty() {
        return Err(EnvelopeError::EmptyOperations);
    }
    for op in &envelope.operations {
        if op.path.is_empty() {
            return Err(EnvelopeError::EmptyPath);
        }
        if let Some(digest) = &op.original_sha256 {
            if !is_sha256_hex(digest) {
                return Err(EnvelopeError::InvalidDigest {
                    path: op.path.clone(),
                });
            }
        }
    }
    Ok(envelope)
}

/// Serializes an envelope to its canonical compact JSON form.
pub fn emit_envelope(envelope: &Envelope) -> Result<String, EnvelopeError> {
    // LCOV_EXCL_START - policy: docs/testing/README.md#coverage
    serde_json::to_string(envelope).map_err(|err| EnvelopeError::Unserializable(err.to_string()))
    // LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage
}

/// Lowercase hex SHA-256 of `bytes` (frozen envelope contract:
/// bytes owned by `dx_digest` compat shim).
pub fn sha256_hex(bytes: &[u8]) -> String {
    dx_digest::sha256_hex(bytes)
}

/// True for exactly 64 lowercase hex digits (the [`sha256_hex`] output form).
/// Decode round-trip: `hex` accepts any even-length hex (including
/// uppercase), so the re-encode comparison is what pins the lowercase-only,
/// 32-byte form instead of re-implementing the digit loop.
pub fn is_sha256_hex(text: &str) -> bool {
    dx_digest::is_sha256_hex(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Envelope {
        Envelope {
            version: ENVELOPE_VERSION,
            operations: vec![FileOperation {
                path: "a.txt".to_owned(),
                original_sha256: Some(sha256_hex(b"old\n")),
                content: "new\n".to_owned(),
            }],
        }
    }

    #[test]
    fn round_trip_parse_emit() -> Result<(), EnvelopeError> {
        let envelope = sample();
        let json = emit_envelope(&envelope)?;
        assert_eq!(parse_envelope(&json), Ok(envelope));
        Ok(())
    }

    #[test]
    fn create_without_digest_parses() -> Result<(), EnvelopeError> {
        let envelope = Envelope {
            version: ENVELOPE_VERSION,
            operations: vec![FileOperation {
                path: "new.txt".to_owned(),
                original_sha256: None,
                content: "hi\n".to_owned(),
            }],
        };
        let json = emit_envelope(&envelope)?;
        assert_eq!(parse_envelope(&json), Ok(envelope));
        Ok(())
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn missing_required_key_fails() {
        let err = parse_envelope(r#"{"version":1}"#).expect_err("operations is required");
        assert!(matches!(err, EnvelopeError::InvalidJson(_)));
    }

    #[test]
    fn unknown_key_fails() {
        let err = parse_envelope(r#"{"version":1,"operations":[],"extra":true}"#)
            .expect_err("unknown keys are rejected");
        assert!(matches!(err, EnvelopeError::InvalidJson(_)));
    }

    #[test]
    fn wrong_version_fails() {
        assert_eq!(
            parse_envelope(r#"{"version":2,"operations":[]}"#),
            Err(EnvelopeError::UnsupportedVersion { got: 2 })
        );
    }

    #[test]
    fn empty_operations_fail() {
        assert_eq!(
            parse_envelope(r#"{"version":1,"operations":[]}"#),
            Err(EnvelopeError::EmptyOperations)
        );
    }

    #[test]
    fn malformed_digest_fails() {
        let json = r#"{"version":1,"operations":[{"path":"a.txt","original_sha256":"xyz","content":"x"}]}"#;
        assert_eq!(
            parse_envelope(json),
            Err(EnvelopeError::InvalidDigest {
                path: "a.txt".to_owned()
            })
        );
    }

    #[test]
    fn uppercase_digest_fails() {
        let digest = sha256_hex(b"old\n").to_uppercase();
        let json = format!(
            r#"{{"version":1,"operations":[{{"path":"a.txt","original_sha256":"{digest}","content":"x"}}]}}"#
        );
        assert_eq!(
            parse_envelope(&json),
            Err(EnvelopeError::InvalidDigest {
                path: "a.txt".to_owned()
            })
        );
    }

    #[test]
    fn empty_path_fails() {
        let json = r#"{"version":1,"operations":[{"path":"","content":"x"}]}"#;
        assert_eq!(parse_envelope(json), Err(EnvelopeError::EmptyPath));
    }
}
