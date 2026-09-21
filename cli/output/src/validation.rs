//! Protocol-shape validation for the `dx` CLI.
//!
//! Split from `super` (`lib.rs`): owns [`OutputError`], [`Edit`],
//! [`check_path`], [`parse_digest`], and [`check_edits`]. Re-exported
//! through `super` so the public path stays `dx_output::{...}`.

/// Output or protocol failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OutputError {
    #[error("unknown output mode {value:?}")]
    UnknownOutputMode { value: String },
    /// A stdout report combined with `--output diff` or `--output json`.
    #[error("conflicting stdout report with {mode} mode: reports must target files")]
    ConflictingStdoutReport { mode: &'static str },
    /// More than one standard report targets stdout.
    #[error("second stdout report: at most one report may target stdout")]
    SecondStdoutReport,
    #[error("empty {field}: want a non-empty value")]
    EmptyField { field: &'static str },
    /// `command_started` mode outside `default|check`.
    #[error("invalid command mode {value:?}: want default or check")]
    BadCommandMode { value: String },
    #[error("invalid severity {value:?}")]
    BadSeverity { value: String },
    #[error("invalid threshold {value:?}")]
    BadThreshold { value: String },
    #[error("invalid path {path:?}: {reason}")]
    BadPath { path: String, reason: &'static str },
    #[error("invalid digest for {field} {value:?}")]
    BadDigest { field: &'static str, value: String },
    #[error("invalid edit at index {index}: {reason}")]
    BadEdit { index: usize, reason: &'static str },
    #[error("range without path: ranges require a file path")]
    RangeWithoutPath,
    #[error("inverted range: start must not exceed end")]
    InvertedRange,
    /// A default-mode initial diagnostic without its required resolution.
    #[error("missing resolution: default-mode diagnostics require a resolution")]
    MissingResolution,
    /// A check-mode or terminal diagnostic carrying a resolution.
    #[error("unexpected resolution: check-mode diagnostics carry no resolution")]
    UnexpectedResolution,
    /// A mutation without its required failure reason, or an applied
    /// mutation carrying one.
    #[error("missing failure reason for {path:?}")]
    MissingReason { path: String },
    #[error("unexpected failure reason for {path:?}: applied mutations carry no reason")]
    UnexpectedReason { path: String },
    /// A value that is not an NDJSON event object.
    #[error("not an event: value is not an NDJSON event object")]
    NotAnEvent,
    #[error("I/O error: {0}")]
    Io(String),
    /// Invalid `correlation` grouping identifier for interleaved operations.
    #[error("invalid correlation {value:?}: want 1-128 chars of [A-Za-z0-9/_:.-]")]
    BadCorrelation { value: String },
}

/// Validates a normalized workspace-relative source path: valid UTF-8,
/// slash-separated, non-empty, lexical, no `.` or `..` component, beneath
/// the main workspace. Mirrors the result-protocol path rules so JSON-shape
/// failures surface before diff output or mutation planning.
pub fn check_path(path: &str) -> Result<(), OutputError> {
    // Ladder order and messages mirror `dx_path::classify` one-to-one;
    // only the error payload stays crate-local (slice 5).
    let reason = match dx_path::classify(path) {
        None => None,
        Some(dx_path::PathProblem::Empty) => Some("path must be non-empty"),
        Some(dx_path::PathProblem::Absolute) => {
            Some("path must be workspace-relative, not absolute")
        }
        Some(dx_path::PathProblem::Backslash) => Some("path must use forward slashes"),
        Some(dx_path::PathProblem::EmptyComponent) => Some("path must have no empty component"),
        Some(dx_path::PathProblem::Dot) => Some("path must have no '.' component"),
        Some(dx_path::PathProblem::DotDot) => Some("path must have no '..' component"),
    };
    match reason {
        Some(reason) => Err(OutputError::BadPath {
            path: path.to_owned(),
            reason,
        }),
        None => Ok(()),
    }
}

/// Parses exactly 64 lowercase hexadecimal characters encoding a
/// BLAKE3-256 digest, used for change source digests and selection IDs.
/// Spelling owned by `dx_digest`.
pub fn parse_digest(field: &'static str, text: &str) -> Result<[u8; 32], OutputError> {
    dx_digest::parse_hex(text).map_err(|_| OutputError::BadDigest {
        field,
        value: text.to_owned(),
    })
}

/// Validates an optional NDJSON `correlation` grouping identifier for
/// interleaved operations: 1-128 ASCII chars from `[A-Za-z0-9/_:.-]`.
/// Empty and oversized values fail; consumers tolerate absence.
/// See: `docs/cli/output-protocol.md#ndjson-envelope`.
pub fn check_correlation(value: &str) -> Result<(), OutputError> {
    if value.is_empty() || value.len() > 128 {
        return Err(OutputError::BadCorrelation {
            value: value.to_owned(),
        });
    }
    let ok = value.bytes().all(|b| {
        b.is_ascii_alphanumeric() || b == b'/' || b == b'_' || b == b':' || b == b'.' || b == b'-'
    });
    if !ok {
        return Err(OutputError::BadCorrelation {
            value: value.to_owned(),
        });
    }
    Ok(())
}

/// One exact replacement edit: a half-open UTF-8 byte range in the
/// original file plus its exact new UTF-8 content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub start: u64,
    pub end: u64,
    pub replacement: String,
}

/// Validates edit shape without source bytes: the set is non-empty, in
/// strictly increasing `start` order with distinct starts, every
/// `start <= end`, and no insertion sits inside a replaced range
/// (`prev.end <= next.start`). No-op edits, UTF-8 boundaries, and digest
/// agreement need source bytes and are validated during apply.
pub fn check_edits(edits: &[Edit]) -> Result<(), OutputError> {
    if edits.is_empty() {
        return Err(OutputError::BadEdit {
            index: 0,
            reason: "edit set must be non-empty",
        });
    }
    for (index, edit) in edits.iter().enumerate() {
        if edit.start > edit.end {
            return Err(OutputError::BadEdit {
                index,
                reason: "start must not exceed end",
            });
        }
        if index > 0 {
            let prev = &edits[index - 1];
            if edit.start <= prev.start {
                return Err(OutputError::BadEdit {
                    index,
                    reason: "edits must be in strictly increasing start order",
                });
            }
            if prev.end > edit.start {
                return Err(OutputError::BadEdit {
                    index,
                    reason: "an insertion cannot sit inside a replaced range",
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn paths_mirror_result_protocol_rules() {
        check_path("src/app.py").expect("valid");
        assert!(check_path("").is_err());
        assert!(check_path("/abs").is_err());
        assert!(check_path("a\\b").is_err());
        assert!(check_path("a//b").is_err());
        assert!(check_path("./a").is_err());
        assert!(check_path("a/../b").is_err());
    }

    #[test]
    fn digests_require_lowercase_hex() {
        assert_eq!(parse_digest("d", DIGEST).expect("valid").len(), 32);
        assert!(parse_digest("d", &DIGEST.to_uppercase()).is_err());
        assert!(parse_digest("d", "0123").is_err());
        assert!(parse_digest("d", &format!("{DIGEST}00")).is_err());
        assert!(parse_digest("d", &"zz".repeat(32)).is_err());
    }

    #[test]
    fn edit_shapes_validated() {
        let edits = vec![
            Edit {
                start: 0,
                end: 5,
                replacement: String::new(),
            },
            Edit {
                start: 5,
                end: 5,
                replacement: "x".to_owned(),
            },
        ];
        check_edits(&edits).expect("adjacent ok");
        assert!(check_edits(&[]).is_err());
        assert!(check_edits(&[Edit {
            start: 9,
            end: 3,
            replacement: String::new(),
        }])
        .is_err());
        // Shared start offsets are invalid even for pure insertions.
        assert!(check_edits(&[
            Edit {
                start: 4,
                end: 4,
                replacement: "a".to_owned(),
            },
            Edit {
                start: 4,
                end: 4,
                replacement: "b".to_owned(),
            },
        ])
        .is_err());
        // An insertion inside a replaced range is invalid.
        assert!(check_edits(&[
            Edit {
                start: 2,
                end: 8,
                replacement: "a".to_owned(),
            },
            Edit {
                start: 5,
                end: 5,
                replacement: "b".to_owned(),
            },
        ])
        .is_err());
    }

    #[test]
    fn display_is_human_readable() {
        assert_eq!(
            OutputError::SecondStdoutReport.to_string(),
            "second stdout report: at most one report may target stdout"
        );
        assert_eq!(
            OutputError::MissingResolution.to_string(),
            "missing resolution: default-mode diagnostics require a resolution"
        );
    }

    #[test]
    fn correlation_shape_validated() {
        // See: `docs/cli/output-protocol.md#ndjson-envelope`.
        check_correlation("update:cargo").expect("namespaced");
        check_correlation("run//app:bin").expect("target scope");
        check_correlation("check/format").expect("umbrella phase");
        assert!(check_correlation("").is_err());
        assert!(check_correlation("has space").is_err());
        assert!(check_correlation(&"x".repeat(129)).is_err());
        assert_eq!(
            check_correlation("bad!").expect_err("punctuation"),
            OutputError::BadCorrelation {
                value: "bad!".to_owned()
            }
        );
    }
}
