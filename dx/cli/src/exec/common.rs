//! Shared execution plumbing for every `dx` command family: stable error codes, the execution environment, BEP collection, and mutation helpers.

use std::collections::{BTreeMap, BTreeSet};

use crate::args::Invocation;
use crate::plan::OUTPUT_GROUP;
use crate::resolve::QueryRunner;
use dx_bep::{collect, ArtifactReader, CollectorConfig};
use dx_output::{
    command_finished, write_event, ChangeEvent, ChangeKind, DiagnosticEvent, FinishedCounts,
    OutputMode, Severity, Snapshot,
};
use dx_process::{operational_code, pre_exec_code, Runner};
use quality_result::{decode_validated, digest, proto};
use std::io::{self, BufReader, Write};
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

/// Normalized collection: current findings, validated changes, and the
/// executed tool set. `complete` is false when Bazel failed, a target
/// failed, or any result was undecodable; default mode never mutates
/// while incomplete.
pub(crate) struct Collected {
    pub(crate) tools: Vec<String>,
    pub(crate) initial: Vec<DiagnosticEvent>,
    pub(crate) terminal: Vec<DiagnosticEvent>,
    pub(crate) changes: Vec<FileChange>,
    pub(crate) terminal_digests: BTreeMap<String, [u8; 32]>,
    pub(crate) complete: bool,
}

/// Lowercase hexadecimal over the 32 raw digest bytes.
pub(crate) fn hex_digest(bytes: &[u8; 32]) -> String {
    hex::encode(bytes)
}

pub(crate) fn map_severity(value: i32) -> Option<Severity> {
    match proto::Severity::try_from(value).ok()? {
        proto::Severity::Unspecified => None,
        proto::Severity::Info => Some(Severity::Info),
        proto::Severity::Warning => Some(Severity::Warning),
        proto::Severity::Error => Some(Severity::Error),
    }
}

pub(crate) fn map_diagnostic(
    diagnostic: &proto::Diagnostic,
    snapshot: Snapshot,
) -> Option<DiagnosticEvent> {
    let range = match (diagnostic.start_byte, diagnostic.end_byte) {
        (Some(start), Some(end)) => Some((start, end)),
        (None, None) => None,
        _ => return None,
    };
    let path = (!diagnostic.path.is_empty()).then(|| diagnostic.path.clone());
    if path.is_none() && range.is_some() {
        return None;
    }
    Some(DiagnosticEvent {
        severity: map_severity(diagnostic.severity)?,
        tool: diagnostic.tool_id.clone(),
        message: diagnostic.message.clone(),
        rule: (!diagnostic.rule_id.is_empty()).then(|| diagnostic.rule_id.clone()),
        path,
        range,
        snapshot,
        fixable: diagnostic.fixable,
        resolution: None,
    })
}

pub(crate) fn map_change(change: &proto::FileEdits) -> Option<FileChange> {
    let original_digest: [u8; 32] = change.original_digest.as_slice().try_into().ok()?;
    let mut edits = Vec::with_capacity(change.edits.len());
    for edit in &change.edits {
        edits.push((edit.start_byte, edit.end_byte, edit.replacement.clone()));
    }
    Some(FileChange {
        path: change.path.clone(),
        original_digest,
        edits,
    })
}

/// Collects, decodes, and maps every `dx_results` artifact in the BEP
/// stream at `bep`. Undecodable results and failed targets mark the
/// collection incomplete while retaining validated findings.
pub(crate) fn collect_results(bep: &Path) -> Result<Collected, (String, String)> {
    collect_results_in(bep, OUTPUT_GROUP)
}

/// Collects artifacts for one output group. Split from
/// [`collect_results`] so unit tests can prove the invalid-group arm
/// without touching the pinned production group.
pub(crate) fn collect_results_in(bep: &Path, group: &str) -> Result<Collected, (String, String)> {
    let file = std::fs::File::open(bep).map_err(|err| {
        (
            CODE_UNREADABLE_BEP.to_owned(),
            format!("failed to read build events: {err}"),
        )
    })?;
    let config = CollectorConfig::new(group).map_err(|err| {
        (
            CODE_INVALID_BEP.to_owned(),
            format!("invalid BEP config: {err:?}"),
        )
    })?;
    let targets = collect(BufReader::new(file), &config, &FsArtifacts).map_err(|err| {
        (
            CODE_INVALID_BEP.to_owned(),
            format!("invalid build events: {err:?}"),
        )
    })?;
    let mut tools = BTreeSet::new();
    let mut initial = Vec::new();
    let mut terminal = Vec::new();
    let mut changes = Vec::new();
    let mut terminal_digests = BTreeMap::new();
    let mut complete = true;
    for target in &targets {
        if !target.success {
            complete = false;
            continue;
        }
        let mut staged_initial = Vec::new();
        let mut staged_terminal = Vec::new();
        let mut staged_changes = Vec::new();
        let mut staged_digests = Vec::new();
        let mut staged_tools = Vec::new();
        let mut target_ok = true;
        for artifact in &target.artifacts {
            let result = match decode_validated(&artifact.bytes) {
                Ok(result) => result,
                Err(_) => {
                    target_ok = false;
                    break;
                }
            };
            for stage in &result.stages {
                staged_tools.push(stage.tool_id.clone());
            }
            for snapshot in &result.terminal_snapshot {
                match snapshot.digest.as_slice().try_into() {
                    Ok(digest) => staged_digests.push((snapshot.path.clone(), digest)),
                    Err(_) => {
                        target_ok = false; // LCOV_EXCL_LINE - reason: defense-in-depth; quality_result validation enforces DIGEST_LEN snapshots before mapping, so this arm is unreachable via decode_validated.
                        break; // LCOV_EXCL_LINE - reason: defense-in-depth; unreachable with the line above.
                    }
                }
            }
            if !target_ok {
                break; // LCOV_EXCL_LINE - reason: defense-in-depth; the guarded mapping arm above is unreachable via decode_validated, so this break never fires.
            }
            for diagnostic in &result.initial_diagnostics {
                match map_diagnostic(diagnostic, Snapshot::Initial) {
                    Some(mapped) => staged_initial.push(mapped),
                    None => {
                        target_ok = false; // LCOV_EXCL_LINE - reason: defense-in-depth; quality_result validation enforces mappable diagnostic shapes before mapping, so this arm is unreachable via decode_validated.
                        break; // LCOV_EXCL_LINE - reason: defense-in-depth; unreachable with the line above.
                    }
                }
            }
            if !target_ok {
                break; // LCOV_EXCL_LINE - reason: defense-in-depth; the guarded initial-diagnostic mapping arm above is unreachable via decode_validated, so this break never fires.
            }
            for diagnostic in &result.terminal_diagnostics {
                match map_diagnostic(diagnostic, Snapshot::Terminal) {
                    Some(mapped) => staged_terminal.push(mapped),
                    None => {
                        target_ok = false; // LCOV_EXCL_LINE - reason: defense-in-depth; quality_result validation enforces mappable diagnostic shapes before mapping, so this arm is unreachable via decode_validated.
                        break; // LCOV_EXCL_LINE - reason: defense-in-depth; unreachable with the line above.
                    }
                }
            }
            if !target_ok {
                break; // LCOV_EXCL_LINE - reason: defense-in-depth; the guarded terminal-diagnostic mapping arm above is unreachable via decode_validated, so this break never fires.
            }
            for file in &result.replacements {
                match map_change(file) {
                    Some(mapped) => staged_changes.push(mapped),
                    None => {
                        target_ok = false; // LCOV_EXCL_LINE - reason: defense-in-depth; quality_result validation enforces DIGEST_LEN change digests before mapping, so this arm is unreachable via decode_validated.
                        break; // LCOV_EXCL_LINE - reason: defense-in-depth; unreachable with the line above.
                    }
                }
            }
            if !target_ok {
                break; // LCOV_EXCL_LINE - reason: defense-in-depth; the guarded change mapping arm above is unreachable via decode_validated, so this break never fires.
            }
        }
        if !target_ok {
            complete = false;
            continue;
        }
        tools.extend(staged_tools);
        initial.extend(staged_initial);
        terminal.extend(staged_terminal);
        changes.extend(staged_changes);
        terminal_digests.extend(staged_digests);
    }
    Ok(Collected {
        tools: tools.into_iter().collect(),
        initial,
        terminal,
        changes,
        terminal_digests,
        complete,
    })
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
        "usage: dx [--workspace DIR] [--dry-run] [--quiet] [--output text|diff|json] [--report <format>=<destination>]... [--fail-on info|warning|error] [--min-coverage 0-100 (coverage only)] <audit|lint|typecheck|format|generate|build|test|coverage|run|check|fix|clean|update|codegen|env|setup|init|hooks|status|version|watch|owners|deps|why|completion|bazel> [--check] [scope ...] [-- command-options...]"
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

pub(crate) fn change_event_for(change: &FileChange) -> Result<ChangeEvent, String> {
    let mut edits = Vec::with_capacity(change.edits.len());
    for (start, end, replacement) in &change.edits {
        let replacement =
            String::from_utf8(replacement.clone()).map_err(|_| REASON_INVALID_EDITS.to_owned())?;
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
    use super::super::test_support::*;
    use super::*;

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
    fn severity_mapping_covers_all_arms() {
        assert!(map_severity(proto::Severity::Unspecified as i32).is_none());
        assert!(matches!(
            map_severity(proto::Severity::Info as i32),
            Some(Severity::Info)
        ));
        assert!(matches!(
            map_severity(proto::Severity::Warning as i32),
            Some(Severity::Warning)
        ));
        assert!(matches!(
            map_severity(proto::Severity::Error as i32),
            Some(Severity::Error)
        ));
        assert!(map_severity(99).is_none());
    }

    #[test]
    fn diagnostic_mapping_rejects_bad_shapes() {
        let base = Harness::diagnostic("m", false);
        let mut partial = base.clone();
        partial.end_byte = None;
        assert!(map_diagnostic(&partial, Snapshot::Initial).is_none());
        let mut pathless = base.clone();
        pathless.path = String::new();
        assert!(map_diagnostic(&pathless, Snapshot::Initial).is_none());
        assert!(map_diagnostic(&pathless, Snapshot::Terminal).is_none());
        let mut bare = pathless.clone();
        bare.start_byte = None;
        bare.end_byte = None;
        assert!(map_diagnostic(&bare, Snapshot::Terminal).is_some());
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
    fn invalid_output_group_rejected() {
        let dir = temp_dir("group-tmp");
        let bep = dir.join("empty.json");
        std::fs::write(&bep, "").expect("bep");
        let Err((code, message)) = collect_results_in(&bep, "") else {
            panic!("empty output group must fail"); // LCOV_EXCL_LINE - reason: defensive test panic that never fires when the seam rejects correctly.
        };
        assert_eq!(code, CODE_INVALID_BEP);
        assert!(message.contains("invalid BEP config"));
    }
}
