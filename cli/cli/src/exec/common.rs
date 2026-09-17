//! Shared execution plumbing for every `dx` command family: stable error codes, the execution environment, source verification, and mutation helpers.
//!
//! BEP results collection and proto mapping live in [`super::results`]
//! (issue #236); this module keeps the environment, codes, and
//! apply/status helpers.

use crate::args::Invocation;
use crate::resolve::QueryRunner;
use dx_bep::ArtifactReader;
use dx_digest::blake3 as digest;
use dx_output::{
    command_finished, write_event, ChangeEvent, ChangeKind, DiagnosticEvent, FinishedCounts,
    OutputMode,
};
use dx_process::{operational_code, pre_exec_code, Runner};
use std::io::{self, Write};
use std::path::Path;

/// Stable per-file reason: the workspace source changed or vanished
/// after analysis, so recorded byte ranges no longer apply.
pub const REASON_STALE_SOURCE: &str = "stale_source";
/// Stable per-file reason: the workspace source cannot be read.
pub const REASON_UNREADABLE_SOURCE: &str = "unreadable_source";
/// Stable per-file reason: replacement bytes cannot be applied to the
/// verified source bytes.
pub const REASON_INVALID_EDITS: &str = "invalid_edits";
/// Stable per-file reason: incomplete collection prevents all mutation.
pub const REASON_INCOMPLETE_COLLECTION: &str = "incomplete_collection";

/// Execution helper failure (issue #230).
///
/// Variants render the legacy reason strings verbatim so operational
/// diagnostics stay byte-identical while callers gain a matchable type.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ExecError {
    /// Replacement bytes are not valid UTF-8.
    #[error("invalid_edits")]
    InvalidEdits,
    /// Generated logical path is empty.
    #[error("generated logical path is empty")]
    EmptyLogicalPath,
    /// Generated logical path is absolute.
    #[error("generated logical path {path:?} is absolute")]
    AbsoluteLogicalPath { path: String },
    /// Generated logical path escapes its generation.
    #[error("generated logical path {path:?} escapes its generation")]
    EscapingLogicalPath { path: String },
    /// Env identity key is empty.
    #[error("env identity key is empty")]
    EmptyEnvKey,
    /// Env identity key is not a single filename.
    #[error("env identity key {key:?} is not a single filename")]
    BadEnvKey { key: String },
}

/// Stable operational error codes for NDJSON `error` events.
pub(crate) const CODE_LAUNCH_FAILED: &str = "launch_failed";
pub(crate) const CODE_BAZEL_SIGNALLED: &str = "bazel_signalled";
pub(crate) const CODE_UNREADABLE_BEP: &str = "unreadable_bep";
pub(crate) const CODE_INVALID_BEP: &str = "invalid_bep";
pub(crate) const CODE_DIFF_FAILED: &str = "diff_failed";
pub(crate) const CODE_REPORT_FAILED: &str = "report_failed";
pub(crate) const CODE_COVERAGE_BELOW_MINIMUM: &str = "coverage_below_minimum";
/// Stable operational error code for managed-state cleanup failures:
/// a malformed current selection, commit-lock contention, or a prune
/// mutation error all fail closed with nothing adopted or repaired.
pub(crate) const CODE_CLEAN_FAILED: &str = "clean_failed";
/// Stable operational error code for well-formed requests the current
/// result transport cannot serve: a missing, unreadable, or
/// contradictory generation manifest fails closed with no change or
/// mutation output.
pub(crate) const CODE_INVALID_RESULT: &str = "invalid_result";
/// Stable operational error code for managed-state commit failures:
/// lock contention, malformed current selection, record mismatch, or a
/// generation staging mutation error all fail closed with the prior
/// pointer preserved.
pub(crate) const CODE_MANAGED_COMMIT_FAILED: &str = "managed_commit_failed";
/// Stable operational error code for exact setup scopes that provide
/// neither environment nor codegen capability: committing an empty or
/// recycled pair would hide the usage error.
pub(crate) const CODE_MANAGED_NO_CAPABILITY: &str = "no_capability";
/// Stable operational error code for live audit runs while auditor
/// wiring stays deferred: advisory acquisition, tool execution, and
/// SARIF mapping land in later M26 slices (O11/O58). Planning
/// (`--dry-run`) succeeds; live execution fails closed.
pub(crate) const CODE_AUDIT_DEFERRED: &str = "audit_deferred";
/// Stable operational error code for live update runs while resolver
/// backends stay deferred: dependency-set execution and per-set
/// reporting land in later M26 slices (O12). Planning (`--dry-run`)
/// succeeds; live execution fails closed.
pub(crate) const CODE_UPDATE_DEFERRED: &str = "update_deferred";

/// Execution environment: resolved workspace, process seams for the
/// workflow and for ownership queries, temporary directory for the BEP
/// stream, owned output streams, and the CI refusal bit for the
/// local-only `dx run` gate. `ci` is resolved once at process startup
/// from the `CI` environment variable so unit tests never mutate
/// shared process state (parallel test threads would otherwise race
/// on `set_var`).
pub struct Env<'a> {
    pub workspace: &'a Path,
    pub runner: &'a dyn Runner,
    pub query_runner: &'a dyn QueryRunner,
    pub temp_dir: &'a Path,
    pub pid: u32,
    pub nonce: u64,
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
    pub ci: bool,
}

/// Reads reported artifact bytes from the local filesystem. Bazel owns
/// remote materialization; this seam performs no network fetch.
pub(crate) struct FsArtifacts;

impl ArtifactReader for FsArtifacts {
    fn read_artifact(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }
}

/// One validated replacement set against verified original bytes.
pub(crate) struct FileChange {
    pub(crate) path: String,
    pub(crate) original_digest: [u8; 32],
    pub(crate) edits: Vec<(u64, u64, Vec<u8>)>,
}

/// Lowercase hexadecimal over the 32 raw digest bytes (owned by `dx_digest`).
pub(crate) fn hex_digest(bytes: &[u8; 32]) -> String {
    dx_digest::to_hex(bytes)
}

pub(crate) enum SourceRead {
    Bytes(Vec<u8>),
    Unreadable,
    Stale,
}

pub(crate) fn read_verified(workspace: &Path, path: &str, expected: &[u8; 32]) -> SourceRead {
    match std::fs::read(workspace.join(path)) {
        Err(_) => SourceRead::Unreadable,
        Ok(bytes) => {
            if digest(&bytes) == *expected {
                SourceRead::Bytes(bytes)
            } else {
                SourceRead::Stale
            }
        }
    }
}

/// Applies validated edits to verified original bytes. Order,
/// non-overlap, and UTF-8 boundaries are rechecked against the source
/// bytes; any violation fails the file without writing.
pub(crate) fn apply_to_bytes(original: &[u8], edits: &[(u64, u64, Vec<u8>)]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(original).ok()?;
    let mut candidate = Vec::with_capacity(original.len());
    let mut cursor = 0usize;
    for (start, end, replacement) in edits {
        let (start, end) = (*start as usize, *end as usize);
        if start < cursor || start > end || end > original.len() {
            return None;
        }
        if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            return None;
        }
        let head = &original[cursor..start];
        if std::str::from_utf8(head).is_err() {
            return None; // LCOV_EXCL_LINE - reason: defense-in-depth; the whole-text UTF-8 check above accepts only valid UTF-8 sources, so no slice of it can fail; covered logically by the invalid-original test.
        }
        if std::str::from_utf8(replacement).is_err() {
            return None;
        }
        candidate.extend_from_slice(head);
        candidate.extend_from_slice(replacement);
        cursor = end;
    }
    let tail = &original[cursor..];
    if std::str::from_utf8(tail).is_err() {
        return None; // LCOV_EXCL_LINE - reason: defense-in-depth; the whole-text UTF-8 check above accepts only valid UTF-8 sources, so the tail cannot fail; covered logically by the invalid-original test.
    }
    candidate.extend_from_slice(tail);
    Some(candidate)
}

/// One human text line for a status diagnostic. Text is concise and
/// task-oriented, never a machine-stable grammar.
pub(crate) fn text_diagnostic(diagnostic: &DiagnosticEvent) -> String {
    let mut line = format!("{} ", diagnostic.severity.name());
    if let Some(path) = &diagnostic.path {
        line.push_str(path);
        line.push_str(": ");
    }
    line.push_str(&diagnostic.message);
    line.push_str(" [");
    line.push_str(&diagnostic.tool);
    if let Some(rule) = &diagnostic.rule {
        line.push('/');
        line.push_str(rule);
    }
    line.push(']');
    if diagnostic.fixable {
        line.push_str(" (fixable)");
    }
    line
}

/// Pre-execution usage failure: stderr only, exit code 2.
pub(crate) fn pre_exec(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "dx: {message}");
    let _ = writeln!(
        err,
        "usage: dx [--workspace DIR] [--dry-run] [--quiet] [--verbose] [--output text|diff|json] [--report <format>=<destination>]... [--fail-on info|warning|error] [--min-coverage 0-100 (coverage only)] <audit|lint|typecheck|format|generate|build|test|coverage|run|deploy|check|fix|clean|update|codegen|env|setup|init|hooks|status|version|watch|owners|deps|why|completion|bazel> [--check] [scope ...] [-- command-options...]"
    );
    pre_exec_code()
}

/// Operational failure after planning: stderr diagnostic, JSON
/// `error` and `command_finished` events in JSON mode, exit code 1.
pub(crate) fn operational(
    invocation: &Invocation,
    out: &mut dyn Write,
    err: &mut dyn Write,
    code: &str,
    message: &str,
) -> i32 {
    let _ = writeln!(err, "dx: {code}: {message}");
    if invocation.output == OutputMode::Json {
        if let Ok(event) = dx_output::error_event(code, message, None, None, None) {
            let _ = write_event(out, &event);
        }
        let finished = command_finished(
            operational_code(),
            &FinishedCounts {
                results_complete: Some(false),
                ..FinishedCounts::default()
            },
        );
        let _ = write_event(out, &finished);
    }
    operational_code()
}

pub(crate) fn change_event_for(change: &FileChange) -> Result<ChangeEvent, ExecError> {
    let mut edits = Vec::with_capacity(change.edits.len());
    for (start, end, replacement) in &change.edits {
        let replacement =
            String::from_utf8(replacement.clone()).map_err(|_| ExecError::InvalidEdits)?;
        edits.push(dx_output::Edit {
            start: *start,
            end: *end,
            replacement,
        });
    }
    Ok(ChangeEvent {
        path: change.path.clone(),
        kind: ChangeKind::Modify,
        source_digest: Some(hex_digest(&change.original_digest)),
        edits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dx_output::{Severity, Snapshot};

    #[test]
    fn hex_digest_formats_lowercase_hex() {
        assert_eq!(hex_digest(&[0xabu8; 32]), "ab".repeat(32));
        assert_eq!(hex_digest(&[0u8; 32]).len(), 64);
    }

    #[test]
    fn apply_rejects_overlapping_edits_without_writing() {
        let original = b"abcdef";
        assert!(apply_to_bytes(original, &[(0, 2, b"AB".to_vec())]).is_some());
        assert!(
            apply_to_bytes(original, &[(0, 2, b"AB".to_vec()), (1, 3, b"X".to_vec())]).is_none()
        );
        assert!(
            apply_to_bytes(original, &[(0, 6, b"AB".to_vec()), (6, 7, b"X".to_vec())]).is_none()
        );
        assert!(apply_to_bytes(b"\xff\xfe", &[(0, 1, b"a".to_vec())]).is_none());
    }

    #[test]
    fn text_diagnostic_renders_identity() {
        let mut diagnostic = DiagnosticEvent {
            severity: Severity::Warning,
            tool: "lint-tool".to_owned(),
            message: "mapped".to_owned(),
            rule: Some("lint-tool/rule".to_owned()),
            path: Some("src/a.py".to_owned()),
            range: None,
            snapshot: Snapshot::Terminal,
            fixable: true,
            resolution: None,
        };
        assert_eq!(
            text_diagnostic(&diagnostic),
            "warning src/a.py: mapped [lint-tool/lint-tool/rule] (fixable)"
        );
        diagnostic.path = None;
        diagnostic.rule = None;
        diagnostic.fixable = false;
        assert_eq!(text_diagnostic(&diagnostic), "warning mapped [lint-tool]");
    }

    #[test]
    fn apply_rejects_boundary_and_encoding_violations() {
        // Splitting the two-byte é (bytes 1..3 of "héllo") is rejected.
        assert!(apply_to_bytes("héllo".as_bytes(), &[(2, 3, b"X".to_vec())]).is_none());
        assert!(apply_to_bytes("héllo".as_bytes(), &[(1, 2, b"X".to_vec())]).is_none());
        assert!(apply_to_bytes(b"ab", &[(0, 1, b"\xff".to_vec())]).is_none());
        assert_eq!(
            apply_to_bytes(b"ab", &[(0, 1, b"X".to_vec())]),
            Some(b"Xb".to_vec())
        );
    }

    #[test]
    fn change_event_is_deterministic_and_reconstructs_candidate() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // one deterministic exact change event per valid candidate path,
        // reconstructable from digest plus UTF-8 ranges and replacements
        // with byte-for-byte equality to default mode planned input.
        let original = b"BAD\n";
        let terminal = b"GOOD\n";
        let change = FileChange {
            path: "src/lib.rs".to_owned(),
            original_digest: digest(original),
            edits: vec![(0, original.len() as u64, terminal.to_vec())],
        };
        let first = change_event_for(&change).expect("change event");
        let second = change_event_for(&change).expect("change event");
        assert_eq!(first.path, "src/lib.rs");
        assert_eq!(first.path, second.path);
        assert_eq!(first.source_digest, second.source_digest);
        assert_eq!(first.source_digest, Some(hex_digest(&digest(original))));
        assert_eq!(first.edits.len(), 1);
        assert_eq!(first.edits[0].start, 0);
        assert_eq!(first.edits[0].end, original.len() as u64);
        assert_eq!(first.edits[0].replacement, "GOOD\n");
        let planned = apply_to_bytes(original, &change.edits).expect("apply");
        assert_eq!(planned, terminal);
        let other = FileChange {
            path: "src/other.rs".to_owned(),
            original_digest: digest(original),
            edits: vec![(0, original.len() as u64, terminal.to_vec())],
        };
        let other_event = change_event_for(&other).expect("other event");
        assert_ne!(first.path, other_event.path);
    }
}
