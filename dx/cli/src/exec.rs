//! Quality command execution (M07 WP3).
//!
//! Contract: `docs/cli/commands/quality.md`,
//! `docs/cli/output-protocol.md`, and `docs/cli/standard-reports.md`.
//! Runs the planned Bazel workflow, collects `dx_results` through BEP,
//! projects normalized results into text, diff, NDJSON, and SARIF forms,
//! applies agreed stable candidates in default mode, and exits `0` on
//! full success, `1` on policy, operational, partial, or report
//! failures, and `2` on pre-execution usage failures.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufReader, Write};
use std::path::Path;

use crate::args::{Command, Invocation};
use crate::plan::{bep_path, plan_build, plan_run, plan_workflow, WorkflowVerb, OUTPUT_GROUP};
use crate::reports::{
    junit_infrastructure_case, parse_test_xml, plan_reports, render_junit, render_sarif,
    validate_lcov, Destination, JunitCase, PlannedReport, ReportError,
};
use crate::resolve::{resolve, resolve_for_test, resolve_run, QueryRunner, ResolveError};
use dx_apply::{FileSystem, RealFileSystem};
use dx_bep::{collect, collect_test_outputs, ArtifactReader, CollectorConfig};
use dx_diff::{render_patch, FilePatch, PatchKind};
use dx_output::{
    change_event, command_finished, command_started, diagnostic_event, meets_threshold,
    mutation_event, report_event, write_event, ChangeEvent, ChangeKind, DiagnosticEvent,
    FinishedCounts, MutationOutcome, OutputMode, Resolution, Severity, Snapshot,
};
use dx_process::{operational_code, pre_exec_code, Runner};
use quality_result::{decode_validated, digest, proto};

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
const CODE_LAUNCH_FAILED: &str = "launch_failed";
const CODE_BAZEL_SIGNALLED: &str = "bazel_signalled";
const CODE_UNREADABLE_BEP: &str = "unreadable_bep";
const CODE_INVALID_BEP: &str = "invalid_bep";
const CODE_DIFF_FAILED: &str = "diff_failed";
const CODE_REPORT_FAILED: &str = "report_failed";

/// Execution environment: resolved workspace, process seams for the
/// workflow and for ownership queries, temporary directory for the BEP
/// stream, and owned output streams.
pub struct Env<'a> {
    pub workspace: &'a Path,
    pub runner: &'a dyn Runner,
    pub query_runner: &'a dyn QueryRunner,
    pub temp_dir: &'a Path,
    pub pid: u32,
    pub nonce: u64,
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
}

/// Reads reported artifact bytes from the local filesystem. Bazel owns
/// remote materialization; this seam performs no network fetch.
struct FsArtifacts;

impl ArtifactReader for FsArtifacts {
    fn read_artifact(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }
}

/// One validated replacement set against verified original bytes.
struct FileChange {
    path: String,
    original_digest: [u8; 32],
    edits: Vec<(u64, u64, Vec<u8>)>,
}

/// Normalized collection: current findings, validated changes, and the
/// executed tool set. `complete` is false when Bazel failed, a target
/// failed, or any result was undecodable; default mode never mutates
/// while incomplete.
struct Collected {
    tools: Vec<String>,
    initial: Vec<DiagnosticEvent>,
    terminal: Vec<DiagnosticEvent>,
    changes: Vec<FileChange>,
    terminal_digests: BTreeMap<String, [u8; 32]>,
    complete: bool,
}

/// Lowercase hexadecimal over the 32 raw digest bytes.
fn hex_digest(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(64);
    for byte in bytes {
        text.push(HEX[(byte >> 4) as usize] as char);
        text.push(HEX[(byte & 0x0f) as usize] as char);
    }
    text
}

fn map_severity(value: i32) -> Option<Severity> {
    match proto::Severity::try_from(value).ok()? {
        proto::Severity::Unspecified => None,
        proto::Severity::Info => Some(Severity::Info),
        proto::Severity::Warning => Some(Severity::Warning),
        proto::Severity::Error => Some(Severity::Error),
    }
}

fn map_diagnostic(diagnostic: &proto::Diagnostic, snapshot: Snapshot) -> Option<DiagnosticEvent> {
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

fn map_change(change: &proto::FileEdits) -> Option<FileChange> {
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
fn collect_results(bep: &Path) -> Result<Collected, (String, String)> {
    collect_results_in(bep, OUTPUT_GROUP)
}

/// Collects artifacts for one output group. Split from
/// [`collect_results`] so unit tests can prove the invalid-group arm
/// without touching the pinned production group.
fn collect_results_in(bep: &Path, group: &str) -> Result<Collected, (String, String)> {
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

enum SourceRead {
    Bytes(Vec<u8>),
    Unreadable,
    Stale,
}

fn read_verified(workspace: &Path, path: &str, expected: &[u8; 32]) -> SourceRead {
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
fn apply_to_bytes(original: &[u8], edits: &[(u64, u64, Vec<u8>)]) -> Option<Vec<u8>> {
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
fn text_diagnostic(diagnostic: &DiagnosticEvent) -> String {
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
fn pre_exec(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "dx: {message}");
    let _ = writeln!(
        err,
        "usage: dx [--workspace DIR] [--dry-run] [--quiet] [--output text|diff|json] [--report <format>=<destination>]... [--fail-on info|warning|error] <lint|typecheck|format|build|test|coverage|run> [--check] [scope ...] [-- command-options...]"
    );
    pre_exec_code()
}

/// Operational failure after planning: stderr diagnostic, JSON
/// `error` and `command_finished` events in JSON mode, exit code 1.
fn operational(
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

fn change_event_for(change: &FileChange) -> Result<ChangeEvent, String> {
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

/// Runs the quality command to completion and returns the process exit
/// code. All dx-owned bytes go to `out` except operational diagnostics
/// (always `err`); diff mode never emits dx-owned prose or diagnostics
/// on stdout, and a stdout report owns stdout while human text moves
/// to stderr.
pub fn execute(invocation: &Invocation, env: Env<'_>) -> i32 {
    if invocation.command.is_workflow() {
        return execute_workflow(invocation, env);
    }
    let Env {
        workspace,
        runner,
        query_runner,
        temp_dir,
        pid,
        nonce,
        out,
        err,
    } = env;
    let planned_reports = match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(planned) => planned,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let stdout_report = planned_reports
        .iter()
        .any(|report| report.destination == Destination::Stdout);
    let bep = bep_path(temp_dir, pid, nonce);
    let Some(bep_text) = bep.to_str() else {
        return operational(
            invocation,
            out,
            err,
            CODE_UNREADABLE_BEP,
            "temporary event path is not UTF-8",
        );
    };
    let build = match resolve(&invocation.targets, workspace, query_runner)
        .map_err(|error| error.to_string())
        .and_then(|resolved| {
            plan_build(
                invocation.command,
                &resolved,
                &invocation.bazel_options,
                bep_text,
            )
            .map_err(|error| format!("{error:?}"))
        }) {
        Ok(build) => build,
        Err(message) => return pre_exec(err, &message),
    };
    let mode = if invocation.check { "check" } else { "default" };
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, mode) {
                let _ = write_event(out, &event);
            }
            let finished = command_finished(0, &FinishedCounts::default());
            let _ = write_event(out, &finished);
        } else if !matches!(invocation.output, OutputMode::Diff)
            && !matches!(invocation.output, OutputMode::Text { quiet: true })
            && !stdout_report
            && !invocation.quiet
        {
            let _ = writeln!(out, "{}", build.summary);
        }
        return 0;
    }
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), false, mode) {
            let _ = write_event(out, &event);
        }
    } else if matches!(invocation.output, OutputMode::Text { quiet: false })
        && !stdout_report
        && !invocation.quiet
    {
        let _ = writeln!(out, "{}", build.summary);
    }
    let status = match runner.run(&build.argv, workspace) {
        Ok(status) => status,
        Err(error) => {
            return operational(
                invocation,
                out,
                err,
                CODE_LAUNCH_FAILED,
                &format!("failed to launch Bazel: {error}"),
            );
        }
    };
    let Some(bazel_code) = status.code else {
        return operational(
            invocation,
            out,
            err,
            CODE_BAZEL_SIGNALLED,
            "Bazel terminated by signal",
        );
    };
    let mut collected = match collect_results(&bep) {
        Ok(collected) => collected,
        Err((code, message)) => {
            let _ = std::fs::remove_file(&bep);
            return operational(invocation, out, err, &code, &message);
        }
    };
    let _ = std::fs::remove_file(&bep);
    collected.complete = collected.complete && bazel_code == 0;
    dx_output::sort_diagnostics(&mut collected.initial);
    dx_output::sort_diagnostics(&mut collected.terminal);
    collected
        .changes
        .sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));

    let fs = RealFileSystem;
    // Verified source bytes for changed files, read before any
    // mutation while the workspace still matches the analysis.
    let mut sources: BTreeMap<String, SourceRead> = BTreeMap::new();
    for change in &collected.changes {
        sources
            .entry(change.path.clone())
            .or_insert_with(|| read_verified(workspace, &change.path, &change.original_digest));
    }

    let mut applied: BTreeMap<String, bool> = BTreeMap::new();
    let mut not_applied: Vec<(String, &'static str)> = Vec::new();
    if invocation.check {
        // Check mode never mutates; any proposed change fails the run.
    } else if !collected.complete {
        for change in &collected.changes {
            applied.insert(change.path.clone(), false);
            not_applied.push((change.path.clone(), REASON_INCOMPLETE_COLLECTION));
        }
    } else {
        for change in &collected.changes {
            let reason = match sources.get(&change.path) {
                Some(SourceRead::Bytes(original)) => {
                    match apply_to_bytes(original, &change.edits) {
                        Some(candidate) => {
                            match fs.write_atomic(&workspace.join(&change.path), &candidate) {
                                Ok(()) => {
                                    applied.insert(change.path.clone(), true);
                                    None
                                }
                                Err(_) => Some(REASON_UNREADABLE_SOURCE),
                            }
                        }
                        None => Some(REASON_INVALID_EDITS),
                    }
                }
                Some(SourceRead::Unreadable) => Some(REASON_UNREADABLE_SOURCE),
                Some(SourceRead::Stale) | None => Some(REASON_STALE_SOURCE),
            };
            if let Some(reason) = reason {
                applied.insert(change.path.clone(), false);
                not_applied.push((change.path.clone(), reason));
            }
        }
    }

    // Status findings under the command policy: every initial
    // diagnostic in check mode; terminal diagnostics for applied files
    // plus initial diagnostics for all other files in default mode.
    // Fixed initials (applied guaranteed fixes) leave all projections.
    let mut status: Vec<DiagnosticEvent> = Vec::new();
    if invocation.check {
        status.extend(collected.initial.iter().cloned());
    } else {
        status.extend(collected.terminal.iter().cloned());
        for diagnostic in &collected.initial {
            let is_applied = diagnostic
                .path
                .as_ref()
                .is_some_and(|path| applied.get(path).copied().unwrap_or(false));
            if is_applied && diagnostic.fixable {
                continue;
            }
            status.push(diagnostic.clone());
        }
        dx_output::sort_diagnostics(&mut status);
    }
    let failing = status
        .iter()
        .any(|diagnostic| meets_threshold(diagnostic.severity, invocation.fail_on));
    let mut failed = failing;
    if invocation.check && !collected.changes.is_empty() {
        failed = true;
    }

    // Projection: text lines, unified patch, or NDJSON events.
    let mut patch = String::new();
    if invocation.output == OutputMode::Diff {
        let mut owned: Vec<(String, String, String)> = Vec::with_capacity(collected.changes.len());
        for change in &collected.changes {
            let original = match sources.get(&change.path) {
                Some(SourceRead::Bytes(bytes)) => bytes,
                _ => {
                    return operational(
                        invocation,
                        out,
                        err,
                        CODE_DIFF_FAILED,
                        &format!(
                            "cannot render patch without verified source for {}",
                            change.path
                        ),
                    );
                }
            };
            let original_text = match std::str::from_utf8(original) {
                Ok(text) => text,
                Err(_) => {
                    return operational(
                        invocation,
                        out,
                        err,
                        CODE_DIFF_FAILED,
                        &format!("source for {} is not UTF-8 text", change.path),
                    );
                }
            };
            let Some(candidate) = apply_to_bytes(original, &change.edits) else {
                return operational(
                    invocation,
                    out,
                    err,
                    CODE_DIFF_FAILED,
                    &format!("cannot apply recorded edits for {}", change.path),
                );
            };
            let candidate_text = match String::from_utf8(candidate) {
                Ok(text) => text,
                // LCOV_EXCL_START - reason: apply_to_bytes only returns valid UTF-8.
                Err(_) => {
                    return operational(
                        invocation,
                        out,
                        err,
                        CODE_DIFF_FAILED,
                        &format!("candidate for {} is not UTF-8 text", change.path),
                    );
                } // LCOV_EXCL_STOP - reason: end of unreachable candidate arm.
            };
            owned.push((
                change.path.clone(),
                original_text.to_owned(),
                candidate_text,
            ));
        }
        let patches: Vec<FilePatch<'_>> = owned
            .iter()
            .map(|(path, original, candidate)| FilePatch {
                path,
                kind: PatchKind::Modify,
                original,
                candidate,
            })
            .collect();
        match render_patch(&patches) {
            Ok(rendered) => patch = rendered,
            Err(error) => {
                return operational(
                    invocation,
                    out,
                    err,
                    CODE_DIFF_FAILED,
                    &format!("failed to render patch: {error:?}"),
                );
            }
        }
    }

    // Human and machine emission of findings, changes, and mutations.
    let mut applied_count = 0u64;
    let mut not_applied_count = 0u64;
    if invocation.output == OutputMode::Json {
        let mutating = !invocation.check;
        for diagnostic in &status {
            let mut event_diagnostic = diagnostic.clone();
            if mutating && event_diagnostic.snapshot == Snapshot::Initial {
                let is_applied = event_diagnostic
                    .path
                    .as_ref()
                    .is_some_and(|path| applied.get(path).copied().unwrap_or(false));
                event_diagnostic.resolution = Some(if is_applied && event_diagnostic.fixable {
                    Resolution::Fixed // LCOV_EXCL_LINE - reason: defense-in-depth; applied fixable initials are withheld from status by the projection above, so no emitted finding can take this arm; retained for resolution completeness.
                } else if is_applied {
                    Resolution::Remaining
                } else {
                    Resolution::NotApplied
                });
            }
            match diagnostic_event(&event_diagnostic, mutating) {
                Ok(event) => {
                    let _ = write_event(out, &event);
                }
                // LCOV_EXCL_START - reason: defense-in-depth; decode_validated plus the resolution assignment above guarantee valid diagnostic_event inputs (nonempty tool/message, sound paths/ranges, consistent snapshot/resolution), so this arm is unreachable.
                Err(error) => {
                    return operational(
                        invocation,
                        out,
                        err,
                        CODE_INVALID_BEP,
                        &format!("invalid finding for output: {error:?}"),
                    );
                } // LCOV_EXCL_STOP - reason: end of unreachable defensive arm.
            }
        }
        for change in &collected.changes {
            match change_event_for(change) {
                Ok(change_event_value) => match change_event(&change_event_value) {
                    Ok(event) => {
                        let _ = write_event(out, &event);
                    }
                    // LCOV_EXCL_START - reason: defense-in-depth; decode_validated enforces the edit shape check_edits requires (non-empty, ordered, non-overlapping) and change_event_for always supplies valid hex digests, so change_event cannot fail here.
                    Err(error) => {
                        return operational(
                            invocation,
                            out,
                            err,
                            CODE_INVALID_BEP,
                            &format!("invalid change for output: {error:?}"),
                        );
                    } // LCOV_EXCL_STOP - reason: end of unreachable defensive arm.
                },
                // LCOV_EXCL_START - reason: defense-in-depth; decode_validated enforces UTF-8 replacements (InvalidUtf8Replacement), so change_event_for cannot fail on validated results.
                Err(reason) => {
                    return operational(
                        invocation,
                        out,
                        err,
                        CODE_INVALID_BEP,
                        &format!("invalid change for {}: {reason}", change.path),
                    );
                } // LCOV_EXCL_STOP - reason: end of unreachable defensive arm.
            }
        }
        if !invocation.check {
            for change in &collected.changes {
                let is_applied = applied.get(&change.path).copied().unwrap_or(false);
                let reason = if is_applied {
                    None
                } else {
                    Some(
                        not_applied
                            .iter()
                            .find(|(path, _)| path == &change.path)
                            .map(|(_, reason)| *reason)
                            .unwrap_or(REASON_INCOMPLETE_COLLECTION),
                    )
                };
                match mutation_event(
                    &change.path,
                    ChangeKind::Modify,
                    if is_applied {
                        MutationOutcome::Applied
                    } else {
                        MutationOutcome::NotApplied
                    },
                    reason,
                ) {
                    Ok(event) => {
                        if is_applied {
                            applied_count += 1;
                        } else {
                            not_applied_count += 1;
                        }
                        let _ = write_event(out, &event);
                    }
                    // LCOV_EXCL_START - reason: defense-in-depth; the mutation reason is Some exactly for NotApplied (INCOMPLETE fallback) and None for Applied by construction, so mutation_event cannot fail here.
                    Err(error) => {
                        return operational(
                            invocation,
                            out,
                            err,
                            CODE_INVALID_BEP,
                            &format!("invalid mutation for output: {error:?}"),
                        );
                    } // LCOV_EXCL_STOP - reason: end of unreachable defensive arm.
                }
            }
        }
    } else if matches!(invocation.output, OutputMode::Text { .. }) {
        let human: &mut dyn Write = if stdout_report { err } else { out };
        for diagnostic in &status {
            let _ = writeln!(human, "{}", text_diagnostic(diagnostic));
        }
        if !invocation.check {
            applied_count = applied.values().filter(|applied| **applied).count() as u64;
            not_applied_count = not_applied.len() as u64;
            if applied_count > 0 {
                let _ = writeln!(human, "Applied {applied_count} file(s).");
            }
        }
        for (path, reason) in &not_applied {
            let _ = writeln!(err, "Not applied: {path} ({reason})");
        }
    } else {
        for (path, reason) in &not_applied {
            let _ = writeln!(err, "Not applied: {path} ({reason})");
        }
        out.write_all(patch.as_bytes()).ok();
    }
    let change_count = collected.changes.len() as u64;

    // Standard reports: SARIF over current findings with snapshot
    // line regions, written atomically after validation.
    let mut reports_ok = true;
    for planned in &planned_reports {
        let mut snapshots = BTreeMap::new();
        let mut snapshot_result: Result<(), ReportError> = Ok(());
        let mut needed: BTreeSet<&str> = BTreeSet::new();
        for finding in &status {
            if let Some(path) = &finding.path {
                if finding.range.is_some() {
                    needed.insert(path.as_str());
                }
            } // LCOV_EXCL_LINE - reason: closing brace of a fully covered nesting level carries no executable region of its own.
        }
        for path in needed {
            match std::fs::read(workspace.join(path)) {
                Err(_) => {
                    snapshot_result = Err(ReportError::MissingSnapshot {
                        path: path.to_owned(),
                    });
                    break;
                }
                Ok(bytes) => {
                    if let Some(expected) = collected.terminal_digests.get(path) {
                        if digest(&bytes) != *expected {
                            snapshot_result = Err(ReportError::MissingSnapshot {
                                path: path.to_owned(),
                            });
                            break;
                        }
                    } // LCOV_EXCL_LINE - reason: closing brace of a fully covered guard carries no executable region of its own.
                    match String::from_utf8(bytes) {
                        Ok(text) => {
                            snapshots.insert(path.to_owned(), text);
                        }
                        Err(_) => {
                            snapshot_result = Err(ReportError::MissingSnapshot {
                                path: path.to_owned(),
                            });
                            break;
                        }
                    }
                }
            }
        }
        let document = match snapshot_result {
            Err(error) => Err(error),
            Ok(()) => render_sarif(&collected.tools, &status, &snapshots, collected.complete),
        };
        match document {
            Ok(document) => {
                let written = match &planned.destination {
                    Destination::Stdout => out
                        .write_all(document.as_bytes())
                        .and_then(|()| out.write_all(b"\n"))
                        .is_ok(),
                    Destination::File(destination) => {
                        let target = workspace.join(destination);
                        let parent_ok = target
                            .parent()
                            .is_none_or(|parent| parent.as_os_str().is_empty() || parent.is_dir());
                        parent_ok && fs.write_atomic(&target, document.as_bytes()).is_ok()
                    }
                };
                if !written {
                    reports_ok = false;
                    let detail = format!(
                        "failed to write {} report to {}",
                        planned.format.name(),
                        planned.destination.display()
                    );
                    let _ = writeln!(err, "dx: report_failed: {detail}");
                    if invocation.output == OutputMode::Json {
                        if let Ok(event) =
                            dx_output::error_event(CODE_REPORT_FAILED, &detail, None, None, None)
                        {
                            let _ = write_event(out, &event);
                        }
                    } // LCOV_EXCL_LINE - reason: closing brace of a fully covered error-reporting guard carries no executable region of its own.
                    continue;
                }
                if invocation.output == OutputMode::Json {
                    if let Ok(event) = report_event(
                        planned.format.name(),
                        planned.destination.display(),
                        collected.complete,
                    ) {
                        let _ = write_event(out, &event);
                    }
                } else if matches!(invocation.output, OutputMode::Text { .. }) && !stdout_report {
                    let _ = writeln!(
                        out,
                        "Wrote {} report to {}.",
                        planned.format.name(),
                        planned.destination.display()
                    );
                } else if invocation.output == OutputMode::Diff {
                    let _ = writeln!(
                        err,
                        "Wrote {} report to {}.",
                        planned.format.name(),
                        planned.destination.display()
                    );
                }
            }
            Err(error) => {
                reports_ok = false;
                let detail = format!("failed to render SARIF report: {error}");
                let _ = writeln!(err, "dx: report_failed: {detail}");
                if invocation.output == OutputMode::Json {
                    if let Ok(event) =
                        dx_output::error_event(CODE_REPORT_FAILED, &detail, None, None, None)
                    {
                        let _ = write_event(out, &event);
                    }
                }
            }
        }
    }

    // Counts: diagnostics over emitted status findings; changes over
    // validated change records; mutations in default mode only.
    let mut info = 0u64;
    let mut warning = 0u64;
    let mut error = 0u64;
    for diagnostic in &status {
        match diagnostic.severity {
            Severity::Info => info += 1,
            Severity::Warning => warning += 1,
            Severity::Error => error += 1,
        }
    }
    if invocation.output == OutputMode::Json {
        let finished = command_finished(
            if collected.complete && !failed && reports_ok {
                0
            } else {
                1
            },
            &FinishedCounts {
                results_complete: Some(collected.complete),
                diagnostics: Some([info, warning, error]),
                changes: Some([0, change_count]),
                mutations: if invocation.check {
                    None
                } else {
                    Some([applied_count, not_applied_count])
                },
            },
        );
        let _ = write_event(out, &finished);
    }
    if collected.complete && !failed && reports_ok {
        0
    } else {
        1
    }
}

/// Stable operational codes for workflow failures.
const CODE_NO_RUNNABLE: &str = "no_runnable";
const CODE_AMBIGUOUS_RUNNABLE: &str = "ambiguous_runnable";
const CODE_NO_TESTS: &str = "no_tests";

fn resolve_code(error: &ResolveError) -> &'static str {
    match error {
        ResolveError::NoRunnable { .. } => CODE_NO_RUNNABLE,
        ResolveError::AmbiguousRunnable { .. } => CODE_AMBIGUOUS_RUNNABLE,
        ResolveError::NoTests { .. } => CODE_NO_TESTS,
        _ => "scope_error",
    }
}

/// Workflow dispatch: `build`/`test`/`coverage` preserve Bazel status
/// with JUnit/LCOV collection; `run` preserves the application status
/// verbatim. Pre-execution usage failures exit 2; operational failures
/// exit 1.
fn execute_workflow(invocation: &Invocation, env: Env<'_>) -> i32 {
    if invocation.command == Command::Run {
        return execute_run(invocation, env);
    }
    let Env {
        workspace,
        runner,
        query_runner,
        temp_dir,
        pid,
        nonce,
        out,
        err,
    } = env;
    let planned_reports = match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(planned) => planned,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let stdout_report = planned_reports
        .iter()
        .any(|report| report.destination == Destination::Stdout);
    let resolved = match invocation.command {
        Command::Build => resolve(&invocation.targets, workspace, query_runner),
        Command::Test | Command::Coverage => {
            resolve_for_test(&invocation.targets, workspace, query_runner)
        }
        _ => unreachable!("workflow dispatch guards commands"), // LCOV_EXCL_LINE - reason: defense-in-depth; execute routes only Build/Test/Coverage/Run here and Run returns early, so this arm is unreachable
    };
    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let verb = WorkflowVerb::of(invocation.command).expect("workflow verb");
    let bep = bep_path(temp_dir, pid, nonce);
    let bep_text = bep.to_str().map(ToString::to_string);
    let Some(bep_text) = bep_text else {
        return operational(
            invocation,
            out,
            err,
            CODE_UNREADABLE_BEP,
            "temporary event path is not UTF-8",
        );
    };
    let bep_arg = if verb.collects_reports() {
        Some(bep_text.as_str())
    } else {
        None
    };
    let plan = match plan_workflow(verb, &resolved, &invocation.bazel_options, bep_arg) {
        Ok(plan) => plan,
        Err(error) => return pre_exec(err, &format!("{error:?}")),
    };
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
                let _ = write_event(out, &event);
            }
            let _ = write_event(out, &command_finished(0, &FinishedCounts::default()));
        } else if !matches!(invocation.output, OutputMode::Diff)
            && !matches!(invocation.output, OutputMode::Text { quiet: true })
            && !stdout_report
            && !invocation.quiet
        {
            let _ = writeln!(out, "{}", plan.summary);
        }
        return 0;
    }
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), false, "default") {
            let _ = write_event(out, &event);
        }
    } else if matches!(invocation.output, OutputMode::Text { quiet: false })
        && !stdout_report
        && !invocation.quiet
    {
        let _ = writeln!(out, "{}", plan.summary);
    }
    let status = match runner.run(&plan.argv, workspace) {
        Ok(status) => status,
        Err(error) => {
            return operational(
                invocation,
                out,
                err,
                CODE_LAUNCH_FAILED,
                &format!("failed to launch Bazel: {error}"),
            );
        }
    };
    let Some(bazel_code) = status.code else {
        return operational(
            invocation,
            out,
            err,
            CODE_BAZEL_SIGNALLED,
            "Bazel terminated by signal",
        );
    };
    if verb == WorkflowVerb::Build {
        if invocation.output == OutputMode::Json {
            let _ = write_event(
                out,
                &command_finished(bazel_code, &FinishedCounts::default()),
            );
        }
        return bazel_code;
    }
    execute_test_reports(
        invocation,
        workspace,
        out,
        err,
        verb,
        &bep,
        &planned_reports,
        stdout_report,
        bazel_code,
    )
}

/// Collects BEP test outputs into JUnit suites or validated LCOV
/// bytes, renders requested reports, and selects the workflow exit
/// code: Bazel's exact nonzero code is preserved; success with
/// incomplete collection or failed reports exits 1.
#[allow(clippy::too_many_arguments)]
fn execute_test_reports(
    invocation: &Invocation,
    workspace: &Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
    verb: WorkflowVerb,
    bep: &Path,
    planned_reports: &[PlannedReport],
    stdout_report: bool,
    bazel_code: i32,
) -> i32 {
    let outputs = match std::fs::File::open(bep).map_err(|err| {
        (
            CODE_UNREADABLE_BEP.to_owned(),
            format!("failed to read build events: {err}"),
        )
    }) {
        Ok(file) => match collect_test_outputs(BufReader::new(file)) {
            Ok(outputs) => outputs,
            Err(error) => {
                let _ = std::fs::remove_file(bep);
                return operational(
                    invocation,
                    out,
                    err,
                    CODE_INVALID_BEP,
                    &format!("invalid build events: {error:?}"),
                );
            }
        },
        Err((code, message)) => {
            let _ = std::fs::remove_file(bep);
            return operational(invocation, out, err, &code, &message);
        }
    };
    let _ = std::fs::remove_file(bep);
    let reader = FsArtifacts;
    let mut complete = bazel_code == 0;
    let mut detail = String::new();
    let mut suites: Vec<(String, Vec<JunitCase>)> = Vec::new();
    let mut lcov_documents: Vec<String> = Vec::new();
    if verb == WorkflowVerb::Test {
        let mut grouped: BTreeMap<String, Vec<JunitCase>> = BTreeMap::new();
        for output in &outputs {
            if output.name != "test.xml" {
                continue;
            }
            let bytes = match reader.read_artifact(&output.exec_path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    complete = false;
                    if detail.is_empty() {
                        detail = format!("unreadable {}: {error}", output.exec_path.display());
                    }
                    continue;
                }
            };
            let shard = output.shard.saturating_sub(1);
            let attempt = output.attempt.saturating_sub(1);
            match parse_test_xml(&bytes, shard, attempt) {
                Ok(cases) => grouped
                    .entry(output.label.clone())
                    .or_default()
                    .extend(cases),
                Err(error) => {
                    complete = false;
                    if detail.is_empty() {
                        detail = format!("invalid {}: {error}", output.exec_path.display());
                    }
                }
            }
        }
        if grouped.is_empty() {
            complete = false;
            if detail.is_empty() {
                detail = "no test.xml artifacts were reported".to_owned();
            }
        }
        suites = grouped.into_iter().collect();
        if !complete {
            suites.push(junit_infrastructure_case(&detail));
        }
    } else {
        for output in &outputs {
            if output.name != "coverage.dat" && output.name != "test.lcov" {
                continue;
            }
            let bytes = match reader.read_artifact(&output.exec_path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    complete = false;
                    if detail.is_empty() {
                        detail = format!("unreadable {}: {error}", output.exec_path.display());
                    }
                    continue;
                }
            };
            match validate_lcov(&bytes) {
                Ok(()) => lcov_documents.push(String::from_utf8_lossy(&bytes).into_owned()),
                Err(error) => {
                    complete = false;
                    if detail.is_empty() {
                        detail = format!("invalid {}: {error}", output.exec_path.display());
                    }
                }
            }
        }
        if lcov_documents.is_empty() {
            complete = false;
            if detail.is_empty() {
                detail = "no coverage.dat artifacts were reported".to_owned();
            }
        }
    }
    let fs = RealFileSystem;
    let mut reports_ok = true;
    for planned in planned_reports {
        let document = match verb {
            WorkflowVerb::Test => Some(render_junit(&suites)),
            WorkflowVerb::Coverage => {
                if lcov_documents.is_empty() {
                    None
                } else {
                    let mut combined = lcov_documents.join("\n");
                    if !combined.ends_with('\n') {
                        combined.push('\n');
                    }
                    Some(combined)
                }
            }
            _ => None, // LCOV_EXCL_LINE - reason: defense-in-depth; execute_test_reports is only reached for Test/Coverage (Build returns early, Run returns earlier), so Build/Run arms are unreachable
        };
        let Some(document) = document else {
            reports_ok = false;
            let detail = format!(
                "failed to render {} report: {detail}",
                planned.format.name()
            );
            let _ = writeln!(err, "dx: report_failed: {detail}");
            if invocation.output == OutputMode::Json {
                if let Ok(event) =
                    dx_output::error_event(CODE_REPORT_FAILED, &detail, None, None, None)
                {
                    let _ = write_event(out, &event);
                }
            }
            continue;
        };
        let written = match &planned.destination {
            Destination::Stdout => out
                .write_all(document.as_bytes())
                .and_then(|()| out.write_all(b"\n"))
                .is_ok(),
            Destination::File(destination) => {
                let target = workspace.join(destination);
                let parent_ok = target
                    .parent()
                    .is_none_or(|parent| parent.as_os_str().is_empty() || parent.is_dir());
                parent_ok && fs.write_atomic(&target, document.as_bytes()).is_ok()
            }
        };
        if !written {
            reports_ok = false;
            let detail = format!(
                "failed to write {} report to {}",
                planned.format.name(),
                planned.destination.display()
            );
            let _ = writeln!(err, "dx: report_failed: {detail}");
            if invocation.output == OutputMode::Json {
                if let Ok(event) =
                    dx_output::error_event(CODE_REPORT_FAILED, &detail, None, None, None)
                {
                    let _ = write_event(out, &event);
                }
            }
            continue;
        }
        if invocation.output == OutputMode::Json {
            if let Ok(event) = report_event(
                planned.format.name(),
                planned.destination.display(),
                complete && reports_ok,
            ) {
                let _ = write_event(out, &event);
            }
        } else if matches!(invocation.output, OutputMode::Text { .. }) && !stdout_report {
            let _ = writeln!(
                out,
                "Wrote {} report to {}.",
                planned.format.name(),
                planned.destination.display()
            );
        } else if invocation.output == OutputMode::Diff {
            let _ = writeln!(
                err,
                "Wrote {} report to {}.",
                planned.format.name(),
                planned.destination.display()
            );
        }
    }
    if !complete && !detail.is_empty() && planned_reports.is_empty() {
        if invocation.output == OutputMode::Json {
            if let Ok(event) =
                dx_output::error_event("incomplete_results", &detail, None, None, None)
            {
                let _ = write_event(out, &event);
            }
        } else {
            let _ = writeln!(err, "dx: incomplete_results: {detail}");
        }
    }
    let code = if bazel_code != 0 {
        bazel_code
    } else if complete && reports_ok {
        0
    } else {
        1
    };
    if invocation.output == OutputMode::Json {
        let _ = write_event(
            out,
            &command_finished(
                code,
                &FinishedCounts {
                    results_complete: Some(complete && reports_ok),
                    ..FinishedCounts::default()
                },
            ),
        );
    }
    code
}

/// Executes `dx run`: local-only single-runnable launcher with
/// verbatim application exit codes. Lifecycle prose goes to stderr;
/// the application keeps stdout through the process runner.
fn execute_run(invocation: &Invocation, env: Env<'_>) -> i32 {
    let Env {
        workspace,
        runner,
        query_runner,
        temp_dir: _,
        pid: _,
        nonce: _,
        out,
        err,
    } = env;
    if std::env::var("CI").is_ok_and(|value| value == "true") {
        return pre_exec(err, "dx run refuses when CI=true: local-only command");
    }
    let planned_reports = match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(planned) => planned,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    debug_assert!(planned_reports.is_empty(), "dx run takes no --report");
    let targets = match resolve_run(&invocation.targets, workspace, query_runner) {
        Ok(targets) => targets,
        Err(error) => {
            let code = resolve_code(&error);
            let message = error.to_string();
            if matches!(
                error,
                ResolveError::NoRunnable { .. } | ResolveError::AmbiguousRunnable { .. }
            ) {
                return operational(invocation, out, err, code, &message);
            }
            return pre_exec(err, &message);
        }
    };
    let plan = if targets.len() == 1 {
        plan_run(&targets[0], &invocation.bazel_options)
    } else {
        plan_run_multi(&targets, &invocation.bazel_options)
    };
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
                let _ = write_event(out, &event);
            }
            let _ = write_event(out, &command_finished(0, &FinishedCounts::default()));
        } else if !invocation.quiet {
            let _ = writeln!(err, "{}", plan.summary);
        }
        return 0;
    }
    if !invocation.quiet {
        let _ = writeln!(err, "{}", plan.summary);
    }
    let status = match runner.run(&plan.argv, workspace) {
        Ok(status) => status,
        Err(error) => {
            return operational(
                invocation,
                out,
                err,
                CODE_LAUNCH_FAILED,
                &format!("failed to launch Bazel: {error}"),
            );
        }
    };
    let Some(code) = status.code else {
        return operational(
            invocation,
            out,
            err,
            CODE_BAZEL_SIGNALLED,
            "Bazel terminated by signal",
        );
    };
    code
}

/// Plans a label-only multi-target `dx run` argv Bazel owns.
///
/// File/directory scopes enforce single-runnable selection in
/// [`resolve_run`]; label scopes pass through unchanged, including
/// multiple labels. `bazel run` rejects multi-target requests itself,
/// so this preserves Bazel's exact diagnostic and status.
fn plan_run_multi(targets: &[String], app_args: &[String]) -> crate::plan::BuildPlan {
    use crate::plan::workspace_flag;
    use dx_process::{launcher_argv0, WORKFLOW_STARTUP_OPTS};
    let mut argv = Vec::new();
    argv.push(launcher_argv0().to_owned());
    argv.extend(WORKFLOW_STARTUP_OPTS.iter().map(ToString::to_string));
    argv.push("run".to_owned());
    argv.push(workspace_flag());
    argv.extend(targets.iter().cloned());
    if !app_args.is_empty() {
        argv.push("--".to_owned());
        argv.extend(app_args.iter().cloned());
    }
    let summary = format!("Running run for {}", targets.join(" "));
    crate::plan::BuildPlan { argv, summary }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::parse;
    use crate::resolve::{QueryResult, QueryRunner};
    use dx_process::{ChildStatus, Runner};
    use quality_result::proto::{Capability, Convergence, FileSnapshot, QualityResult, Stage};
    use quality_result::{encode_validated, SCHEMA_MAJOR, SCHEMA_MINOR};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn invocation(words: &[&str]) -> Invocation {
        parse(&words.iter().map(ToString::to_string).collect::<Vec<_>>()).expect("parse")
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dx-exec-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    struct Harness {
        workspace: PathBuf,
        temp: PathBuf,
        results: HashMap<String, Vec<u8>>,
        bazel_code: i32,
        fail_target: bool,
        io_error: bool,
        signalled: bool,
        skip_bep: bool,
        raw_bep: Option<Vec<String>>,
        query: ScriptQuery,
    }

    /// Scripted ownership-query runner: replays canned outputs in call
    /// order and records argv. Empty outputs panic, so tests that never
    /// resolve file scopes prove they issue no queries.
    struct ScriptQuery {
        calls: RefCell<Vec<Vec<String>>>,
        outputs: RefCell<Vec<QueryResult>>,
    }

    impl ScriptQuery {
        fn script_owners(&self, owners: &str) {
            self.outputs.borrow_mut().push(QueryResult {
                code: Some(0),
                stdout: owners.as_bytes().to_vec(),
                stderr: Vec::new(),
            });
        }
    }

    impl QueryRunner for ScriptQuery {
        fn run_query(&self, argv: &[String], _cwd: &Path) -> io::Result<QueryResult> {
            self.calls.borrow_mut().push(argv.to_vec());
            Ok(self.outputs.borrow_mut().remove(0))
        }
    }

    impl Harness {
        fn new(name: &str) -> Self {
            Harness {
                workspace: temp_dir(&format!("{name}-ws")),
                temp: temp_dir(&format!("{name}-tmp")),
                results: HashMap::new(),
                bazel_code: 0,
                fail_target: false,
                io_error: false,
                signalled: false,
                skip_bep: false,
                raw_bep: None,
                query: ScriptQuery {
                    calls: RefCell::new(Vec::new()),
                    outputs: RefCell::new(Vec::new()),
                },
            }
        }

        fn write_source(&self, path: &str, text: &str) {
            let full = self.workspace.join(path);
            std::fs::create_dir_all(full.parent().expect("parent")).expect("parent dir");
            std::fs::write(full, text).expect("write source");
        }

        fn valid_result(
            &self,
            initial: Vec<proto::Diagnostic>,
            replacements: Vec<proto::FileEdits>,
        ) -> Vec<u8> {
            let original = std::fs::read(self.workspace.join("src/a.py")).unwrap_or_default();
            self.result_full(
                initial,
                vec![],
                replacements,
                vec![FileSnapshot {
                    path: "src/a.py".to_owned(),
                    digest: digest(&original).to_vec(),
                }],
            )
        }

        fn result_full(
            &self,
            initial: Vec<proto::Diagnostic>,
            terminal: Vec<proto::Diagnostic>,
            replacements: Vec<proto::FileEdits>,
            snapshots: Vec<FileSnapshot>,
        ) -> Vec<u8> {
            let result = QualityResult {
                schema_major: SCHEMA_MAJOR,
                schema_minor: SCHEMA_MINOR,
                producer: "//test:corpus".to_owned(),
                capability: Capability::Lint as i32,
                stages: vec![Stage {
                    tool_id: "lint-tool".to_owned(),
                    class_ids: vec!["python".to_owned()],
                    source_paths: vec!["src/a.py".to_owned()],
                }],
                completed_rounds: 1,
                convergence: Convergence::Stable as i32,
                original_snapshot: snapshots.clone(),
                terminal_snapshot: snapshots,
                initial_diagnostics: initial,
                terminal_diagnostics: terminal,
                replacements,
            };
            encode_validated(&result).expect("encode")
        }

        fn diagnostic(message: &str, fixable: bool) -> proto::Diagnostic {
            Self::diagnostic_with(
                proto::Severity::Warning as i32,
                "lint-tool",
                "src/a.py",
                message,
                fixable,
            )
        }

        fn diagnostic_with(
            severity: i32,
            tool: &str,
            path: &str,
            message: &str,
            fixable: bool,
        ) -> proto::Diagnostic {
            proto::Diagnostic {
                severity,
                message: message.to_owned(),
                tool_id: tool.to_owned(),
                rule_id: format!("{tool}/rule"),
                path: path.to_owned(),
                start_byte: Some(0),
                end_byte: Some(1),
                fixable,
            }
        }

        fn replacement(&self, replacement: &[u8]) -> proto::FileEdits {
            let original = std::fs::read(self.workspace.join("src/a.py")).expect("source");
            self.replacement_at(
                "src/a.py",
                digest(&original).to_vec(),
                vec![proto::Edit {
                    start_byte: 0,
                    end_byte: 1,
                    replacement: replacement.to_vec(),
                }],
            )
        }

        fn replacement_at(
            &self,
            path: &str,
            original_digest: Vec<u8>,
            edits: Vec<proto::Edit>,
        ) -> proto::FileEdits {
            proto::FileEdits {
                path: path.to_owned(),
                original_digest,
                edits,
            }
        }

        fn runner(&self) -> FakeRunner {
            if let Some(lines) = &self.raw_bep {
                return FakeRunner {
                    code: Some(self.bazel_code),
                    bep_lines: lines.clone(),
                    io_error: self.io_error,
                    skip_bep: self.skip_bep,
                };
            }
            let mut lines = Vec::new();
            let mut files = Vec::new();
            for (label, bytes) in &self.results {
                let safe = label.replace(['/', ':'], "_");
                let artifact = self.temp.join(format!("{safe}.pb"));
                std::fs::write(&artifact, bytes).expect("artifact");
                files.push(format!(
                    "{{\"uri\": \"file://{}\"}}",
                    artifact.to_string_lossy()
                ));
            }
            lines.push(format!(
                "{{\"id\": {{\"namedSet\": {{\"id\": \"0\"}}}}, \"namedSetOfFiles\": {{\"files\": [{}]}}}}",
                files.join(",")
            ));
            let success = if self.fail_target { "false" } else { "true" };
            lines.push(format!(
                "{{\"id\": {{\"targetCompleted\": {{\"label\": \"//test:corpus\"}}}}, \"completed\": {{\"success\": {success}, \"outputGroup\": [{{\"name\": \"dx_results\", \"fileSets\": [{{\"id\": \"0\"}}]}}]}}}}"
            ));
            FakeRunner {
                code: if self.signalled {
                    None
                } else {
                    Some(self.bazel_code)
                },
                bep_lines: lines,
                io_error: self.io_error,
                skip_bep: self.skip_bep,
            }
        }

        fn run(&self, words: &[&str]) -> (i32, String, String) {
            let inv = invocation(words);
            let runner = self.runner();
            let mut out = Vec::new();
            let mut err = Vec::new();
            let code = execute(
                &inv,
                Env {
                    workspace: &self.workspace,
                    runner: &runner,
                    query_runner: &self.query,
                    temp_dir: &self.temp,
                    pid: std::process::id(),
                    nonce: 0,
                    out: &mut out,
                    err: &mut err,
                },
            );
            (
                code,
                String::from_utf8(out).expect("stdout"),
                String::from_utf8(err).expect("stderr"),
            )
        }
    }

    struct FakeRunner {
        code: Option<i32>,
        bep_lines: Vec<String>,
        io_error: bool,
        skip_bep: bool,
    }

    impl Runner for FakeRunner {
        fn run(&self, argv: &[String], _cwd: &Path) -> io::Result<ChildStatus> {
            if self.io_error {
                return Err(io::Error::other("fake launch failure"));
            }
            if !self.skip_bep {
                if let Some(bep) = argv
                    .iter()
                    .find_map(|arg| arg.strip_prefix("--build_event_json_file="))
                {
                    std::fs::write(bep, self.bep_lines.join("\n")).expect("BEP file");
                }
            }
            Ok(ChildStatus { code: self.code })
        }
    }

    #[test]
    fn hex_digest_formats_lowercase_hex() {
        assert_eq!(hex_digest(&[0xabu8; 32]), "ab".repeat(32));
        assert_eq!(hex_digest(&[0u8; 32]).len(), 64);
    }

    #[test]
    fn check_mode_reports_findings_and_fails_on_changes() {
        let mut harness = Harness::new("check-fails");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![Harness::diagnostic("unused", true)],
                vec![harness.replacement(b"y")],
            ),
        );
        let (code, out, _) = harness.run(&["lint", "--check", "--output=text"]);
        assert_eq!(code, 1);
        assert!(out.contains("Running lint analysis for //..."));
        assert!(out.contains("warning src/a.py: unused [lint-tool/lint-tool/rule] (fixable)"));
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n"
        );
    }

    #[test]
    fn check_mode_clean_run_succeeds() {
        let mut harness = Harness::new("check-clean");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![], vec![]),
        );
        let (code, out, err) = harness.run(&["lint", "--check", "--output=text"]);
        assert_eq!(code, 0);
        assert!(out.contains("Running lint analysis for"));
        assert_eq!(err, "");
    }

    #[test]
    fn default_mode_applies_and_hides_fixed_findings() {
        let mut harness = Harness::new("default-apply");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![Harness::diagnostic("unused", true)],
                vec![harness.replacement(b"y")],
            ),
        );
        let (code, out, _) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 0);
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"y = 1\n"
        );
        assert!(out.contains("Applied 1 file(s)."));
        assert!(!out.contains("unused"));
    }

    #[test]
    fn failed_target_prevents_mutation() {
        let harness = Harness {
            fail_target: true,
            ..Harness::new("partial")
        };
        harness.write_source("src/a.py", "x = 1\n");
        let (code, _, _) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 1);
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n"
        );
    }

    #[test]
    fn json_mode_emits_lifecycle_with_counts() {
        let mut harness = Harness::new("json");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![Harness::diagnostic("unused", true)],
                vec![harness.replacement(b"y")],
            ),
        );
        let (code, out, _) = harness.run(&["lint", "--check", "--output=json"]);
        assert_eq!(code, 1);
        let events: Vec<serde_json::Value> = out
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        let kinds: Vec<&str> = events
            .iter()
            .map(|event| event["event"].as_str().expect("event"))
            .collect();
        assert_eq!(
            kinds,
            vec![
                "command_started",
                "diagnostic",
                "change",
                "command_finished"
            ]
        );
        let finished = events.last().expect("finished");
        assert_eq!(finished["exit_code"], serde_json::json!(1));
        assert_eq!(finished["results_complete"], serde_json::json!(true));
        assert_eq!(
            finished["diagnostics"],
            serde_json::json!({"info": 0, "warning": 1, "error": 0})
        );
        assert_eq!(
            finished["changes"],
            serde_json::json!({"create": 0, "modify": 1})
        );
    }

    #[test]
    fn diff_mode_emits_patch_only() {
        let mut harness = Harness::new("diff");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![Harness::diagnostic("unused", true)],
                vec![harness.replacement(b"y")],
            ),
        );
        let (code, out, _) = harness.run(&["lint", "--check", "--output=diff"]);
        assert_eq!(code, 1);
        assert!(out.contains("--- a/src/a.py"));
        assert!(out.contains("+++ b/src/a.py"));
        assert!(!out.contains("Running lint"));
    }

    #[test]
    fn sarif_file_report_writes_after_validation() {
        let mut harness = Harness::new("sarif");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![Harness::diagnostic("unused", false)], vec![]),
        );
        let (code, out, _) = harness.run(&[
            "lint",
            "--check",
            "--output=text",
            "--report=sarif=out.sarif",
        ]);
        assert_eq!(code, 1);
        assert!(out.contains("Wrote sarif report to out.sarif."));
        let document = std::fs::read(harness.workspace.join("out.sarif")).expect("sarif");
        let parsed: serde_json::Value = serde_json::from_slice(&document).expect("JSON");
        assert_eq!(parsed["version"], serde_json::json!("2.1.0"));
        assert_eq!(
            parsed["runs"][0]["results"][0]["level"],
            serde_json::json!("warning")
        );
    }

    #[test]
    fn dry_run_prints_summary_without_executing() {
        let harness = Harness::new("dry");
        let (code, out, _) = harness.run(&["lint", "--dry-run", "--", "--jobs=99"]);
        assert_eq!(code, 0);
        assert!(out.contains("Running lint analysis for //..."));
    }

    #[test]
    fn conflicting_user_option_fails_before_execution() {
        let harness = Harness::new("conflict");
        let (code, _, err) = harness.run(&["lint", "--", "--nokeep_going"]);
        assert_eq!(code, 2);
        assert!(err.contains("usage"));
    }

    #[test]
    fn file_scope_resolves_to_owners_before_planning() {
        let harness = Harness::new("file-scope");
        harness.write_source("pkg/BUILD.bazel", "");
        harness.write_source("pkg/a.py", "x = 1\n");
        harness.query.script_owners("//pkg:lib\n//pkg:extra\n");
        let (code, out, _) = harness.run(&["lint", "--dry-run", "pkg/a.py"]);
        assert_eq!(code, 0);
        assert!(
            out.contains("Running lint analysis for //pkg:extra //pkg:lib"),
            "{out}"
        );
        let calls = harness.query.calls.borrow();
        assert_eq!(calls.len(), 1, "one query per file");
        assert!(calls[0].iter().any(|arg| arg == "query"));
    }

    #[test]
    fn directory_scope_plans_pattern_without_query() {
        let harness = Harness::new("dir-scope");
        harness.write_source("src/a.py", "x = 1\n");
        let (code, out, _) = harness.run(&["lint", "--dry-run", "src"]);
        assert_eq!(code, 0);
        assert!(out.contains("Running lint analysis for //src/..."), "{out}");
        assert!(harness.query.calls.borrow().is_empty());
    }

    #[test]
    fn missing_path_scope_fails_pre_execution() {
        let harness = Harness::new("missing-scope");
        let (code, _, err) = harness.run(&["lint", "nope.py"]);
        assert_eq!(code, 2);
        assert!(err.contains("nope.py"), "{err}");
        assert!(harness.query.calls.borrow().is_empty());
    }

    #[test]
    fn external_scope_fails_pre_execution() {
        let harness = Harness::new("external-scope");
        let (code, _, err) = harness.run(&["lint", "@repo//pkg/..."]);
        assert_eq!(code, 2);
        assert!(err.contains("@repo//pkg/..."), "{err}");
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
    fn undecodable_artifact_marks_collection_incomplete() {
        let mut harness = Harness::new("undecodable");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            b"not-a-validated-result".to_vec(),
        );
        let (code, _, _) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 1);
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n"
        );
    }

    #[test]
    fn launch_failure_is_operational() {
        let harness = Harness {
            io_error: true,
            ..Harness::new("launch-fail")
        };
        let (code, _, err) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: launch_failed: failed to launch Bazel"));
    }

    #[test]
    fn launch_failure_in_json_mode_emits_error_events() {
        let harness = Harness {
            io_error: true,
            ..Harness::new("launch-json")
        };
        let (code, out, _) = harness.run(&["lint", "--output=json"]);
        assert_eq!(code, 1);
        let events: Vec<serde_json::Value> = out
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        let kinds: Vec<&str> = events
            .iter()
            .map(|event| event["event"].as_str().expect("event"))
            .collect();
        assert_eq!(kinds, vec!["command_started", "error", "command_finished"]);
        let finished = events.last().expect("finished");
        assert_eq!(finished["exit_code"], serde_json::json!(1));
        assert_eq!(finished["results_complete"], serde_json::json!(false));
    }

    #[test]
    fn signalled_bazel_is_operational() {
        let harness = Harness {
            signalled: true,
            ..Harness::new("signalled")
        };
        let (code, _, err) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: bazel_signalled: Bazel terminated by signal"));
    }

    #[test]
    fn malformed_bep_stream_is_operational() {
        let harness = Harness {
            raw_bep: Some(vec!["{not json".to_owned()]),
            ..Harness::new("bad-bep")
        };
        let (code, _, err) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: invalid_bep: invalid build events"));
    }

    #[test]
    fn missing_bep_file_is_operational() {
        let harness = Harness {
            skip_bep: true,
            ..Harness::new("missing-bep")
        };
        let (code, _, err) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: unreadable_bep: failed to read build events"));
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

    #[test]
    #[cfg(unix)]
    fn non_utf8_temp_path_is_operational() {
        use std::os::unix::ffi::OsStringExt;
        let harness = Harness::new("nonutf8-tmp");
        let mut raw = harness.temp.join("x").into_os_string().into_vec();
        raw.push(0xff);
        let temp = PathBuf::from(std::ffi::OsString::from_vec(raw));
        let invocation = invocation(&["lint", "--check"]);
        let runner = harness.runner();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute(
            &invocation,
            Env {
                workspace: &harness.workspace,
                runner: &runner,
                query_runner: &harness.query,
                temp_dir: &temp,
                pid: std::process::id(),
                nonce: 0,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 1);
        assert!(String::from_utf8(err)
            .expect("stderr")
            .contains("dx: unreadable_bep: temporary event path is not UTF-8"));
    }

    #[test]
    fn dry_run_with_report_is_pre_exec() {
        let harness = Harness::new("dry-report");
        let (code, _, err) = harness.run(&["lint", "--dry-run", "--report=sarif=x.sarif"]);
        assert_eq!(code, 2);
        assert!(err.contains("usage: dx"));
    }

    #[test]
    fn dry_run_json_emits_lifecycle() {
        let harness = Harness::new("dry-json");
        let (code, out, _) = harness.run(&["lint", "--dry-run", "--output=json"]);
        assert_eq!(code, 0);
        let events: Vec<serde_json::Value> = out
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        let kinds: Vec<&str> = events
            .iter()
            .map(|event| event["event"].as_str().expect("event"))
            .collect();
        assert_eq!(kinds, vec!["command_started", "command_finished"]);
        assert_eq!(
            events.last().expect("finished")["exit_code"],
            serde_json::json!(0)
        );
    }

    #[test]
    fn failed_bazel_with_changes_skips_mutation() {
        let mut harness = Harness {
            bazel_code: 1,
            ..Harness::new("bazel-fails")
        };
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![Harness::diagnostic("unused", true)],
                vec![harness.replacement(b"y")],
            ),
        );
        let (code, _, err) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 1);
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n"
        );
        assert!(err.contains("Not applied: src/a.py (incomplete_collection)"));
    }

    #[test]
    fn terminal_diagnostics_map_with_terminal_snapshot() {
        let mut harness = Harness::new("terminal-diag");
        harness.write_source("src/a.py", "x = 1\n");
        let digest = digest(b"x = 1\n").to_vec();
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.result_full(
                vec![],
                vec![Harness::diagnostic("terminal unused", false)],
                vec![],
                vec![FileSnapshot {
                    path: "src/a.py".to_owned(),
                    digest,
                }],
            ),
        );
        let (code, out, _) = harness.run(&["lint", "--output=json"]);
        assert_eq!(code, 1);
        let events = json_events(&out);
        let diagnostics: Vec<&serde_json::Value> = events
            .iter()
            .filter(|event| event["event"] == serde_json::json!("diagnostic"))
            .collect();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0]["snapshot"], serde_json::json!("terminal"));
    }

    #[test]
    fn diff_mode_lists_unapplied_changes_after_failed_bazel() {
        let mut harness = Harness {
            bazel_code: 1,
            ..Harness::new("bazel-fails-diff")
        };
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![Harness::diagnostic("unused", true)],
                vec![harness.replacement(b"y")],
            ),
        );
        let (code, out, err) = harness.run(&["lint", "--output=diff"]);
        assert_eq!(code, 1);
        assert!(out.contains("--- a/src/a.py"));
        assert!(err.contains("Not applied: src/a.py (incomplete_collection)"));
    }

    #[test]
    fn json_mode_lists_unapplied_changes_after_failed_bazel() {
        let mut harness = Harness {
            bazel_code: 1,
            ..Harness::new("bazel-fails-json")
        };
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![Harness::diagnostic("unused", true)],
                vec![harness.replacement(b"y")],
            ),
        );
        let (code, out, _) = harness.run(&["lint", "--output=json"]);
        assert_eq!(code, 1);
        let events = json_events(&out);
        let finished = event(&events, "command_finished");
        assert_eq!(
            finished["mutations"],
            serde_json::json!({"applied": 0, "not_applied": 1})
        );
    }

    #[test]
    fn stale_source_skips_mutation() {
        let mut harness = Harness::new("stale");
        harness.write_source("src/a.py", "x = 1\n");
        let bytes = harness.valid_result(
            vec![Harness::diagnostic("unused", true)],
            vec![harness.replacement(b"y")],
        );
        harness.results.insert("//test:corpus".to_owned(), bytes);
        harness.write_source("src/a.py", "z = 2\n");
        let (code, _, err) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 1);
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"z = 2\n"
        );
        assert!(err.contains("Not applied: src/a.py (stale_source)"));
    }

    #[test]
    fn missing_source_is_unreadable() {
        let mut harness = Harness::new("missing-src");
        harness.write_source("src/a.py", "x = 1\n");
        let change = harness.replacement_at(
            "src/missing.py",
            vec![0u8; 32],
            vec![proto::Edit {
                start_byte: 0,
                end_byte: 1,
                replacement: b"y".to_vec(),
            }],
        );
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![Harness::diagnostic("unused", true)], vec![change]),
        );
        let (code, _, err) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 1);
        assert!(err.contains("Not applied: src/missing.py (unreadable_source)"));
    }

    #[test]
    fn out_of_bounds_edit_is_invalid() {
        let mut harness = Harness::new("oob-edit");
        harness.write_source("src/a.py", "x = 1\n");
        let original = std::fs::read(harness.workspace.join("src/a.py")).expect("source");
        let change = harness.replacement_at(
            "src/a.py",
            digest(&original).to_vec(),
            vec![proto::Edit {
                start_byte: 0,
                end_byte: 100,
                replacement: b"x".to_vec(),
            }],
        );
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![Harness::diagnostic("unused", true)], vec![change]),
        );
        let (code, _, err) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 1);
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n"
        );
        assert!(err.contains("Not applied: src/a.py (invalid_edits)"));
    }

    #[test]
    fn staging_collision_fails_atomic_write() {
        let mut harness = Harness::new("staging-blocked");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![Harness::diagnostic("unused", true)],
                vec![harness.replacement(b"y")],
            ),
        );
        std::fs::create_dir_all(harness.workspace.join("src/.a.py.dx-apply-tmp"))
            .expect("staging dir");
        let (code, _, err) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 1);
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n"
        );
        assert!(err.contains("Not applied: src/a.py (unreadable_source)"));
    }

    #[test]
    fn diff_missing_source_fails() {
        let mut harness = Harness::new("diff-missing");
        harness.write_source("src/a.py", "x = 1\n");
        let change = harness.replacement_at(
            "src/missing.py",
            vec![0u8; 32],
            vec![proto::Edit {
                start_byte: 0,
                end_byte: 1,
                replacement: b"y".to_vec(),
            }],
        );
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![], vec![change]),
        );
        let (code, _, err) = harness.run(&["lint", "--check", "--output=diff"]);
        assert_eq!(code, 1);
        assert!(err.contains("cannot render patch without verified source for src/missing.py"));
    }

    #[test]
    fn diff_non_utf8_source_fails() {
        let mut harness = Harness::new("diff-nonutf8");
        harness.write_source("src/a.py", "x = 1\n");
        std::fs::write(harness.workspace.join("src/a.py"), b"\xff\xfe").expect("bytes");
        let original = std::fs::read(harness.workspace.join("src/a.py")).expect("source");
        let change = harness.replacement_at(
            "src/a.py",
            digest(&original).to_vec(),
            vec![proto::Edit {
                start_byte: 0,
                end_byte: 1,
                replacement: b"y".to_vec(),
            }],
        );
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![], vec![change]),
        );
        let (code, _, err) = harness.run(&["lint", "--check", "--output=diff"]);
        assert_eq!(code, 1);
        assert!(err.contains("source for src/a.py is not UTF-8 text"));
    }

    #[test]
    fn diff_unappliable_edit_fails() {
        let mut harness = Harness::new("diff-oob");
        harness.write_source("src/a.py", "x = 1\n");
        let original = std::fs::read(harness.workspace.join("src/a.py")).expect("source");
        let change = harness.replacement_at(
            "src/a.py",
            digest(&original).to_vec(),
            vec![proto::Edit {
                start_byte: 0,
                end_byte: 100,
                replacement: b"x".to_vec(),
            }],
        );
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![], vec![change]),
        );
        let (code, _, err) = harness.run(&["lint", "--check", "--output=diff"]);
        assert_eq!(code, 1);
        assert!(err.contains("cannot apply recorded edits for src/a.py"));
    }

    #[test]
    fn diff_non_utf8_candidate_fails() {
        let mut harness = Harness::new("diff-candidate");
        harness.write_source("src/a.py", "héllo\n");
        let original = std::fs::read(harness.workspace.join("src/a.py")).expect("source");
        let change = harness.replacement_at(
            "src/a.py",
            digest(&original).to_vec(),
            vec![proto::Edit {
                // Splitting the two-byte é (bytes 1..3) makes the edit unappliable.
                start_byte: 1,
                end_byte: 2,
                replacement: b"X".to_vec(),
            }],
        );
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![], vec![change]),
        );
        let (code, _, err) = harness.run(&["lint", "--check", "--output=diff"]);
        assert_eq!(code, 1);
        assert!(err.contains("cannot apply recorded edits for src/a.py"));
    }

    #[test]
    fn diff_identical_candidate_fails_render() {
        let mut harness = Harness::new("diff-noop");
        harness.write_source("src/a.py", "x = 1\n");
        let original = std::fs::read(harness.workspace.join("src/a.py")).expect("source");
        let change = harness.replacement_at(
            "src/a.py",
            digest(&original).to_vec(),
            vec![proto::Edit {
                start_byte: 0,
                end_byte: 1,
                replacement: b"x".to_vec(),
            }],
        );
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![], vec![change]),
        );
        let (code, _, err) = harness.run(&["lint", "--check", "--output=diff"]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: diff_failed: failed to render patch"));
    }

    fn json_events(out: &str) -> Vec<serde_json::Value> {
        out.lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON")
    }

    fn event<'a>(events: &'a [serde_json::Value], kind: &str) -> &'a serde_json::Value {
        events
            .iter()
            .find(|event| event["event"] == serde_json::json!(kind))
            .unwrap_or_else(|| panic!("missing {kind} event"))
    }

    #[test]
    fn json_default_marks_remaining_resolution() {
        let mut harness = Harness::new("remaining");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![Harness::diagnostic("unused", false)],
                vec![harness.replacement(b"y")],
            ),
        );
        let (code, out, _) = harness.run(&["lint", "--output=json"]);
        assert_eq!(code, 1);
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"y = 1\n"
        );
        let events = json_events(&out);
        assert_eq!(
            event(&events, "diagnostic")["resolution"],
            serde_json::json!("remaining")
        );
        assert_eq!(
            event(&events, "mutation")["outcome"],
            serde_json::json!("applied")
        );
        let finished = event(&events, "command_finished");
        assert_eq!(
            finished["mutations"],
            serde_json::json!({"applied": 1, "not_applied": 0})
        );
    }

    #[test]
    fn json_default_marks_not_applied_resolution() {
        let mut harness = Harness {
            bazel_code: 1,
            ..Harness::new("not-applied")
        };
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![Harness::diagnostic("unused", true)],
                vec![harness.replacement(b"y")],
            ),
        );
        let (code, out, _) = harness.run(&["lint", "--output=json"]);
        assert_eq!(code, 1);
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n"
        );
        let events = json_events(&out);
        assert_eq!(
            event(&events, "diagnostic")["resolution"],
            serde_json::json!("not_applied")
        );
        let mutation = event(&events, "mutation");
        assert_eq!(mutation["outcome"], serde_json::json!("not_applied"));
        assert_eq!(
            mutation["reason"],
            serde_json::json!("incomplete_collection")
        );
    }

    #[test]
    fn json_counts_cover_all_severities() {
        let mut harness = Harness::new("counts");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![
                    Harness::diagnostic_with(
                        proto::Severity::Info as i32,
                        "lint-tool",
                        "src/a.py",
                        "note",
                        false,
                    ),
                    Harness::diagnostic_with(
                        proto::Severity::Error as i32,
                        "lint-tool",
                        "src/a.py",
                        "broken",
                        false,
                    ),
                ],
                vec![],
            ),
        );
        let (code, out, _) = harness.run(&["lint", "--check", "--output=json"]);
        assert_eq!(code, 1);
        let events = json_events(&out);
        assert_eq!(
            event(&events, "command_finished")["diagnostics"],
            serde_json::json!({"info": 1, "warning": 0, "error": 1})
        );
    }

    #[test]
    fn json_clean_check_succeeds() {
        let mut harness = Harness::new("json-clean");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![], vec![]),
        );
        let (code, out, _) = harness.run(&["lint", "--check", "--output=json"]);
        assert_eq!(code, 0);
        let events = json_events(&out);
        assert_eq!(
            event(&events, "command_finished")["exit_code"],
            serde_json::json!(0)
        );
    }

    #[test]
    fn sarif_unreadable_snapshot_fails_report() {
        let mut harness = Harness::new("sarif-unreadable");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![Harness::diagnostic("unused", false)], vec![]),
        );
        std::fs::remove_file(harness.workspace.join("src/a.py")).expect("remove");
        let (code, _, err) = harness.run(&[
            "lint",
            "--check",
            "--output=text",
            "--report=sarif=out.sarif",
        ]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: report_failed: failed to render SARIF report"));
    }

    #[test]
    fn sarif_stale_snapshot_fails_report() {
        let mut harness = Harness::new("sarif-stale");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![Harness::diagnostic("unused", false)], vec![]),
        );
        harness.write_source("src/a.py", "changed = true\n");
        let (code, _, err) = harness.run(&[
            "lint",
            "--check",
            "--output=text",
            "--report=sarif=out.sarif",
        ]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: report_failed: failed to render SARIF report"));
    }

    #[test]
    fn sarif_non_utf8_snapshot_fails_report() {
        let mut harness = Harness::new("sarif-nonutf8");
        std::fs::create_dir_all(harness.workspace.join("src")).expect("dirs");
        std::fs::write(harness.workspace.join("src/a.py"), b"\xff").expect("bytes");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![Harness::diagnostic("unused", false)], vec![]),
        );
        let (code, _, err) = harness.run(&[
            "lint",
            "--check",
            "--output=text",
            "--report=sarif=out.sarif",
        ]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: report_failed: failed to render SARIF report"));
    }

    #[test]
    fn sarif_stdout_report_owns_stdout() {
        let mut harness = Harness::new("sarif-stdout");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![Harness::diagnostic("unused", false)], vec![]),
        );
        let (code, out, err) =
            harness.run(&["lint", "--check", "--output=text", "--report=sarif=-"]);
        assert_eq!(code, 1);
        assert!(out.contains("2.1.0"));
        // The prose summary is suppressed while stdout carries the report;
        // the human-readable finding moves to stderr.
        assert!(!out.contains("Running lint"));
        assert!(!err.contains("Running lint"));
        assert!(err.contains("unused"));
    }

    #[test]
    fn json_report_write_failure_emits_error() {
        let mut harness = Harness::new("report-write-fail");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![], vec![]),
        );
        let (code, out, err) = harness.run(&[
            "lint",
            "--check",
            "--output=json",
            "--report=sarif=nodir/out.sarif",
        ]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: report_failed: failed to write sarif report to nodir/out.sarif"));
        let events = json_events(&out);
        assert_eq!(
            event(&events, "error")["code"],
            serde_json::json!("report_failed")
        );
    }

    #[test]
    fn json_file_report_emits_report_event() {
        let mut harness = Harness::new("report-ok");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![], vec![]),
        );
        let (code, out, _) = harness.run(&[
            "lint",
            "--check",
            "--output=json",
            "--report=sarif=out.sarif",
        ]);
        assert_eq!(code, 0);
        let events = json_events(&out);
        let report = event(&events, "report");
        assert_eq!(report["format"], serde_json::json!("sarif"));
        assert!(harness.workspace.join("out.sarif").exists());
    }

    #[test]
    fn diff_file_report_notes_to_stderr() {
        let mut harness = Harness::new("diff-report");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![], vec![]),
        );
        let (code, _, err) = harness.run(&[
            "lint",
            "--check",
            "--output=diff",
            "--report=sarif=out.sarif",
        ]);
        assert_eq!(code, 0);
        assert!(err.contains("Wrote sarif report to out.sarif."));
        assert!(harness.workspace.join("out.sarif").exists());
    }

    #[test]
    fn unknown_tool_finding_fails_sarif_render() {
        let mut harness = Harness::new("unknown-tool");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![Harness::diagnostic_with(
                    proto::Severity::Warning as i32,
                    "other-tool",
                    "src/a.py",
                    "stray",
                    false,
                )],
                vec![],
            ),
        );
        let (code, out, err) = harness.run(&[
            "lint",
            "--check",
            "--output=json",
            "--report=sarif=out.sarif",
        ]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: report_failed: failed to render SARIF report"));
        let events = json_events(&out);
        assert_eq!(
            event(&events, "error")["code"],
            serde_json::json!("report_failed")
        );
    }

    fn write_bep_artifact(harness: &Harness, name: &str, bytes: &[u8]) -> String {
        let path = harness.temp.join(name);
        std::fs::write(&path, bytes).expect("artifact");
        format!("file://{}", path.display())
    }

    fn test_result_line(label: &str, entries: &[(String, String)]) -> String {
        let outputs = entries
            .iter()
            .map(|(name, uri)| format!("{{\"name\": \"{name}\", \"uri\": \"{uri}\"}}"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"id\": {{\"testResult\": {{\"label\": \"{label}\"}} }}, \"testResult\": {{\"status\": \"PASSED\", \"testActionOutput\": [{outputs}]}}}}"
        )
    }

    const MINIMAL_TEST_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?><testsuites><testsuite name="s"><testcase name="passes" classname="c" time="0.1"/></testsuite></testsuites>"#;
    const MINIMAL_LCOV: &str = "SF:src/a.py\nDA:1,1\nend_of_record\n";

    #[test]
    fn build_preserves_bazel_status_verbatim() {
        let harness = Harness::new("build-ok");
        let (code, out, _) = harness.run(&["build", "--output=text"]);
        assert_eq!(code, 0);
        assert!(out.contains("Running build for //..."), "{out}");
        let harness = Harness {
            bazel_code: 3,
            ..Harness::new("build-fails")
        };
        let (code, _, _) = harness.run(&["build", "--output=text"]);
        assert_eq!(code, 3);
    }

    #[test]
    fn test_junit_file_report_succeeds() {
        let harness = Harness::new("test-junit");
        let uri = write_bep_artifact(&harness, "test.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&["test", "--output=text", "--report=junit=out.xml"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("Wrote junit report to out.xml."));
        let document = std::fs::read(harness.workspace.join("out.xml")).expect("junit");
        let text = String::from_utf8(document).expect("utf8");
        assert!(text.contains("<testsuites name=\"dx\""), "{text}");
        assert!(text.contains("<testsuite name=\"//a:t\""), "{text}");
    }

    #[test]
    fn test_stdout_report_owns_stdout() {
        let harness = Harness::new("test-stdout");
        let uri = write_bep_artifact(&harness, "test-stdout.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&["test", "--output=text", "--report=junit=-"]);
        assert_eq!(code, 0);
        assert!(out.contains("<testsuites name=\"dx\""), "{out}");
        assert!(!out.contains("Running test"), "{out}");
    }

    #[test]
    fn test_missing_artifacts_are_incomplete() {
        let harness = Harness {
            raw_bep: Some(vec![String::from(
                "{\"id\": {\"testResult\": {\"label\": \"//a:t\"}}, \"testResult\": {\"status\": \"PASSED\"}}",
            )]),
            ..Harness::new("test-empty")
        };
        let (code, _, err) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 1);
        assert!(err.contains("incomplete_results"), "{err}");
    }

    #[test]
    fn test_preserves_bazel_failure_code() {
        let harness = Harness::new("test-bazel-fails");
        let uri = write_bep_artifact(&harness, "fail.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            bazel_code: 4,
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, _, _) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 4);
    }

    #[test]
    fn coverage_lcov_file_report_succeeds() {
        let harness = Harness::new("cov-ok");
        let uri = write_bep_artifact(&harness, "coverage.dat", MINIMAL_LCOV.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.lcov"), uri)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&["coverage", "--output=text", "--report=lcov=out.lcov"]);
        assert_eq!(code, 0, "{out}");
        let document = std::fs::read(harness.workspace.join("out.lcov")).expect("lcov");
        assert_eq!(document, MINIMAL_LCOV.as_bytes());
    }

    #[test]
    fn coverage_invalid_tracefile_fails() {
        let harness = Harness::new("cov-bad");
        let uri = write_bep_artifact(&harness, "bad.dat", b"not lcov");
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.lcov"), uri)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&["coverage", "--output=text"]);
        assert_eq!(code, 1);
        assert!(err.contains("incomplete_results"), "{err}");
    }

    #[test]
    fn run_label_passthrough_preserves_status_on_stderr() {
        let harness = Harness::new("run-ok");
        let (code, out, err) = harness.run(&["run", "//app:bin", "--", "--port=8080"]);
        assert_eq!(code, 0);
        assert_eq!(out, "");
        assert!(err.contains("Running run for //app:bin"), "{err}");
        let harness = Harness {
            bazel_code: 7,
            ..Harness::new("run-fails")
        };
        let (code, _, _) = harness.run(&["run", "//app:bin"]);
        assert_eq!(code, 7);
    }

    #[test]
    fn run_empty_scope_is_pre_exec() {
        let harness = Harness::new("run-empty");
        let (code, _, err) = harness.run(&["run"]);
        assert_eq!(code, 2);
        assert!(err.contains("empty scope"), "{err}");
    }

    #[test]
    fn run_file_without_runnable_is_operational() {
        let harness = Harness::new("run-norunnable");
        harness.write_source("pkg/BUILD.bazel", "");
        harness.write_source("pkg/a.py", "x = 1\n");
        harness.query.script_owners("");
        harness.query.script_owners("//pkg:lib\n");
        let (code, _, err) = harness.run(&["run", "pkg/a.py"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("no_runnable"), "{err}");
    }

    #[test]
    fn resolve_code_maps_all_variants() {
        assert_eq!(
            resolve_code(&ResolveError::NoRunnable {
                scopes: vec!["a".to_owned()],
            }),
            "no_runnable"
        );
        assert_eq!(
            resolve_code(&ResolveError::AmbiguousRunnable {
                candidates: vec!["//a:one".to_owned(), "//a:two".to_owned()],
            }),
            "ambiguous_runnable"
        );
        assert_eq!(
            resolve_code(&ResolveError::NoTests {
                owners: vec!["//a:lib".to_owned()],
            }),
            "no_tests"
        );
        assert_eq!(resolve_code(&ResolveError::EmptyScope), "scope_error");
    }

    #[test]
    fn workflow_bad_report_is_pre_exec() {
        let harness = Harness::new("wf-bad-report");
        let (code, _, err) = harness.run(&["build", "--report=junit=a.xml"]);
        assert_eq!(code, 2, "{err}");
    }

    #[test]
    fn workflow_bad_scope_is_pre_exec() {
        let harness = Harness::new("wf-bad-scope");
        let (code, _, err) = harness.run(&["build", "nope.py"]);
        assert_eq!(code, 2, "{err}");
    }

    #[test]
    fn workflow_conflicting_option_is_pre_exec() {
        let harness = Harness::new("wf-conflict");
        let (code, _, err) = harness.run(&[
            "build",
            "--",
            "--@rules_dx//config:workspace=//other:config",
        ]);
        assert_eq!(code, 2, "{err}");
    }

    #[test]
    #[cfg(unix)]
    fn workflow_non_utf8_bep_is_operational() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        let mut harness = Harness::new("wf-nonutf8");
        harness.temp = PathBuf::from(OsString::from_vec(vec![0xff]));
        let (code, _, err) = harness.run(&["build", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("unreadable_bep"), "{err}");
    }

    #[test]
    fn workflow_dry_run_text_and_json() {
        let harness = Harness::new("wf-dry-text");
        let (code, out, _) = harness.run(&["build", "--dry-run", "--output=text"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("Running build"), "{out}");
        let harness = Harness::new("wf-dry-json");
        let (code, out, _) = harness.run(&["build", "--dry-run", "--output=json"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("command_started"), "{out}");
    }

    #[test]
    fn workflow_json_lifecycle_and_build_status() {
        let harness = Harness::new("wf-json");
        let (code, out, _) = harness.run(&["build", "--output=json"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("command_started"), "{out}");
        assert!(out.contains("command_finished"), "{out}");
    }

    #[test]
    fn workflow_launch_failure_is_operational() {
        let mut harness = Harness::new("wf-launch");
        harness.io_error = true;
        let (code, _, err) = harness.run(&["build", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("launch_failed"), "{err}");
    }

    #[test]
    fn workflow_signalled_is_operational() {
        let mut harness = Harness::new("wf-signal");
        harness.signalled = true;
        let (code, _, err) = harness.run(&["build", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("bazel_signalled"), "{err}");
    }

    #[test]
    fn test_unreadable_bep_is_operational() {
        let mut harness = Harness::new("test-nobep");
        harness.skip_bep = true;
        let (code, _, err) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("unreadable_bep"), "{err}");
    }

    #[test]
    fn test_invalid_bep_is_operational() {
        let harness = Harness {
            raw_bep: Some(vec![String::from("not json")]),
            ..Harness::new("test-badbep")
        };
        let (code, _, err) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("invalid_bep"), "{err}");
    }

    #[test]
    fn test_ignores_non_xml_entries() {
        let harness = Harness::new("test-ignore-log");
        let xml = write_bep_artifact(&harness, "ok.xml", MINIMAL_TEST_XML.as_bytes());
        std::fs::write(harness.temp.join("ok.log"), b"log").expect("log");
        let log_uri = format!("file://{}", harness.temp.join("ok.log").display());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[
                    (String::from("test.log"), log_uri),
                    (String::from("test.xml"), xml),
                ],
            )]),
            ..harness
        };
        let (code, _, _) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 0);
    }

    #[test]
    fn test_unreadable_artifact_is_incomplete() {
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(
                    String::from("test.xml"),
                    String::from("file:///nonexistent/a.xml"),
                )],
            )]),
            ..Harness::new("test-unreadable")
        };
        let (code, _, err) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("incomplete_results"), "{err}");
    }

    #[test]
    fn test_invalid_xml_is_incomplete() {
        let harness = Harness::new("test-badxml");
        let uri = write_bep_artifact(&harness, "bad.xml", b"not xml");
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("incomplete_results"), "{err}");
    }

    #[test]
    fn coverage_ignores_non_lcov_and_reports_missing() {
        let harness = Harness::new("cov-ignore");
        let xml = write_bep_artifact(&harness, "x.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), xml)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&["coverage", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("no coverage.dat"), "{err}");
    }

    #[test]
    fn coverage_unreadable_is_incomplete() {
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(
                    String::from("coverage.dat"),
                    String::from("file:///nonexistent/c.dat"),
                )],
            )]),
            ..Harness::new("cov-unreadable")
        };
        let (code, _, err) = harness.run(&["coverage", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("incomplete_results"), "{err}");
    }

    #[test]
    fn coverage_report_failed_when_incomplete() {
        let harness = Harness::new("cov-repfail");
        let uri = write_bep_artifact(&harness, "bad2.dat", b"not lcov");
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.lcov"), uri)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&["coverage", "--output=text", "--report=lcov=out.lcov"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("report_failed"), "{err}");
    }

    #[test]
    fn test_report_write_failure_is_operational() {
        let harness = Harness::new("test-writefail");
        let uri = write_bep_artifact(&harness, "w.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&[
            "test",
            "--output=text",
            "--report=junit=missing-dir/out.xml",
        ]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("report_failed"), "{err}");
    }

    #[test]
    fn test_report_json_emits_report_event() {
        let harness = Harness::new("test-jsonrep");
        let uri = write_bep_artifact(&harness, "j.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&["test", "--output=json", "--report=junit=out.xml"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("report"), "{out}");
        assert!(out.contains("command_finished"), "{out}");
    }

    #[test]
    fn test_report_diff_goes_to_stderr() {
        let harness = Harness::new("test-diffrep");
        let uri = write_bep_artifact(&harness, "d.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&["test", "--output=diff", "--report=junit=out.xml"]);
        assert_eq!(code, 0, "{err}");
        assert!(err.contains("Wrote junit report"), "{err}");
    }

    #[test]
    fn test_incomplete_json_reports_incomplete_event() {
        let harness = Harness {
            raw_bep: Some(vec![String::from(
                "{\"id\": {\"testResult\": {\"label\": \"//a:t\"}}, \"testResult\": {\"status\": \"PASSED\"}}",
            )]),
            ..Harness::new("test-incjson")
        };
        let (code, out, _) = harness.run(&["test", "--output=json"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("incomplete_results"), "{out}");
        assert!(out.contains("command_finished"), "{out}");
    }

    #[test]
    fn run_ambiguous_is_operational() {
        let harness = Harness::new("run-amb");
        std::fs::create_dir_all(harness.workspace.join("app")).expect("dir");
        harness.query.script_owners("//app:two\n//app:one\n");
        let (code, _, err) = harness.run(&["run", "app"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("ambiguous_runnable"), "{err}");
    }

    #[test]
    fn run_bad_report_is_pre_exec() {
        // `parse` rejects `--report` for `run` before execution; assert the
        // usage error directly since `Harness::run` requires parse success.
        let args: Vec<String> = ["run", "//app:bin", "--report=sarif=a.sarif"]
            .iter()
            .map(ToString::to_string)
            .collect();
        let err = parse(&args).expect_err("run report must fail parse");
        assert!(err.to_string().contains("--report"), "{err:?}");
    }

    #[test]
    fn run_dry_run_json_and_text() {
        // `parse` owns `--output=json` rejection for `run`; the text dry-run
        // exercises `execute_run` planning.
        let args: Vec<String> = ["run", "//app:bin", "--dry-run", "--output=json"]
            .iter()
            .map(ToString::to_string)
            .collect();
        let err = parse(&args).expect_err("run json must fail parse");
        assert!(err.to_string().contains("--output"), "{err:?}");
        let harness = Harness::new("run-dry-text");
        let (code, _, err) = harness.run(&["run", "//app:bin", "--dry-run"]);
        assert_eq!(code, 0, "{err}");
        assert!(err.contains("Running run"), "{err}");
    }

    #[test]
    fn run_launch_and_signal_failures() {
        let mut harness = Harness::new("run-launch");
        harness.io_error = true;
        let (code, _, err) = harness.run(&["run", "//app:bin"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("launch_failed"), "{err}");
        let mut harness = Harness::new("run-signal");
        harness.signalled = true;
        let (code, _, err) = harness.run(&["run", "//app:bin"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("bazel_signalled"), "{err}");
    }

    #[test]
    fn run_multi_target_uses_multi_plan() {
        let harness = Harness::new("run-multi");
        let (code, _, err) = harness.run(&["run", "//a:bin", "//b:bin"]);
        assert_eq!(code, 0, "{err}");
        assert!(err.contains("//a:bin //b:bin"), "{err}");
        // Multi-target with app args covers the `--` forwarding arm.
        let harness = Harness::new("run-multi-args");
        let (code, _, err) = harness.run(&["run", "//a:bin", "//b:bin", "--", "--port=8080"]);
        assert_eq!(code, 0, "{err}");
        assert!(err.contains("//a:bin //b:bin"), "{err}");
    }

    fn run_invocation(
        command: Command,
        output: OutputMode,
        reports: Vec<crate::args::ReportRequest>,
        dry_run: bool,
    ) -> Invocation {
        Invocation {
            command,
            check: false,
            workspace: None,
            dry_run,
            quiet: false,
            output,
            reports,
            fail_on: dx_output::Threshold::Warning,
            targets: vec!["//app:bin".to_owned()],
            bazel_options: Vec::new(),
        }
    }

    fn execute_with(invocation: &Invocation, harness: &Harness) -> (i32, String, String) {
        let runner = harness.runner();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute(
            invocation,
            Env {
                workspace: &harness.workspace,
                runner: &runner,
                query_runner: &harness.query,
                temp_dir: &harness.temp,
                pid: std::process::id(),
                nonce: 0,
                out: &mut out,
                err: &mut err,
            },
        );
        (
            code,
            String::from_utf8(out).expect("stdout"),
            String::from_utf8(err).expect("stderr"),
        )
    }

    #[test]
    fn run_manual_invocations_cover_defense_branches() {
        // `parse` rejects `--report` and JSON output for `run`; construct the
        // invocation directly to cover `execute_run` defense branches.
        let harness = Harness::new("run-manual-report");
        let inv = run_invocation(
            Command::Run,
            OutputMode::Text { quiet: false },
            vec![crate::args::ReportRequest {
                format: "junit".to_owned(),
                destination: "out.xml".to_owned(),
            }],
            false,
        );
        let (code, _, err) = execute_with(&inv, &harness);
        assert_eq!(code, 2, "{err}");

        let harness = Harness::new("run-manual-json");
        let inv = run_invocation(Command::Run, OutputMode::Json, Vec::new(), true);
        let (code, out, _) = execute_with(&inv, &harness);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("command_started"), "{out}");
        assert!(out.contains("command_finished"), "{out}");
    }

    #[test]
    fn coverage_lcov_without_trailing_newline_gets_newline() {
        let harness = Harness::new("cov-nonl");
        let raw = b"SF:src/a.py\nDA:1,1\nend_of_record";
        let uri = write_bep_artifact(&harness, "nonl.dat", raw);
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.lcov"), uri)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&["coverage", "--output=text", "--report=lcov=out.lcov"]);
        assert_eq!(code, 0, "{out}");
        let document = std::fs::read(harness.workspace.join("out.lcov")).expect("lcov");
        assert!(document.ends_with(b"\n"), "{document:?}");
    }

    #[test]
    fn coverage_render_failure_json_reports_error_event() {
        // No lcov documents with a requested report triggers render failure;
        // JSON output must emit the `report_failed` error event.
        let harness = Harness::new("cov-render-json");
        let xml = write_bep_artifact(&harness, "x.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), xml)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&["coverage", "--output=json", "--report=lcov=out.lcov"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("report_failed"), "{out}");
    }

    #[test]
    fn test_report_write_failure_json_reports_error_event() {
        let harness = Harness::new("test-writefail-json");
        let uri = write_bep_artifact(&harness, "w2.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&[
            "test",
            "--output=json",
            "--report=junit=missing-dir/out.xml",
        ]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("report_failed"), "{out}");
    }

    #[test]
    fn run_ci_refusal_is_pre_exec() {
        // Cover both restore arms: first with no prior CI, then with a prior.
        for preset in [None, Some("0")] {
            match preset {
                Some(value) => std::env::set_var("CI", value),
                None => std::env::remove_var("CI"),
            }
            let harness = Harness::new("run-ci");
            let prior = std::env::var("CI").ok();
            std::env::set_var("CI", "true");
            let (code, _, err) = harness.run(&["run", "//app:bin"]);
            match prior {
                Some(value) => std::env::set_var("CI", value),
                None => std::env::remove_var("CI"),
            }
            assert_eq!(code, 2, "{err}");
            assert!(err.contains("CI=true"), "{err}");
        }
        std::env::remove_var("CI");
    }
}
