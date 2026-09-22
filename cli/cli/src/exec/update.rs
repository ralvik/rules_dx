//! Update command execution: live resolver backends with continuation
//! plus the vendored preset fragment.

use super::common::*;
use crate::args::{Command, Invocation};
use crate::reports::plan_reports;
use dx_output::{
    change_event, command_finished, command_started, error_event, mutation_event, notice_event,
    with_correlation, write_event, ChangeEvent, ChangeKind, Edit, FinishedCounts, MutationOutcome,
    NoticeEvent, OutputMode,
};
use std::collections::BTreeMap;

/// Runs `dx update`  with the preset fragment:
/// default mode updates dependency-set/package/target selectors through
/// `dx_update` (mutating without confirmation) plus the preset fragment
/// atomically; `--check` is the non-mutating preset stale gate (exit `0`
/// clean / `1` stale, copying the `generate --check` exit contract) and
/// ignores selectors. `--dry-run` plans without launching or touching
/// the tree. Usage errors exit `2` before any launch.
pub(crate) fn execute_update(invocation: &Invocation, env: Env<'_>) -> i32 {
    debug_assert!(
        invocation.command == Command::Update,
        "update dispatch guards commands"
    );
    match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(_) => {}
        Err(error) => return pre_exec(env.err, &error.to_string()),
    }
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    // Preset stale gate: non-mutating, preset-only, no backend launches.
    if invocation.check {
        let Env {
            workspace,
            out,
            err,
            ..
        } = env;
        let mode = "check";
        if invocation.dry_run {
            if invocation.output == OutputMode::Json {
                if let Ok(event) = command_started(invocation.command.name(), true, mode) {
                    let _ = write_event(out, &event);
                }
                let finished = command_finished(0, &FinishedCounts::default());
                let _ = write_event(out, &finished);
            } else if verbose {
                let _ = writeln!(out, "Would check preset fragment");
            }
            return 0;
        }
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), false, mode) {
                let _ = write_event(out, &event);
            }
        } else if verbose {
            let _ = writeln!(out, "Running update --check for preset");
        }
        match dx_adopt::check_preset(workspace) {
            Ok(()) => {
                if invocation.output == OutputMode::Json {
                    let finished = command_finished(0, &FinishedCounts::default());
                    let _ = write_event(out, &finished);
                } else if verbose {
                    let _ = writeln!(out, "preset clean");
                }
                0
            }
            Err(dx_adopt::PresetError::Stale { detail }) => {
                if invocation.output == OutputMode::Json {
                    if let Ok(event) =
                        error_event(CODE_UPDATE_FAILED, &detail, None, None, Some("execute"))
                    {
                        let _ = write_event(out, &event);
                    }
                    let finished = command_finished(1, &FinishedCounts::default());
                    let _ = write_event(out, &finished);
                } else {
                    // Check failure (like `generate --check`): report the
                    // diff to stdout, exit 1, no stderr.
                    let _ = writeln!(out, "{detail}");
                }
                1
            }
            Err(error) => {
                // Owned collisions and I/O failures are operational.
                operational(invocation, out, err, CODE_UPDATE_FAILED, &error.to_string())
            }
        }
    } else {
        execute_update_default(invocation, env, verbose)
    }
}

fn execute_update_default(invocation: &Invocation, env: Env<'_>, verbose: bool) -> i32 {
    let Env {
        workspace,
        runner,
        out,
        err,
        ..
    } = env;
    let resolved = match dx_update::selector::resolve(&invocation.targets) {
        Ok(resolved) => resolved,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let summary = display_summary(&resolved);
    if invocation.dry_run {
        return emit_update_dry_run(invocation, out, &summary, verbose);
    }
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), false, "default") {
            let _ = write_event(out, &event);
        }
    } else if verbose {
        let _ = writeln!(out, "{summary}");
    }
    // Preset first, atomically, preserving overrides (fail fast on
    // collisions; independent of dependency backends).
    if let Err(error) = dx_adopt::update_preset(workspace) {
        return operational(invocation, out, err, CODE_UPDATE_FAILED, &error.to_string());
    }
    if verbose && invocation.output != OutputMode::Json {
        let _ = writeln!(out, "updated preset (tools/bazelrc/preset.bazelrc)");
    }
    let (attempted, details) = run_update_backends(&resolved, runner, workspace);
    let selected: Vec<String> = resolved.keys().map(|set| set.name().to_owned()).collect();
    let depends: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let report = match dx_update::outcome::aggregate(&selected, &attempted, &depends) {
        Ok(report) => report,
        Err(error) => {
            return operational(invocation, out, err, CODE_UPDATE_FAILED, &error.to_string());
        }
    };
    let exit = dx_update::report::exit_code(&report);
    if invocation.output == OutputMode::Json {
        emit_update_json(out, err, &report, &details, verbose, exit)
    } else {
        emit_update_text(out, err, &report, &details, verbose, exit)
    }
}

/// Dry-run notice for `execute_update_default`: plans without launching
/// or touching the tree. Extracted so the default path stays under the
/// `too_many_lines` budget.
fn emit_update_dry_run(
    invocation: &Invocation,
    out: &mut dyn std::io::Write,
    summary: &str,
    verbose: bool,
) -> i32 {
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), true, "default") {
            let _ = write_event(out, &event);
        }
        let finished = command_finished(0, &FinishedCounts::default());
        let _ = write_event(out, &finished);
    } else if verbose {
        let _ = writeln!(out, "{summary}");
        let _ = writeln!(out, "Would update preset fragment");
    }
    0
}

/// Records one failed set outcome with its user-facing detail.
fn record_set_failed(
    attempted: &mut Vec<dx_update::outcome::SetOutcome>,
    details: &mut BTreeMap<dx_update::sets::SetId, SetDetail>,
    set: dx_update::sets::SetId,
    message: String,
) {
    attempted.push(dx_update::outcome::SetOutcome {
        set: set.name().to_owned(),
        status: dx_update::outcome::SetStatus::Failed,
    });
    details.insert(set, SetDetail::Failed { message });
}

/// Records one successful set outcome with its user-facing detail.
fn record_set_success(
    attempted: &mut Vec<dx_update::outcome::SetOutcome>,
    details: &mut BTreeMap<dx_update::sets::SetId, SetDetail>,
    set: dx_update::sets::SetId,
    message: String,
) {
    attempted.push(dx_update::outcome::SetOutcome {
        set: set.name().to_owned(),
        status: dx_update::outcome::SetStatus::Success,
    });
    details.insert(set, SetDetail::Success { message });
}

/// Live backend execution over the resolved sets in sorted order.
/// Extracted from `execute_update_default` so the orchestrator stays
/// under the `too_many_lines` budget; continuing independent sets after
/// failures preserves the V1 independence contract.
fn run_update_backends(
    resolved: &BTreeMap<dx_update::sets::SetId, dx_update::selector::SetRequest>,
    runner: &dyn dx_process::Runner,
    workspace: &std::path::Path,
) -> (
    Vec<dx_update::outcome::SetOutcome>,
    BTreeMap<dx_update::sets::SetId, SetDetail>,
) {
    // Live: run backends in sorted set order, continuing independent sets
    // after failures. V1 sets are independent (distinct locks), so the
    // depends-on relation is empty; `aggregate` still derives `Blocked`
    // for any future dependent that lacks a result.
    let mut attempted: Vec<dx_update::outcome::SetOutcome> = Vec::new();
    let mut details: BTreeMap<dx_update::sets::SetId, SetDetail> = BTreeMap::new();
    for (set, request) in resolved {
        let plan = match dx_update::backend::plan(*set, request) {
            Ok(plan) => plan,
            Err(error) => {
                let reason = match error {
                    dx_update::backend::BackendError::Unsupported { reason, .. } => reason,
                };
                let detail = format!("unsupported update: {reason}");
                record_set_failed(
                    &mut attempted,
                    &mut details,
                    *set,
                    format!("failed to update {}: {detail}", set.name()),
                );
                continue;
            }
        };
        match plan {
            dx_update::backend::BackendPlan::Noop => {
                record_set_success(
                    &mut attempted,
                    &mut details,
                    *set,
                    success_line(*set, request),
                );
            }
            dx_update::backend::BackendPlan::Run { argv, env: extra } => {
                run_update_backend(BackendRun {
                    attempted: &mut attempted,
                    details: &mut details,
                    set: *set,
                    request,
                    runner,
                    workspace,
                    argv: &argv,
                    extra: &extra,
                });
            }
        }
    }
    (attempted, details)
}

/// Runs one `Run` backend plan, recording success or the launch/exit/
/// signal failure. Extracted so `run_update_backends` stays focused on
/// planning and dispatch. Grouped as one params struct so the 8-value
/// backend run takes one argument instead of eight positionals.
struct BackendRun<'a> {
    attempted: &'a mut Vec<dx_update::outcome::SetOutcome>,
    details: &'a mut BTreeMap<dx_update::sets::SetId, SetDetail>,
    set: dx_update::sets::SetId,
    request: &'a dx_update::selector::SetRequest,
    runner: &'a dyn dx_process::Runner,
    workspace: &'a std::path::Path,
    argv: &'a [String],
    extra: &'a [(String, String)],
}

fn run_update_backend(run: BackendRun<'_>) {
    let BackendRun {
        attempted,
        details,
        set,
        request,
        runner,
        workspace,
        argv,
        extra,
    } = run;
    let env_refs: Vec<(&str, &str)> = extra
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    match runner.run(argv, workspace, &env_refs) {
        Err(error) => {
            record_set_failed(
                attempted,
                details,
                set,
                format!(
                    "failed to update {}: failed to launch updater: {error}",
                    set.name()
                ),
            );
        }
        Ok(status) => match status.code {
            Some(0) => {
                record_set_success(attempted, details, set, success_line(set, request));
            }
            Some(code) => {
                record_set_failed(
                    attempted,
                    details,
                    set,
                    format!("failed to update {}: updater exited {code}", set.name()),
                );
            }
            None => {
                record_set_failed(
                    attempted,
                    details,
                    set,
                    format!(
                        "failed to update {}: updater terminated by signal",
                        set.name()
                    ),
                );
            }
        },
    }
}

/// JSON report emission for `execute_update_default`: per-set terminal
/// reports plus recovery hint plus `command_finished`. Extracted so the
/// orchestrator stays under the `too_many_lines` budget.
/// See: `docs/cli/output-protocol.md#ndjson-envelope`.
fn emit_update_json(
    out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
    report: &dx_update::outcome::UpdateReport,
    details: &BTreeMap<dx_update::sets::SetId, SetDetail>,
    verbose: bool,
    exit: i32,
) -> i32 {
    // Minor-1.1 `correlation` groups each per-set terminal report plus
    // its file events under `update:<set>`; line order stays
    // authoritative and v1.0 consumers ignore the field.
    // See: `docs/cli/output-protocol.md#ndjson-envelope`.
    for outcome in &report.outcomes {
        let set_name = outcome.set.as_str();
        let correlation = format!("update:{set_name}");
        match outcome.status {
            dx_update::outcome::ReportedStatus::Success => {
                let message = details
                    .get(&parse_set(set_name))
                    .and_then(|detail| match detail {
                        SetDetail::Success { message } => Some(message.clone()),
                        SetDetail::Failed { .. } => None,
                    })
                    .unwrap_or_else(|| format!("updated {set_name}"));
                if let Ok(event) = notice_event(&NoticeEvent {
                    level: "info".to_owned(),
                    code: "update_set_success".to_owned(),
                    message,
                    related_command: Some("update".to_owned()),
                    scope: Some(vec![set_name.to_owned()]),
                    path: None,
                    language: None,
                    import: None,
                }) {
                    let event = with_correlation(event.clone(), &correlation).unwrap_or(event);
                    // Validated backend manifests project to
                    // `change`/`mutation` pairs grouped under the same
                    // correlation before the per-set terminal report.
                    // Live backends currently supply manifest bytes only
                    // for the Go no-op (empty, so no events); other sets
                    // supply none yet because Git scan/BUILD parse/rerun
                    // inference stays rejected. Synthetic manifests are
                    // pinned by unit fixtures plus
                    // `cli/update/tests/fixtures/correlation_manifest/`.
                    // See: `docs/cli/output-protocol.md#mutation`.
                    if let Some(manifest) = live_success_manifest(set_name) {
                        if let Ok(file_events) = project_manifest_events(&manifest) {
                            for file_event in &file_events {
                                let _ = write_event(out, file_event);
                            }
                        }
                    }
                    let _ = write_event(out, &event);
                }
            }
            dx_update::outcome::ReportedStatus::Failed => {
                let message = details
                    .get(&parse_set(set_name))
                    .and_then(|detail| match detail {
                        SetDetail::Failed { message } => Some(message.clone()),
                        SetDetail::Success { .. } => None,
                    })
                    .unwrap_or_else(|| format!("failed to update {set_name}"));
                if let Ok(event) =
                    error_event(CODE_UPDATE_FAILED, &message, None, None, Some("execute"))
                {
                    let event = with_correlation(event.clone(), &correlation).unwrap_or(event);
                    let _ = write_event(out, &event);
                }
                let _ = writeln!(err, "dx: {CODE_UPDATE_FAILED}: {message}");
            }
            dx_update::outcome::ReportedStatus::Blocked => {
                let message = format!("blocked {set_name} (depends on a failed update)");
                if let Ok(event) = notice_event(&NoticeEvent {
                    level: "warning".to_owned(),
                    code: "update_set_blocked".to_owned(),
                    message: message.clone(),
                    related_command: Some("update".to_owned()),
                    scope: Some(vec![set_name.to_owned()]),
                    path: None,
                    language: None,
                    import: None,
                }) {
                    let event = with_correlation(event.clone(), &correlation).unwrap_or(event);
                    let _ = write_event(out, &event);
                }
                if verbose {
                    let _ = writeln!(out, "{message}");
                }
            }
        }
    }
    // Recovery hint (update_recovery): per-set commits are kept (no automatic rollback);
    // print the idempotent retry plus manual restore so a partial run
    // never reads as silent success.
    if let Some(plan) = dx_update::recovery::plan(report) {
        if let Ok(event) = notice_event(&NoticeEvent {
            level: "warning".to_owned(),
            code: dx_update::recovery::RECOVERY_CODE.to_owned(),
            message: plan.message.clone(),
            related_command: Some("update".to_owned()),
            scope: Some(plan.retry_sets.clone()),
            path: None,
            language: None,
            import: None,
        }) {
            let _ = write_event(out, &event);
        }
        let _ = writeln!(
            err,
            "dx: {}: {}",
            dx_update::recovery::RECOVERY_CODE,
            plan.message
        );
    }
    let finished = command_finished(
        exit,
        &FinishedCounts {
            results_complete: Some(true),
            ..FinishedCounts::default()
        },
    );
    let _ = write_event(out, &finished);
    exit
}

/// Text report emission for `execute_update_default`: successes to stdout
/// when verbose, failures always to stderr, plus the recovery hint and
/// the aggregate summary. Extracted so the orchestrator stays under the
/// `too_many_lines` budget.
fn emit_update_text(
    out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
    report: &dx_update::outcome::UpdateReport,
    details: &BTreeMap<dx_update::sets::SetId, SetDetail>,
    verbose: bool,
    exit: i32,
) -> i32 {
    // Text mode: successes to stdout when verbose, failures always to
    // stderr, plus a final aggregate summary when verbose.
    for outcome in &report.outcomes {
        match outcome.status {
            dx_update::outcome::ReportedStatus::Success => {
                if verbose {
                    if let Some(SetDetail::Success { message }) =
                        details.get(&parse_set(outcome.set.as_str()))
                    {
                        let _ = writeln!(out, "{message}");
                    }
                }
            }
            dx_update::outcome::ReportedStatus::Failed => {
                if let Some(SetDetail::Failed { message }) =
                    details.get(&parse_set(outcome.set.as_str()))
                {
                    let _ = writeln!(err, "dx: {CODE_UPDATE_FAILED}: {message}");
                }
            }
            dx_update::outcome::ReportedStatus::Blocked => {
                if verbose {
                    let _ = writeln!(out, "blocked {} (depends on a failed update)", outcome.set);
                }
            }
        }
    }
    // Text recovery hint (update_recovery) on failure: always to stderr (never silent
    // partial success), even when the per-set summary below is quiet.
    if let Some(plan) = dx_update::recovery::plan(report) {
        let _ = writeln!(
            err,
            "dx: {}: {}",
            dx_update::recovery::RECOVERY_CODE,
            plan.message
        );
    }
    if verbose {
        let succeeded = report
            .outcomes
            .iter()
            .filter(|outcome| outcome.status == dx_update::outcome::ReportedStatus::Success)
            .count();
        let failed = report
            .outcomes
            .iter()
            .filter(|outcome| outcome.status == dx_update::outcome::ReportedStatus::Failed)
            .count();
        let blocked = report
            .outcomes
            .iter()
            .filter(|outcome| outcome.status == dx_update::outcome::ReportedStatus::Blocked)
            .count();
        let _ = writeln!(
            out,
            "dx update: {succeeded} succeeded, {failed} failed, {blocked} blocked"
        );
    }
    exit
}

enum SetDetail {
    Success { message: String },
    Failed { message: String },
}

/// Live manifest bytes for one successful set: the Go pinned no-op owns
/// an empty manifest (no file delta, so no events); other backends own
/// no CLI-readable manifest yet because the protocol forbids Git scan,
/// BUILD parse, and rerun inference. Returns `None` when no manifest is
/// available so the CLI emits no forged `change`/`mutation` events.
/// See: `docs/cli/output-protocol.md#mutation`.
fn live_success_manifest(set_name: &str) -> Option<dx_update::manifest::CommittedManifest> {
    if set_name == "go" {
        Some(dx_update::manifest::CommittedManifest {
            set: "go".to_owned(),
            changes: Vec::new(),
        })
    } else {
        None
    }
}

/// Projects one validated backend manifest to its NDJSON `change` plus
/// terminal `applied` `mutation` pairs, grouped under `update:<set>`.
/// Each file emits its `change` first, then its `applied` mutation, in
/// normalized path order; empty manifests emit nothing, preserving v1.0.
/// Returns the ordered event values (callers stream them before the
/// per-set terminal report). Fails closed on any manifest shape violation.
/// See: `docs/cli/output-protocol.md#mutation`.
pub(crate) fn project_manifest_events(
    manifest: &dx_update::manifest::CommittedManifest,
) -> Result<Vec<serde_json::Value>, dx_update::manifest::ManifestError> {
    let projected = dx_update::manifest::project(manifest)?;
    let correlation = format!("update:{}", manifest.set);
    let mut events = Vec::with_capacity(projected.len() * 2);
    for file in &projected {
        let kind = match file.kind {
            dx_update::manifest::CommittedKind::Modify => ChangeKind::Modify,
            dx_update::manifest::CommittedKind::Create => ChangeKind::Create,
        };
        let change = ChangeEvent {
            path: file.path.clone(),
            kind,
            source_digest: file.source_digest.clone(),
            edits: vec![Edit {
                start: file.start_byte,
                end: file.end_byte,
                replacement: file.replacement.clone(),
            }],
        };
        let change =
            change_event(&change).map_err(|_| dx_update::manifest::ManifestError::BadContent {
                path: file.path.clone(),
            })?;
        let change = with_correlation(change, &correlation).map_err(|_| {
            dx_update::manifest::ManifestError::BadContent {
                path: file.path.clone(),
            }
        })?;
        events.push(change);
        let mutation =
            mutation_event(&file.path, kind, MutationOutcome::Applied, None).map_err(|_| {
                dx_update::manifest::ManifestError::BadContent {
                    path: file.path.clone(),
                }
            })?;
        let mutation = with_correlation(mutation, &correlation).map_err(|_| {
            dx_update::manifest::ManifestError::BadContent {
                path: file.path.clone(),
            }
        })?;
        events.push(mutation);
    }
    Ok(events)
}

fn parse_set(name: &str) -> dx_update::sets::SetId {
    // Aggregate outcomes use canonical set names produced above; fall back
    // to Cargo only to keep reporting total (unreachable in practice).
    dx_update::sets::SetId::parse(name).unwrap_or(dx_update::sets::SetId::Cargo)
}

fn display_summary(
    resolved: &BTreeMap<dx_update::sets::SetId, dx_update::selector::SetRequest>,
) -> String {
    let all_full = resolved.len() == dx_update::sets::SetId::ALL.len()
        && resolved
            .values()
            .all(|request| *request == dx_update::selector::SetRequest::Full);
    if all_full {
        return "Running update for all dependency sets".to_owned();
    }
    let mut parts = Vec::new();
    for (set, request) in resolved {
        match request {
            dx_update::selector::SetRequest::Full => parts.push(set.name().to_owned()),
            dx_update::selector::SetRequest::Packages(packages) => {
                for package in packages {
                    parts.push(format!("{}:{package}", set.name()));
                }
            }
        }
    }
    format!("Running update for {}", parts.join(", "))
}

fn success_line(set: dx_update::sets::SetId, request: &dx_update::selector::SetRequest) -> String {
    match request {
        dx_update::selector::SetRequest::Full => match set {
            dx_update::sets::SetId::Cargo => {
                "updated cargo (rust/tests/fixtures/hello/Cargo.lock, cargo-bazel-lock.json)"
                    .to_owned()
            }
            dx_update::sets::SetId::Npm => "updated npm (pnpm-lock.yaml)".to_owned(),
            dx_update::sets::SetId::Maven => {
                "updated maven (third_party/jvm/maven_install.json)".to_owned()
            }
            dx_update::sets::SetId::NuGet => {
                "updated nuget (third_party/dotnet/paket.lock)".to_owned()
            }
            dx_update::sets::SetId::Go => {
                "updated go (pinned module lock; no-op success)".to_owned()
            }
        },
        dx_update::selector::SetRequest::Packages(packages) => {
            let locks = set.locks().join(", ");
            let selections: Vec<String> = packages
                .iter()
                .map(|package| format!("{}:{package}", set.name()))
                .collect();
            if locks.is_empty() {
                format!("updated {} (nothing to update)", selections.join(", "))
            } else {
                format!("updated {} ({locks})", selections.join(", "))
            }
        }
    }
}

#[cfg(test)]
#[path = "update_tests_a.rs"]
mod update_tests_a;
#[cfg(test)]
#[path = "update_tests_b.rs"]
mod update_tests_b;
