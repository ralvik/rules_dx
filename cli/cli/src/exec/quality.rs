use super::common::*;
use super::quality_apply::{apply_collected_changes, project_status};
use super::quality_emit::{emit_findings, EmitInputs};
use super::quality_patch::render_diff_patch;
use super::quality_reports::{write_standard_reports, StandardReports};
use super::results::collect_results;
use crate::args::Invocation;
use crate::plan::{bep_path, plan_build};
use crate::reports::{plan_reports, Destination};
use crate::resolve::resolve;
use dx_output::{
    command_finished, command_started, write_event, FinishedCounts, OutputMode, Severity,
};

pub(crate) fn execute_quality(invocation: &Invocation, env: Env<'_>) -> i32 {
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
            .map_err(|error| format!("{error}"))
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
    let mut collected = match collect_results(&bep, workspace) {
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

    // Mutation plus status projection live in `quality_apply` (issue
    // verified-source collection and check/incomplete/apply
    // handling, then check-mode vs default-mode status with the
    // fail-closed `failed` flag.
    let applied_outcome = apply_collected_changes(
        workspace,
        invocation.check,
        collected.complete,
        &collected.changes,
    );
    let sources = applied_outcome.sources;
    let applied = applied_outcome.applied;
    let not_applied = applied_outcome.not_applied;
    let (status, failed) = project_status(
        invocation.check,
        &collected.initial,
        &collected.terminal,
        &applied,
        invocation.fail_on,
        !collected.changes.is_empty(),
    );

    // Projection: text lines, unified patch, or NDJSON events.
    // Diff-patch rendering lives in `quality_patch`.
    let mut patch = String::new();
    if invocation.output == OutputMode::Diff {
        match render_diff_patch(&sources, &collected.changes) {
            Ok(rendered) => patch = rendered,
            Err(error) => {
                return operational(invocation, out, err, CODE_DIFF_FAILED, &error.to_string());
            }
        }
    }

    // Human and machine emission of findings, changes, and mutations
    // lives in `quality_emit`.
    let emit_counts = match emit_findings(
        EmitInputs {
            invocation,
            status: &status,
            changes: &collected.changes,
            applied: &applied,
            not_applied: &not_applied,
            patch: &patch,
            stdout_report,
        },
        out,
        err,
    ) {
        Ok(counts) => counts,
        Err((code, message)) => return operational(invocation, out, err, code, &message),
    };
    let applied_count = emit_counts.applied_count;
    let not_applied_count = emit_counts.not_applied_count;
    let change_count = collected.changes.len() as u64;

    // Standard reports live in `quality_reports`: SARIF
    // over current findings with snapshot line regions, written
    // atomically after validation.
    let reports_ok = write_standard_reports(
        StandardReports {
            workspace,
            collected: &collected,
            status: &status,
            planned: &planned_reports,
            output: &invocation.output,
            stdout_report,
        },
        out,
        err,
    );

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

#[cfg(test)]
#[path = "quality_tests_a.rs"]
mod quality_tests_a;
#[cfg(test)]
#[path = "quality_tests_b.rs"]
mod quality_tests_b;
#[cfg(test)]
#[path = "quality_tests_c.rs"]
mod quality_tests_c;
