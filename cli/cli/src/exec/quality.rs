//! Quality command execution: runs the planned Bazel workflow, collects results, projects reports, and applies stable candidates.
//!
//! Mutation (verified-source collection plus check/incomplete/apply
//! handling) and status projection live in [`super::quality_apply`]
//! (issue #236); diff-patch rendering lives in
//! [`super::quality_patch`]; finding/change/mutation emission lives in
//! [`super::quality_emit`]; standard-report writing lives in
//! [`super::quality_reports`]; this module keeps the dispatch and
//! exit-code selection.

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

/// Runs the quality command to completion and returns the process exit
/// code. All dx-owned bytes go to `out` except operational diagnostics
/// (always `err`); diff mode never emits dx-owned prose or diagnostics
/// on stdout, and a stdout report owns stdout while human text moves
/// to stderr.
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

    // Mutation plus status projection live in `quality_apply` (issue
    // #236): verified-source collection and check/incomplete/apply
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
    // Diff-patch rendering lives in `quality_patch` (issue #236).
    let mut patch = String::new();
    if invocation.output == OutputMode::Diff {
        match render_diff_patch(&sources, &collected.changes) {
            Ok(rendered) => patch = rendered,
            Err(detail) => {
                return operational(invocation, out, err, CODE_DIFF_FAILED, &detail);
            }
        }
    }

    // Human and machine emission of findings, changes, and mutations
    // lives in `quality_emit` (issue #236).
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

    // Standard reports live in `quality_reports` (issue #236): SARIF
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
mod tests {
    use super::super::test_support::*;
    use crate::exec::{execute, Env};
    use dx_digest::blake3 as digest;
    use quality_result::proto;
    use quality_result::proto::FileSnapshot;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;

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
    fn default_mode_applies_without_rerunning_bazel() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // no Bazel rerun after lint/typecheck/format application; terminal
        // pipeline findings plus per-file apply failures determine the
        // current invocation status. Default apply with one fixable finding
        // must launch Bazel exactly once, apply the candidate, and succeed
        // without a second verification build; check mode with pending
        // changes must likewise launch exactly once and fail on the
        // recorded change without re-executing.
        let mut harness = Harness::new("no-rerun-after-apply");
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
        assert_eq!(
            harness.seen_env.borrow().len(),
            1,
            "default apply must launch Bazel exactly once, no post-apply rerun"
        );
        let mut check = Harness::new("no-rerun-after-apply-check");
        check.write_source("src/a.py", "x = 1\n");
        check.results.insert(
            "//test:corpus".to_owned(),
            check.valid_result(
                vec![Harness::diagnostic("unused", true)],
                vec![check.replacement(b"y")],
            ),
        );
        let (check_code, _, _) = check.run(&["lint", "--check", "--output=text"]);
        assert_eq!(check_code, 1);
        assert_eq!(
            std::fs::read(check.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n"
        );
        assert_eq!(
            check.seen_env.borrow().len(),
            1,
            "check mode must launch Bazel exactly once"
        );
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
    #[cfg(unix)]
    fn non_utf8_temp_path_is_operational() {
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
    fn mixed_applied_and_not_applied_fail_together() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // mixed per-file results to emit applied and not_applied together
        // and fail when any path is rejected.
        let mut harness = Harness::new("mixed-apply");
        harness.write_source("src/a.py", "x = 1\n");
        harness.write_source("src/b.py", "a = 1\n");
        let original_a = std::fs::read(harness.workspace.join("src/a.py")).expect("source");
        let original_b = std::fs::read(harness.workspace.join("src/b.py")).expect("source");
        let change_a = harness.replacement_at(
            "src/a.py",
            digest(&original_a).to_vec(),
            vec![proto::Edit {
                start_byte: 0,
                end_byte: 1,
                replacement: b"y".to_vec(),
            }],
        );
        let change_b = harness.replacement_at(
            "src/b.py",
            digest(&original_b).to_vec(),
            vec![proto::Edit {
                start_byte: 0,
                end_byte: 1,
                replacement: b"b".to_vec(),
            }],
        );
        let bytes = harness.result_full(
            vec![
                Harness::diagnostic("unused", true),
                Harness::diagnostic_with(
                    proto::Severity::Warning as i32,
                    "lint-tool",
                    "src/b.py",
                    "unused",
                    true,
                ),
            ],
            vec![],
            vec![change_a, change_b],
            vec![
                FileSnapshot {
                    path: "src/a.py".to_owned(),
                    digest: digest(&original_a).to_vec(),
                },
                FileSnapshot {
                    path: "src/b.py".to_owned(),
                    digest: digest(&original_b).to_vec(),
                },
            ],
        );
        harness.results.insert("//test:corpus".to_owned(), bytes);
        harness.write_source("src/b.py", "z = 2\n");
        let (code, out, err) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 1);
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"y = 1\n"
        );
        assert_eq!(
            std::fs::read(harness.workspace.join("src/b.py")).expect("source"),
            b"z = 2\n"
        );
        assert!(out.contains("Applied 1 file(s)."));
        assert!(err.contains("Not applied: src/b.py (stale_source)"));
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
    fn legacy_staging_dir_does_not_block_atomic_write() {
        let mut harness = Harness::new("staging-blocked");
        harness.write_source("src/a.py", "x = 1\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(
                vec![Harness::diagnostic("unused", true)],
                vec![harness.replacement(b"y")],
            ),
        );
        // Legacy fixed staging path from before the race-free write (#74):
        // `write_atomic` now stages via an OS-random `NamedTempFile`, so a
        // leftover `.dx-apply-tmp` directory must not block the apply.
        std::fs::create_dir_all(harness.workspace.join("src/.a.py.dx-apply-tmp"))
            .expect("staging dir");
        let (code, out, err) = harness.run(&["lint", "--output=text"]);
        assert_eq!(code, 0);
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"y = 1\n"
        );
        assert!(out.contains("Applied 1 file(s)."));
        assert!(!err.contains("Not applied: src/a.py"));
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
}
