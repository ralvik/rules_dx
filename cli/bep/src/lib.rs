//! Streaming Bazel Build Event Protocol collector for the `dx` CLI
//! (M06 WP2).
//!
//! Contract: `docs/quality/quality-result-protocol.md` (transport and
//! collection) and `docs/testing/environments.md` (BEP and projection
//! tests). The collector parses one newline-delimited JSON build-event
//! stream incrementally, resolves the requested output group (such as
//! `dx_results`) through BEP named sets, and reads exactly the reported
//! artifact bytes through an injected reader.
//!
//! The collector never walks `bazel-out`, never constructs artifact paths
//! from output-tree layout, and never fetches over the network: Bazel
//! materializes requested remote outputs before local collection, so a
//! non-`file://` URI (such as `bytestream://`) fails instead of triggering
//! a CLI download. Unknown event kinds are ignored for forward
//! compatibility within one stream; malformed lines, references to
//! undefined named sets, and unreadable reported files fail the whole
//! collection.
//!
//! Memory is bounded by the stream index, not by artifact contents: only
//! named-set ids with their reported URIs and one record per matching
//! completed label are retained while streaming. Artifact bytes are read
//! after the stream ends, one file at a time, so peak memory is the index
//! plus the collected result bytes. Returned records sort by label bytes
//! and artifacts sort by path bytes, so consensus never depends on BEP
//! arrival order.

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

pub mod outputs;
pub mod test_outputs;

pub use outputs::collect;
pub use test_outputs::{collect_test_outputs, TestOutputFile};

use std::path::{Path, PathBuf};

/// Reads reported artifact bytes from the local filesystem. Bazel owns
/// remote materialization; this seam performs no network fetch.
pub trait ArtifactReader {
    fn read_artifact(&self, path: &Path) -> std::io::Result<Vec<u8>>;
}

/// Which output group to collect, such as `dx_results`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorConfig {
    output_group: String,
}

impl CollectorConfig {
    pub fn new(output_group: &str) -> Result<Self, BepError> {
        if output_group.is_empty() {
            return Err(BepError::EmptyOutputGroup);
        }
        Ok(CollectorConfig {
            output_group: output_group.to_owned(),
        })
    }

    pub fn output_group(&self) -> &str {
        &self.output_group
    }
}

/// One collected artifact: the local path parsed from its reported
/// `file://` URI plus its exact bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedArtifact {
    pub exec_path: PathBuf,
    pub bytes: Vec<u8>,
}

/// One completed label in the requested output group. `success=false`
/// records a failed action whose results are unavailable; valid results
/// from other keep-going actions stay available for partial reports, but
/// no mutation is allowed until complete collection is validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetOutput {
    pub label: String,
    pub success: bool,
    pub artifacts: Vec<CollectedArtifact>,
}

/// BEP collection failure (issue #211 slice).
///
/// Every variant renders human-readable via `Display` for CLI
/// operational diagnostics; binaries render via `to_string()`, never
/// Rust `Debug`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BepError {
    /// The requested output group name is empty.
    #[error("empty output group: want a non-empty output group name")]
    EmptyOutputGroup,
    /// A stream line is not a JSON build-event object with the required
    /// shape. `reason` carries the parse or shape detail without secrets.
    #[error("malformed event at line {line}: {reason}")]
    MalformedEvent { line: u64, reason: String },
    /// A referenced artifact URI that is not a local `file://` URI, such
    /// as a remote `bytestream://` that Bazel never materialized. The CLI
    /// performs no network fetch.
    #[error("unsupported artifact URI {uri:?}: want a local file:// URI")]
    UnsupportedUri { uri: String },
    /// A completed target references a named set the stream never defined.
    /// `line` is the completion line holding the dangling reference.
    #[error("missing named set {id:?} referenced at line {line}")]
    MissingNamedSet { id: String, line: u64 },
    /// A reported local file cannot be read.
    #[error("unreadable artifact {path:?}: {message}")]
    UnreadableArtifact { path: String, message: String },
}

/// JSON-path-annotated shape failure for one BEP stream line (issue
/// #231). `path` is a JSON-pointer-style location such as
/// `id.testResult.label` or `testResult.testActionOutput[2].uri`, so
/// malformed-line diagnostics name the offending field. `detail` keeps
/// the legacy human-readable wording, so existing reason-text
/// assertions keep matching.
pub(crate) fn malformed(line: u64, path: &str, detail: &str) -> BepError {
    BepError::MalformedEvent {
        line,
        reason: format!("{path}: {detail}"),
    }
}

/// Parses a reported `file://` URI into a local path without touching the
/// filesystem. Any other scheme (notably remote `bytestream://`) fails so
/// the CLI never performs a network fetch for unmaterialized outputs.
/// `Url::parse` validates URI structure first (scheme `file`, empty or
/// `localhost` host); the path itself keeps the exact legacy byte derivation
/// with no percent-decoding, so accepted inputs resolve identically.
pub(crate) fn file_uri_to_path(uri: &str) -> Result<PathBuf, BepError> {
    let unsupported = || BepError::UnsupportedUri {
        uri: uri.to_owned(),
    };
    let parsed = url::Url::parse(uri).map_err(|_| unsupported())?;
    if parsed.scheme() != "file" {
        return Err(unsupported());
    }
    match parsed.host_str() {
        None | Some("localhost") => {}
        Some(_) => return Err(unsupported()),
    }
    let rest = uri.strip_prefix("file://").ok_or_else(unsupported)?;
    let path = match rest.strip_prefix("localhost/") {
        Some(trailing) => format!("/{trailing}"),
        None => rest.to_owned(),
    };
    if !path.starts_with('/') {
        return Err(unsupported());
    }
    Ok(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uri_forms() {
        assert_eq!(
            file_uri_to_path("file:///out/a.pb").expect("abs"),
            PathBuf::from("/out/a.pb")
        );
        assert_eq!(
            file_uri_to_path("file://localhost/out/a.pb").expect("localhost"),
            PathBuf::from("/out/a.pb")
        );
        assert!(file_uri_to_path("bytestream://x").is_err());
        assert!(file_uri_to_path("file://relative/path").is_err());
        assert!(file_uri_to_path("/plain/path").is_err());
    }
}
