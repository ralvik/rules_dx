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
use std::path::{Path, PathBuf};

use crate::args::{Command, Invocation, ReportRequest};
use crate::finalize::{finalize, FinalizeError, FinalizeInput};
use crate::generate::{project, render_diff, text_lines};
use crate::plan::{
    bep_path, generate_scope_json, intended_path, plan_bazel, plan_build, plan_generate,
    plan_managed, plan_run, plan_workflow, spec, WorkflowVerb, GENERATE_ENV_INTENDED,
    GENERATE_ENV_MODE, GENERATE_ENV_SCOPE, OUTPUT_GROUP,
};
use crate::reports::{
    coverage_line_rate, junit_infrastructure_case, parse_test_xml, plan_reports, render_junit,
    render_sarif, validate_lcov, Destination, JunitCase, PlannedReport, ReportError,
};
use crate::resolve::{resolve, resolve_for_test, resolve_run, QueryRunner, ResolveError};
use dx_apply::{FileSystem, RealFileSystem};
use dx_bep::{collect, collect_test_outputs, ArtifactReader, CollectorConfig};
use dx_clean::{
    apply_plan, bazel_forward_argv, collect_inventory_with_scan, measure_prune_bytes,
    render_dry_run, RECOVERY_GUIDANCE,
};
use dx_diff::{render_patch, FilePatch, PatchKind};
use dx_output::{
    change_event, command_finished, command_started, diagnostic_event, meets_threshold,
    mutation_event, notice_event, report_event, write_event, ChangeEvent, ChangeKind,
    DiagnosticEvent, FinishedCounts, MutationOutcome, OutputMode, Resolution, Severity, Snapshot,
};
use dx_process::{operational_code, pre_exec_code, ForwardError, Runner};
use quality_result::{decode_validated, digest, proto};
use serde_json::{json, Value};

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
const CODE_COVERAGE_BELOW_MINIMUM: &str = "coverage_below_minimum";
/// Stable operational error code for managed-state cleanup failures:
/// a malformed current selection, commit-lock contention, or a prune
/// mutation error all fail closed with nothing adopted or repaired.
const CODE_CLEAN_FAILED: &str = "clean_failed";
/// Stable operational error code for well-formed requests the current
/// result transport cannot serve: a missing, unreadable, or
/// contradictory generation manifest fails closed with no change or
/// mutation output.
const CODE_INVALID_RESULT: &str = "invalid_result";
/// Stable operational error code for managed-state commit failures:
/// lock contention, malformed current selection, record mismatch, or a
/// generation staging mutation error all fail closed with the prior
/// pointer preserved.
const CODE_MANAGED_COMMIT_FAILED: &str = "managed_commit_failed";
/// Stable operational error code for exact setup scopes that provide
/// neither environment nor codegen capability: committing an empty or
/// recycled pair would hide the usage error.
const CODE_MANAGED_NO_CAPABILITY: &str = "no_capability";
/// Stable operational error code for live audit runs while auditor
/// wiring stays deferred: advisory acquisition, tool execution, and
/// SARIF mapping land in later M26 slices (O11/O58). Planning
/// (`--dry-run`) succeeds; live execution fails closed.
const CODE_AUDIT_DEFERRED: &str = "audit_deferred";
/// Stable operational error code for live update runs while resolver
/// backends stay deferred: dependency-set execution and per-set
/// reporting land in later M26 slices (O12). Planning (`--dry-run`)
/// succeeds; live execution fails closed.
const CODE_UPDATE_DEFERRED: &str = "update_deferred";

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
        "usage: dx [--workspace DIR] [--dry-run] [--quiet] [--output text|diff|json] [--report <format>=<destination>]... [--fail-on info|warning|error] [--min-coverage 0-100 (coverage only)] <audit|lint|typecheck|format|generate|build|test|coverage|run|check|fix|clean|update|codegen|env|setup|init|hooks|status|version|watch|owners|deps|why|completion|bazel> [--check] [scope ...] [-- command-options...]"
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
    if invocation.command.is_adoption() {
        let Env {
            workspace,
            query_runner,
            out,
            err,
            ..
        } = env;
        return crate::adopt::execute_adoption(
            invocation,
            crate::adopt::AdoptEnv {
                workspace,
                query_runner,
                out,
                err,
            },
        );
    }
    if invocation.command.is_umbrella() {
        return execute_umbrella(invocation, env);
    }
    if invocation.command.is_workflow() {
        return execute_workflow(invocation, env);
    }
    if invocation.command == Command::Bazel {
        return execute_bazel(invocation, env);
    }
    if invocation.command == Command::Generate {
        return execute_generate(invocation, env);
    }
    if invocation.command == Command::Clean {
        return execute_clean(invocation, env);
    }
    if invocation.command.is_managed() {
        return execute_managed(invocation, env);
    }
    if invocation.command == Command::Audit {
        return execute_audit(invocation, env);
    }
    if invocation.command == Command::Update {
        return execute_update(invocation, env);
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
        ci: _,
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
    let status = match runner.run(&build.argv, workspace, &[]) {
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
        ci: _,
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
        _ => {
            return pre_exec(
                err,
                &ForwardError::UnsupportedCommand {
                    command: invocation.command.name().to_owned(),
                }
                .to_string(),
            );
        }
    };
    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let Some(verb) = WorkflowVerb::of(invocation.command) else {
        return pre_exec(
            err,
            &ForwardError::UnsupportedCommand {
                command: invocation.command.name().to_owned(),
            }
            .to_string(),
        );
    };
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
    let status = match runner.run(&plan.argv, workspace, &[]) {
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

/// Executes `dx bazel`: raw launcher passthrough for the M26 WP4
/// helper surface (`dx bazel version`, `dx bazel audit`/
/// `dx bazel update` when those helpers exist).
///
/// The child inherits stdio and its exit code forwards verbatim: no
/// scope resolution, no quality thresholds or reports, no BEP
/// stream, and no dx-owned output beyond the dry-run/quiet summary
/// line. Argument parsing guarantees text output, so only quiet
/// suppresses the summary.
fn execute_bazel(invocation: &Invocation, env: Env<'_>) -> i32 {
    let Env {
        workspace,
        runner,
        out,
        err,
        ..
    } = env;
    let plan = plan_bazel(&invocation.bazel_options);
    if invocation.dry_run {
        if !matches!(invocation.output, OutputMode::Text { quiet: true }) && !invocation.quiet {
            let _ = writeln!(out, "{}", plan.summary);
        }
        return 0;
    }
    if matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet {
        let _ = writeln!(out, "{}", plan.summary);
    }
    let status = match runner.run(&plan.argv, workspace, &[]) {
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
    bazel_code
}

/// Closes a generate run that produced no reportable manifest: JSON
/// mode finishes the envelope with `results_complete: false` and no
/// change, mutation, or notice events; other modes stay silent. The
/// caller-supplied code (Gazelle's failure or the Bazel status) is
/// preserved.
fn finish_incomplete_generate(invocation: &Invocation, out: &mut dyn Write, code: i32) -> i32 {
    if invocation.output == OutputMode::Json {
        let _ = write_event(
            out,
            &command_finished(
                code,
                &FinishedCounts {
                    results_complete: Some(false),
                    ..FinishedCounts::default()
                },
            ),
        );
    }
    code
}

/// Runs `dx generate` through the canonical `//dx:generate` Gazelle
/// runner, or `//dx:generate_check` for `--check` (M10 WP1, O13
/// dispatch). Contract: `docs/cli/commands/generate.md` for the
/// target surface. Scope positionals resolve through the canonical
/// target resolution and narrow the runner traversal to the resolved
/// directories; empty scope stays repo-wide (`//...`).
///
/// Dispatch sets the private protocol environment on the Gazelle run:
/// `DX_GENERATE_INTENDED` (witness destination under the temp dir),
/// `DX_GENERATE_SCOPE` (resolved scope JSON), and `DX_GENERATE_MODE`
/// (`check` or `default`). The extension witnesses its exact BUILD
/// changes there; [`finalize`] turns the witness into the versioned
/// manifest and text, diff, and NDJSON render from that manifest
/// without rerunning Gazelle.
///
/// Gazelle owns its output and exit status: a nonzero Bazel code is
/// preserved through the projection, and a structurally valid manifest
/// that ends after a late failure still reports its validated
/// attempted prefix. A missing witness after a failed run degrades to
/// the incomplete envelope; a missing or contradictory witness after a
/// successful run fails closed (`invalid_result`) with no change or
/// mutation output, even if Gazelle changed workspace files first.
/// Scope resolution failures (unknown paths, external scopes) fail
/// pre-execution like every other command.
fn execute_generate(invocation: &Invocation, env: Env<'_>) -> i32 {
    match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(_) => {}
        Err(error) => return pre_exec(env.err, &error.to_string()),
    }
    let resolved = match resolve(&invocation.targets, env.workspace, env.query_runner) {
        Ok(resolved) => resolved,
        Err(error) => return pre_exec(env.err, &error.to_string()),
    };
    let plan = match plan_generate(&resolved, &invocation.bazel_options, invocation.check) {
        Ok(plan) => plan,
        Err(error) => return pre_exec(env.err, &format!("{error:?}")),
    };
    let mode = if invocation.check { "check" } else { "default" };
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, mode) {
                let _ = write_event(env.out, &event);
            }
            let _ = write_event(env.out, &command_finished(0, &FinishedCounts::default()));
        } else if matches!(invocation.output, OutputMode::Text { quiet: false })
            && !invocation.quiet
        {
            let _ = writeln!(env.out, "{}", plan.summary);
        }
        return 0;
    }
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), false, mode) {
            let _ = write_event(env.out, &event);
        }
    } else if matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet {
        let _ = writeln!(env.out, "{}", plan.summary);
    }
    let intended = intended_path(env.temp_dir, env.pid, env.nonce);
    let intended_str = intended.display().to_string();
    let scope_json = generate_scope_json(&resolved);
    let dispatch = [
        (GENERATE_ENV_INTENDED, intended_str.as_str()),
        (GENERATE_ENV_SCOPE, scope_json.as_str()),
        (GENERATE_ENV_MODE, mode),
    ];
    let status = match env.runner.run(&plan.argv, env.workspace, &dispatch) {
        Ok(status) => status,
        Err(error) => {
            return operational(
                invocation,
                env.out,
                env.err,
                CODE_LAUNCH_FAILED,
                &format!("failed to launch Bazel: {error}"),
            );
        }
    };
    let Some(bazel_code) = status.code else {
        return operational(
            invocation,
            env.out,
            env.err,
            CODE_BAZEL_SIGNALLED,
            "Bazel terminated by signal",
        );
    };
    let Some(witness) = std::fs::read(&intended).ok() else {
        if bazel_code != 0 {
            return finish_incomplete_generate(invocation, env.out, bazel_code);
        }
        return operational(
            invocation,
            env.out,
            env.err,
            CODE_INVALID_RESULT,
            "generate completed without a result manifest",
        );
    };
    let manifest = match finalize(&FinalizeInput {
        intended_json: &witness,
        workspace: env.workspace,
        check: invocation.check,
        gazelle_ok: bazel_code == 0,
    }) {
        Ok(manifest) => manifest,
        Err(FinalizeError::IncompleteCheck) => {
            return finish_incomplete_generate(invocation, env.out, bazel_code);
        }
        Err(error) => {
            return operational(
                invocation,
                env.out,
                env.err,
                CODE_INVALID_RESULT,
                &format!("invalid generation manifest: {error}"),
            );
        }
    };
    let projected = match project(&manifest) {
        Ok(projected) => projected,
        // LCOV_EXCL_START - reason: defense-in-depth; finalize returns a validated manifest and project revalidates the same value deterministically, so projection cannot fail here.
        Err(error) => {
            return operational(
                invocation,
                env.out,
                env.err,
                CODE_INVALID_RESULT,
                &format!("invalid generation manifest: {error:?}"),
            );
        } // LCOV_EXCL_STOP - reason: end of unreachable projection arm.
    };
    if invocation.output == OutputMode::Json {
        for file in projected.sorted_files() {
            match change_event(&file.change) {
                Ok(event) => {
                    let _ = write_event(env.out, &event);
                }
                // LCOV_EXCL_START - reason: defense-in-depth; project builds changes from a validated manifest (sound paths, non-empty ordered edits, valid hex digests), so change_event cannot fail here.
                Err(error) => {
                    return operational(
                        invocation,
                        env.out,
                        env.err,
                        CODE_INVALID_RESULT,
                        &format!("invalid change for output: {error:?}"),
                    );
                } // LCOV_EXCL_STOP - reason: end of unreachable change arm.
            }
        }
        if !invocation.check {
            for file in &projected.files {
                let (outcome, reason) = match file.outcome {
                    Some(MutationOutcome::Applied) => (MutationOutcome::Applied, None),
                    Some(MutationOutcome::NotApplied) => {
                        (MutationOutcome::NotApplied, file.failure_code.as_deref())
                    }
                    // LCOV_EXCL_START - reason: defense-in-depth; project maps every default-mode file to Applied or NotApplied, so a missing outcome is unreachable here.
                    None => {
                        return operational(
                            invocation,
                            env.out,
                            env.err,
                            CODE_INVALID_RESULT,
                            &format!("invalid mutation for output: {}", file.change.path),
                        );
                    } // LCOV_EXCL_STOP - reason: end of unreachable outcome arm.
                };
                match mutation_event(&file.change.path, file.kind(), outcome, reason) {
                    Ok(event) => {
                        let _ = write_event(env.out, &event);
                    }
                    // LCOV_EXCL_START - reason: defense-in-depth; manifest validation requires a nonempty failure_code exactly for NotApplied, so mutation_event cannot fail here.
                    Err(error) => {
                        return operational(
                            invocation,
                            env.out,
                            env.err,
                            CODE_INVALID_RESULT,
                            &format!("invalid mutation for output: {error:?}"),
                        );
                    } // LCOV_EXCL_STOP - reason: end of unreachable mutation arm.
                }
            }
        }
        for notice in &projected.notices {
            match notice_event(notice) {
                Ok(event) => {
                    let _ = write_event(env.out, &event);
                }
                // LCOV_EXCL_START - reason: defense-in-depth; project builds notices from validated ignored imports (warning level, nonempty code/message/path/language/import), so notice_event cannot fail here.
                Err(error) => {
                    return operational(
                        invocation,
                        env.out,
                        env.err,
                        CODE_INVALID_RESULT,
                        &format!("invalid notice for output: {error:?}"),
                    );
                } // LCOV_EXCL_STOP - reason: end of unreachable notice arm.
            }
        }
        let code = projected.exit_code(bazel_code);
        let _ = write_event(
            env.out,
            &command_finished(code, &projected.finished_counts()),
        );
        return code;
    }
    if invocation.output == OutputMode::Diff {
        // Diff reserves stdout for the validated patch: no summaries,
        // notices, or diagnostics move anywhere.
        match render_diff(&projected) {
            Ok(patch) => {
                env.out.write_all(patch.as_bytes()).ok();
            }
            // LCOV_EXCL_START - reason: defense-in-depth; validated manifests hold unique paths, empty originals on creates, and no noop edits, so render_diff cannot fail here.
            Err(error) => {
                return operational(
                    invocation,
                    env.out,
                    env.err,
                    CODE_DIFF_FAILED,
                    &format!("failed to render generate patch: {error}"),
                );
            } // LCOV_EXCL_STOP - reason: end of unreachable diff arm.
        }
    } else {
        for line in text_lines(&projected) {
            let _ = writeln!(env.out, "{line}");
        }
    }
    projected.exit_code(bazel_code)
}

/// Runs `dx clean [--dry-run] [--bazel]` (issue #20): collects the
/// workspace managed-state inventory with the process scan (live shells
/// or actions holding `.dx` paths pin their hexes as active), plans the
/// prune set over validated unselected records and generations, and
/// either renders the `--dry-run` listing with reclaimable bytes
/// (deleting nothing, holding no lock) or applies the prune set under
/// the shared commit lock. An explicit `--bazel` additionally forwards
/// exactly `bazel clean` after pruning and prints the dangling-link
/// recovery guidance; under `--dry-run` the forward is listed, never
/// run. Argument parsing guarantees text output with no scopes,
/// reports, or quality options on this path.
///
/// Exits `0` on success (including an empty prune set), `1` on
/// inventory, lock, or prune failures, and propagates the Bazel exit
/// code for the explicit forward.
fn execute_clean(invocation: &Invocation, env: Env<'_>) -> i32 {
    let Env {
        workspace,
        runner,
        out,
        err,
        ..
    } = env;
    let inventory = match collect_inventory_with_scan(workspace) {
        Ok(inventory) => inventory,
        Err(error) => {
            return operational(invocation, out, err, CODE_CLEAN_FAILED, &error.to_string());
        }
    };
    let plan = inventory.plan();
    // Reclaimable bytes measure the planned prune set before any lock or
    // deletion: symlinks count, targets never do, and vanished entries
    // measure zero so measure and idempotent apply agree.
    let bytes = match measure_prune_bytes(workspace, &plan) {
        Ok(bytes) => bytes,
        Err(error) => {
            return operational(invocation, out, err, CODE_CLEAN_FAILED, &error.to_string());
        }
    };
    // Human prose is the only output on this path: `--dry-run` and the
    // prune summary print in text mode unless `--quiet` suppresses them.
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    if invocation.dry_run {
        if verbose {
            let _ = writeln!(out, "{}", render_dry_run(&plan, &bytes));
            if invocation.bazel_clean {
                let _ = writeln!(out, "would forward: bazel clean");
            }
        }
        return 0;
    }
    let outcome = match apply_plan(workspace, &plan) {
        Ok(outcome) => outcome,
        Err(error) => {
            return operational(invocation, out, err, CODE_CLEAN_FAILED, &error.to_string());
        }
    };
    if verbose {
        if outcome.removed_setup_records.is_empty() && outcome.removed_generations.is_empty() {
            let _ = writeln!(out, "dx clean: nothing to prune");
        } else {
            let _ = writeln!(
                out,
                "dx clean: pruned {} setup records and {} generations ({} bytes reclaimed)",
                outcome.removed_setup_records.len(),
                outcome.removed_generations.len(),
                bytes.reclaimed(&outcome)
            );
        }
    }
    if !invocation.bazel_clean {
        return 0;
    }
    // The explicit forward is exactly `bazel clean` (never any other
    // verb): the argv pins to the frozen `dx_clean` forward shape.
    let mut argv = vec!["bazel".to_owned()];
    argv.extend(bazel_forward_argv());
    let status = match runner.run(&argv, workspace, &[]) {
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
    if verbose {
        let _ = writeln!(out, "{}", RECOVERY_GUIDANCE);
    }
    bazel_code
}

/// Collects BEP-reported artifacts for one managed output group.
/// Split from [`collect_managed_codegen`] and [`collect_managed_env`]
/// so each selection proves its own group transport without touching
/// the other group's stream.
fn collect_managed_group(
    bep: &Path,
    group: &str,
) -> Result<Vec<dx_bep::TargetOutput>, (String, String)> {
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
    collect(BufReader::new(file), &config, &FsArtifacts).map_err(|err| {
        (
            CODE_INVALID_BEP.to_owned(),
            format!("invalid build events: {err:?}"),
        )
    })
}

/// Validated managed codegen collection: raw group outputs plus the
/// merged plan and its read-only projection. The raw outputs stay
/// alongside so exact-scope callers can tell a capability-absent side
/// (no contributing target) from a present but empty plan.
type ManagedCodegenCollection = (
    Vec<dx_bep::TargetOutput>,
    dx_codegen::CollectedPlan,
    Vec<dx_codegen::ProjectionEntry>,
);

/// Validated managed environment collection, mirroring
/// [`ManagedCodegenCollection`] over the env output group.
type ManagedEnvCollection = (
    Vec<dx_bep::TargetOutput>,
    dx_env_plan::CollectedPlan,
    Vec<dx_env_plan::ProjectionEntry>,
);

/// Collects and validates one managed codegen plan: decodes shards,
/// rejects conflicts, merges deterministically, and plans the read-only
/// projection through the same index collection validates. An empty
/// shard set validates as an empty plan.
fn collect_managed_codegen(bep: &Path) -> Result<ManagedCodegenCollection, (String, String)> {
    let outputs = collect_managed_group(bep, dx_codegen::OUTPUT_GROUP)?;
    let plan = dx_codegen::collect_plan(&outputs).map_err(|err| {
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid codegen plan: {err:?}"),
        )
    })?;
    let projection = dx_codegen::plan_projection(&plan.records, &outputs).map_err(|err| {
        // LCOV_EXCL_START - reason: defense-in-depth; collect_plan runs the identical artifact-index validation over the same outputs, so projection cannot fail after a successful collect; retained so a future divergence fails closed as invalid_result rather than panicking.
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid codegen plan: {err:?}"),
        )
        // LCOV_EXCL_STOP - reason: end of unreachable projection-failure mapping.
    })?;
    Ok((outputs, plan, projection))
}

/// Collects and validates one managed environment plan, mirroring
/// [`collect_managed_codegen`] over the env output group.
fn collect_managed_env(bep: &Path) -> Result<ManagedEnvCollection, (String, String)> {
    let outputs = collect_managed_group(bep, dx_env_plan::OUTPUT_GROUP)?;
    let plan = dx_env_plan::collect_plan(&outputs).map_err(|err| {
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid env plan: {err:?}"),
        )
    })?;
    let projection = dx_env_plan::plan_projection(&plan.records, &outputs).map_err(|err| {
        // LCOV_EXCL_START - reason: defense-in-depth; collect_plan runs the identical artifact-index validation over the same outputs, so projection cannot fail after a successful collect; retained so a future divergence fails closed as invalid_result rather than panicking.
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid env plan: {err:?}"),
        )
        // LCOV_EXCL_STOP - reason: end of unreachable projection-failure mapping.
    })?;
    Ok((outputs, plan, projection))
}

/// Managed empty environment identity: the deterministic empty-plan
/// digest (`"[]"` fingerprint) pairing a first independent codegen
/// selection with a real immutable identity, per
/// `docs/environments/managed-state.md`.
fn empty_env_id() -> Result<dx_setup::GenerationId, (String, String)> {
    // LCOV_EXCL_START - reason: defense-in-depth; plan_hex always renders a valid generation id, so construction cannot fail; retained so a future divergence fails closed as invalid_result rather than panicking.
    dx_setup::GenerationId::new(&dx_env_plan::plan_hex("[]")).map_err(|err| {
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid empty env plan digest: {err}"),
        )
    })
    // LCOV_EXCL_STOP - reason: end of unreachable digest-construction exclusion.
}

/// Managed empty generated-code identity, mirroring [`empty_env_id`].
fn empty_generated_id() -> Result<dx_setup::GenerationId, (String, String)> {
    // LCOV_EXCL_START - reason: defense-in-depth; plan_hex always renders a valid generation id, so construction cannot fail; retained so a future divergence fails closed as invalid_result rather than panicking.
    dx_setup::GenerationId::new(&dx_codegen::plan_hex("[]")).map_err(|err| {
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid empty codegen plan digest: {err}"),
        )
    })
    // LCOV_EXCL_STOP - reason: end of unreachable digest-construction exclusion.
}

/// Ensures the hash-addressed generation directory exists as a managed
/// directory. A present file, symlink, or other non-directory fails
/// closed so foreign state is never adopted; any other inspection
/// failure falls through to creation, which fails closed with the
/// underlying error.
fn ensure_generation_dir(
    workspace: &Path,
    dir_name: &str,
    hex: &str,
) -> Result<PathBuf, (String, String)> {
    if !workspace.is_dir() {
        return Err((
            CODE_MANAGED_COMMIT_FAILED.to_owned(),
            format!("workspace root {} is not a directory", workspace.display()),
        ));
    }
    let dir = workspace.join(".dx").join(dir_name).join(hex);
    match std::fs::symlink_metadata(&dir) {
        Ok(meta) => {
            if !meta.file_type().is_dir() || meta.file_type().is_symlink() {
                return Err((
                    CODE_MANAGED_COMMIT_FAILED.to_owned(),
                    format!(
                        "{} is not a managed generation directory; refusing to adopt foreign state",
                        dir.display()
                    ),
                ));
            }
        }
        Err(_) => {
            std::fs::create_dir_all(&dir).map_err(|e| {
                (
                    CODE_MANAGED_COMMIT_FAILED.to_owned(),
                    format!("cannot create {}: {e}", dir.display()),
                )
            })?;
        }
    }
    Ok(dir)
}

/// Platform symlink primitive for generation mirror leaves.
#[cfg(windows)]
fn symlink_leaf(target: &Path, link: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_file(target, link)
}

/// Platform symlink primitive for generation mirror leaves.
#[cfg(not(windows))]
fn symlink_leaf(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

/// Rejects workspace-absolute, escaping, or empty logical paths before
/// any mutation.
fn validate_logical_path(logical_path: &str) -> Result<(), String> {
    if logical_path.is_empty() {
        return Err("generated logical path is empty".to_owned());
    }
    let path = Path::new(logical_path);
    if path.is_absolute() {
        return Err(format!(
            "generated logical path {logical_path:?} is absolute"
        ));
    }
    if path.components().any(|c| {
        matches!(
            c,
            std::path::Component::ParentDir | std::path::Component::Prefix(_)
        )
    }) {
        return Err(format!(
            "generated logical path {logical_path:?} escapes its generation"
        ));
    }
    Ok(())
}

/// Stages one immutable codegen generation: validates every mirror leaf
/// against the current BEP result, refuses logical paths colliding with
/// checked-in sources, and installs deterministic symlinks to Bazel-owned
/// artifacts. Missing artifacts fail before selection; Bazel owns remote
/// materialization and the CLI performs no fetch. Leaves install
/// idempotently so concurrent preparation of one generation never fails;
/// a leaf pointing elsewhere is reconstructed.
fn stage_codegen_generation(
    workspace: &Path,
    id: &dx_setup::GenerationId,
    projection: &[dx_codegen::ProjectionEntry],
) -> Result<(), (String, String)> {
    let dir = ensure_generation_dir(workspace, dx_setup::GENERATED_DIR_NAME, id.as_str())?;
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for entry in projection {
        if let Some(previous) = seen.insert(entry.logical_path.as_str(), entry.artifact.as_str()) {
            if previous != entry.artifact.as_str() {
                return Err((
                    CODE_INVALID_RESULT.to_owned(),
                    format!(
                        "generated logical path {:?} maps to multiple artifacts",
                        entry.logical_path
                    ),
                ));
            }
        }
    }
    for entry in projection {
        validate_logical_path(&entry.logical_path).map_err(|reason| {
            (
                CODE_INVALID_RESULT.to_owned(),
                format!("invalid codegen plan: {reason}"),
            )
        })?;
        if workspace
            .join(&entry.logical_path)
            .symlink_metadata()
            .is_ok()
        {
            return Err((
                CODE_INVALID_RESULT.to_owned(),
                format!(
                    "invalid codegen plan: generated logical path {:?} collides with a workspace source",
                    entry.logical_path
                ),
            ));
        }
        let artifact = Path::new(&entry.artifact);
        if artifact.symlink_metadata().is_err() {
            return Err((
                CODE_INVALID_RESULT.to_owned(),
                format!(
                    "invalid codegen plan: referenced artifact {:?} is missing; Bazel owns materialization",
                    entry.artifact
                ),
            ));
        }
        let leaf = dir.join(&entry.logical_path);
        if let Some(parent) = leaf.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                (
                    CODE_MANAGED_COMMIT_FAILED.to_owned(),
                    format!("cannot create {}: {e}", parent.display()),
                )
            })?;
        }
        // Reuse is exact-identity reuse: a leaf already pointing at the
        // current BEP-reported artifact stays; any other existing leaf is
        // reconstructed so a stale or foreign leaf never survives
        // selection. An unreadable leaf falls through to replacement,
        // which fails closed below when the filesystem is unusable.
        let needs_link = match std::fs::symlink_metadata(&leaf) {
            Ok(meta) => {
                if meta.file_type().is_dir() && !meta.file_type().is_symlink() {
                    return Err((
                        CODE_INVALID_RESULT.to_owned(),
                        format!(
                            "invalid codegen plan: generated logical path {:?} collides within its generation",
                            entry.logical_path
                        ),
                    ));
                }
                match std::fs::read_link(&leaf) {
                    Ok(current) if current == *artifact => false,
                    _ => {
                        std::fs::remove_file(&leaf).map_err(|e| {
                            (
                                CODE_MANAGED_COMMIT_FAILED.to_owned(),
                                format!("cannot replace {}: {e}", leaf.display()),
                            )
                        })?;
                        true
                    }
                }
            }
            Err(_) => true,
        };
        if needs_link {
            symlink_leaf(artifact, &leaf).map_err(|e| {
                (
                    CODE_MANAGED_COMMIT_FAILED.to_owned(),
                    format!("cannot link {}: {e}", leaf.display()),
                )
            })?;
        }
    }
    Ok(())
}

/// Rejects env keys that are not safe single-path filenames before any
/// mutation.
fn validate_env_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("env identity key is empty".to_owned());
    }
    if key.contains('/') || key.contains('\\') || key == "." || key == ".." {
        return Err(format!("env identity key {key:?} is not a single filename"));
    }
    Ok(())
}

/// Stages one immutable environment generation: validates every backing
/// leaf against the current BEP result and installs deterministic
/// symlinks to Bazel-owned artifacts under `artifacts/` plus a
/// deterministic `values.json` carrying the key-to-value identity
/// inputs. Layout mirrors the codegen mirror leaf shape so selection
/// commits one deterministic link tree; language-native facades and
/// `.dx/bin` refresh stay outside this layer.
fn stage_env_generation(
    workspace: &Path,
    id: &dx_setup::GenerationId,
    projection: &[dx_env_plan::ProjectionEntry],
) -> Result<(), (String, String)> {
    let dir = ensure_generation_dir(workspace, dx_setup::ENVIRONMENTS_DIR_NAME, id.as_str())?;
    let mut seen: BTreeMap<&str, (&str, &str)> = BTreeMap::new();
    for entry in projection {
        validate_env_key(&entry.key).map_err(|reason| {
            (
                CODE_INVALID_RESULT.to_owned(),
                format!("invalid env plan: {reason}"),
            )
        })?;
        if let Some((value, artifact)) = seen.insert(
            entry.key.as_str(),
            (entry.value.as_str(), entry.artifact.as_str()),
        ) {
            if value != entry.value.as_str() || artifact != entry.artifact.as_str() {
                return Err((
                    CODE_INVALID_RESULT.to_owned(),
                    format!("env identity key {:?} maps to multiple inputs", entry.key),
                ));
            }
        }
    }
    let artifacts = dir.join("artifacts");
    std::fs::create_dir_all(&artifacts).map_err(|e| {
        (
            CODE_MANAGED_COMMIT_FAILED.to_owned(),
            format!("cannot create {}: {e}", artifacts.display()),
        )
    })?;
    for entry in projection {
        let artifact = Path::new(&entry.artifact);
        if artifact.symlink_metadata().is_err() {
            return Err((
                CODE_INVALID_RESULT.to_owned(),
                format!(
                    "invalid env plan: referenced artifact {:?} is missing; Bazel owns materialization",
                    entry.artifact
                ),
            ));
        }
        // Reuse mirrors the codegen mirror leaves: a leaf already
        // pointing at the current BEP-reported artifact stays, any other
        // existing leaf is reconstructed, and an unreadable leaf falls
        // through to replacement, which fails closed below.
        let leaf = artifacts.join(&entry.key);
        let needs_link = match std::fs::symlink_metadata(&leaf) {
            Ok(meta) => {
                if meta.file_type().is_dir() && !meta.file_type().is_symlink() {
                    return Err((
                        CODE_INVALID_RESULT.to_owned(),
                        format!(
                            "invalid env plan: env identity key {:?} collides within its generation",
                            entry.key
                        ),
                    ));
                }
                match std::fs::read_link(&leaf) {
                    Ok(current) if current == *artifact => false,
                    _ => {
                        std::fs::remove_file(&leaf).map_err(|e| {
                            (
                                CODE_MANAGED_COMMIT_FAILED.to_owned(),
                                format!("cannot replace {}: {e}", leaf.display()),
                            )
                        })?;
                        true
                    }
                }
            }
            Err(_) => true,
        };
        if needs_link {
            symlink_leaf(artifact, &leaf).map_err(|e| {
                (
                    CODE_MANAGED_COMMIT_FAILED.to_owned(),
                    format!("cannot link {}: {e}", leaf.display()),
                )
            })?;
        }
    }
    let mut values = String::from("{");
    let mut keys: Vec<&str> = seen.keys().copied().collect();
    keys.sort();
    for (index, key) in keys.iter().enumerate() {
        if index > 0 {
            values.push(',');
        }
        let value = seen[key].0;
        values.push_str(&serde_json::Value::String((*key).to_owned()).to_string());
        values.push(':');
        values.push_str(&serde_json::Value::String(value.to_owned()).to_string());
    }
    values.push('}');
    let target = dir.join("values.json");
    let staging = dir.join("values.json.next");
    std::fs::write(&staging, values.as_bytes()).map_err(|e| {
        (
            CODE_MANAGED_COMMIT_FAILED.to_owned(),
            format!("cannot stage {}: {e}", staging.display()),
        )
    })?;
    std::fs::rename(&staging, &target).map_err(|e| {
        (
            CODE_MANAGED_COMMIT_FAILED.to_owned(),
            format!("cannot publish {}: {e}", target.display()),
        )
    })?;
    Ok(())
}

/// Maps a setup commit failure into the stable managed error vocabulary.
/// Only the capability error carries its own code; every other commit
/// failure preserves the prior pointer and reports `managed_commit_failed`.
fn map_commit_error(error: dx_setup::CommitError) -> (String, String) {
    match error {
        dx_setup::CommitError::NoCapability => (
            CODE_MANAGED_NO_CAPABILITY.to_owned(),
            "selected scope provides neither environment nor codegen capability".to_owned(),
        ),
        other => (CODE_MANAGED_COMMIT_FAILED.to_owned(), other.to_string()),
    }
}

/// Stages one validated codegen side: derives its immutable identity
/// from the plan digest and installs the mirror leaves. Returns the
/// prepared generation identity.
fn stage_codegen_side(
    workspace: &Path,
    plan: &dx_codegen::CollectedPlan,
    projection: &[dx_codegen::ProjectionEntry],
) -> Result<dx_setup::GenerationId, (String, String)> {
    let id = dx_setup::GenerationId::new(&plan.hex()).map_err(|err| {
        // LCOV_EXCL_START - reason: defense-in-depth; plan digests always render valid generation ids, so construction cannot fail; retained so a future divergence fails closed as invalid_result rather than panicking.
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid codegen plan digest: {err}"),
        )
        // LCOV_EXCL_STOP - reason: end of unreachable digest-construction exclusion.
    })?;
    stage_codegen_generation(workspace, &id, projection)?;
    Ok(id)
}

/// Stages one validated environment side, mirroring
/// [`stage_codegen_side`] over the env generation layout.
fn stage_env_side(
    workspace: &Path,
    plan: &dx_env_plan::CollectedPlan,
    projection: &[dx_env_plan::ProjectionEntry],
) -> Result<dx_setup::GenerationId, (String, String)> {
    let id = dx_setup::GenerationId::new(&plan.hex()).map_err(|err| {
        // LCOV_EXCL_START - reason: defense-in-depth; plan digests always render valid generation ids, so construction cannot fail; retained so a future divergence fails closed as invalid_result rather than panicking.
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid env plan digest: {err}"),
        )
        // LCOV_EXCL_STOP - reason: end of unreachable digest-construction exclusion.
    })?;
    stage_env_generation(workspace, &id, projection)?;
    Ok(id)
}

/// Collects, validates, and stages the prepared sides for one managed
/// command without committing: independent commands always prepare
/// their own side (even an empty plan, which clears a stale selection,
/// paired downstream with the managed empty counterpart), while an
/// exact `dx setup` leaves a side with no contributing target
/// unprepared so the commit carries the current generation forward (or
/// the managed empty generation on first selection). Repository setup
/// always prepares both canonical sides. Staged-but-unselected
/// generations are ordinary retained cache, never selection state.
fn prepare_managed_sides(
    command: Command,
    repository: bool,
    workspace: &Path,
    bep: &Path,
) -> Result<dx_setup::PreparedSides, (String, String)> {
    let empties = || -> Result<dx_setup::PreparedSides, (String, String)> {
        Ok(dx_setup::PreparedSides {
            prepared_environment: None,
            prepared_generated: None,
            empty_environment: empty_env_id()?,
            empty_generated: empty_generated_id()?,
        })
    };
    match command {
        Command::Codegen => {
            let (_, plan, projection) = collect_managed_codegen(bep)?;
            let generated = stage_codegen_side(workspace, &plan, &projection)?;
            Ok(dx_setup::PreparedSides {
                prepared_generated: Some(generated),
                ..empties()?
            })
        }
        Command::Env => {
            let (_, plan, projection) = collect_managed_env(bep)?;
            let environment = stage_env_side(workspace, &plan, &projection)?;
            Ok(dx_setup::PreparedSides {
                prepared_environment: Some(environment),
                ..empties()?
            })
        }
        Command::Setup => {
            let (codegen_outputs, codegen_plan, codegen_projection) = collect_managed_codegen(bep)?;
            let (env_outputs, env_plan, env_projection) = collect_managed_env(bep)?;
            let prepared_generated = if repository || !codegen_outputs.is_empty() {
                Some(stage_codegen_side(
                    workspace,
                    &codegen_plan,
                    &codegen_projection,
                )?)
            } else {
                None
            };
            let prepared_environment = if repository || !env_outputs.is_empty() {
                Some(stage_env_side(workspace, &env_plan, &env_projection)?)
            } else {
                None
            };
            Ok(dx_setup::PreparedSides {
                prepared_environment,
                prepared_generated,
                ..empties()?
            })
        }
        // LCOV_EXCL_START - reason: defense-in-depth; execute routes only managed commands here, so this arm is unreachable; retained to fail closed as invalid_result instead of panicking.
        _ => {
            debug_assert!(false, "managed dispatch guards commands");
            Err((
                CODE_INVALID_RESULT.to_owned(),
                "unsupported managed command".to_owned(),
            ))
        } // LCOV_EXCL_STOP - reason: end of unreachable managed-dispatch arm.
    }
}

/// Runs `dx codegen`, `dx env`, and `dx setup` (M25 WP3/WP5):
/// validates the label-only scope through the shared setup scope rules,
/// plans the Bazel collection request with [`plan_managed`], and either
/// renders the `--dry-run` summary (planning nothing else, launching
/// nothing) or runs the live Bazel build, collects and validates the
/// plan shards, stages the immutable generations, and commits the
/// selection through one atomic `.dx/setups/current` replacement under
/// the shared O36 commit lock. Independent commits re-read the current
/// pair under the lock, so a concurrently completed opposite side is
/// carried forward instead of lost. Build, staging, validation, or
/// commit failure leaves the current setup unchanged; staged but
/// unselected generations may remain as retained cache. Argument
/// parsing guarantees text output with no quality-only options on this
/// path.
///
/// Exits `0` on `--dry-run` and on committed selection (including an
/// already-current reselection), the Bazel exit code verbatim when the
/// live build fails, `1` on operational, collection, staging, or commit
/// failures (including a scope with neither capability), and `2` on
/// scope or policy conflicts found before execution.
fn execute_managed(invocation: &Invocation, env: Env<'_>) -> i32 {
    debug_assert!(
        invocation.command.is_managed(),
        "managed dispatch guards commands"
    );
    let Env {
        workspace,
        runner,
        temp_dir,
        pid,
        nonce,
        out,
        err,
        ..
    } = env;
    if !invocation.command.is_managed() {
        return pre_exec(
            err,
            &ForwardError::UnsupportedCommand {
                command: invocation.command.name().to_owned(),
            }
            .to_string(),
        );
    }
    let scope = match dx_setup::resolve_scope(&invocation.targets) {
        Ok(scope) => scope,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
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
    let plan = match plan_managed(
        invocation.command,
        &scope,
        &invocation.bazel_options,
        bep_text,
    ) {
        Ok(plan) => plan,
        Err(error) => return pre_exec(err, &format!("{error:?}")),
    };
    // Human prose is the only output on this path: the planned
    // operation prints unless `--quiet` suppresses it, in both
    // `--dry-run` and live modes.
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    if invocation.dry_run {
        if verbose {
            let _ = writeln!(out, "{}", plan.summary);
        }
        return 0;
    }
    if verbose {
        let _ = writeln!(out, "{}", plan.summary);
    }
    let status = match runner.run(&plan.argv, workspace, &[]) {
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
        let _ = std::fs::remove_file(&bep);
        return operational(
            invocation,
            out,
            err,
            CODE_BAZEL_SIGNALLED,
            "Bazel terminated by signal",
        );
    };
    if bazel_code != 0 {
        let _ = std::fs::remove_file(&bep);
        return bazel_code;
    }
    let repository = matches!(scope, dx_setup::SetupScope::Repository);
    let sides = match prepare_managed_sides(invocation.command, repository, workspace, &bep) {
        Ok(sides) => sides,
        Err((code, message)) => {
            let _ = std::fs::remove_file(&bep);
            return operational(invocation, out, err, &code, &message);
        }
    };
    let (pair, outcome) = match dx_setup::commit_prepared(workspace, sides) {
        Ok(committed) => committed,
        Err(error) => {
            let (code, message) = map_commit_error(error);
            let _ = std::fs::remove_file(&bep);
            return operational(invocation, out, err, &code, &message);
        }
    };
    let _ = std::fs::remove_file(&bep);
    if verbose {
        let setup = dx_setup::setup_hex(&pair);
        if outcome == dx_setup::CommitOutcome::AlreadyCurrent {
            let _ = writeln!(
                out,
                "dx {}: already selected setup {setup} (environment {}, generated {})",
                invocation.command.name(),
                pair.environment.as_str(),
                pair.generated.as_str(),
            );
        } else {
            let _ = writeln!(
                out,
                "dx {}: selected setup {setup} (environment {}, generated {})",
                invocation.command.name(),
                pair.environment.as_str(),
                pair.generated.as_str(),
            );
        }
    }
    0
}

/// Runs `dx audit` planning (M26 WP1 slice 1): family selection and
/// scope defaults through `dx_audit`, never the quality aspect
/// pipeline. `--dry-run` prints the planned families and scopes and
/// exits `0`; live execution fails closed with `audit_deferred`
/// because auditor wiring, advisory acquisition, and SARIF mapping
/// land in later M26 slices (O11/O58). Audit is non-mutating.
fn execute_audit(invocation: &Invocation, env: Env<'_>) -> i32 {
    debug_assert!(
        invocation.command == Command::Audit,
        "audit dispatch guards commands"
    );
    let Env { out, err, .. } = env;
    let request = match dx_audit::plan_audit(&invocation.targets) {
        Ok(request) => request,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(_) => {}
        Err(error) => return pre_exec(err, &error.to_string()),
    }
    let families = request
        .families
        .iter()
        .map(|family| family.as_str())
        .collect::<Vec<_>>()
        .join("+");
    let scopes = request.effective_scopes().join(", ");
    let summary = format!("Running audit {families} for {scopes}");
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
                let _ = write_event(out, &event);
            }
            let finished = command_finished(0, &FinishedCounts::default());
            let _ = write_event(out, &finished);
        } else if verbose {
            let _ = writeln!(out, "{summary}");
        }
        return 0;
    }
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), false, "default") {
            let _ = write_event(out, &event);
        }
    } else if verbose {
        let _ = writeln!(out, "{summary}");
    }
    operational(
        invocation,
        out,
        err,
        CODE_AUDIT_DEFERRED,
        "audit tool execution is deferred: auditor wiring, advisory acquisition, and SARIF mapping land in later M26 slices (O11/O58); use --dry-run for planning",
    )
}

/// Runs `dx update` planning (M26 WP2 slice 1): dependency-set and
/// package selectors through `dx_update`, mutating without
/// confirmation. `--dry-run` prints the planned selection and exits
/// `0`; live execution fails closed with `update_deferred` because
/// resolver backends and per-set reporting land in later M26 slices
/// (O12).
fn execute_update(invocation: &Invocation, env: Env<'_>) -> i32 {
    debug_assert!(
        invocation.command == Command::Update,
        "update dispatch guards commands"
    );
    let Env { out, err, .. } = env;
    match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(_) => {}
        Err(error) => return pre_exec(err, &error.to_string()),
    }
    let request = dx_update::UpdateRequest::plan(&invocation.targets);
    let summary = match request.selection() {
        dx_update::UpdateSelection::AllSets => "Running update for all dependency sets".to_owned(),
        dx_update::UpdateSelection::Selected(selectors) => {
            format!("Running update for {}", selectors.join(", "))
        }
    };
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
                let _ = write_event(out, &event);
            }
            let finished = command_finished(0, &FinishedCounts::default());
            let _ = write_event(out, &finished);
        } else if verbose {
            let _ = writeln!(out, "{summary}");
        }
        return 0;
    }
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), false, "default") {
            let _ = write_event(out, &event);
        }
    } else if verbose {
        let _ = writeln!(out, "{summary}");
    }
    operational(
        invocation,
        out,
        err,
        CODE_UPDATE_DEFERRED,
        "update resolver execution is deferred: backend operation and per-set reporting land in later M26 slices (O12); use --dry-run for planning",
    )
}

/// Umbrella phases in contract order (M10 WP4, O59): format, lint,
/// typecheck, then generate freshness or mutation.
const UMBRELLA_PHASES: [Command; 4] = [
    Command::Format,
    Command::Lint,
    Command::Typecheck,
    Command::Generate,
];

/// One executed umbrella phase: the phase identity plus the
/// phase-private SARIF capture when the phase supports the format.
struct UmbrellaPhase {
    command: Command,
    sarif_capture: Option<PathBuf>,
}

/// Sequential `dx check` / `dx fix` umbrella (M10 WP4, O59): each phase
/// reuses its wrapped command's scope resolution, Bazel invocation,
/// result collection, mutation, reporting, and exit-status behavior
/// verbatim through [`execute`] with captured streams. The first
/// nonzero phase stops the umbrella; its exit code is preserved. One
/// umbrella `command_started`/`command_finished` pair brackets the
/// verbatim per-phase streams in phase order, `--output diff`
/// concatenates each executed phase's validated patch in phase order,
/// and each SARIF request merges the executed SARIF-capable phases'
/// `runs` in phase order into one document.
fn execute_umbrella(invocation: &Invocation, env: Env<'_>) -> i32 {
    let Env {
        workspace,
        runner,
        query_runner,
        temp_dir,
        pid,
        nonce,
        out,
        err,
        ci,
    } = env;
    let umbrella_check = invocation.command == Command::Check;
    // The umbrella kind dictates the phase mode; an explicit `--check`
    // additionally forces check mode under `fix` (passthrough).
    let phase_check = umbrella_check || invocation.check;
    let mode = if phase_check { "check" } else { "default" };
    // Report planning reuses the umbrella registry (SARIF only in M10):
    // dry-run conflicts, unsupported formats, and duplicates fail here
    // before any phase starts.
    if let Err(error) = plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        return pre_exec(err, &error.to_string());
    }
    // A stdout report destination would let every phase claim the
    // reserved stdout document: fail closed before execution.
    for request in &invocation.reports {
        if request.destination == "-" {
            return pre_exec(
                err,
                &format!(
                    "option \"--report={}={}\" is not supported by dx {}: phases share one stdout document",
                    request.format,
                    request.destination,
                    invocation.command.name(),
                ),
            );
        }
    }
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), invocation.dry_run, mode) {
            let _ = write_event(out, &event);
        }
    }
    let mut executed: Vec<UmbrellaPhase> = Vec::new();
    let mut stop_code: Option<i32> = None;
    for (index, phase) in UMBRELLA_PHASES.iter().enumerate() {
        // Phases share the temporary directory, so each phase derives
        // its own nonce: BEP streams and intended manifests must never
        // alias across phases, including after a failed phase that
        // leaves its artifacts behind.
        let phase_nonce = nonce.wrapping_add(index as u64);
        // Route SARIF requests through one phase-private capture when
        // the phase registry supports the format; other phases
        // contribute nothing to that request.
        let mut phase_reports = Vec::new();
        let mut sarif_capture: Option<PathBuf> = None;
        for request in &invocation.reports {
            if !spec(*phase).reports.contains(&request.format.as_str()) {
                continue;
            }
            if sarif_capture.is_none() {
                let capture = temp_dir.join(format!(
                    "umbrella-{}-{pid}-{phase_nonce}.sarif",
                    phase.name()
                ));
                let Some(capture_text) = capture.to_str() else {
                    return pre_exec(err, "temporary report path is not UTF-8"); // LCOV_EXCL_LINE - reason: defense-in-depth; capture paths join the ASCII temporary directory with ASCII phase names, so non-UTF-8 paths are unreachable.
                };
                phase_reports.push(ReportRequest {
                    format: request.format.clone(),
                    destination: capture_text.to_owned(),
                });
                sarif_capture = Some(capture);
            } // LCOV_EXCL_LINE - reason: closing brace of a fully covered guard carries no executable region of its own.
        }
        let phase_invocation = Invocation {
            command: *phase,
            check: phase_check,
            workspace: invocation.workspace.clone(),
            dry_run: invocation.dry_run,
            quiet: invocation.quiet,
            output: invocation.output,
            reports: phase_reports,
            fail_on: invocation.fail_on,
            min_coverage: invocation.min_coverage,
            targets: invocation.targets.clone(),
            bazel_options: invocation.bazel_options.clone(),
            bazel_clean: false,
            pin: None,
            rollback: false,
            configured: false,
        };
        let mut phase_out = Vec::new();
        let mut phase_err = Vec::new();
        let code = {
            let phase_env = Env {
                workspace,
                runner,
                query_runner,
                temp_dir,
                pid,
                nonce: phase_nonce,
                out: &mut phase_out,
                err: &mut phase_err,
                ci,
            };
            if *phase == Command::Generate {
                execute_generate(&phase_invocation, phase_env)
            } else {
                execute(&phase_invocation, phase_env)
            }
        };
        let _ = out.write_all(&phase_out);
        let _ = err.write_all(&phase_err);
        executed.push(UmbrellaPhase {
            command: *phase,
            sarif_capture,
        });
        if code != 0 {
            stop_code = Some(code);
            break;
        }
    }
    // Merged standard reports: one document per request over the
    // executed phases only. Absent or unparsable captures contribute
    // nothing: a completed phase always leaves a validated capture,
    // while a stopped phase's partial follows its own collection rule.
    let complete = stop_code.is_none();
    let fs = RealFileSystem;
    let mut reports_ok = true;
    for request in &invocation.reports {
        let mut runs: Vec<Value> = Vec::new();
        let mut schema = json!("https://json.schemastore.org/sarif-2.1.0.json");
        let mut version = json!("2.1.0");
        for phase in &executed {
            if !spec(phase.command)
                .reports
                .contains(&request.format.as_str())
            {
                continue;
            }
            let Some(capture) = &phase.sarif_capture else {
                continue; // LCOV_EXCL_LINE - reason: by construction a phase supporting the requested format always carries its capture, so this fallback never fires.
            }; // LCOV_EXCL_LINE - reason: closing brace of a fully covered guard carries no executable region of its own.
            let Ok(bytes) = std::fs::read(capture) else {
                continue; // LCOV_EXCL_LINE - reason: captures are written atomically by executed phases and removed only after the merge, so a missing capture file is unreachable without external interference.
            };
            let Ok(document) = serde_json::from_slice::<Value>(&bytes) else {
                continue; // LCOV_EXCL_LINE - reason: captures are rendered by render_sarif, which always emits valid JSON, so an unparsable capture is unreachable.
            };
            if runs.is_empty() {
                if let Some(value) = document.get("$schema") {
                    schema = value.clone();
                }
                if let Some(value) = document.get("version") {
                    version = value.clone();
                }
            }
            if let Some(Value::Array(phase_runs)) = document.get("runs").cloned() {
                runs.extend(phase_runs);
            }
        }
        let document = json!({
            "version": version,
            "$schema": schema,
            "runs": runs,
        })
        .to_string();
        let target = workspace.join(&request.destination);
        let parent_ok = target
            .parent()
            .is_none_or(|parent| parent.as_os_str().is_empty() || parent.is_dir());
        if !(parent_ok && fs.write_atomic(&target, document.as_bytes()).is_ok()) {
            reports_ok = false;
            let detail = format!(
                "failed to write {} report to {}",
                request.format, request.destination
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
            if let Ok(event) = report_event(&request.format, &request.destination, complete) {
                let _ = write_event(out, &event);
            }
        } else if matches!(invocation.output, OutputMode::Text { .. }) {
            let _ = writeln!(
                out,
                "Wrote {} report to {}.",
                request.format, request.destination
            );
        } else if invocation.output == OutputMode::Diff {
            let _ = writeln!(
                err,
                "Wrote {} report to {}.",
                request.format, request.destination
            );
        }
    }
    for phase in &executed {
        if let Some(capture) = &phase.sarif_capture {
            let _ = std::fs::remove_file(capture);
        }
    }
    let code = match stop_code {
        Some(phase_code) if reports_ok => phase_code,
        Some(_) => 1,
        None if reports_ok => 0,
        None => 1,
    };
    if invocation.output == OutputMode::Json {
        let _ = write_event(out, &command_finished(code, &FinishedCounts::default()));
    }
    code
}
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
    let mut threshold_ok = true;
    if verb == WorkflowVerb::Coverage {
        if let Some(minimum) = invocation.min_coverage {
            let summary = match coverage_line_rate(&lcov_documents, &|path| {
                std::fs::read_to_string(workspace.join(path)).ok()
            }) {
                Ok((covered, eligible)) if eligible > 0 => {
                    let percent = 100.0 * covered as f64 / eligible as f64;
                    let passed = covered * 100 >= u64::from(minimum) * eligible;
                    threshold_ok = passed;
                    if passed {
                        format!(
                            "coverage {percent:.2}% ({covered}/{eligible} lines) meets minimum {minimum}%"
                        )
                    } else {
                        format!(
                            "coverage_below_minimum: coverage {percent:.2}% ({covered}/{eligible} lines) below minimum {minimum}%"
                        )
                    }
                }
                Ok(_) => {
                    threshold_ok = false;
                    "coverage_below_minimum: no executable lines in the collected LCOV".to_owned()
                }
                Err(error) => {
                    threshold_ok = false;
                    format!("coverage_below_minimum: {error}")
                }
            };
            if invocation.output == OutputMode::Json {
                if !threshold_ok {
                    if let Ok(event) = dx_output::error_event(
                        CODE_COVERAGE_BELOW_MINIMUM,
                        &summary,
                        None,
                        None,
                        None,
                    ) {
                        let _ = write_event(out, &event);
                    }
                }
            } else {
                let _ = writeln!(err, "dx: {summary}");
            }
        }
    }
    let code = if bazel_code != 0 {
        bazel_code
    } else if complete && reports_ok && threshold_ok {
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
        ci,
    } = env;
    if ci {
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
    let status = match runner.run(&plan.argv, workspace, &[]) {
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
/// so this preserves Bazel's exact diagnostic and status. Shares the
/// single [`crate::plan::plan_run_targets`] builder with [`plan_run`]
/// so the launcher, startup options, and workspace policy cannot drift.
fn plan_run_multi(targets: &[String], app_args: &[String]) -> crate::plan::BuildPlan {
    crate::plan::plan_run_targets(targets, app_args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::parse;
    use crate::resolve::{QueryResult, QueryRunner};
    use dx_process::{ChildStatus, Runner};
    use dx_setup::{
        commit_pair, read_current_pair, setup_hex, GenerationId, SetupPair, ENVIRONMENTS_DIR_NAME,
        GENERATED_DIR_NAME,
    };
    use quality_result::proto::{Capability, Convergence, FileSnapshot, QualityResult, Stage};
    use quality_result::{encode_validated, SCHEMA_MAJOR, SCHEMA_MINOR};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::rc::Rc;

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
        /// Canned `DX_GENERATE_INTENDED` witness the fake runner writes
        /// for generate runs; `None` exercises the missing-witness
        /// paths.
        intended: Option<Vec<u8>>,
        /// Dispatch environments observed by the fake runner, one entry
        /// per launch in call order.
        seen_env: Rc<RefCell<Vec<Vec<(String, String)>>>>,
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
                intended: None,
                seen_env: Rc::new(RefCell::new(Vec::new())),
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
                    intended: self.intended.clone(),
                    seen_env: Rc::clone(&self.seen_env),
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
                intended: self.intended.clone(),
                seen_env: Rc::clone(&self.seen_env),
            }
        }

        fn run(&self, words: &[&str]) -> (i32, String, String) {
            self.run_with_ci(words, false)
        }

        /// `dx run` CI-gate probe: drives `execute` with the startup
        /// refusal bit set, without touching process-global
        /// environment (parallel tests share one process).
        fn run_with_ci(&self, words: &[&str], ci: bool) -> (i32, String, String) {
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
                    ci,
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
        intended: Option<Vec<u8>>,
        seen_env: Rc<RefCell<Vec<Vec<(String, String)>>>>,
    }

    impl Runner for FakeRunner {
        fn run(
            &self,
            argv: &[String],
            _cwd: &Path,
            env: &[(&str, &str)],
        ) -> io::Result<ChildStatus> {
            if self.io_error {
                return Err(io::Error::other("fake launch failure"));
            }
            self.seen_env.borrow_mut().push(
                env.iter()
                    .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                    .collect(),
            );
            if !self.skip_bep {
                if let Some(bep) = argv
                    .iter()
                    .find_map(|arg| arg.strip_prefix("--build_event_json_file="))
                {
                    std::fs::write(bep, self.bep_lines.join("\n")).expect("BEP file");
                }
            }
            if let Some(path) = env
                .iter()
                .find_map(|(key, value)| (*key == GENERATE_ENV_INTENDED).then_some(*value))
            {
                if let Some(witness) = &self.intended {
                    std::fs::write(path, witness).expect("intended witness");
                }
            }
            Ok(ChildStatus { code: self.code })
        }
    }

    /// Standard base64 for canned intended-manifest witnesses.
    fn b64(bytes: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
        for chunk in bytes.chunks(3) {
            let b0 = u32::from(chunk[0]);
            let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
            let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
            let n = (b0 << 16) | (b1 << 8) | b2;
            out.push(ALPHABET[(n >> 18) as usize & 63] as char);
            out.push(ALPHABET[(n >> 12) as usize & 63] as char);
            out.push(if chunk.len() > 1 {
                ALPHABET[(n >> 6) as usize & 63] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                ALPHABET[n as usize & 63] as char
            } else {
                '='
            });
        }
        out
    }

    /// Canned `DX_GENERATE_INTENDED` witness: one scope, the given file
    /// entries, and the given ignored-import entries.
    fn intended_witness(mode: &str, complete: bool, files: &str, ignored: &str) -> Vec<u8> {
        format!(
            concat!(
                r#"{{"schema_major":1,"schema_minor":0,"mode":"{mode}","#,
                r#""scopes":[{{"value":"//...","results_complete":{complete}}}],"#,
                r#""files":[{files}],"ignored_imports":[{ignored}]}}"#
            ),
            mode = mode,
            complete = complete,
            files = files,
            ignored = ignored,
        )
        .into_bytes()
    }

    /// One modify entry replacing `original` with `candidate` through a
    /// single full-span edit.
    fn intended_modify(path: &str, original: &[u8], candidate: &[u8]) -> String {
        format!(
            concat!(
                r#"{{"path":{path},"scope_index":0,"original_content":"{original}","#,
                r#""edits":[{{"start_byte":0,"end_byte":{end},"replacement":"{candidate}"}}]}}"#
            ),
            path = serde_json::to_string(path).expect("path JSON"),
            original = b64(original),
            end = original.len(),
            candidate = b64(candidate),
        )
    }

    /// One ignored-import audit entry.
    fn intended_ignored(path: &str, language: &str, import: &str) -> String {
        format!(
            r#"{{"path":{path},"language":{language},"import":{import},"scope_index":0}}"#,
            path = serde_json::to_string(path).expect("path JSON"),
            language = serde_json::to_string(language).expect("language JSON"),
            import = serde_json::to_string(import).expect("import JSON"),
        )
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
                ci: false,
            },
        );
        assert_eq!(code, 1);
        assert!(String::from_utf8(err)
            .expect("stderr")
            .contains("dx: unreadable_bep: temporary event path is not UTF-8"));
    }

    /// Argv-recording launcher probe for passthrough tests: the
    /// shared `FakeRunner` never observes argv, which is the whole
    /// contract under test here.
    struct ArgvProbe {
        code: Option<i32>,
        seen: Rc<RefCell<Vec<Vec<String>>>>,
    }

    impl Runner for ArgvProbe {
        fn run(
            &self,
            argv: &[String],
            _cwd: &Path,
            _env: &[(&str, &str)],
        ) -> io::Result<ChildStatus> {
            self.seen.borrow_mut().push(argv.to_vec());
            Ok(ChildStatus { code: self.code })
        }
    }

    #[test]
    fn bazel_forwards_argv_verbatim_and_exit_code() {
        let harness = Harness::new("bazel-passthrough");
        let seen = Rc::new(RefCell::new(Vec::new()));
        let probe = ArgvProbe {
            code: Some(3),
            seen: Rc::clone(&seen),
        };
        let inv = invocation(&["bazel", "build", "//...", "--", "--jobs=4"]);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute(
            &inv,
            Env {
                workspace: &harness.workspace,
                runner: &probe,
                query_runner: &harness.query,
                temp_dir: &harness.temp,
                pid: std::process::id(),
                nonce: 0,
                out: &mut out,
                err: &mut err,
                ci: false,
            },
        );
        assert_eq!(code, 3);
        let seen = seen.borrow();
        assert_eq!(seen.len(), 1);
        assert_eq!(
            seen[0],
            vec![
                "bazel".to_owned(),
                "build".to_owned(),
                "//...".to_owned(),
                "--jobs=4".to_owned(),
            ]
        );
        assert!(String::from_utf8(out)
            .expect("stdout")
            .contains("Running bazel build //... --jobs=4"));
    }

    #[test]
    fn bazel_dry_run_launches_nothing() {
        let harness = Harness::new("bazel-dry");
        let seen = Rc::new(RefCell::new(Vec::new()));
        let probe = ArgvProbe {
            code: Some(0),
            seen: Rc::clone(&seen),
        };
        let inv = invocation(&["--dry-run", "bazel", "version"]);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute(
            &inv,
            Env {
                workspace: &harness.workspace,
                runner: &probe,
                query_runner: &harness.query,
                temp_dir: &harness.temp,
                pid: std::process::id(),
                nonce: 0,
                out: &mut out,
                err: &mut err,
                ci: false,
            },
        );
        assert_eq!(code, 0);
        assert!(seen.borrow().is_empty());
        assert!(String::from_utf8(out)
            .expect("stdout")
            .contains("Running bazel version"));
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

    const HALF_LCOV: &str = "SF:src/a.py\nDA:1,1\nDA:2,0\nend_of_record\n";

    fn coverage_harness(name: &str, tracefile: &[u8]) -> Harness {
        let harness = Harness::new(name);
        let uri = write_bep_artifact(&harness, "coverage.dat", tracefile);
        Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.lcov"), uri)],
            )]),
            ..harness
        }
    }

    #[test]
    fn coverage_min_coverage_passes_at_threshold() {
        let harness = coverage_harness("cov-threshold-ok", MINIMAL_LCOV.as_bytes());
        let (code, _, err) = harness.run(&["coverage", "--output=text", "--min-coverage=100"]);
        assert_eq!(code, 0, "{err}");
        assert!(err.contains("meets minimum 100%"), "{err}");
    }

    #[test]
    fn coverage_min_coverage_fails_below_threshold() {
        let harness = coverage_harness("cov-threshold-low", HALF_LCOV.as_bytes());
        let (code, _, err) = harness.run(&["coverage", "--output=text", "--min-coverage=80"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("coverage_below_minimum"), "{err}");
        assert!(err.contains("50.00% (1/2 lines)"), "{err}");
    }

    #[test]
    fn coverage_min_coverage_failure_reports_json_event() {
        let harness = coverage_harness("cov-threshold-json", HALF_LCOV.as_bytes());
        let (code, out, _) = harness.run(&["coverage", "--output=json", "--min-coverage=80"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("coverage_below_minimum"), "{out}");
    }

    #[test]
    fn coverage_min_coverage_fails_without_executable_lines() {
        let harness = coverage_harness("cov-threshold-empty", b"SF:src/a.py\nend_of_record\n");
        let (code, _, err) = harness.run(&["coverage", "--output=text", "--min-coverage=80"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("no executable lines"), "{err}");
    }

    #[test]
    fn coverage_min_coverage_rejects_invalid_markers() {
        let harness = Harness::new("cov-threshold-markers");
        let source = harness.workspace.join("src/lib.rs");
        std::fs::create_dir_all(source.parent().expect("parent")).expect("mkdir");
        std::fs::write(&source, "// LCOV_EXCL_LINE\nfn a() {}\n").expect("write");
        let uri = write_bep_artifact(
            &harness,
            "coverage.dat",
            b"SF:src/lib.rs\nDA:1,1\nDA:2,1\nend_of_record\n",
        );
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.lcov"), uri)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&["coverage", "--output=text", "--min-coverage=80"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("coverage_below_minimum"), "{err}");
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
            min_coverage: None,
            targets: vec!["//app:bin".to_owned()],
            bazel_options: Vec::new(),
            bazel_clean: false,
            pin: None,
            rollback: false,
            configured: false,
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
                ci: false,
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
        // The refusal bit travels inside `Env`, never through
        // process-global environment: parallel test threads share one
        // process, so `set_var("CI", ...)` here used to flake
        // unrelated `run` tests with spurious CI refusals.
        let harness = Harness::new("run-ci");
        let (code, _, err) = harness.run_with_ci(&["run", "//app:bin"], true);
        assert_eq!(code, 2, "{err}");
        assert!(err.contains("CI=true"), "{err}");
        // The same invocation without the bit proceeds past the gate
        // (here: into ambiguous-runnable resolution).
        std::fs::create_dir_all(harness.workspace.join("app")).expect("dir");
        harness.query.script_owners("//app:two\n//app:one\n");
        let (code, _, err) = harness.run(&["run", "app"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("ambiguous_runnable"), "{err}");
    }

    /// Default-mode witness for one `rust/hello/BUILD.bazel` modify
    /// (`abc` to `xyz`); the workspace holds `workspace_text` so the
    /// caller selects the write outcome.
    fn generate_witness(workspace_text: &str, name: &str) -> Harness {
        let mut harness = Harness::new(name);
        harness.write_source("rust/hello/BUILD.bazel", workspace_text);
        harness.intended = Some(intended_witness(
            "default",
            true,
            &intended_modify("rust/hello/BUILD.bazel", b"abc\n", b"xyz\n"),
            "",
        ));
        harness
    }

    #[test]
    fn generate_runs_repo_wide_and_preserves_bazel_status() {
        let harness = generate_witness("xyz\n", "generate-text");
        let (code, out, err) = harness.run(&["generate", "--output=text"]);
        assert_eq!(code, 0, "{err}");
        assert!(out.contains("Running generate for //..."), "{out}");
        assert!(out.contains("Modified rust/hello/BUILD.bazel"), "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            harness.query.calls.borrow().is_empty(),
            "repo-wide generate issues no ownership queries"
        );

        let mut failing = generate_witness("xyz\n", "generate-fails");
        failing.bazel_code = 2;
        let (code, _, _) = failing.run(&["generate", "--output=text"]);
        assert_eq!(code, 2);
    }

    #[test]
    fn generate_json_reports_changes_mutations_and_notices() {
        let mut harness = Harness::new("generate-json");
        harness.write_source("rust/hello/BUILD.bazel", "xyz\n");
        harness.intended = Some(intended_witness(
            "default",
            true,
            &intended_modify("rust/hello/BUILD.bazel", b"abc\n", b"xyz\n"),
            &intended_ignored("rust/hello/BUILD.bazel", "rust", "serde"),
        ));
        let (code, out, err) = harness.run(&["generate", "--output=json"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("\"command_started\""), "{out}");
        assert!(out.contains("\"mode\":\"default\""), "{out}");
        assert!(out.contains("\"event\":\"change\""), "{out}");
        assert!(out.contains("\"event\":\"mutation\""), "{out}");
        assert!(out.contains("\"outcome\":\"applied\""), "{out}");
        assert!(out.contains("\"code\":\"ignored_import\""), "{out}");
        assert!(out.contains("\"results_complete\":true"), "{out}");
        assert!(
            out.contains("\"changes\":{\"create\":0,\"modify\":1}"),
            "{out}"
        );
        assert!(
            out.contains("\"mutations\":{\"applied\":1,\"not_applied\":0}"),
            "{out}"
        );
        assert!(out.contains("\"exit_code\":0"), "{out}");
    }

    #[test]
    fn generate_failure_without_witness_stays_incomplete() {
        for (name, bazel_code, exit_code) in
            [("generate-json-ok", 0, 1), ("generate-json-fail", 2, 2)]
        {
            let mut harness = Harness::new(name);
            harness.bazel_code = bazel_code;
            let (code, out, _) = harness.run(&["generate", "--output=json"]);
            assert_eq!(code, exit_code, "{out}");
            assert!(out.contains("\"command_started\""), "{out}");
            assert!(out.contains("\"command_finished\""), "{out}");
            assert!(out.contains("\"results_complete\":false"), "{out}");
            assert!(!out.contains("\"changes\""), "{out}");
            assert!(!out.contains("\"mutations\""), "{out}");
            assert!(!out.contains("\"event\":\"change\""), "{out}");
            assert!(!out.contains("\"event\":\"mutation\""), "{out}");
            if bazel_code == 0 {
                assert!(out.contains("\"code\":\"invalid_result\""), "{out}");
            }
        }
    }

    #[test]
    fn generate_success_without_witness_fails_closed_in_text() {
        let harness = Harness::new("generate-no-witness");
        let (code, out, err) = harness.run(&["generate", "--output=text"]);
        assert_eq!(code, 1, "{out}");
        assert!(err.contains("invalid_result"), "{err}");
        assert!(err.contains("without a result manifest"), "{err}");
        assert!(!out.contains("Modified"), "{out}");
    }

    #[test]
    fn generate_malformed_witness_fails_closed() {
        let mut harness = Harness::new("generate-malformed");
        harness.intended = Some(b"not json".to_vec());
        let (code, out, err) = harness.run(&["generate", "--output=json"]);
        assert_eq!(code, 1, "{out}");
        assert!(err.contains("invalid_result"), "{err}");
        assert!(out.contains("\"code\":\"invalid_result\""), "{out}");
        assert!(!out.contains("\"event\":\"change\""), "{out}");
        assert!(!out.contains("\"event\":\"mutation\""), "{out}");
        assert!(out.contains("\"results_complete\":false"), "{out}");
    }

    #[test]
    fn generate_invalid_witness_fails_closed() {
        // Structurally sound JSON that crate validation rejects: a
        // check run that did not finish its scope.
        let mut harness = Harness::new("generate-invalid");
        harness.intended = Some(intended_witness("check", false, "", ""));
        let (code, _, err) = harness.run(&["generate", "--check", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("invalid_result"), "{err}");
    }

    #[test]
    fn generate_partial_default_reports_attempted_prefix() {
        // A late Gazelle failure after a valid witness still reports
        // the validated prefix, then keeps the Gazelle code.
        let mut harness = generate_witness("xyz\n", "generate-partial");
        harness.bazel_code = 2;
        let (code, out, _) = harness.run(&["generate", "--output=json"]);
        assert_eq!(code, 2, "{out}");
        assert!(out.contains("\"event\":\"change\""), "{out}");
        assert!(out.contains("\"event\":\"mutation\""), "{out}");
        assert!(out.contains("\"results_complete\":false"), "{out}");
        assert!(out.contains("\"exit_code\":2"), "{out}");
    }

    #[test]
    fn generate_not_applied_mutation_fails() {
        // The workspace disagrees with the intended bytes: Gazelle did
        // not (or not yet) write them, so the run fails with a
        // machine-readable reason.
        let harness = generate_witness("stale\n", "generate-not-applied");
        let (code, out, err) = harness.run(&["generate", "--output=json"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(out.contains("\"outcome\":\"not_applied\""), "{out}");
        assert!(out.contains("write_mismatch"), "{out}");
        assert!(out.contains("\"exit_code\":1"), "{out}");
    }

    #[test]
    fn generate_check_reports_changes_without_mutations() {
        let mut harness = Harness::new("generate-check-json");
        harness.intended = Some(intended_witness(
            "check",
            true,
            &intended_modify("rust/hello/BUILD.bazel", b"abc\n", b"xyz\n"),
            &intended_ignored("rust/hello/BUILD.bazel", "rust", "serde"),
        ));
        let (code, out, err) = harness.run(&["generate", "--check", "--output=json"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(out.contains("\"mode\":\"check\""), "{out}");
        assert!(out.contains("\"event\":\"change\""), "{out}");
        assert!(!out.contains("\"event\":\"mutation\""), "{out}");
        assert!(out.contains("\"code\":\"ignored_import\""), "{out}");
        assert!(!out.contains("\"mutations\""), "{out}");
        assert!(out.contains("\"exit_code\":1"), "{out}");
    }

    #[test]
    fn generate_check_reports_changes_despite_diff_exit() {
        // Upstream `-mode diff` exits 1 (`ErrDiff`) exactly when the
        // witness carries changes, after `AfterResolvingDeps` wrote it:
        // the complete witness still reports them with no mutations.
        let mut harness = Harness::new("generate-check-diff-exit");
        harness.intended = Some(intended_witness(
            "check",
            true,
            &intended_modify("rust/hello/BUILD.bazel", b"abc\n", b"xyz\n"),
            &intended_ignored("rust/hello/BUILD.bazel", "rust", "serde"),
        ));
        harness.bazel_code = 1;
        let (code, out, err) = harness.run(&["generate", "--check", "--output=json"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(out.contains("\"mode\":\"check\""), "{out}");
        assert!(out.contains("\"event\":\"change\""), "{out}");
        assert!(!out.contains("\"event\":\"mutation\""), "{out}");
        assert!(out.contains("\"code\":\"ignored_import\""), "{out}");
        assert!(!out.contains("\"mutations\""), "{out}");
        assert!(out.contains("\"exit_code\":1"), "{out}");
    }

    #[test]
    fn generate_check_text_lists_changes_and_clean_succeeds() {
        let mut harness = Harness::new("generate-check-text");
        harness.intended = Some(intended_witness(
            "check",
            true,
            &intended_modify("rust/hello/BUILD.bazel", b"abc\n", b"xyz\n"),
            "",
        ));
        let (code, out, err) = harness.run(&["generate", "--check", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(out.contains("Running generate for //..."), "{out}");
        assert!(out.contains("Modified rust/hello/BUILD.bazel"), "{out}");

        let mut clean = Harness::new("generate-check-clean");
        clean.intended = Some(intended_witness("check", true, "", ""));
        let (code, out, err) = clean.run(&["generate", "--check", "--output=text"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Running generate for //..."), "{out}");
        assert!(!out.contains("Modified"), "{out}");
    }

    #[test]
    fn generate_incomplete_check_reports_nothing() {
        // Check mode without a successful Gazelle run carries no
        // trustworthy witness: no changes, Gazelle's code kept.
        let mut harness = Harness::new("generate-incomplete-check");
        harness.write_source("rust/hello/BUILD.bazel", "xyz\n");
        harness.intended = Some(intended_witness(
            "check",
            false,
            &intended_modify("rust/hello/BUILD.bazel", b"abc\n", b"xyz\n"),
            "",
        ));
        harness.bazel_code = 2;
        let (code, out, _) = harness.run(&["generate", "--check", "--output=json"]);
        assert_eq!(code, 2, "{out}");
        assert!(!out.contains("\"event\":\"change\""), "{out}");
        assert!(!out.contains("\"event\":\"mutation\""), "{out}");
        assert!(out.contains("\"results_complete\":false"), "{out}");
    }

    #[test]
    fn generate_diff_renders_validated_patch_only() {
        let harness = generate_witness("xyz\n", "generate-diff");
        let (code, out, err) = harness.run(&["generate", "--output=diff"]);
        assert_eq!(code, 0, "{err}");
        assert!(out.contains("rust/hello/BUILD.bazel"), "{out}");
        assert!(out.contains("-abc"), "{out}");
        assert!(out.contains("+xyz"), "{out}");
        assert!(!out.contains("Running generate"), "{out}");
        assert!(!out.contains("Ignored import"), "{out}");
        assert_eq!(err, "", "{err}");
    }

    #[test]
    fn generate_dispatch_env_carries_scope_mode_and_witness() {
        let harness = generate_witness("xyz\n", "generate-dispatch");
        let (code, out, err) = harness.run(&["generate", "//a:one", "--output=text"]);
        assert_eq!(code, 0, "{out}{err}");
        let seen = harness.seen_env.borrow();
        assert_eq!(seen.len(), 1, "{seen:?}");
        let env: std::collections::HashMap<&str, &str> = seen[0]
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        assert_eq!(
            env.get(GENERATE_ENV_SCOPE).copied(),
            Some(r#"[{"dirs":["a"],"element":"//a:one"}]"#),
            "{seen:?}"
        );
        assert_eq!(env.get(GENERATE_ENV_MODE).copied(), Some("default"));
        let intended = env
            .get(GENERATE_ENV_INTENDED)
            .copied()
            .expect("witness path");
        assert!(intended.ends_with(".json"), "{intended}");
        assert_eq!(
            std::fs::read(intended).expect("witness bytes"),
            harness.intended.expect("canned witness"),
            "runner observes the dispatched witness path"
        );

        let mut check = Harness::new("generate-dispatch-check");
        check.intended = Some(intended_witness("check", true, "", ""));
        let (code, _, err) = check.run(&["generate", "--check", "--output=text"]);
        assert_eq!(code, 0, "{err}");
        let seen = check.seen_env.borrow();
        assert_eq!(
            seen[0]
                .iter()
                .find_map(|(key, value)| (*key == GENERATE_ENV_MODE).then_some(value.as_str())),
            Some("check"),
            "{seen:?}"
        );
    }

    #[test]
    fn generate_dry_run_plans_without_launching() {
        // A nonzero Bazel code would surface if the runner launched:
        // dry-run plans only.
        let mut harness = Harness::new("generate-dryrun");
        harness.bazel_code = 3;
        let (code, out, _) = harness.run(&["generate", "--dry-run", "--output=text"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("Running generate for //..."), "{out}");

        let harness = Harness::new("generate-dryrun-json");
        let (code, out, _) = harness.run(&["generate", "--dry-run", "--output=json"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("\"command_started\""), "{out}");
        assert!(out.contains("\"command_finished\""), "{out}");

        let mut quiet = Harness::new("generate-quiet");
        quiet.intended = Some(intended_witness("default", true, "", ""));
        let (code, out, _) = quiet.run(&["generate", "--quiet"]);
        assert_eq!(code, 0, "{out}");
        assert_eq!(out, "", "{out:?}");
    }

    #[test]
    fn generate_scoped_run_resolves_labels_without_queries() {
        let harness = generate_witness("xyz\n", "generate-scoped");
        let (code, out, err) = harness.run(&["generate", "//a:one", "--output=text"]);
        assert_eq!(code, 0, "{err}");
        assert!(out.contains("Running generate for //a:one"), "{out}");
        assert!(
            harness.query.calls.borrow().is_empty(),
            "label scope issues no ownership queries"
        );
    }

    #[test]
    fn generate_unresolvable_scope_fails_pre_exec() {
        let harness = Harness::new("generate-bad-scope");
        let (code, _, err) = harness.run(&["generate", "no/such/dir"]);
        assert_eq!(code, 2, "{err}");
    }

    #[test]
    fn generate_reports_and_conflicts_fail_pre_exec() {
        let harness = Harness::new("generate-report");
        let (code, _, err) = harness.run(&["generate", "--report=sarif=out.sarif"]);
        assert_eq!(code, 2, "{err}");
        assert!(err.contains("no standard report exists"), "{err}");

        let harness = Harness::new("generate-conflict");
        let (code, _, err) = harness.run(&[
            "generate",
            "--",
            "--@rules_dx//config:workspace=//other:config",
        ]);
        assert_eq!(code, 2, "{err}");
    }

    #[test]
    fn generate_launch_failure_and_signal_are_operational() {
        let mut harness = Harness::new("generate-launchfail");
        harness.io_error = true;
        let (code, out, err) = harness.run(&["generate", "--output=json"]);
        assert_eq!(code, 1, "{err}");
        assert!(out.contains("launch_failed"), "{out}");

        let mut harness = Harness::new("generate-signalled");
        harness.signalled = true;
        let (code, _, err) = harness.run(&["generate", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("bazel_signalled"), "{err}");
    }

    /// Managed-state fixture for the `dx clean` exec tests (issue
    /// #20): commits `pair` through the real `dx_setup` commit path and
    /// materializes both generation directories, returning the setup
    /// record hex. Digest tags mirror the `dx_clean` fixtures (one
    /// lowercase-hex character repeated to 64).
    fn commit_clean_pair(harness: &Harness, env: char, gen: char) -> String {
        let pair = SetupPair {
            environment: GenerationId::new(&env.to_string().repeat(64))
                .expect("environment digest"),
            generated: GenerationId::new(&gen.to_string().repeat(64)).expect("generated digest"),
        };
        commit_pair(&harness.workspace, &pair).expect("commit pair");
        for (dir, tag) in [(ENVIRONMENTS_DIR_NAME, env), (GENERATED_DIR_NAME, gen)] {
            std::fs::create_dir_all(
                harness
                    .workspace
                    .join(".dx")
                    .join(dir)
                    .join(tag.to_string().repeat(64)),
            )
            .expect("generation dir");
        }
        setup_hex(&pair)
    }

    #[test]
    fn clean_dry_run_empty_workspace_reports_nothing() {
        let harness = Harness::new("clean-dryrun-empty");
        let (code, out, err) = harness.run(&["clean", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("nothing to prune"), "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
        assert!(
            !harness.workspace.join(".dx").exists(),
            "dry-run creates no managed state"
        );
    }

    #[test]
    fn clean_dry_run_lists_stale_record_and_deletes_nothing() {
        let harness = Harness::new("clean-dryrun-list");
        // Commit order selects the last pair: stale first, current last.
        let stale = commit_clean_pair(&harness, '3', '4');
        let current = commit_clean_pair(&harness, '1', '2');
        let (code, out, err) = harness.run(&["clean", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains(&format!("prune setup record: .dx/setups/{stale}")),
            "{out}"
        );
        // The listing reports per-entry reclaimable bytes plus the
        // total; the stale record holds pair links (nonzero) while the
        // empty generation dirs measure zero.
        assert!(out.contains("bytes)"), "{out}");
        assert!(out.contains("reclaimable total:"), "{out}");
        assert!(
            out.contains(&format!("preserve current: .dx/setups/{current}")),
            "{out}"
        );
        assert!(
            !out.contains(&format!("prune setup record: .dx/setups/{current}")),
            "{out}"
        );
        assert!(
            harness.workspace.join(".dx/setups").join(&stale).exists(),
            "dry-run deletes nothing"
        );
        assert!(harness.seen_env.borrow().is_empty());
    }

    #[test]
    fn clean_apply_prunes_stale_record_and_orphaned_generations() {
        let harness = Harness::new("clean-apply");
        let stale = commit_clean_pair(&harness, '3', '4');
        let current = commit_clean_pair(&harness, '1', '2');
        let (code, out, err) = harness.run(&["clean"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("pruned 1 setup records and 2 generations"),
            "{out}"
        );
        assert!(out.contains("bytes reclaimed"), "{out}");
        let dx_dir = harness.workspace.join(".dx");
        assert!(!dx_dir.join("setups").join(&stale).exists());
        assert!(dx_dir.join("setups").join(&current).exists());
        assert!(!dx_dir
            .join("environments")
            .join('3'.to_string().repeat(64))
            .exists());
        assert!(!dx_dir
            .join("generated")
            .join('4'.to_string().repeat(64))
            .exists());
        assert!(
            dx_dir
                .join("environments")
                .join('1'.to_string().repeat(64))
                .exists(),
            "current generations survive"
        );
        assert_eq!(
            read_current_pair(&harness.workspace).expect("read current"),
            read_current_pair(&harness.workspace).expect("reread current"),
            "selection read is stable"
        );
        let live = read_current_pair(&harness.workspace).expect("live pair");
        let live_pair = live.expect("current still selected");
        assert_eq!(setup_hex(&live_pair), current);
    }

    #[test]
    fn clean_apply_preserves_generations_shared_with_current() {
        let harness = Harness::new("clean-apply-shared");
        commit_clean_pair(&harness, '3', '2');
        commit_clean_pair(&harness, '1', '2');
        let (code, out, err) = harness.run(&["clean"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("pruned 1 setup records and 1 generations"),
            "{out}"
        );
        assert!(out.contains("bytes reclaimed"), "{out}");
        assert!(
            harness
                .workspace
                .join(".dx/generated")
                .join('2'.to_string().repeat(64))
                .exists(),
            "generation shared with current survives"
        );
    }

    #[test]
    fn clean_bazel_forward_runs_after_prune_and_propagates_code() {
        let harness = Harness::new("clean-bazel");
        let (code, out, err) = harness.run(&["clean", "--bazel"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("nothing to prune"), "{out}");
        assert!(out.contains("re-run `dx setup`"), "{out}");
        assert_eq!(
            harness.seen_env.borrow().len(),
            1,
            "exactly one Bazel launch for the explicit forward"
        );

        let mut failing = Harness::new("clean-bazel-fail");
        failing.bazel_code = 3;
        let (code, _, err) = failing.run(&["clean", "--bazel"]);
        assert_eq!(code, 3, "{err}");

        let dry = Harness::new("clean-bazel-dryrun");
        let (code, out, err) = dry.run(&["clean", "--dry-run", "--bazel"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("would forward: bazel clean"), "{out}");
        assert!(
            dry.seen_env.borrow().is_empty(),
            "dry-run lists the forward without launching"
        );
    }

    #[test]
    fn clean_malformed_current_fails_closed_without_pruning() {
        let harness = Harness::new("clean-bad-current");
        let stale = commit_clean_pair(&harness, '3', '4');
        let pointer = harness.workspace.join(".dx/setups/current");
        std::fs::remove_file(&pointer).expect("remove pointer");
        std::fs::write(&pointer, "not a symlink").expect("file pointer");
        for args in [&["clean", "--dry-run"][..], &["clean"][..]] {
            let (code, out, err) = harness.run(args);
            assert_eq!(code, 1, "{out}{err}");
            let combined = format!("{out}{err}");
            assert!(combined.contains("clean_failed"), "{combined}");
        }
        assert!(
            harness.workspace.join(".dx/setups").join(&stale).exists(),
            "failure prunes nothing"
        );
    }

    #[test]
    fn clean_unmanaged_paths_are_refused_and_preserved() {
        let harness = Harness::new("clean-unmanaged");
        let setups = harness.workspace.join(".dx/setups");
        std::fs::create_dir_all(setups.join("notes")).expect("unmanaged record");
        let environments = harness.workspace.join(".dx/environments");
        std::fs::create_dir_all(&environments).expect("environments dir");
        std::fs::write(environments.join("README"), "operator notes").expect("unmanaged file");
        let (code, out, err) = harness.run(&["clean", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("refuse unmanaged path: README"), "{out}");
        assert!(out.contains("refuse unmanaged path: notes"), "{out}");
        let (code, out, err) = harness.run(&["clean"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("nothing to prune"), "{out}");
        assert!(setups.join("notes").exists(), "unmanaged paths survive");
        assert!(environments.join("README").exists());
    }

    #[test]
    fn managed_dry_run_prints_summary_without_launching() {
        for command in ["codegen", "env", "setup"] {
            let name = format!("managed-dryrun-{command}");
            let harness = Harness::new(&name);
            let (code, out, err) = harness.run(&[command, "--dry-run"]);
            assert_eq!(code, 0, "{out}{err}");
            assert!(
                out.contains(&format!("Running {command} for //...")),
                "{out}"
            );
            assert_eq!(err, "", "{err}");
            assert!(
                harness.seen_env.borrow().is_empty(),
                "dry-run launches nothing"
            );
        }
    }

    #[test]
    fn managed_dry_run_exact_scope_selects_label() {
        let harness = Harness::new("managed-dryrun-exact");
        let (code, out, err) = harness.run(&["env", "//a:one", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Running env for //a:one"), "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
    }

    #[test]
    fn managed_dry_run_quiet_prints_nothing() {
        let harness = Harness::new("managed-dryrun-quiet");
        let (code, out, err) = harness.run(&["setup", "--dry-run", "--quiet"]);
        assert_eq!(code, 0, "{out}{err}");
        assert_eq!(out, "", "{out}");
        assert_eq!(err, "", "{err}");
    }

    #[test]
    fn audit_dry_run_plans_families_without_launching() {
        let harness = Harness::new("audit-dryrun");
        let (code, out, err) = harness.run(&["audit", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("Running audit security+license for //..."),
            "{out}"
        );
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );

        let harness = Harness::new("audit-dryrun-family");
        let (code, out, err) = harness.run(&["audit", "security", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Running audit security for //..."), "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
    }

    #[test]
    fn audit_live_is_deferred_operational() {
        let harness = Harness::new("audit-live");
        let (code, out, err) = harness.run(&["audit"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("audit_deferred"), "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "deferred run launches nothing"
        );
    }

    #[test]
    fn update_dry_run_plans_selection_without_launching() {
        let harness = Harness::new("update-dryrun");
        let (code, out, err) = harness.run(&["update", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("Running update for all dependency sets"),
            "{out}"
        );
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );

        let harness = Harness::new("update-dryrun-selected");
        let (code, out, err) = harness.run(&["update", "crates", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Running update for crates"), "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
    }

    #[test]
    fn update_live_is_deferred_operational() {
        let harness = Harness::new("update-live");
        let (code, out, err) = harness.run(&["update"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("update_deferred"), "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "deferred run launches nothing"
        );
    }

    #[test]
    fn audit_dry_run_json_emits_lifecycle() {
        let harness = Harness::new("audit-dryrun-json");
        let (code, out, err) = harness.run(&["audit", "--dry-run", "--output=json"]);
        assert_eq!(code, 0, "{out}{err}");
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
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
    }

    #[test]
    fn audit_live_json_emits_deferred_lifecycle() {
        let harness = Harness::new("audit-live-json");
        let (code, out, err) = harness.run(&["audit", "--output=json"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("audit_deferred"), "{err}");
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
        assert!(
            harness.seen_env.borrow().is_empty(),
            "deferred run launches nothing"
        );
    }

    #[test]
    fn audit_update_dry_run_quiet_prints_nothing() {
        for argv in [
            vec!["audit", "--dry-run", "--quiet"],
            vec!["update", "--dry-run", "--quiet"],
        ] {
            let name = format!("dryrun-quiet-{}", argv[0]);
            let harness = Harness::new(&name);
            let (code, out, err) = harness.run(&argv);
            assert_eq!(code, 0, "{out}{err}");
            assert_eq!(out, "", "{out}");
            assert_eq!(err, "", "{err}");
            assert!(
                harness.seen_env.borrow().is_empty(),
                "dry-run launches nothing"
            );
        }
    }

    #[test]
    fn managed_live_empty_selection_commits_with_empty_counterparts() {
        for command in ["codegen", "env", "setup"] {
            let name = format!("managed-commit-{command}");
            let harness = Harness::new(&name);
            let (code, out, err) = harness.run(&[command]);
            assert_eq!(code, 0, "{out}{err}");
            assert!(
                out.contains(&format!("Running {command} for //...")),
                "{out}"
            );
            assert!(out.contains("selected setup "), "{out}");
            assert_eq!(err, "", "{err}");
            assert_eq!(
                harness.seen_env.borrow().len(),
                1,
                "committed selection launches one Bazel build"
            );
            // An empty plan stages the managed empty generations, so the
            // first selection pairs each prepared side with the managed
            // empty counterpart from the same digest family. Only staged
            // sides materialize; the record links to an unstaged empty
            // counterpart dangle with ordinary missing-target behavior.
            let pair = read_current_pair(&harness.workspace)
                .expect("read current")
                .expect("selection committed");
            assert_eq!(pair.environment, empty_env_id().expect("empty digest"));
            assert_eq!(pair.generated, empty_generated_id().expect("empty digest"));
            for side in match command {
                "codegen" => vec![GENERATED_DIR_NAME],
                "env" => vec![ENVIRONMENTS_DIR_NAME],
                _ => vec![ENVIRONMENTS_DIR_NAME, GENERATED_DIR_NAME],
            } {
                let id = match side {
                    GENERATED_DIR_NAME => pair.generated.as_str(),
                    _ => pair.environment.as_str(),
                };
                assert!(
                    harness.workspace.join(".dx").join(side).join(id).is_dir(),
                    "{command} stages its {side} generation"
                );
            }
            // Reselection is a no-op success reporting the current setup.
            let (code, out, err) = harness.run(&[command]);
            assert_eq!(code, 0, "{out}{err}");
            assert!(out.contains("already selected setup "), "{out}");
            assert_eq!(err, "", "{err}");
        }
    }

    #[test]
    fn managed_live_exact_sides_commit_with_empty_counterparts() {
        for command in ["codegen", "env"] {
            let name = format!("managed-exact-{command}");
            let harness = Harness::new(&name);
            let (code, out, err) = harness.run(&[command, "//a:one"]);
            assert_eq!(code, 0, "{out}{err}");
            assert!(out.contains("selected setup "), "{out}");
            let pair = read_current_pair(&harness.workspace)
                .expect("read current")
                .expect("selection committed");
            assert_eq!(pair.environment, empty_env_id().expect("empty digest"));
            assert_eq!(pair.generated, empty_generated_id().expect("empty digest"));
        }
    }

    #[test]
    fn managed_live_exact_setup_without_capability_fails_closed() {
        let harness = Harness::new("managed-setup-nocap");
        let (code, out, err) = harness.run(&["setup", "//a:one"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("dx: no_capability:"), "{err}");
        assert!(out.contains("Running setup for //a:one"), "{out}");
        assert_eq!(
            read_current_pair(&harness.workspace).expect("read current"),
            None,
            "capability failure commits nothing"
        );
    }

    #[test]
    fn managed_live_quiet_commit_prints_nothing() {
        let harness = Harness::new("managed-quiet-commit");
        let (code, out, err) = harness.run(&["codegen", "--quiet"]);
        assert_eq!(code, 0, "{out}{err}");
        assert_eq!(out, "", "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            read_current_pair(&harness.workspace)
                .expect("read current")
                .is_some(),
            "quiet still commits"
        );
    }

    #[test]
    fn managed_live_malformed_current_fails_commit_without_mutation() {
        let harness = Harness::new("managed-bad-current");
        let (code, _, _) = harness.run(&["codegen"]);
        assert_eq!(code, 0);
        let generations = harness.workspace.join(".dx").join(GENERATED_DIR_NAME);
        assert!(generations.is_dir(), "first commit stages generations");
        let pointer = harness.workspace.join(".dx/setups/current");
        std::fs::remove_file(&pointer).expect("remove pointer");
        std::fs::write(&pointer, "not a symlink").expect("file pointer");
        let (code, _, err) = harness.run(&["codegen"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("dx: managed_commit_failed:"), "{err}");
        assert!(
            generations.is_dir(),
            "commit failure preserves staged cache"
        );
        assert_eq!(
            std::fs::read(&pointer).expect("pointer bytes"),
            b"not a symlink",
            "commit failure leaves the malformed pointer untouched"
        );
    }

    /// Canned BEP stream referencing one shard file for one managed
    /// output group: the fake runner writes these lines verbatim.
    fn managed_shard_bep(shard: &Path, group: &str) -> Vec<String> {
        vec![
            format!(
                "{{\"id\": {{\"namedSet\": {{\"id\": \"0\"}}}}, \"namedSetOfFiles\": {{\"files\": [{{\"uri\": \"file://{}\"}}]}}}}",
                shard.to_string_lossy()
            ),
            format!(
                "{{\"id\": {{\"targetCompleted\": {{\"label\": \"//a:one\"}}}}, \"completed\": {{\"success\": true, \"outputGroup\": [{{\"name\": \"{group}\", \"fileSets\": [{{\"id\": \"0\"}}]}}]}}}}"
            ),
        ]
    }

    #[test]
    fn managed_live_invalid_codegen_shard_fails_closed() {
        let mut harness = Harness::new("managed-bad-codegen-shard");
        let shard = harness.temp.join("bad.dxcodegen.pb");
        std::fs::write(&shard, b"not a codegen shard").expect("shard");
        harness.raw_bep = Some(managed_shard_bep(&shard, dx_codegen::OUTPUT_GROUP));
        let (code, _, err) = harness.run(&["codegen"]);
        assert_eq!(code, 1, "{err}");
        assert!(
            err.contains("dx: invalid_result: invalid codegen plan"),
            "{err}"
        );
        assert_eq!(
            read_current_pair(&harness.workspace).expect("read current"),
            None,
            "collection failure commits nothing"
        );
    }

    #[test]
    fn managed_live_invalid_env_shard_fails_closed() {
        let mut harness = Harness::new("managed-bad-env-shard");
        let shard = harness.temp.join("bad.dxenv.pb");
        std::fs::write(&shard, b"not an env shard").expect("shard");
        harness.raw_bep = Some(managed_shard_bep(&shard, dx_env_plan::OUTPUT_GROUP));
        let (code, _, err) = harness.run(&["env"]);
        assert_eq!(code, 1, "{err}");
        assert!(
            err.contains("dx: invalid_result: invalid env plan"),
            "{err}"
        );
        assert_eq!(
            read_current_pair(&harness.workspace).expect("read current"),
            None,
            "collection failure commits nothing"
        );
    }

    #[test]
    fn managed_group_config_failure_is_operational() {
        let harness = Harness::new("managed-bad-group");
        let bep = harness.temp.join("empty.json");
        std::fs::write(&bep, "").expect("bep");
        let (code, message) = collect_managed_group(&bep, "").expect_err("empty group");
        assert_eq!(code, CODE_INVALID_BEP);
        assert!(message.contains("invalid BEP config"), "{message}");
    }

    #[test]
    fn managed_empty_sides_derive_the_managed_empty_identities() {
        let workspace = temp_dir("managed-empty-sides-ws");
        let codegen_plan = dx_codegen::collect_plan(&[]).expect("empty codegen plan");
        let staged = stage_codegen_side(&workspace, &codegen_plan, &[]).expect("stage");
        assert_eq!(staged, empty_generated_id().expect("empty digest"));
        let env_plan = dx_env_plan::collect_plan(&[]).expect("empty env plan");
        let staged = stage_env_side(&workspace, &env_plan, &[]).expect("stage");
        assert_eq!(staged, empty_env_id().expect("empty digest"));
        let values = std::fs::read_to_string(
            workspace
                .join(".dx")
                .join(ENVIRONMENTS_DIR_NAME)
                .join(empty_env_id().expect("empty digest").as_str())
                .join("values.json"),
        )
        .expect("values");
        assert_eq!(values, "{}");
    }

    #[test]
    fn managed_generation_dir_guards_foreign_state() {
        let workspace = temp_dir("managed-gendir-ws");
        let hex = empty_generated_id().expect("empty digest");
        let first =
            ensure_generation_dir(&workspace, GENERATED_DIR_NAME, hex.as_str()).expect("create");
        assert!(first.is_dir());
        let second =
            ensure_generation_dir(&workspace, GENERATED_DIR_NAME, hex.as_str()).expect("reuse");
        assert_eq!(first, second);
        // A present non-directory is never adopted.
        let other = "0".repeat(64);
        std::fs::write(
            workspace.join(".dx").join(GENERATED_DIR_NAME).join(&other),
            "foreign",
        )
        .expect("foreign file");
        let (code, message) =
            ensure_generation_dir(&workspace, GENERATED_DIR_NAME, &other).expect_err("refuse");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(
            message.contains("not a managed generation directory"),
            "{message}"
        );
        // Creation failures surface the underlying error.
        let file_workspace = workspace.join("ws-file");
        std::fs::write(&file_workspace, "not a dir").expect("workspace file");
        let (code, message) = ensure_generation_dir(&file_workspace, GENERATED_DIR_NAME, &other)
            .expect_err("workspace file");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("is not a directory"), "{message}");
    }

    #[test]
    fn managed_generation_dir_create_failure_surfaces() {
        let workspace = temp_dir("managed-gendir-create-ws");
        // `.dx/generated` as a file makes directory creation fail.
        std::fs::create_dir_all(workspace.join(".dx")).expect("dx dir");
        std::fs::write(workspace.join(".dx").join(GENERATED_DIR_NAME), "file").expect("blocker");
        let (code, message) =
            ensure_generation_dir(&workspace, GENERATED_DIR_NAME, &"1".repeat(64))
                .expect_err("create fails");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot create"), "{message}");
    }

    #[test]
    fn managed_logical_paths_validate() {
        assert!(validate_logical_path("gen/out.rs").is_ok());
        assert!(validate_logical_path("a/./b").is_ok());
        for bad in ["", "/absolute", "../escape", "a/../../escape"] {
            assert!(
                validate_logical_path(bad).is_err(),
                "logical path {bad:?} must fail"
            );
        }
    }

    #[test]
    fn managed_env_keys_validate() {
        assert!(validate_env_key("key.json").is_ok());
        for bad in ["", "a/b", "a\\b", ".", ".."] {
            assert!(validate_env_key(bad).is_err(), "env key {bad:?} must fail");
        }
    }

    /// Fresh managed workspace plus two real artifact files the tests
    /// stage mirror leaves against.
    fn managed_stage_fixture(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let workspace = temp_dir(&format!("{name}-ws"));
        let artifacts = temp_dir(&format!("{name}-artifacts"));
        let first = artifacts.join("first.txt");
        let second = artifacts.join("second.txt");
        std::fs::write(&first, "first").expect("artifact");
        std::fs::write(&second, "second").expect("artifact");
        (workspace, first, second)
    }

    fn codegen_entry(logical: &str, artifact: &Path) -> dx_codegen::ProjectionEntry {
        dx_codegen::ProjectionEntry {
            logical_path: logical.to_owned(),
            artifact: artifact.to_string_lossy().into_owned(),
            import_root: String::new(),
            namespace: String::new(),
        }
    }

    fn env_entry(key: &str, value: &str, artifact: &Path) -> dx_env_plan::ProjectionEntry {
        dx_env_plan::ProjectionEntry {
            key: key.to_owned(),
            value: value.to_owned(),
            artifact: artifact.to_string_lossy().into_owned(),
        }
    }

    /// Sets Unix permission bits; the managed suites run on Linux-only
    /// CI, so filesystem-failure injection through read-only
    /// directories is deterministic.
    fn set_mode(path: &Path, mode: u32) {
        let mut permissions = std::fs::metadata(path)
            .expect("mode metadata")
            .permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, mode);
        std::fs::set_permissions(path, permissions).expect("set mode");
    }

    #[test]
    fn managed_stage_codegen_mirrors_and_reuses_leaves() {
        let (workspace, first, second) = managed_stage_fixture("managed-codegen-mirror");
        let id = empty_generated_id().expect("empty digest");
        let projection = vec![
            codegen_entry("gen/a.txt", &first),
            codegen_entry("nested/b.txt", &second),
        ];
        stage_codegen_generation(&workspace, &id, &projection).expect("stage");
        let dir = workspace
            .join(".dx")
            .join(GENERATED_DIR_NAME)
            .join(id.as_str());
        assert_eq!(
            std::fs::read_link(dir.join("gen/a.txt")).expect("leaf"),
            first
        );
        assert_eq!(
            std::fs::read_link(dir.join("nested/b.txt")).expect("leaf"),
            second
        );
        // Restaging is exact-identity reuse: nothing changes.
        stage_codegen_generation(&workspace, &id, &projection).expect("restage");
        assert_eq!(
            std::fs::read_link(dir.join("gen/a.txt")).expect("leaf"),
            first
        );
        // A stale leaf pointing elsewhere is reconstructed.
        std::fs::remove_file(dir.join("gen/a.txt")).expect("remove leaf");
        symlink_leaf(&second, &dir.join("gen/a.txt")).expect("stale leaf");
        stage_codegen_generation(&workspace, &id, &projection).expect("repair stale");
        assert_eq!(
            std::fs::read_link(dir.join("gen/a.txt")).expect("leaf"),
            first
        );
        // A foreign regular file at a leaf is reconstructed too.
        std::fs::remove_file(dir.join("gen/a.txt")).expect("remove leaf");
        std::fs::write(dir.join("gen/a.txt"), "foreign").expect("foreign leaf");
        stage_codegen_generation(&workspace, &id, &projection).expect("repair foreign");
        assert_eq!(
            std::fs::read_link(dir.join("gen/a.txt")).expect("leaf"),
            first
        );
        // Duplicate entries resolving to the same artifact are one leaf.
        let doubled = vec![
            codegen_entry("gen/a.txt", &first),
            codegen_entry("gen/a.txt", &first),
        ];
        stage_codegen_generation(&workspace, &id, &doubled).expect("identical duplicates");
    }

    #[test]
    fn managed_stage_codegen_rejects_bad_plans() {
        let (workspace, first, second) = managed_stage_fixture("managed-codegen-reject");
        let id = empty_generated_id().expect("empty digest");
        for projection in [
            vec![codegen_entry("", &first)],
            vec![codegen_entry("/absolute", &first)],
            vec![codegen_entry("../escape", &first)],
        ] {
            let (code, message) =
                stage_codegen_generation(&workspace, &id, &projection).expect_err("bad path");
            assert_eq!(code, CODE_INVALID_RESULT);
            assert!(message.contains("invalid codegen plan"), "{message}");
        }
        let (code, message) = stage_codegen_generation(
            &workspace,
            &id,
            &[
                codegen_entry("gen/a.txt", &first),
                codegen_entry("gen/a.txt", &second),
            ],
        )
        .expect_err("conflict");
        assert_eq!(code, CODE_INVALID_RESULT);
        assert!(message.contains("multiple artifacts"), "{message}");
        // A logical path colliding with a checked-in source fails.
        std::fs::create_dir_all(workspace.join("gen")).expect("source dir");
        std::fs::write(workspace.join("gen/owned.txt"), "source").expect("source");
        let (code, message) =
            stage_codegen_generation(&workspace, &id, &[codegen_entry("gen/owned.txt", &first)])
                .expect_err("workspace collision");
        assert_eq!(code, CODE_INVALID_RESULT);
        assert!(
            message.contains("collides with a workspace source"),
            "{message}"
        );
        // Missing artifacts fail before selection; Bazel owns materialization.
        let missing = workspace.join("no-such-artifact.txt");
        let (code, message) = stage_codegen_generation(
            &workspace,
            &id,
            &[codegen_entry("gen/missing.txt", &missing)],
        )
        .expect_err("missing artifact");
        assert_eq!(code, CODE_INVALID_RESULT);
        assert!(message.contains("Bazel owns materialization"), "{message}");
        // A real directory at a leaf collides within its generation.
        let dir_id = dx_setup::GenerationId::new(&"2".repeat(64)).expect("fixture id");
        let dir = ensure_generation_dir(&workspace, GENERATED_DIR_NAME, dir_id.as_str())
            .expect("gen dir");
        std::fs::create_dir_all(dir.join("gen/blocked")).expect("blocking dir");
        let (code, message) =
            stage_codegen_generation(&workspace, &dir_id, &[codegen_entry("gen/blocked", &first)])
                .expect_err("generation collision");
        assert_eq!(code, CODE_INVALID_RESULT);
        assert!(
            message.contains("collides within its generation"),
            "{message}"
        );
        // A file where an intermediate directory belongs fails creation.
        let parent_id = dx_setup::GenerationId::new(&"3".repeat(64)).expect("fixture id");
        let parent_dir = ensure_generation_dir(&workspace, GENERATED_DIR_NAME, parent_id.as_str())
            .expect("gen dir");
        std::fs::write(parent_dir.join("sub"), "file").expect("blocking file");
        let (code, message) = stage_codegen_generation(
            &workspace,
            &parent_id,
            &[codegen_entry("sub/leaf.txt", &first)],
        )
        .expect_err("parent creation");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot create"), "{message}");
    }

    #[test]
    fn managed_stage_codegen_filesystem_failures_fail_closed() {
        let (workspace, first, second) = managed_stage_fixture("managed-codegen-fs");
        let id = empty_generated_id().expect("empty digest");
        // Top-level leaves so the generation directory itself is the
        // leaf parent under test.
        stage_codegen_generation(&workspace, &id, &[codegen_entry("a.txt", &first)])
            .expect("stage");
        let dir = workspace
            .join(".dx")
            .join(GENERATED_DIR_NAME)
            .join(id.as_str());
        set_mode(&dir, 0o555);
        // Replacing a stale leaf without write permission fails.
        let (code, message) =
            stage_codegen_generation(&workspace, &id, &[codegen_entry("a.txt", &second)])
                .expect_err("cannot replace");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot replace"), "{message}");
        // Linking a fresh leaf without write permission fails.
        let (code, message) = stage_codegen_generation(
            &workspace,
            &id,
            &[
                codegen_entry("a.txt", &first),
                codegen_entry("fresh.txt", &second),
            ],
        )
        .expect_err("cannot link");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot link"), "{message}");
        set_mode(&dir, 0o755);
    }

    #[test]
    fn managed_stage_env_mirrors_leaves_and_values() {
        let (workspace, first, second) = managed_stage_fixture("managed-env-mirror");
        let id = empty_env_id().expect("empty digest");
        let projection = vec![
            env_entry("k2", "x\"y", &second),
            env_entry("k1", "v1", &first),
        ];
        stage_env_generation(&workspace, &id, &projection).expect("stage");
        let dir = workspace
            .join(".dx")
            .join(ENVIRONMENTS_DIR_NAME)
            .join(id.as_str());
        assert_eq!(
            std::fs::read_link(dir.join("artifacts/k1")).expect("leaf"),
            first
        );
        assert_eq!(
            std::fs::read_link(dir.join("artifacts/k2")).expect("leaf"),
            second
        );
        // `values.json` is deterministic over sorted keys with JSON escaping.
        let values = std::fs::read_to_string(dir.join("values.json")).expect("values");
        assert_eq!(values, "{\"k1\":\"v1\",\"k2\":\"x\\\"y\"}");
        assert!(
            !dir.join("values.json.next").exists(),
            "staging file is always published"
        );
        // Restaging reuses exact-identity leaves and republishes values.
        stage_env_generation(&workspace, &id, &projection).expect("restage");
        assert_eq!(
            std::fs::read_link(dir.join("artifacts/k1")).expect("leaf"),
            first
        );
        // A stale leaf is reconstructed.
        std::fs::remove_file(dir.join("artifacts/k1")).expect("remove leaf");
        symlink_leaf(&second, &dir.join("artifacts/k1")).expect("stale leaf");
        stage_env_generation(&workspace, &id, &projection).expect("repair stale");
        assert_eq!(
            std::fs::read_link(dir.join("artifacts/k1")).expect("leaf"),
            first
        );
        // Identical duplicates are one leaf.
        let doubled = vec![env_entry("k1", "v1", &first), env_entry("k1", "v1", &first)];
        stage_env_generation(&workspace, &id, &doubled).expect("identical duplicates");
    }

    #[test]
    fn managed_stage_env_rejects_bad_plans() {
        let (workspace, first, second) = managed_stage_fixture("managed-env-reject");
        let id = empty_env_id().expect("empty digest");
        for key in ["", "a/b", ".", ".."] {
            let (code, message) =
                stage_env_generation(&workspace, &id, &[env_entry(key, "v", &first)])
                    .expect_err("bad key");
            assert_eq!(code, CODE_INVALID_RESULT);
            assert!(message.contains("invalid env plan"), "{message}");
        }
        for projection in [
            vec![env_entry("k", "v1", &first), env_entry("k", "v2", &first)],
            vec![env_entry("k", "v", &first), env_entry("k", "v", &second)],
        ] {
            let (code, message) =
                stage_env_generation(&workspace, &id, &projection).expect_err("conflict");
            assert_eq!(code, CODE_INVALID_RESULT);
            assert!(message.contains("multiple inputs"), "{message}");
        }
        let missing = workspace.join("no-such-artifact.txt");
        let (code, message) =
            stage_env_generation(&workspace, &id, &[env_entry("k", "v", &missing)])
                .expect_err("missing artifact");
        assert_eq!(code, CODE_INVALID_RESULT);
        assert!(message.contains("Bazel owns materialization"), "{message}");
        // A real directory at a leaf collides within its generation.
        let dir_id = dx_setup::GenerationId::new(&"4".repeat(64)).expect("fixture id");
        let dir = ensure_generation_dir(&workspace, ENVIRONMENTS_DIR_NAME, dir_id.as_str())
            .expect("gen dir");
        std::fs::create_dir_all(dir.join("artifacts")).expect("artifacts dir");
        std::fs::create_dir_all(dir.join("artifacts/k")).expect("blocking dir");
        let (code, message) =
            stage_env_generation(&workspace, &dir_id, &[env_entry("k", "v", &first)])
                .expect_err("generation collision");
        assert_eq!(code, CODE_INVALID_RESULT);
        assert!(
            message.contains("collides within its generation"),
            "{message}"
        );
        // A file where `artifacts/` belongs fails creation.
        let blocked_id = dx_setup::GenerationId::new(&"5".repeat(64)).expect("fixture id");
        let blocked_dir =
            ensure_generation_dir(&workspace, ENVIRONMENTS_DIR_NAME, blocked_id.as_str())
                .expect("gen dir");
        std::fs::write(blocked_dir.join("artifacts"), "file").expect("blocking file");
        let (code, message) =
            stage_env_generation(&workspace, &blocked_id, &[env_entry("k", "v", &first)])
                .expect_err("artifacts creation");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot create"), "{message}");
        // A directory at the staging path fails the values write.
        let staging_id = dx_setup::GenerationId::new(&"6".repeat(64)).expect("fixture id");
        let staging_dir =
            ensure_generation_dir(&workspace, ENVIRONMENTS_DIR_NAME, staging_id.as_str())
                .expect("gen dir");
        std::fs::create_dir_all(staging_dir.join("values.json.next")).expect("blocking dir");
        let (code, message) =
            stage_env_generation(&workspace, &staging_id, &[env_entry("k", "v", &first)])
                .expect_err("values staging");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot stage"), "{message}");
        // A directory at `values.json` fails the publish rename.
        let publish_id = dx_setup::GenerationId::new(&"7".repeat(64)).expect("fixture id");
        let publish_dir =
            ensure_generation_dir(&workspace, ENVIRONMENTS_DIR_NAME, publish_id.as_str())
                .expect("gen dir");
        std::fs::create_dir_all(publish_dir.join("values.json")).expect("blocking dir");
        let (code, message) =
            stage_env_generation(&workspace, &publish_id, &[env_entry("k", "v", &first)])
                .expect_err("values publish");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot publish"), "{message}");
    }

    #[test]
    fn managed_stage_env_filesystem_failures_fail_closed() {
        let (workspace, first, second) = managed_stage_fixture("managed-env-fs");
        let id = empty_env_id().expect("empty digest");
        stage_env_generation(&workspace, &id, &[env_entry("k", "v", &first)]).expect("stage");
        let dir = workspace
            .join(".dx")
            .join(ENVIRONMENTS_DIR_NAME)
            .join(id.as_str());
        // Leaves live under `artifacts/`, so that directory is the leaf
        // parent under test; `values.json` still publishes above it.
        set_mode(&dir.join("artifacts"), 0o555);
        let (code, message) =
            stage_env_generation(&workspace, &id, &[env_entry("k", "v", &second)])
                .expect_err("cannot replace");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot replace"), "{message}");
        let (code, message) =
            stage_env_generation(&workspace, &id, &[env_entry("fresh", "v", &second)])
                .expect_err("cannot link");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot link"), "{message}");
        set_mode(&dir.join("artifacts"), 0o755);
    }

    #[test]
    fn managed_commit_errors_map_to_stable_codes() {
        let (code, message) = map_commit_error(dx_setup::CommitError::NoCapability);
        assert_eq!(code, CODE_MANAGED_NO_CAPABILITY);
        assert!(
            message.contains("neither environment nor codegen"),
            "{message}"
        );
        let (code, _) = map_commit_error(dx_setup::CommitError::WorkspaceRoot {
            path: PathBuf::from("missing"),
        });
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
    }

    #[test]
    fn managed_live_launch_failure_is_operational() {
        let harness = Harness {
            io_error: true,
            ..Harness::new("managed-launch-failed")
        };
        let (code, _, err) = harness.run(&["codegen"]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: launch_failed: failed to launch Bazel"));
    }

    #[test]
    fn managed_live_signalled_bazel_is_operational() {
        let harness = Harness {
            signalled: true,
            ..Harness::new("managed-signalled")
        };
        let (code, _, err) = harness.run(&["env"]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: bazel_signalled: Bazel terminated by signal"));
    }

    #[test]
    fn managed_live_bazel_failure_returns_exit_verbatim() {
        let harness = Harness {
            bazel_code: 3,
            ..Harness::new("managed-bazel-failed")
        };
        let (code, out, err) = harness.run(&["setup"]);
        assert_eq!(code, 3, "{out}{err}");
        assert!(out.contains("Running setup for //..."), "{out}");
    }

    #[test]
    fn managed_live_missing_bep_is_operational() {
        let harness = Harness {
            skip_bep: true,
            ..Harness::new("managed-missing-bep")
        };
        let (code, _, err) = harness.run(&["codegen"]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: unreadable_bep: failed to read build events"));
    }

    #[test]
    fn managed_live_malformed_bep_is_operational() {
        let harness = Harness {
            raw_bep: Some(vec!["{not json".to_owned()]),
            ..Harness::new("managed-bad-bep")
        };
        let (code, _, err) = harness.run(&["env"]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: invalid_bep: invalid build events"));
    }

    #[test]
    fn managed_policy_conflict_fails_before_execution() {
        let harness = Harness::new("managed-conflict");
        let (code, _, _) = harness.run(&["setup", "--", "--aspects=//other.bzl%aspect"]);
        assert_eq!(code, 2);
        assert!(
            harness.seen_env.borrow().is_empty(),
            "policy conflict launches nothing"
        );
    }

    /// Umbrella fixture: clean quality results plus a clean
    /// check-mode generation witness, so every phase succeeds.
    fn umbrella_clean(name: &str) -> Harness {
        let mut harness = Harness::new(name);
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![], vec![]),
        );
        harness.intended = Some(intended_witness("check", true, "", ""));
        harness
    }

    /// Umbrella fixture: one fixable lint finding, so the format
    /// phase reports changes in check mode and applies them in
    /// default mode (staling the replayed fixture downstream).
    fn umbrella_findings(name: &str) -> Harness {
        let mut harness = Harness::new(name);
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![Harness::diagnostic("unused", true)],
                vec![harness.replacement(b"y")],
            ),
        );
        harness
    }

    #[test]
    fn check_clean_runs_every_phase_in_order() {
        let harness = umbrella_clean("umbrella-check-clean");
        let (code, out, err) = harness.run(&["check", "--output=text"]);
        assert_eq!(code, 0, "{out}{err}");
        let format = out
            .find("Running format analysis for //...")
            .expect("format");
        let lint = out.find("Running lint analysis for //...").expect("lint");
        let typecheck = out
            .find("Running typecheck analysis for //...")
            .expect("typecheck");
        let generate = out.find("Running generate for //...").expect("generate");
        assert!(
            format < lint && lint < typecheck && typecheck < generate,
            "{out}"
        );
        assert_eq!(err, "", "{err}");
        assert_eq!(
            harness.seen_env.borrow().len(),
            4,
            "one Bazel launch per phase"
        );
    }

    #[test]
    fn check_json_brackets_phase_lifecycles() {
        let harness = umbrella_clean("umbrella-check-json");
        let (code, out, err) = harness.run(&["check", "--output=json"]);
        assert_eq!(code, 0, "{out}{err}");
        let events: Vec<serde_json::Value> = out
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        assert_eq!(events.first().expect("first")["event"], "command_started");
        assert_eq!(events.first().expect("first")["command"], "check");
        assert_eq!(events.last().expect("last")["event"], "command_finished");
        assert_eq!(events.last().expect("last")["exit_code"], 0);
        let started: Vec<&str> = events
            .iter()
            .filter(|event| event["event"] == "command_started")
            .map(|event| event["command"].as_str().expect("command"))
            .collect();
        assert_eq!(
            started,
            vec!["check", "format", "lint", "typecheck", "generate"]
        );
    }

    #[test]
    fn check_stops_at_first_failing_phase() {
        let harness = umbrella_findings("umbrella-stop");
        let (code, out, _) = harness.run(&["check", "--output=text"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("Running format analysis for //..."), "{out}");
        assert!(!out.contains("Running lint analysis"), "{out}");
        assert!(!out.contains("Running typecheck analysis"), "{out}");
        assert!(!out.contains("Running generate"), "{out}");
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n",
            "check mode never mutates"
        );
        assert_eq!(
            harness.seen_env.borrow().len(),
            1,
            "later phases never launch"
        );
    }

    #[test]
    fn check_json_stop_reports_umbrella_failure() {
        let harness = umbrella_findings("umbrella-stop-json");
        let (code, out, _) = harness.run(&["check", "--output=json"]);
        assert_eq!(code, 1, "{out}");
        let events: Vec<serde_json::Value> = out
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        assert_eq!(events.first().expect("first")["command"], "check");
        assert_eq!(events.last().expect("last")["event"], "command_finished");
        assert_eq!(events.last().expect("last")["exit_code"], 1);
        let started: Vec<&str> = events
            .iter()
            .filter(|event| event["event"] == "command_started")
            .map(|event| event["command"].as_str().expect("command"))
            .collect();
        assert_eq!(started, vec!["check", "format"]);
    }

    #[test]
    fn fix_check_flag_forces_check_mode() {
        let harness = umbrella_findings("umbrella-fix-check");
        let (code, out, _) = harness.run(&["fix", "--check", "--output=text"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("Running format analysis for //..."), "{out}");
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n",
            "forced check mode never mutates"
        );
        assert_eq!(
            harness.seen_env.borrow().len(),
            1,
            "later phases never launch"
        );
    }

    #[test]
    fn fix_applies_generate_mutation_after_clean_quality() {
        let mut harness = Harness::new("umbrella-fix");
        harness.write_source("src/a.py", "x = 1\n");
        harness.write_source("rust/hello/BUILD.bazel", "xyz\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![], vec![]),
        );
        harness.intended = Some(intended_witness(
            "default",
            true,
            &intended_modify("rust/hello/BUILD.bazel", b"abc\n", b"xyz\n"),
            "",
        ));
        let (code, out, err) = harness.run(&["fix", "--output=text"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Modified rust/hello/BUILD.bazel"), "{out}");
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n"
        );
    }

    #[test]
    fn fix_stops_when_later_phase_goes_stale() {
        let harness = umbrella_findings("umbrella-stale");
        let (code, out, _) = harness.run(&["fix", "--output=text"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("Running format analysis for //..."), "{out}");
        assert!(out.contains("Running lint analysis for //..."), "{out}");
        assert!(!out.contains("Running typecheck analysis"), "{out}");
        assert!(!out.contains("Running generate"), "{out}");
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"y = 1\n",
            "format applied before lint went stale"
        );
        let launches = harness.seen_env.borrow();
        assert_eq!(launches.len(), 2, "typecheck and generate never launch");
        assert!(
            launches
                .iter()
                .flatten()
                .all(|(key, _)| key != GENERATE_ENV_INTENDED),
            "generate dispatch never ran"
        );
    }

    #[test]
    fn umbrella_stdout_report_is_rejected_pre_exec() {
        let harness = umbrella_clean("umbrella-stdout-report");
        let (code, _, err) = harness.run(&["check", "--report=sarif=-", "--output=text"]);
        assert_eq!(code, 2, "{err}");
        assert!(err.contains("share one stdout document"), "{err}");
        assert!(harness.seen_env.borrow().is_empty(), "no phase launched");
    }

    #[test]
    fn umbrella_unknown_report_is_rejected_pre_exec() {
        let harness = umbrella_clean("umbrella-unknown-report");
        let (code, _, err) = harness.run(&["check", "--report=junit=out.xml"]);
        assert_eq!(code, 2, "{err}");
        assert!(err.contains("unsupported report format"), "{err}");
        assert!(harness.seen_env.borrow().is_empty(), "no phase launched");
    }

    /// Only lint and typecheck carry SARIF in their registries:
    /// format and generate contribute no runs to the merged
    /// document, which concatenates one run per executed
    /// SARIF-capable phase in phase order.
    #[test]
    fn umbrella_sarif_merges_executed_phases_in_order() {
        let harness = umbrella_clean("umbrella-sarif");
        let (code, out, err) = harness.run(&["check", "--output=json", "--report=sarif=out.sarif"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("\"format\":\"sarif\""), "{out}");
        let document: serde_json::Value = serde_json::from_slice(
            &std::fs::read(harness.workspace.join("out.sarif")).expect("sarif"),
        )
        .expect("SARIF JSON");
        assert_eq!(
            document["runs"].as_array().expect("runs").len(),
            2,
            "lint and typecheck each contribute one run"
        );
        assert_eq!(
            document["runs"][0], document["runs"][1],
            "both phases replay the shared clean fixture"
        );

        let (code, out, _) = harness.run(&["check", "--output=text", "--report=sarif=out.sarif"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("Wrote sarif report to out.sarif."), "{out}");
    }

    #[test]
    fn umbrella_sarif_covers_only_executed_phases() {
        // The format phase fails before any SARIF-capable phase
        // runs, so the merged document is a valid empty run list.
        let harness = umbrella_findings("umbrella-sarif-stop");
        let (code, _, _) = harness.run(&["check", "--output=json", "--report=sarif=out.sarif"]);
        assert_eq!(code, 1);
        let document: serde_json::Value = serde_json::from_slice(
            &std::fs::read(harness.workspace.join("out.sarif")).expect("sarif"),
        )
        .expect("SARIF JSON");
        assert_eq!(
            document["runs"].as_array().expect("runs").len(),
            0,
            "no executed SARIF-capable phase, no runs"
        );

        let (code, _, err) = harness.run(&["check", "--output=diff", "--report=sarif=out.sarif"]);
        assert_eq!(code, 1);
        assert!(err.contains("Wrote sarif report to out.sarif."), "{err}");
    }

    #[test]
    fn umbrella_report_write_failure_fails() {
        let harness = umbrella_clean("umbrella-report-fail");
        let (code, _, err) = harness.run(&[
            "check",
            "--output=text",
            "--report=sarif=missing-dir/out.sarif",
        ]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("report_failed"), "{err}");

        let (code, out, _) = harness.run(&[
            "check",
            "--output=json",
            "--report=sarif=missing-dir/out.sarif",
        ]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("\"code\":\"report_failed\""), "{out}");
    }

    #[test]
    fn umbrella_failed_phase_and_failed_report_still_fails() {
        let harness = umbrella_findings("umbrella-report-fail-stop");
        let (code, _, err) = harness.run(&[
            "check",
            "--output=text",
            "--report=sarif=missing-dir/out.sarif",
        ]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("report_failed"), "{err}");
    }

    #[test]
    fn fix_json_reports_umbrella_lifecycle() {
        let mut harness = umbrella_clean("umbrella-fix-json");
        harness.intended = Some(intended_witness("default", true, "", ""));
        let (code, out, err) = harness.run(&["fix", "--output=json"]);
        assert_eq!(code, 0, "{out}{err}");
        let events: Vec<serde_json::Value> = out
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        assert_eq!(events.first().expect("first")["command"], "fix");
        assert_eq!(
            events.first().expect("first")["mode"],
            "default",
            "fix mutates without --check"
        );
        assert_eq!(events.last().expect("last")["exit_code"], 0);
    }

    #[test]
    fn umbrella_diff_stops_after_first_phase_patch() {
        let harness = umbrella_findings("umbrella-diff");
        let (code, out, _) = harness.run(&["check", "--output=diff"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("src/a.py"), "{out}");
        assert_eq!(
            harness.seen_env.borrow().len(),
            1,
            "later phases never launch"
        );
    }
}
